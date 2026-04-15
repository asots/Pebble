use crate::events;
use crate::state::AppState;
use pebble_core::{PebbleError, SnoozedMessage, now_timestamp};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn snooze_message(
    state: State<'_, AppState>,
    message_id: String,
    until: i64,
    return_to: String,
) -> std::result::Result<(), PebbleError> {
    let snooze = SnoozedMessage {
        message_id,
        snoozed_at: now_timestamp(),
        unsnoozed_at: until,
        return_to,
    };
    state.store.snooze_message(&snooze)
}

#[tauri::command]
pub async fn unsnooze_message(
    app: AppHandle,
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<(), PebbleError> {
    // Look up return_to before deleting so we can emit it in the event.
    let return_to = state
        .store
        .get_snoozed_message(&message_id)?
        .map(|s| s.return_to);
    state.store.unsnooze_message(&message_id)?;
    let _ = app.emit(
        events::MAIL_UNSNOOZED,
        serde_json::json!({
            "message_id": message_id,
            "return_to": return_to,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn list_snoozed(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<SnoozedMessage>, PebbleError> {
    state.store.list_snoozed_messages()
}
