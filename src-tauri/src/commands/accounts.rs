use crate::state::AppState;
use crate::commands::oauth::ensure_account_oauth_tokens;
use pebble_core::{Account, PebbleError, ProviderType, new_id, now_timestamp};
use pebble_mail::{ConnectionSecurity, ImapConfig, ProxyConfig, SmtpConfig};
use pebble_mail::GmailProvider;
use pebble_mail::OutlookProvider;
use pebble_core::traits::FolderProvider;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Typed view of the encrypted `auth_data` blob for an IMAP/SMTP account.
///
/// Prior code patched this blob with hand-written `serde_json::Value`
/// mutations, which silently dropped fields when serde and JSON shapes
/// drifted. Parsing into this struct makes the shape explicit and reuses
/// `ImapConfig` / `SmtpConfig`'s own legacy-aware deserializers.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountCredentials {
    imap: ImapConfig,
    smtp: SmtpConfig,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct AddAccountRequest {
    pub email: String,
    pub display_name: String,
    pub provider: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub imap_security: ConnectionSecurity,
    pub smtp_security: ConnectionSecurity,
    #[serde(default)]
    pub proxy_host: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<u16>,
}

impl std::fmt::Debug for AddAccountRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddAccountRequest")
            .field("email", &self.email)
            .field("provider", &self.provider)
            .field("password", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[tauri::command]
pub async fn add_account(
    state: State<'_, AppState>,
    request: AddAccountRequest,
) -> std::result::Result<Account, PebbleError> {
    let now = now_timestamp();
    let provider = match request.provider.to_lowercase().as_str() {
        "gmail" => ProviderType::Gmail,
        "outlook" => ProviderType::Outlook,
        _ => ProviderType::Imap,
    };

    let account = Account {
        id: new_id(),
        email: request.email.clone(),
        display_name: request.display_name.clone(),
        provider: provider.clone(),
        created_at: now,
        updated_at: now,
    };

    state.store.insert_account(&account)?;

    // Build proxy config if provided
    let proxy = match (request.proxy_host, request.proxy_port) {
        (Some(h), Some(p)) if !h.is_empty() => Some(ProxyConfig { host: h, port: p }),
        _ => None,
    };

    // Build typed IMAP + SMTP credentials
    let credentials = AccountCredentials {
        imap: ImapConfig {
            host: request.imap_host,
            port: request.imap_port,
            username: request.username.clone(),
            password: request.password.clone(),
            security: request.imap_security,
            proxy: proxy.clone(),
        },
        smtp: SmtpConfig {
            host: request.smtp_host,
            port: request.smtp_port,
            username: request.username,
            password: request.password,
            security: request.smtp_security,
            proxy,
        },
    };

    // Encrypt credentials and store as auth_data
    let config_bytes = serde_json::to_vec(&credentials)
        .map_err(|e| PebbleError::Internal(format!("Failed to serialize config: {e}")))?;
    let encrypted = state.crypto.encrypt(&config_bytes)?;
    state.store.set_auth_data(&account.id, &encrypted)?;

    // Store non-secret metadata in sync_state
    let provider_slug = match provider {
        ProviderType::Gmail => "gmail",
        ProviderType::Outlook => "outlook",
        ProviderType::Imap => "imap",
    };
    state.store.update_sync_state(&account.id, |s| {
        s.provider = Some(provider_slug.to_string());
    })?;

    Ok(account)
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, AppState>,
    account_id: String,
    email: String,
    display_name: String,
    password: Option<String>,
    imap_host: Option<String>,
    imap_port: Option<u16>,
    smtp_host: Option<String>,
    smtp_port: Option<u16>,
    imap_security: Option<ConnectionSecurity>,
    smtp_security: Option<ConnectionSecurity>,
    proxy_host: Option<String>,
    proxy_port: Option<u16>,
) -> std::result::Result<(), PebbleError> {
    state.store.update_account(&account_id, &email, &display_name)?;

    let credentials_dirty = password.is_some() || imap_host.is_some() || smtp_host.is_some()
        || imap_port.is_some() || smtp_port.is_some()
        || imap_security.is_some() || smtp_security.is_some()
        || proxy_host.is_some() || proxy_port.is_some();
    if !credentials_dirty {
        return Ok(());
    }

    // Parse the existing encrypted blob into a typed view. If the row is
    // missing (first-time edit, or a legacy OAuth-only account moving to
    // IMAP), seed a blank template that the mutations below can fill in.
    let mut creds: AccountCredentials = match state.store.get_auth_data(&account_id)? {
        Some(encrypted) => {
            let decrypted = state.crypto.decrypt(&encrypted)?;
            serde_json::from_slice(&decrypted)
                .map_err(|e| PebbleError::Internal(format!("Failed to parse config: {e}")))?
        }
        None => AccountCredentials {
            imap: ImapConfig {
                host: String::new(),
                port: 0,
                username: String::new(),
                password: String::new(),
                security: ConnectionSecurity::default(),
                proxy: None,
            },
            smtp: SmtpConfig {
                host: String::new(),
                port: 0,
                username: String::new(),
                password: String::new(),
                security: ConnectionSecurity::default(),
                proxy: None,
            },
        },
    };

    // IMAP side
    if let Some(h) = imap_host { creds.imap.host = h; }
    if let Some(p) = imap_port { creds.imap.port = p; }
    if let Some(ref pw) = password { creds.imap.password = pw.clone(); }
    if let Some(sec) = imap_security { creds.imap.security = sec; }
    if proxy_host.is_some() || proxy_port.is_some() {
        creds.imap.proxy = match (&proxy_host, &proxy_port) {
            (Some(h), Some(p)) if !h.is_empty() => {
                Some(ProxyConfig { host: h.clone(), port: *p })
            }
            _ => None,
        };
    }
    if creds.imap.username.is_empty() {
        creds.imap.username = email.clone();
    }

    // SMTP side
    if let Some(h) = smtp_host { creds.smtp.host = h; }
    if let Some(p) = smtp_port { creds.smtp.port = p; }
    if let Some(ref pw) = password { creds.smtp.password = pw.clone(); }
    if let Some(sec) = smtp_security { creds.smtp.security = sec; }
    // Mirror IMAP proxy to SMTP — both connections share the same network path.
    if proxy_host.is_some() || proxy_port.is_some() {
        creds.smtp.proxy = creds.imap.proxy.clone();
    }
    if creds.smtp.username.is_empty() {
        creds.smtp.username = email.clone();
    }

    let config_bytes = serde_json::to_vec(&creds)
        .map_err(|e| PebbleError::Internal(format!("Failed to serialize config: {e}")))?;
    let encrypted = state.crypto.encrypt(&config_bytes)?;
    state.store.set_auth_data(&account_id, &encrypted)?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_security: ConnectionSecurity,
    #[serde(default)]
    pub proxy_host: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<u16>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[tauri::command]
pub async fn test_imap_connection(
    request: TestConnectionRequest,
) -> std::result::Result<String, PebbleError> {
    let proxy = match (request.proxy_host, request.proxy_port) {
        (Some(h), Some(p)) if !h.is_empty() => Some(pebble_mail::ProxyConfig { host: h, port: p }),
        _ => None,
    };
    let has_credentials = request.username.as_ref().is_some_and(|u| !u.is_empty())
        && request.password.as_ref().is_some_and(|p| !p.is_empty());
    let config = pebble_mail::ImapConfig {
        host: request.imap_host,
        port: request.imap_port,
        username: request.username.unwrap_or_default(),
        password: request.password.unwrap_or_default(),
        security: request.imap_security,
        proxy,
    };
    if has_credentials {
        pebble_mail::ImapProvider::test_connection_with_login(&config).await
    } else {
        pebble_mail::ImapProvider::test_connection(&config).await
    }
}

#[tauri::command]
pub async fn test_account_connection(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<String, PebbleError> {
    let account = state
        .store
        .get_account(&account_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Account not found: {account_id}")))?;

    if matches!(account.provider, ProviderType::Gmail) {
        let tokens = ensure_account_oauth_tokens(&state, &account_id, "gmail").await?;
        let provider = GmailProvider::new(tokens.access_token);
        let (email, _history_id) = provider.get_profile().await?;
        if email.is_empty() {
            return Ok("Gmail connection successful".to_string());
        }
        return Ok(format!("Gmail connection successful ({email})"));
    }

    if matches!(account.provider, ProviderType::Outlook) {
        let tokens = ensure_account_oauth_tokens(&state, &account_id, "outlook").await?;
        let provider = OutlookProvider::new(tokens.access_token, account_id.clone());
        // Graph connectivity check: list mail folders.
        let folders = provider.list_folders().await?;
        return Ok(format!(
            "Outlook connection successful ({} folders)",
            folders.len()
        ));
    }

    let existing = state.store.get_auth_data(&account_id)?
        .ok_or_else(|| PebbleError::Internal("No auth data found".into()))?;
    let decrypted = state.crypto.decrypt(&existing)?;
    let config: serde_json::Value = serde_json::from_slice(&decrypted)
        .map_err(|e| PebbleError::Internal(format!("Failed to parse config: {e}")))?;
    let imap_config: pebble_mail::ImapConfig = serde_json::from_value(
        config.get("imap").cloned().ok_or_else(|| PebbleError::Internal("No IMAP config".into()))?
    ).map_err(|e| PebbleError::Internal(format!("Failed to parse IMAP config: {e}")))?;
    pebble_mail::ImapProvider::test_connection(&imap_config).await
}

#[tauri::command]
pub async fn list_accounts(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<Account>, PebbleError> {
    state.store.list_accounts()
}

#[tauri::command]
pub async fn delete_account(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<(), PebbleError> {
    // 1. Stop sync if running
    {
        let mut handles = state.sync_handles.lock().await;
        if let Some(handle) = handles.remove(&account_id) {
            let _ = handle.stop_tx.send(true);
            handle.task.abort();
        }
    }

    // 2. Collect message IDs for attachment cleanup (before DB delete)
    let message_ids = match state.store.list_message_ids_by_account(&account_id) {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!("Failed to collect message IDs for attachment cleanup (account {account_id}): {e}");
            Vec::new()
        }
    };

    // 3. Remove all documents from search index
    if let Err(e) = state.search.delete_by_account(&account_id) {
        tracing::warn!("Failed to clean search index for account {account_id}: {e}");
    }

    // 4. Delete account from DB (CASCADE handles related rows)
    state.store.delete_account(&account_id)?;

    // 5. Clean up attachment files on disk
    let attachments_dir = state.attachments_dir.clone();
    let account_id_for_log = account_id.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || {
        for msg_id in &message_ids {
            let msg_dir = attachments_dir.join(msg_id);
            if msg_dir.exists() {
                if let Err(e) = std::fs::remove_dir_all(&msg_dir) {
                    tracing::warn!("Failed to remove attachments for message {msg_id}: {e}");
                }
            }
        }
    }).await {
        tracing::warn!("Attachment cleanup task failed for account {account_id_for_log}: {e}");
    }

    Ok(())
}
