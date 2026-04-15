use crate::state::AppState;
use pebble_core::traits::LabelProvider;
use pebble_core::{Message, PebbleError, ProviderType};
use pebble_mail::{ImapConfig, ImapProvider};
use tauri::State;
use tracing::warn;

use super::{connect_gmail, connect_outlook, load_imap_config};

/// Data resolved from the local DB that the async writeback branches need.
enum WritebackInfo {
    Gmail {
        msg: Message,
    },
    Outlook {
        msg: Message,
    },
    Imap {
        msg: Message,
        folder_remote_id: String,
        imap_config: ImapConfig,
    },
    None,
}

#[tauri::command]
pub async fn update_message_flags(
    state: State<'_, AppState>,
    message_id: String,
    is_read: Option<bool>,
    is_starred: Option<bool>,
) -> std::result::Result<(), PebbleError> {
    // 1. Local DB work — offloaded to the blocking pool so it doesn't stall
    //    the Tokio runtime.
    let store = state.store.clone();
    let crypto = state.crypto.clone();
    let msg_id = message_id.clone();

    let writeback_info = tokio::task::spawn_blocking(move || -> std::result::Result<WritebackInfo, PebbleError> {
        // 1a. Persist the flag change.
        store.update_message_flags(&msg_id, is_read, is_starred)?;

        // 1b. Fetch the message we just updated.
        let msg = match store.get_message(&msg_id)? {
            Some(m) => m,
            None => return Ok(WritebackInfo::None),
        };

        // 1c. Resolve the provider type.
        let provider_type = store
            .get_account(&msg.account_id)?
            .map(|account| account.provider);

        match provider_type {
            Some(ProviderType::Gmail) => Ok(WritebackInfo::Gmail { msg }),
            Some(ProviderType::Outlook) => Ok(WritebackInfo::Outlook { msg }),
            Some(ProviderType::Imap) | None => {
                // For IMAP we also need the folder's remote_id and the IMAP
                // config — both require store / crypto access, so resolve them
                // here inside the blocking task.
                let folder_ids = store.get_message_folder_ids(&msg_id)?;
                let folders = store.list_folders(&msg.account_id)?;
                let folder = folder_ids
                    .iter()
                    .find_map(|fid| folders.iter().find(|f| &f.id == fid))
                    .cloned();

                // Missing config here means the account has no IMAP writeback
                // target — degrade gracefully to `None` rather than failing the
                // whole flag update, which is local-only-valid in that case.
                let imap_config: Option<ImapConfig> =
                    load_imap_config(&store, &crypto, &msg.account_id).ok();

                match (folder, imap_config) {
                    (Some(f), Some(cfg)) => Ok(WritebackInfo::Imap {
                        msg,
                        folder_remote_id: f.remote_id.clone(),
                        imap_config: cfg,
                    }),
                    _ => Ok(WritebackInfo::None),
                }
            }
        }
    })
    .await
    .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))??;

    // 2. Provider-specific remote writeback (fire-and-forget, async — must
    //    stay OUTSIDE spawn_blocking).
    match writeback_info {
        WritebackInfo::Gmail { msg } => {
            let mut add = Vec::new();
            let mut remove = Vec::new();
            if let Some(read) = is_read {
                if read {
                    remove.push("UNREAD".to_string());
                } else {
                    add.push("UNREAD".to_string());
                }
            }
            if let Some(starred) = is_starred {
                if starred {
                    add.push("STARRED".to_string());
                } else {
                    remove.push("STARRED".to_string());
                }
            }

            if !add.is_empty() || !remove.is_empty() {
                let remote_id = msg.remote_id.clone();
                if let Ok(provider) = connect_gmail(&state, &msg.account_id).await {
                    tokio::task::spawn(async move {
                        if let Err(e) = provider.modify_labels(&remote_id, &add, &remove).await {
                            warn!("Gmail flag writeback failed: {e}");
                        }
                    });
                }
            }
        }
        WritebackInfo::Outlook { msg } => {
            let remote_id = msg.remote_id.clone();
            if let Ok(provider) = connect_outlook(&state, &msg.account_id).await {
                tokio::task::spawn(async move {
                    if let Some(read) = is_read {
                        if let Err(e) = provider.update_read_status(&remote_id, read).await {
                            warn!("Outlook read status writeback failed: {e}");
                        }
                    }
                    if let Some(starred) = is_starred {
                        if let Err(e) = provider.update_flag_status(&remote_id, starred).await {
                            warn!("Outlook flag writeback failed: {e}");
                        }
                    }
                });
            }
        }
        WritebackInfo::Imap { msg, folder_remote_id, imap_config } => {
            if let Ok(uid) = msg.remote_id.parse::<u32>() {
                tokio::task::spawn(async move {
                    let provider = ImapProvider::new(imap_config);
                    if let Err(e) = provider.connect().await {
                        warn!("IMAP flag writeback connect failed: {e}");
                        return;
                    }
                    if let Err(e) = provider.set_flags(&folder_remote_id, uid, is_read, is_starred).await {
                        warn!("IMAP flag writeback failed: {e}");
                    }
                    let _ = provider.disconnect().await;
                });
            }
        }
        WritebackInfo::None => {}
    }

    Ok(())
}
