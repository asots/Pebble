use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub release_url: String,
    pub is_newer: bool,
}

#[tauri::command]
pub async fn check_for_update(current_version: String) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent("Pebble-Email-Client")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let resp = client
        .get("https://api.github.com/repos/QingJ01/Pebble/releases/latest")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status {}", resp.status()));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    let tag = data["tag_name"]
        .as_str()
        .ok_or("Missing tag_name in response")?;
    let latest = tag.trim_start_matches('v').to_string();
    let release_url = data["html_url"]
        .as_str()
        .unwrap_or("https://github.com/QingJ01/Pebble/releases")
        .to_string();

    Ok(UpdateInfo {
        is_newer: latest != current_version,
        latest_version: latest,
        release_url,
    })
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    // Only allow safe URL schemes to prevent command injection via opener::open / ShellExecuteW
    if !url.starts_with("https://") && !url.starts_with("http://") && !url.starts_with("mailto:") {
        return Err("Only https://, http://, and mailto: URLs are permitted".to_string());
    }
    opener::open(&url).map_err(|e| format!("Failed to open URL: {e}"))
}

#[tauri::command]
pub fn health_check(state: State<'_, AppState>) -> Result<String, String> {
    match state.store.list_accounts() {
        Ok(accounts) => Ok(format!(
            "Pebble is healthy. {} account(s) configured.",
            accounts.len()
        )),
        Err(e) => Err(format!("Health check failed: {}", e)),
    }
}
