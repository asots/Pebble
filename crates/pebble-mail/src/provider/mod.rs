pub mod gmail;
pub mod imap_provider;
pub mod outlook;

use std::sync::Arc;

use pebble_core::{PebbleError, ProviderType, Result};
use pebble_core::traits::MailProvider;

/// Create a trait-based mail provider from the given provider type and credentials.
pub async fn create_provider(
    provider_type: &ProviderType,
    credentials: &serde_json::Value,
) -> Result<Arc<dyn MailProvider>> {
    match provider_type {
        ProviderType::Imap => {
            let imap_config: crate::imap::ImapConfig =
                serde_json::from_value(credentials.clone())
                    .map_err(|e| PebbleError::Auth(format!("Invalid IMAP config: {e}")))?;
            let provider = imap_provider::ImapMailProvider::new(imap_config);
            Ok(Arc::new(provider))
        }
        ProviderType::Gmail => {
            let token = credentials
                .get("access_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    PebbleError::Auth("Missing access_token for Gmail".to_string())
                })?
                .to_string();
            let provider = gmail::GmailProvider::new(token);
            Ok(Arc::new(provider))
        }
        ProviderType::Outlook => {
            let token = credentials
                .get("access_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    PebbleError::Auth("Missing access_token for Outlook".to_string())
                })?
                .to_string();
            let provider = outlook::OutlookProvider::new(token);
            Ok(Arc::new(provider))
        }
    }
}
