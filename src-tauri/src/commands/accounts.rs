use crate::state::AppState;
use crate::commands::oauth::ensure_account_oauth_tokens;
use pebble_core::{Account, PebbleError, ProviderType, new_id, now_timestamp};
use pebble_mail::ConnectionSecurity;
use pebble_mail::GmailProvider;
use serde::Deserialize;
use tauri::State;

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
        (Some(h), Some(p)) if !h.is_empty() => Some(pebble_mail::ProxyConfig { host: h, port: p }),
        _ => None,
    };

    // Build IMAP + SMTP config
    let imap_config = pebble_mail::ImapConfig {
        host: request.imap_host,
        port: request.imap_port,
        username: request.username.clone(),
        password: request.password.clone(),
        security: request.imap_security,
        proxy,
    };
    let smtp_config = pebble_mail::SmtpConfig {
        host: request.smtp_host,
        port: request.smtp_port,
        username: request.username,
        password: request.password,
        security: request.smtp_security,
    };

    // Encrypt credentials and store as auth_data
    let config = serde_json::json!({ "imap": imap_config, "smtp": smtp_config });
    let config_bytes = serde_json::to_vec(&config)
        .map_err(|e| PebbleError::Internal(format!("Failed to serialize config: {e}")))?;
    let encrypted = state.crypto.encrypt(&config_bytes)?;
    state.store.set_auth_data(&account.id, &encrypted)?;

    // Store non-secret metadata in sync_state
    let provider_slug = match provider {
        ProviderType::Gmail => "gmail",
        ProviderType::Outlook => "outlook",
        ProviderType::Imap => "imap",
    };
    let metadata = serde_json::json!({ "provider": provider_slug });
    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| PebbleError::Internal(format!("Failed to serialize metadata: {e}")))?;
    state
        .store
        .update_account_sync_state(&account.id, &metadata_json)?;

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

    // Update credentials if any connection fields changed
    if password.is_some() || imap_host.is_some() || smtp_host.is_some()
        || imap_port.is_some() || smtp_port.is_some()
        || imap_security.is_some() || smtp_security.is_some()
        || proxy_host.is_some() || proxy_port.is_some()
    {
        // Read existing config
        let existing = state.store.get_auth_data(&account_id)?;
        let mut config: serde_json::Value = if let Some(encrypted) = existing {
            let decrypted = state.crypto.decrypt(&encrypted)?;
            serde_json::from_slice(&decrypted)
                .map_err(|e| PebbleError::Internal(format!("Failed to parse config: {e}")))?
        } else {
            serde_json::json!({ "imap": {}, "smtp": {} })
        };

        // Update IMAP fields
        if let Some(imap) = config.get_mut("imap") {
            if let Some(ref h) = imap_host { imap["host"] = serde_json::json!(h); }
            if let Some(p) = imap_port { imap["port"] = serde_json::json!(p); }
            if let Some(ref pw) = password { imap["password"] = serde_json::json!(pw); }
            if let Some(ref sec) = imap_security {
                imap["security"] = serde_json::to_value(sec).unwrap();
                if let Some(obj) = imap.as_object_mut() { obj.remove("use_tls"); }
            }
            // Update proxy
            if proxy_host.is_some() || proxy_port.is_some() {
                match (&proxy_host, &proxy_port) {
                    (Some(h), Some(p)) if !h.is_empty() => {
                        imap["proxy"] = serde_json::json!({"host": h, "port": p});
                    }
                    _ => {
                        if let Some(obj) = imap.as_object_mut() { obj.remove("proxy"); }
                    }
                }
            }
            if imap.get("username").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                imap["username"] = serde_json::json!(email);
            }
        }
        // Update SMTP fields
        if let Some(smtp) = config.get_mut("smtp") {
            if let Some(ref h) = smtp_host { smtp["host"] = serde_json::json!(h); }
            if let Some(p) = smtp_port { smtp["port"] = serde_json::json!(p); }
            if let Some(ref pw) = password { smtp["password"] = serde_json::json!(pw); }
            if let Some(ref sec) = smtp_security {
                smtp["security"] = serde_json::to_value(sec).unwrap();
                if let Some(obj) = smtp.as_object_mut() { obj.remove("use_tls"); }
            }
            if smtp.get("username").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                smtp["username"] = serde_json::json!(email);
            }
        }

        let config_bytes = serde_json::to_vec(&config)
            .map_err(|e| PebbleError::Internal(format!("Failed to serialize config: {e}")))?;
        let encrypted = state.crypto.encrypt(&config_bytes)?;
        state.store.set_auth_data(&account_id, &encrypted)?;
    }

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
    // Stop sync if running
    {
        let mut handles = state.sync_handles.lock().await;
        if let Some(handle) = handles.remove(&account_id) {
            let _ = handle.stop_tx.send(true);
            handle.task.abort();
        }
    }

    state.store.delete_account(&account_id)
}
