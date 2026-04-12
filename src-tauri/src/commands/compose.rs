use std::path::PathBuf;

use crate::state::AppState;
use crate::commands::oauth::ensure_account_oauth_tokens;
use pebble_core::traits::{MailTransport, OutgoingMessage};
use pebble_core::{EmailAddress, PebbleError, ProviderType};
use pebble_mail::{GmailProvider, OutlookProvider};
use pebble_mail::smtp::SmtpSender;
use tauri::State;

/// Validate that all attachment paths are within allowed directories.
fn validate_attachment_paths(paths: &[String], attachments_dir: &std::path::Path) -> std::result::Result<Vec<String>, PebbleError> {
    let mut allowed_dirs: Vec<PathBuf> = vec![attachments_dir.to_path_buf()];

    // Add user home subdirectories (Documents, Downloads, Desktop) and temp dir
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let home = PathBuf::from(home);
        for sub in &["Documents", "Downloads", "Desktop"] {
            let dir = home.join(sub);
            if dir.exists() {
                allowed_dirs.push(dir);
            }
        }
    }
    if let Ok(tmp) = std::env::temp_dir().canonicalize() {
        allowed_dirs.push(tmp);
    }

    // Canonicalize allowed dirs for consistent comparison
    let allowed_dirs: Vec<PathBuf> = allowed_dirs
        .into_iter()
        .filter_map(|d| std::fs::canonicalize(&d).ok())
        .collect();

    let mut validated = Vec::with_capacity(paths.len());
    for raw_path in paths {
        let canonical = std::fs::canonicalize(raw_path)
            .map_err(|e| PebbleError::Internal(format!("Attachment path not found: {raw_path} ({e})")))?;

        let is_allowed = allowed_dirs.iter().any(|dir| canonical.starts_with(dir));
        if !is_allowed {
            return Err(PebbleError::Internal(format!(
                "Attachment path is outside allowed directories: {}",
                canonical.display()
            )));
        }
        validated.push(canonical.to_string_lossy().into_owned());
    }
    Ok(validated)
}

fn parse_recipients(addresses: Vec<String>) -> Vec<EmailAddress> {
    addresses
        .into_iter()
        .map(|address| EmailAddress {
            name: None,
            address: address.trim().to_string(),
        })
        .filter(|address| !address.address.is_empty())
        .collect()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn send_email(
    state: State<'_, AppState>,
    account_id: String,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    body_text: String,
    body_html: Option<String>,
    in_reply_to: Option<String>,
    attachment_paths: Option<Vec<String>>,
) -> std::result::Result<(), PebbleError> {
    let raw_paths = attachment_paths.unwrap_or_default();
    let attachment_paths = if raw_paths.is_empty() {
        raw_paths
    } else {
        validate_attachment_paths(&raw_paths, &state.attachments_dir)?
    };
    let account = state
        .store
        .get_account(&account_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Account not found: {account_id}")))?;

    if matches!(account.provider, ProviderType::Gmail | ProviderType::Outlook) {
        let provider_name = match account.provider {
            ProviderType::Gmail => "gmail",
            ProviderType::Outlook => "outlook",
            _ => unreachable!(),
        };
        let tokens = ensure_account_oauth_tokens(&state, &account_id, provider_name).await?;
        let message = OutgoingMessage {
            to: parse_recipients(to),
            cc: parse_recipients(cc),
            bcc: parse_recipients(bcc),
            subject,
            body_text,
            body_html,
            in_reply_to,
            attachment_paths: attachment_paths.clone(),
        };
        return match account.provider {
            ProviderType::Gmail => {
                let provider = GmailProvider::new(tokens.access_token);
                provider.send_message(&message).await
            }
            ProviderType::Outlook => {
                let provider = OutlookProvider::new(tokens.access_token, account_id);
                provider.send_message(&message).await
            }
            _ => unreachable!(),
        };
    }

    // Read SMTP config from encrypted auth_data (where add_account stores it)
    let encrypted = state
        .store
        .get_auth_data(&account_id)?
        .ok_or_else(|| {
            PebbleError::Internal(format!("No auth data found for account {account_id}"))
        })?;
    let decrypted = state.crypto.decrypt(&encrypted)?;
    let config: serde_json::Value = serde_json::from_slice(&decrypted)
        .map_err(|e| PebbleError::Internal(format!("Failed to parse decrypted config: {e}")))?;

    let smtp_config: pebble_mail::SmtpConfig = serde_json::from_value(
        config
            .get("smtp")
            .cloned()
            .ok_or_else(|| PebbleError::Internal("No SMTP config in auth data".to_string()))?,
    )
    .map_err(|e| PebbleError::Internal(format!("Failed to deserialize SMTP config: {e}")))?;

    let sender = SmtpSender::new(
        smtp_config.host,
        smtp_config.port,
        smtp_config.username,
        smtp_config.password,
        smtp_config.security,
        smtp_config.proxy,
    );

    let from_email = account.email.clone();
    sender
        .send(
            &from_email,
            &to,
            &cc,
            &bcc,
            &subject,
            &body_text,
            body_html.as_deref(),
            in_reply_to.as_deref(),
            &attachment_paths,
        )
        .await
}
