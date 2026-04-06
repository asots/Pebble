use crate::state::AppState;
use pebble_core::{Message, MessageSummary, PebbleError};
use tauri::State;

#[tauri::command]
pub async fn list_messages(
    state: State<'_, AppState>,
    folder_id: String,
    folder_ids: Option<Vec<String>>,
    limit: u32,
    offset: u32,
) -> std::result::Result<Vec<MessageSummary>, PebbleError> {
    match folder_ids {
        Some(ids) if !ids.is_empty() => state.store.list_messages_by_folders(&ids, limit, offset),
        _ => state.store.list_messages_by_folder(&folder_id, limit, offset),
    }
}

#[tauri::command]
pub async fn list_starred_messages(
    state: State<'_, AppState>,
    account_id: String,
    limit: u32,
    offset: u32,
) -> std::result::Result<Vec<MessageSummary>, PebbleError> {
    state.store.list_starred_messages(&account_id, limit, offset)
}

#[tauri::command]
pub async fn get_message(
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<Option<Message>, PebbleError> {
    state.store.get_message(&message_id)
}

#[tauri::command]
pub async fn get_messages_batch(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
) -> std::result::Result<Vec<Message>, PebbleError> {
    state.store.get_messages_batch(&message_ids)
}
