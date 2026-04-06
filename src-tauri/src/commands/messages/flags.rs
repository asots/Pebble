use crate::state::AppState;
use pebble_core::traits::LabelProvider;
use pebble_core::{PebbleError, ProviderType};
use pebble_mail::ImapProvider;
use tauri::State;
use tracing::warn;

use super::{connect_gmail, find_message_folder, get_imap_config};

#[tauri::command]
pub async fn update_message_flags(
    state: State<'_, AppState>,
    message_id: String,
    is_read: Option<bool>,
    is_starred: Option<bool>,
) -> std::result::Result<(), PebbleError> {
    // 1. Local update
    state
        .store
        .update_message_flags(&message_id, is_read, is_starred)?;

    // 2. Provider-specific remote writeback (fire-and-forget)
    if let Ok(Some(msg)) = state.store.get_message(&message_id) {
        let provider_type = state
            .store
            .get_account(&msg.account_id)?
            .map(|account| account.provider);

        match provider_type {
            Some(ProviderType::Gmail) => {
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
            Some(ProviderType::Imap) | Some(ProviderType::Outlook) | None => {
                if let Ok(folder) = find_message_folder(&state, &message_id, &msg.account_id) {
                    if let Ok(uid) = msg.remote_id.parse::<u32>() {
                        if let Ok(imap_config) = get_imap_config(&state, &msg.account_id) {
                            let remote_id = folder.remote_id.clone();
                            tokio::task::spawn(async move {
                                let provider = ImapProvider::new(imap_config);
                                if let Err(e) = provider.connect().await {
                                    warn!("IMAP flag writeback connect failed: {e}");
                                    return;
                                }
                                if let Err(e) = provider.set_flags(&remote_id, uid, is_read, is_starred).await {
                                    warn!("IMAP flag writeback failed: {e}");
                                }
                                let _ = provider.disconnect().await;
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
