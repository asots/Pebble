use crate::state::AppState;
use pebble_core::{Message, PebbleError, PrivacyMode, RenderedHtml, TrustType};
use pebble_privacy::PrivacyGuard;
use tauri::State;

#[tauri::command]
pub async fn get_rendered_html(
    state: State<'_, AppState>,
    message_id: String,
    privacy_mode: PrivacyMode,
) -> std::result::Result<RenderedHtml, PebbleError> {
    let message = state
        .store
        .get_message(&message_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Message not found: {message_id}")))?;

    let effective_mode = resolve_privacy_mode(&state, &message, privacy_mode)?;
    let guard = PrivacyGuard::new();
    let rendered = guard.render_safe_html(&message.body_html_raw, &effective_mode);
    Ok(rendered)
}

#[tauri::command]
pub async fn get_message_with_html(
    state: State<'_, AppState>,
    message_id: String,
    privacy_mode: PrivacyMode,
) -> std::result::Result<Option<(Message, RenderedHtml)>, PebbleError> {
    let message = match state.store.get_message(&message_id)? {
        Some(m) => m,
        None => return Ok(None),
    };

    let effective_mode = resolve_privacy_mode(&state, &message, privacy_mode)?;
    let guard = PrivacyGuard::new();
    let rendered = guard.render_safe_html(&message.body_html_raw, &effective_mode);
    Ok(Some((message, rendered)))
}

#[tauri::command]
pub async fn is_trusted_sender(
    state: State<'_, AppState>,
    account_id: String,
    email: String,
) -> std::result::Result<bool, PebbleError> {
    Ok(state.store.is_trusted_sender(&account_id, &email)?.is_some())
}

fn resolve_privacy_mode(
    state: &AppState,
    message: &Message,
    privacy_mode: PrivacyMode,
) -> std::result::Result<PrivacyMode, PebbleError> {
    if matches!(privacy_mode, PrivacyMode::Strict) {
        match state
            .store
            .is_trusted_sender(&message.account_id, &message.from_address)?
        {
            Some(TrustType::All) => Ok(PrivacyMode::TrustSender(message.from_address.clone())),
            Some(TrustType::Images) => Ok(PrivacyMode::LoadOnce),
            None => Ok(privacy_mode),
        }
    } else {
        Ok(privacy_mode)
    }
}
