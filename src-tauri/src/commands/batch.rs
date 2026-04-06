use crate::state::AppState;
use pebble_core::PebbleError;
use std::collections::HashMap;
use tauri::State;
use tracing::{info, warn};

#[tauri::command]
pub async fn batch_archive(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
) -> std::result::Result<u32, PebbleError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    // Load all messages in one query and cache archive folders per account
    let mut archive_folders: HashMap<String, Option<String>> = HashMap::new();
    let mut success_count: u32 = 0;
    let mut archived_ids = Vec::new();

    for message_id in &message_ids {
        let msg = match state.store.get_message(message_id) {
            Ok(Some(m)) => m,
            Ok(None) => {
                warn!("Message not found for archive: {message_id}");
                continue;
            }
            Err(e) => {
                warn!("Failed to get message {message_id}: {e}");
                continue;
            }
        };

        // Cache the archive folder lookup per account
        let archive_folder_id = archive_folders
            .entry(msg.account_id.clone())
            .or_insert_with(|| {
                state
                    .store
                    .list_folders(&msg.account_id)
                    .ok()
                    .and_then(|folders| {
                        folders
                            .iter()
                            .find(|f| f.role == Some(pebble_core::FolderRole::Archive))
                            .map(|f| f.id.clone())
                    })
            });

        let result = match archive_folder_id {
            Some(folder_id) => state.store.move_message_to_folder(message_id, folder_id),
            None => state.store.soft_delete_message(message_id),
        };

        match result {
            Ok(()) => {
                success_count += 1;
                archived_ids.push(message_id.clone());
            }
            Err(e) => warn!("Failed to archive message {message_id}: {e}"),
        }
    }

    // Update search index for archived messages
    for id in &archived_ids {
        if let Err(e) = state.search.remove_message(id) {
            warn!("Failed to remove archived message {id} from search index: {e}");
        }
    }
    if !archived_ids.is_empty() {
        if let Err(e) = state.search.commit() {
            warn!("Failed to commit search index after batch archive: {e}");
        }
    }

    info!(
        "Batch archive: {}/{} messages archived",
        success_count,
        message_ids.len()
    );
    Ok(success_count)
}

#[tauri::command]
pub async fn batch_delete(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
) -> std::result::Result<u32, PebbleError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    // Use bulk method instead of N individual updates
    state.store.bulk_soft_delete(&message_ids)?;
    let success_count = message_ids.len() as u32;

    // Update search index — remove deleted messages
    for id in &message_ids {
        if let Err(e) = state.search.remove_message(id) {
            warn!("Failed to remove deleted message {id} from search index: {e}");
        }
    }
    if let Err(e) = state.search.commit() {
        warn!("Failed to commit search index after batch delete: {e}");
    }

    info!(
        "Batch delete: {}/{} messages deleted",
        success_count,
        message_ids.len()
    );
    Ok(success_count)
}

#[tauri::command]
pub async fn batch_mark_read(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
    is_read: bool,
) -> std::result::Result<u32, PebbleError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    // Use bulk method instead of N individual updates
    let changes: Vec<(String, Option<bool>, Option<bool>)> = message_ids
        .iter()
        .map(|id| (id.clone(), Some(is_read), None))
        .collect();
    state.store.bulk_update_flags(&changes)?;
    let success_count = message_ids.len() as u32;

    info!(
        "Batch mark_read({}): {}/{} messages updated",
        is_read,
        success_count,
        message_ids.len()
    );
    Ok(success_count)
}
