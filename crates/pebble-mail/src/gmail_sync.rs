use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use pebble_core::traits::FolderProvider;
use pebble_core::{new_id, now_timestamp, Result};
use pebble_store::Store;
use std::sync::Mutex as StdMutex;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::provider::gmail::GmailProvider;
use crate::sync::{StoredMessage, SyncConfig, SyncError};
use crate::thread::compute_thread_id;

/// Callback that refreshes the OAuth token and returns the new access token.
pub type TokenRefresher =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Send + Sync>;

/// A sync worker for Gmail accounts using the REST API (HTTPS on port 443).
pub struct GmailSyncWorker {
    account_id: String,
    provider: Arc<GmailProvider>,
    store: Arc<Store>,
    stop_rx: watch::Receiver<bool>,
    #[allow(dead_code)]
    attachments_dir: PathBuf,
    error_tx: Option<mpsc::UnboundedSender<SyncError>>,
    message_tx: Option<mpsc::UnboundedSender<StoredMessage>>,
    token_refresher: Option<Arc<TokenRefresher>>,
    /// Last known token expiry (unix timestamp).
    token_expires_at: StdMutex<Option<i64>>,
}

impl GmailSyncWorker {
    pub fn new(
        account_id: impl Into<String>,
        provider: Arc<GmailProvider>,
        store: Arc<Store>,
        stop_rx: watch::Receiver<bool>,
        attachments_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            provider,
            store,
            stop_rx,
            attachments_dir: attachments_dir.into(),
            error_tx: None,
            message_tx: None,
            token_refresher: None,
            token_expires_at: StdMutex::new(None),
        }
    }

    pub fn with_error_tx(mut self, tx: mpsc::UnboundedSender<SyncError>) -> Self {
        self.error_tx = Some(tx);
        self
    }

    pub fn with_message_tx(mut self, tx: mpsc::UnboundedSender<StoredMessage>) -> Self {
        self.message_tx = Some(tx);
        self
    }

    pub fn with_token_refresher(mut self, refresher: TokenRefresher, expires_at: Option<i64>) -> Self {
        self.token_refresher = Some(Arc::new(refresher));
        *self.token_expires_at.lock().unwrap() = expires_at;
        self
    }

    fn emit_error(&self, error_type: &str, message: &str) {
        if let Some(tx) = &self.error_tx {
            let _ = tx.send(SyncError {
                error_type: error_type.to_string(),
                message: message.to_string(),
                timestamp: now_timestamp() as u64,
            });
        }
    }

    /// Ensure the access token is still valid; refresh if needed.
    async fn ensure_valid_token(&self) -> Result<()> {
        let now = now_timestamp();
        let needs_refresh = {
            let expires = self.token_expires_at.lock().unwrap();
            match *expires {
                Some(exp) => now >= exp - 300, // 5 minute buffer
                None => false,                 // No expiry info — assume valid
            }
        };

        if needs_refresh {
            if let Some(ref refresher) = self.token_refresher {
                debug!("Refreshing Gmail OAuth token for account {}", self.account_id);
                match refresher().await {
                    Ok(new_token) => {
                        self.provider.set_access_token(new_token);
                        // Assume the new token is valid for 1 hour
                        let mut expires = self.token_expires_at.lock().unwrap();
                        *expires = Some(now + 3600);
                        info!("Gmail OAuth token refreshed for account {}", self.account_id);
                    }
                    Err(e) => {
                        warn!("Failed to refresh OAuth token: {}", e);
                        self.emit_error("auth", &format!("Token refresh failed: {e}"));
                    }
                }
            }
        }
        Ok(())
    }

    /// Perform initial sync: list folders, fetch messages for each label.
    async fn initial_sync(&self) -> Result<()> {
        info!("Starting Gmail initial sync for account {}", self.account_id);

        // Sync folders (labels) — hidden labels are already filtered by the provider
        let folders = self.provider.list_folders().await?;
        for mut folder in folders {
            folder.account_id = self.account_id.clone();
            let _ = self.store.insert_folder(&folder);
        }

        // Clean up any previously-synced hidden Gmail labels from the local store
        let hidden = [
            "CHAT", "IMPORTANT", "STARRED", "UNREAD",
            "CATEGORY_FORUMS", "CATEGORY_UPDATES", "CATEGORY_PERSONAL",
            "CATEGORY_PROMOTIONS", "CATEGORY_SOCIAL",
        ];
        for label_id in &hidden {
            let _ = self.store.delete_folder_by_remote_id(&self.account_id, label_id);
        }

        // Ensure an Archive folder exists locally
        let local_folders = self.store.list_folders(&self.account_id)?;
        let has_archive = local_folders.iter().any(|f| f.role == Some(pebble_core::FolderRole::Archive));
        if !has_archive {
            let archive = pebble_core::Folder {
                id: new_id(),
                account_id: self.account_id.clone(),
                remote_id: "__local_archive__".to_string(),
                name: "Archive".to_string(),
                folder_type: pebble_core::FolderType::Folder,
                role: Some(pebble_core::FolderRole::Archive),
                parent_id: None,
                color: None,
                is_system: true,
                sort_order: 3,
            };
            let _ = self.store.insert_folder(&archive);
        }

        // Get the stored sync cursor (historyId) if any
        let stored_cursor = self
            .store
            .get_sync_cursor(&self.account_id)
            .ok()
            .flatten();

        // Get the user profile for the latest historyId
        let (_email, profile_history_id) = self.provider.get_profile().await?;

        // Fetch messages from key labels: INBOX first, then SENT
        let labels_to_sync = ["INBOX", "SENT"];
        let limit = if stored_cursor.is_some() { 50 } else { 200 };

        for label_id in &labels_to_sync {
            if let Err(e) = self.sync_label(label_id, limit).await {
                warn!("Gmail sync label {} failed: {}", label_id, e);
            }
        }

        // Store the historyId as sync cursor for future delta syncs
        if !profile_history_id.is_empty() {
            let _ = self.store.set_sync_cursor(&self.account_id, &profile_history_id);
        }

        info!("Gmail initial sync completed for account {}", self.account_id);
        Ok(())
    }

    /// Sync messages for a specific Gmail label.
    async fn sync_label(&self, label_id: &str, limit: u32) -> Result<u32> {
        let folder = self
            .store
            .list_folders(&self.account_id)?
            .into_iter()
            .find(|f| f.remote_id == label_id);
        let folder_id = match &folder {
            Some(f) => f.id.clone(),
            None => {
                debug!("No local folder found for label {}, skipping", label_id);
                return Ok(0);
            }
        };

        // List message IDs from Gmail
        let (msg_refs, _next_page) = self.provider.list_message_ids(label_id, limit, None).await?;
        if msg_refs.is_empty() {
            return Ok(0);
        }

        // Bulk-check which remote IDs already exist
        let remote_ids: Vec<String> = msg_refs.iter().map(|r| r.id.clone()).collect();
        let existing = self
            .store
            .get_existing_remote_ids(&self.account_id, &remote_ids)
            .unwrap_or_default();

        let new_ids: Vec<&str> = msg_refs
            .iter()
            .filter(|r| !existing.contains(&r.id))
            .map(|r| r.id.as_str())
            .collect();

        if new_ids.is_empty() {
            debug!("No new messages for label {}", label_id);
            return Ok(0);
        }

        info!("Fetching {} new messages for label {}", new_ids.len(), label_id);

        // Load thread mappings for thread ID computation
        let mut thread_mappings = self
            .store
            .get_thread_mappings(&self.account_id)
            .unwrap_or_default();

        let mut stored_count = 0u32;

        // Fetch each message (could be parallelized later)
        for gmail_id in new_ids {
            match self.provider.fetch_full_message(gmail_id, &self.account_id).await {
                Ok(mut msg) => {
                    // Compute thread ID
                    let thread_id = compute_thread_id(&msg, &thread_mappings);
                    msg.thread_id = Some(thread_id);

                    match self.store.insert_message(&msg, std::slice::from_ref(&folder_id)) {
                        Ok(()) => {
                            stored_count += 1;
                            if let (Some(mid), Some(tid)) = (&msg.message_id_header, &msg.thread_id) {
                                thread_mappings.push((mid.clone(), tid.clone()));
                            }
                            if let Some(tx) = &self.message_tx {
                                let _ = tx.send(StoredMessage {
                                    message: msg,
                                    folder_ids: vec![folder_id.clone()],
                                });
                            }
                        }
                        Err(e) => {
                            error!("Failed to store Gmail message {}: {}", gmail_id, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch Gmail message {}: {}", gmail_id, e);
                }
            }
        }

        if stored_count > 0 {
            info!("Stored {} messages for label {}", stored_count, label_id);
        }
        Ok(stored_count)
    }

    /// Poll for new messages using the Gmail History API (delta sync).
    async fn poll_changes(&self) -> Result<()> {
        let cursor = self
            .store
            .get_sync_cursor(&self.account_id)
            .ok()
            .flatten();
        let history_id = match cursor {
            Some(id) if !id.is_empty() => id,
            _ => {
                debug!("No history cursor, doing full re-sync");
                return self.initial_sync().await;
            }
        };

        let url = format!(
            "https://www.googleapis.com/gmail/v1/users/me/history?startHistoryId={history_id}"
        );
        let resp = self.provider.get(&url).await?;

        #[derive(serde::Deserialize)]
        struct HistoryList {
            history: Option<Vec<HistoryEntry>>,
            #[serde(rename = "historyId")]
            history_id: Option<String>,
        }
        #[derive(serde::Deserialize)]
        struct HistoryEntry {
            #[serde(rename = "messagesAdded")]
            messages_added: Option<Vec<HistoryMsg>>,
            #[serde(rename = "messagesDeleted")]
            messages_deleted: Option<Vec<HistoryMsg>>,
        }
        #[derive(serde::Deserialize)]
        struct HistoryMsg {
            message: MsgRef,
        }
        #[derive(serde::Deserialize)]
        struct MsgRef {
            id: String,
        }

        let history: HistoryList = resp
            .json()
            .await
            .map_err(|e| pebble_core::PebbleError::Network(format!("Parse history: {e}")))?;

        let mut new_ids = Vec::new();
        let mut deleted_ids = Vec::new();

        if let Some(entries) = &history.history {
            for entry in entries {
                if let Some(ref added) = entry.messages_added {
                    for m in added {
                        new_ids.push(m.message.id.clone());
                    }
                }
                if let Some(ref deleted) = entry.messages_deleted {
                    for m in deleted {
                        deleted_ids.push(m.message.id.clone());
                    }
                }
            }
        }

        // Handle deletions
        if !deleted_ids.is_empty() {
            for remote_id in &deleted_ids {
                if let Ok(Some(local_id)) = self.store.find_message_id_by_remote(&self.account_id, remote_id) {
                    let _ = self.store.soft_delete_message(&local_id);
                }
            }
            info!("Deleted {} messages via history", deleted_ids.len());
        }

        // Fetch new messages
        if !new_ids.is_empty() {
            // Filter out already-stored messages
            let existing = self
                .store
                .get_existing_remote_ids(&self.account_id, &new_ids)
                .unwrap_or_default();
            let truly_new: Vec<&str> = new_ids
                .iter()
                .filter(|id| !existing.contains(*id))
                .map(|s| s.as_str())
                .collect();

            if !truly_new.is_empty() {
                info!("Fetching {} new messages via history", truly_new.len());
                let inbox_folder = self
                    .store
                    .list_folders(&self.account_id)?
                    .into_iter()
                    .find(|f| f.remote_id == "INBOX");
                let folder_id = inbox_folder.map(|f| f.id).unwrap_or_default();

                let mut thread_mappings = self
                    .store
                    .get_thread_mappings(&self.account_id)
                    .unwrap_or_default();

                for gmail_id in truly_new {
                    match self.provider.fetch_full_message(gmail_id, &self.account_id).await {
                        Ok(mut msg) => {
                            let thread_id = compute_thread_id(&msg, &thread_mappings);
                            msg.thread_id = Some(thread_id);
                            if let Ok(()) = self.store.insert_message(&msg, &[folder_id.clone()]) {
                                if let (Some(mid), Some(tid)) = (&msg.message_id_header, &msg.thread_id) {
                                    thread_mappings.push((mid.clone(), tid.clone()));
                                }
                                if let Some(tx) = &self.message_tx {
                                    let _ = tx.send(StoredMessage {
                                        message: msg,
                                        folder_ids: vec![folder_id.clone()],
                                    });
                                }
                            }
                        }
                        Err(e) => warn!("Failed to fetch history message {}: {}", gmail_id, e),
                    }
                }
            }
        }

        // Update cursor
        if let Some(new_hid) = history.history_id {
            let _ = self.store.set_sync_cursor(&self.account_id, &new_hid);
        }

        Ok(())
    }

    /// Main sync loop.
    pub async fn run(&self, config: SyncConfig) {
        let poll_interval = tokio::time::Duration::from_secs(config.poll_interval_secs);

        // Ensure token is valid before starting
        if let Err(e) = self.ensure_valid_token().await {
            error!("Token validation failed for account {}: {}", self.account_id, e);
            self.emit_error("auth", &format!("Token validation failed: {e}"));
            return;
        }

        // Initial sync
        if let Err(e) = self.initial_sync().await {
            error!("Gmail initial sync failed for account {}: {}", self.account_id, e);
            self.emit_error("sync", &format!("Initial sync failed: {e}"));
            // Don't return — still enter poll loop so we can retry
        }

        let mut poll_ticker = tokio::time::interval(poll_interval);
        let mut stop_rx = self.stop_rx.clone();

        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    if *stop_rx.borrow() {
                        info!("Gmail sync stopped for account {}", self.account_id);
                        break;
                    }
                }
                _ = poll_ticker.tick() => {
                    if let Err(e) = self.ensure_valid_token().await {
                        warn!("Token refresh failed: {}", e);
                        continue;
                    }
                    if let Err(e) = self.poll_changes().await {
                        warn!("Gmail poll failed for account {}: {}", self.account_id, e);
                        self.emit_error("sync", &format!("Poll failed: {e}"));
                    }
                }
            }
        }
    }
}

