use crate::state::AppState;
use pebble_core::PebbleError;
use pebble_core::traits::SearchHit;
use tauri::State;

#[tauri::command]
pub async fn search_messages(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> std::result::Result<Vec<SearchHit>, PebbleError> {
    let limit = limit.unwrap_or(50);
    let search = state.search.clone();
    tokio::task::spawn_blocking(move || search.search(&query, limit))
        .await
        .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))?
}
