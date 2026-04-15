use crate::state::AppState;
use pebble_core::{Folder, FolderRole, FolderType, PebbleError, new_id};
use tauri::State;

#[tauri::command]
pub async fn list_folders(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<Vec<Folder>, PebbleError> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || {
        let folders = store.list_folders(&account_id)?;

        // Ensure a local archive folder exists
        if !folders.iter().any(|f| f.role == Some(FolderRole::Archive)) {
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
