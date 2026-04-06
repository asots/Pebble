use crate::state::AppState;
use pebble_core::{Account, PebbleError, ProviderType, new_id, now_timestamp};
use pebble_oauth::{OAuthConfig, OAuthManager};
use tauri::State;
use tracing::debug;

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (a, b) in left.as_bytes().iter().zip(right.as_bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// Fetch the user's email and display name from the OAuth provider's userinfo endpoint.
async fn fetch_userinfo(provider: &str, access_token: &str) -> Result<(String, String), PebbleError> {
    let url = match provider.to_lowercase().as_str() {
        "gmail" => "https://www.googleapis.com/oauth2/v2/userinfo",
        "outlook" => "https://graph.microsoft.com/v1.0/me",
        _ => return Err(PebbleError::UnsupportedProvider(provider.to_string())),
    };

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| PebbleError::Network(format!("Userinfo request failed: {e}")))?
        .json()
        .await
        .map_err(|e| PebbleError::Network(format!("Userinfo parse failed: {e}")))?;

    let email = resp["email"].as_str()
        .or_else(|| resp["mail"].as_str())
        .or_else(|| resp["userPrincipalName"].as_str())
        .unwrap_or("")
        .to_string();

    let name = resp["name"].as_str()
        .or_else(|| resp["displayName"].as_str())
        .unwrap_or("")
        .to_string();

    debug!(email = %email, name = %name, "Fetched userinfo from OAuth provider");
    Ok((email, name))
}

/// OAuth config for Gmail (Google).
fn gmail_oauth_config() -> OAuthConfig {
    OAuthConfig {
        client_id: option_env!("GOOGLE_CLIENT_ID")
            .unwrap_or("GOOGLE_CLIENT_ID_PLACEHOLDER")
            .to_string(),
        client_secret: Some(
            option_env!("GOOGLE_CLIENT_SECRET")
                .unwrap_or("GOOGLE_CLIENT_SECRET_PLACEHOLDER")
                .to_string(),
        ),
        auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        token_url: "https://oauth2.googleapis.com/token".to_string(),
        scopes: vec![
            "https://mail.google.com/".to_string(),
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
            "https://www.googleapis.com/auth/userinfo.profile".to_string(),
        ],
        redirect_port: 8756,
    }
}

/// OAuth config for Outlook (Microsoft).
fn outlook_oauth_config() -> OAuthConfig {
    OAuthConfig {
        client_id: std::env::var("MICROSOFT_CLIENT_ID")
            .unwrap_or_else(|_| "MICROSOFT_CLIENT_ID_PLACEHOLDER".to_string()),
        client_secret: std::env::var("MICROSOFT_CLIENT_SECRET").ok(),
        auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
        token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
        scopes: vec![
            "https://graph.microsoft.com/Mail.ReadWrite".to_string(),
            "https://graph.microsoft.com/Mail.Send".to_string(),
            "https://graph.microsoft.com/User.Read".to_string(),
            "offline_access".to_string(),
        ],
        redirect_port: 8757,
    }
}

/// Resolve an `OAuthConfig` from a provider name, or return an error.
fn config_for_provider(provider: &str) -> Result<OAuthConfig, PebbleError> {
    match provider.to_lowercase().as_str() {
        "gmail" => Ok(gmail_oauth_config()),
        "outlook" => Ok(outlook_oauth_config()),
        _ => Err(PebbleError::UnsupportedProvider(format!(
            "Unknown OAuth provider: {provider}"
        ))),
    }
}

/// Resolve a `ProviderType` from a provider name.
fn provider_type(provider: &str) -> Result<ProviderType, PebbleError> {
    match provider.to_lowercase().as_str() {
        "gmail" => Ok(ProviderType::Gmail),
        "outlook" => Ok(ProviderType::Outlook),
        _ => Err(PebbleError::UnsupportedProvider(provider.to_string())),
    }
}

/// Start the OAuth flow for a provider.
///
/// Returns the authorization URL that the frontend should open in the system
/// browser via `shell.open()`.
#[tauri::command]
pub async fn start_oauth_flow(
    provider: String,
) -> std::result::Result<String, PebbleError> {
    let config = config_for_provider(&provider)?;
    let manager = OAuthManager::new(config);

    let (auth_url, _pkce_state) = manager
        .start_auth()
        .await
        .map_err(|e| PebbleError::OAuth(format!("Failed to start OAuth flow: {e}")))?;

    // TODO: Store pkce_state for later use in complete_oauth. For now we use the
    // combined flow in complete_oauth_flow which starts its own auth + redirect
    // listener.

    Ok(auth_url)
}

/// Complete the OAuth flow end-to-end.
///
/// Starts a redirect listener, waits for the browser callback, exchanges the
/// authorization code for tokens, encrypts and stores the tokens, and creates
/// the account record.
#[tauri::command]
pub async fn complete_oauth_flow(
    state: State<'_, AppState>,
    provider: String,
    email: String,
    display_name: String,
) -> std::result::Result<Account, PebbleError> {
    let config = config_for_provider(&provider)?;
    let manager = OAuthManager::new(config);

    // Start auth flow (generates PKCE challenge)
    let (auth_url, pkce_state) = manager
        .start_auth()
        .await
        .map_err(|e| PebbleError::OAuth(format!("Failed to start OAuth flow: {e}")))?;

    // Open the authorization URL in the system browser
    opener::open(&auth_url)
        .map_err(|e| PebbleError::OAuth(format!("Failed to open browser: {e}")))?;

    // Wait for the redirect callback with the authorization code
    let redirect = manager
        .wait_for_redirect()
        .await
        .map_err(|e| PebbleError::OAuth(format!("OAuth redirect failed: {e}")))?;

    if !constant_time_eq(&redirect.state, pkce_state.csrf_token.secret()) {
        return Err(PebbleError::OAuth("OAuth state mismatch".to_string()));
    }

    // Exchange code for tokens
    let token_pair = manager
        .complete_auth(&redirect.code, pkce_state)
        .await
        .map_err(|e| PebbleError::OAuth(format!("Token exchange failed: {e}")))?;

    // Fetch user info from Google/Microsoft to get actual email and display name
    let (real_email, real_name) = fetch_userinfo(&provider, &token_pair.access_token).await
        .unwrap_or_else(|_| (email.clone(), display_name.clone()));

    let final_email = if real_email.is_empty() { email } else { real_email };
    let final_name = if real_name.is_empty() { display_name } else { real_name };

    // Create the account
    let now = now_timestamp();
    let account = Account {
        id: new_id(),
        email: final_email,
        display_name: final_name,
        provider: provider_type(&provider)?,
        created_at: now,
        updated_at: now,
    };

    state.store.insert_account(&account)?;

    // Encrypt tokens and store as auth_data
    let tokens_json = serde_json::json!({
        "access_token": token_pair.access_token,
        "refresh_token": token_pair.refresh_token,
        "expires_at": token_pair.expires_at,
        "scopes": token_pair.scopes,
    });
    let config_bytes = serde_json::to_vec(&tokens_json)
        .map_err(|e| PebbleError::Internal(format!("Failed to serialize tokens: {e}")))?;
    let encrypted = state.crypto.encrypt(&config_bytes)?;
    state.store.set_auth_data(&account.id, &encrypted)?;

    // Store provider metadata in sync_state
    let metadata = serde_json::json!({ "provider": provider.to_lowercase() });
    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| PebbleError::Internal(format!("Failed to serialize metadata: {e}")))?;
    state
        .store
        .update_account_sync_state(&account.id, &metadata_json)?;

    Ok(account)
}
