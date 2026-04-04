use std::sync::Arc;

use pebble_core::{Message, Result, new_id, now_timestamp};
use pebble_store::Store;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use crate::imap::ImapProvider;
use crate::parser::parse_raw_email;
use crate::thread::compute_thread_id;

/// Configuration for the sync worker.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// How often to poll for new messages, in seconds.
    pub poll_interval_secs: u64,
    /// How often to do a full reconcile, in seconds.
    pub reconcile_interval_secs: u64,
    /// How many messages to fetch on initial sync.
    pub initial_fetch_limit: u32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            reconcile_interval_secs: 900,
            initial_fetch_limit: 200,
        }
    }
}

/// A worker that syncs mail for one account.
pub struct SyncWorker {
    account_id: String,
    provider: Arc<ImapProvider>,
    store: Arc<Store>,
    stop_rx: watch::Receiver<bool>,
}

impl SyncWorker {
    /// Create a new sync worker.
    pub fn new(
        account_id: impl Into<String>,
        provider: Arc<ImapProvider>,
        store: Arc<Store>,
        stop_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            provider,
            store,
            stop_rx,
        }
    }

    /// Perform the initial full sync: list folders and fetch inbox.
    pub async fn initial_sync(&self) -> Result<()> {
        info!("Starting initial sync for account {}", self.account_id);

        let folders = self.provider.list_folders(&self.account_id).await?;

        for folder in &folders {
            // Upsert folder into store
            // Ignore "already exists" errors
            let _ = self.store.insert_folder(folder);
        }

        // Sync the inbox (or first available folder)
        if let Some(inbox) = folders
            .iter()
            .find(|f| f.role == Some(pebble_core::FolderRole::Inbox))
            .or_else(|| folders.first())
        {
            let limit = 200;
            match self
                .sync_folder(inbox, None, limit)
                .await
            {
                Ok(count) => info!("Initial sync: fetched {} messages from {}", count, inbox.name),
                Err(e) => warn!("Initial sync folder {} failed: {}", inbox.name, e),
            }
        }

        Ok(())
    }

    /// Sync a folder: fetch raw messages, parse, compute threads, store.
    /// Returns the number of new messages stored.
    pub async fn sync_folder(
        &self,
        folder: &pebble_core::Folder,
        since_uid: Option<u32>,
        limit: u32,
    ) -> Result<u32> {
        let raw_messages = self
            .provider
            .fetch_messages_raw(&folder.remote_id, since_uid, limit)
            .await?;

        if raw_messages.is_empty() {
            return Ok(0);
        }

        // Load existing thread mappings for thread ID computation
        let thread_mappings = self
            .store
            .get_thread_mappings(&self.account_id)
            .unwrap_or_default();

        let mut stored_count = 0u32;

        for (uid, raw) in raw_messages {
            let remote_id = uid.to_string();

            // Skip if already stored
            if self
                .store
                .has_message_by_remote_id(&self.account_id, &remote_id)
                .unwrap_or(false)
            {
                debug!("Message UID {} already stored, skipping", uid);
                continue;
            }

            let parsed = match parse_raw_email(&raw) {
                Ok(p) => p,
                Err(e) => {
                    warn!("Failed to parse message UID {}: {}", uid, e);
                    continue;
                }
            };

            let now = now_timestamp();

            // Build a temporary message to compute thread_id
            let mut msg = Message {
                id: new_id(),
                account_id: self.account_id.clone(),
                remote_id: remote_id.clone(),
                message_id_header: parsed.message_id_header.clone(),
                in_reply_to: parsed.in_reply_to.clone(),
                references_header: parsed.references_header.clone(),
                thread_id: None,
                subject: parsed.subject.clone(),
                snippet: parsed.snippet.clone(),
                from_address: parsed.from_address.clone(),
                from_name: parsed.from_name.clone(),
                to_list: parsed.to_list.clone(),
                cc_list: parsed.cc_list.clone(),
                bcc_list: parsed.bcc_list.clone(),
                body_text: parsed.body_text.clone(),
                body_html_raw: parsed.body_html.clone(),
                has_attachments: parsed.has_attachments,
                is_read: false,
                is_starred: false,
                is_draft: false,
                date: parsed.date,
                remote_version: None,
                is_deleted: false,
                deleted_at: None,
                created_at: now,
                updated_at: now,
            };

            let thread_id = compute_thread_id(&msg, &thread_mappings);
            msg.thread_id = Some(thread_id);

            match self
                .store
                .insert_message(&msg, std::slice::from_ref(&folder.id))
            {
                Ok(()) => stored_count += 1,
                Err(e) => {
                    error!("Failed to store message UID {}: {}", uid, e);
                }
            }
        }

        Ok(stored_count)
    }

    /// Poll the inbox for new messages since the highest known UID.
    pub async fn poll_new_messages(&self) -> Result<()> {
        let folders = self.store.list_folders(&self.account_id)?;

        let inbox = match folders
            .iter()
            .find(|f| f.role == Some(pebble_core::FolderRole::Inbox))
            .or_else(|| folders.first())
        {
            Some(f) => f.clone(),
            None => return Ok(()),
        };

        let since_uid = self
            .store
            .get_max_remote_id(&self.account_id, &inbox.id)?
            .and_then(|s| s.parse::<u32>().ok());

        match self.sync_folder(&inbox, since_uid, 50).await {
            Ok(count) if count > 0 => {
                info!("Polled {} new messages for account {}", count, self.account_id)
            }
            Ok(_) => debug!("No new messages for account {}", self.account_id),
            Err(e) => warn!("Poll failed for account {}: {}", self.account_id, e),
        }

        Ok(())
    }

    /// Run the sync worker loop until the stop signal is received.
    pub async fn run(&self, config: SyncConfig) {
        let poll_interval = tokio::time::Duration::from_secs(config.poll_interval_secs);
        let reconcile_interval =
            tokio::time::Duration::from_secs(config.reconcile_interval_secs);

        let mut poll_ticker = tokio::time::interval(poll_interval);
        let mut reconcile_ticker = tokio::time::interval(reconcile_interval);

        // Connect and do initial sync
        if let Err(e) = self.provider.connect().await {
            error!("Failed to connect for account {}: {}", self.account_id, e);
            return;
        }

        if let Err(e) = self.initial_sync().await {
            error!("Initial sync failed for account {}: {}", self.account_id, e);
        }

        let mut stop_rx = self.stop_rx.clone();

        loop {
            tokio::select! {
                _ = poll_ticker.tick() => {
                    if let Err(e) = self.poll_new_messages().await {
                        warn!("Poll error for account {}: {}", self.account_id, e);
                    }
                }
                _ = reconcile_ticker.tick() => {
                    // Full reconcile: re-list folders and sync all
                    if let Err(e) = self.initial_sync().await {
                        warn!("Reconcile error for account {}: {}", self.account_id, e);
                    }
                }
                Ok(()) = stop_rx.changed() => {
                    if *stop_rx.borrow() {
                        info!("Sync worker stopping for account {}", self.account_id);
                        let _ = self.provider.disconnect().await;
                        break;
                    }
                }
            }
        }
    }
}
