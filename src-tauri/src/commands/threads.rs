use pebble_core::{Message, PebbleError, ThreadSummary};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn list_thread_messages(
    state: State<'_, AppState>,
    thread_id: String,
) -> std::result::Result<Vec<Message>, PebbleError> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.list_messages_by_thread(&thread_id))
        .await
        .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))?
}

#[tauri::command]
pub async fn list_threads(
    state: State<'_, AppState>,
    folder_id: String,
    limit: u32,
    offset: u32,
) -> std::result::Result<Vec<ThreadSummary>, PebbleError> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.list_threads_by_folder(&folder_id, limit, offset))
        .await
        .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))?
}
