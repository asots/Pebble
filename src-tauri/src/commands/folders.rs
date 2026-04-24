use crate::state::AppState;
use pebble_core::{new_id, Folder, FolderRole, FolderType, PebbleError};
use tauri::State;

fn provider_folders_have_arrived(folders: &[Folder]) -> bool {
    folders
        .iter()
        .any(|folder| !folder.remote_id.starts_with("__local_"))
}

fn should_seed_local_archive(folders: &[Folder]) -> bool {
    let has_archive = folders
        .iter()
        .any(|folder| folder.role == Some(FolderRole::Archive));

    provider_folders_have_arrived(folders) && !has_archive
}

#[tauri::command]
pub async fn list_folders(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<Vec<Folder>, PebbleError> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || {
        let folders = store.list_folders(&account_id)?;

        if !provider_folders_have_arrived(&folders) {
            return Ok(Vec::new());
        }

        // Ensure a local archive folder exists after provider folders have arrived.
        // During first OAuth sign-in, folders may still be syncing; returning an
        // empty list lets the sidebar keep its placeholder folders instead of
        // caching a misleading "Archive only" account.
        if should_seed_local_archive(&folders) {
            let archive = Folder {
                id: new_id(),
                account_id: account_id.clone(),
                remote_id: "__local_archive__".to_string(),
                name: "Archive".to_string(),
                folder_type: FolderType::Folder,
                role: Some(FolderRole::Archive),
                parent_id: None,
                color: None,
                is_system: true,
                sort_order: 3,
            };
            let _ = store.insert_folder(&archive);
            return store.list_folders(&account_id);
        }

        Ok(folders)
    })
    .await
    .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(role: FolderRole, remote_id: &str) -> Folder {
        Folder {
            id: new_id(),
            account_id: "account-1".to_string(),
            remote_id: remote_id.to_string(),
            name: remote_id.to_string(),
            folder_type: FolderType::Folder,
            role: Some(role),
            parent_id: None,
            color: None,
            is_system: true,
            sort_order: 0,
        }
    }

    #[test]
    fn archive_seed_waits_until_provider_folders_exist() {
        assert!(!provider_folders_have_arrived(&[]));
        assert!(!provider_folders_have_arrived(&[folder(
            FolderRole::Archive,
            "__local_archive__"
        )]));
        assert!(provider_folders_have_arrived(&[folder(
            FolderRole::Inbox,
            "INBOX"
        )]));

        assert!(!should_seed_local_archive(&[]));
        assert!(!should_seed_local_archive(&[folder(
            FolderRole::Sent,
            "__local_outbox__"
        )]));
        assert!(should_seed_local_archive(&[folder(
            FolderRole::Inbox,
            "INBOX"
        )]));
        assert!(!should_seed_local_archive(&[folder(
            FolderRole::Archive,
            "__local_archive__"
        )]));
    }
}
