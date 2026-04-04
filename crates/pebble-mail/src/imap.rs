use std::sync::Arc;

use async_imap::Client;
use async_native_tls::TlsStream;
use futures::TryStreamExt;
use pebble_core::{Folder, FolderRole, FolderType, PebbleError, Result, new_id, now_timestamp};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::debug;

/// Configuration for an IMAP connection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
}

/// Configuration for an SMTP connection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
}

/// Type alias for a TLS-wrapped IMAP session using Tokio.
type TlsSession = async_imap::Session<TlsStream<TcpStream>>;
/// Type alias for a plain-TCP IMAP session using Tokio.
type PlainSession = async_imap::Session<TcpStream>;

/// The underlying session, either TLS or plain.
enum ImapSession {
    Tls(Box<TlsSession>),
    Plain(Box<PlainSession>),
}

/// An IMAP provider that manages a connection and session.
pub struct ImapProvider {
    config: ImapConfig,
    session: Arc<Mutex<Option<ImapSession>>>,
}

impl ImapProvider {
    /// Create a new provider with the given configuration.
    pub fn new(config: ImapConfig) -> Self {
        Self {
            config,
            session: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to the IMAP server and log in.
    pub async fn connect(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let tcp = TcpStream::connect(&addr)
            .await
            .map_err(|e| PebbleError::Network(format!("TCP connect to {addr}: {e}")))?;

        let session = if self.config.use_tls {
            let tls_stream = async_native_tls::connect(&self.config.host, tcp)
                .await
                .map_err(|e| PebbleError::Network(format!("TLS handshake: {e}")))?;
            let client = Client::new(tls_stream);
            let sess = client
                .login(&self.config.username, &self.config.password)
                .await
                .map_err(|(e, _)| PebbleError::Auth(format!("IMAP login failed: {e}")))?;
            ImapSession::Tls(Box::new(sess))
        } else {
            let client = Client::new(tcp);
            let sess = client
                .login(&self.config.username, &self.config.password)
                .await
                .map_err(|(e, _)| PebbleError::Auth(format!("IMAP login failed: {e}")))?;
            ImapSession::Plain(Box::new(sess))
        };

        let mut guard = self.session.lock().await;
        *guard = Some(session);
        debug!("IMAP connected to {}", self.config.host);
        Ok(())
    }

    /// List folders for the given account, returning `Folder` structs.
    pub async fn list_folders(&self, account_id: &str) -> Result<Vec<Folder>> {
        let mut guard = self.session.lock().await;
        let sess = guard
            .as_mut()
            .ok_or_else(|| PebbleError::Network("Not connected".to_string()))?;

        let names: Vec<String> = match sess {
            ImapSession::Tls(s) => {
                let stream = s
                    .list(None, Some("*"))
                    .await
                    .map_err(|e| PebbleError::Network(format!("LIST failed: {e}")))?;
                stream
                    .map_ok(|n| n.name().to_string())
                    .try_collect()
                    .await
                    .map_err(|e| PebbleError::Network(format!("LIST collect: {e}")))?
            }
            ImapSession::Plain(s) => {
                let stream = s
                    .list(None, Some("*"))
                    .await
                    .map_err(|e| PebbleError::Network(format!("LIST failed: {e}")))?;
                stream
                    .map_ok(|n| n.name().to_string())
                    .try_collect()
                    .await
                    .map_err(|e| PebbleError::Network(format!("LIST collect: {e}")))?
            }
        };

        let mut folders: Vec<Folder> = names
            .into_iter()
            .map(|name| {
                let role = detect_folder_role(&name);
                let sort_order = folder_sort_order(&role);
                Folder {
                    id: new_id(),
                    account_id: account_id.to_string(),
                    remote_id: name.clone(),
                    name: name.clone(),
                    folder_type: FolderType::Folder,
                    role,
                    parent_id: None,
                    color: None,
                    is_system: true,
                    sort_order,
                }
            })
            .collect();

        folders.sort_by_key(|f| f.sort_order);
        Ok(folders)
    }

    /// Fetch raw message bytes from a mailbox.
    /// Returns a list of `(uid, raw_bytes)` pairs.
    pub async fn fetch_messages_raw(
        &self,
        mailbox: &str,
        since_uid: Option<u32>,
        limit: u32,
    ) -> Result<Vec<(u32, Vec<u8>)>> {
        let mut guard = self.session.lock().await;
        let sess = guard
            .as_mut()
            .ok_or_else(|| PebbleError::Network("Not connected".to_string()))?;

        macro_rules! do_fetch {
            ($s:expr) => {{
                let mailbox_info = $s
                    .select(mailbox)
                    .await
                    .map_err(|e| PebbleError::Network(format!("SELECT failed: {e}")))?;

                let exists = mailbox_info.exists;
                if exists == 0 {
                    return Ok(Vec::new());
                }

                let mut results = Vec::new();

                if let Some(uid) = since_uid {
                    let uid_set = format!("{}:*", uid + 1);
                    let fetches: Vec<async_imap::types::Fetch> = $s
                        .uid_fetch(&uid_set, "(UID BODY.PEEK[])")
                        .await
                        .map_err(|e| PebbleError::Network(format!("UID FETCH failed: {e}")))?
                        .try_collect()
                        .await
                        .map_err(|e| PebbleError::Network(format!("UID FETCH collect: {e}")))?;
                    for fetch in fetches {
                        let msg_uid = fetch.uid.unwrap_or(fetch.message);
                        if let Some(body) = fetch.body() {
                            results.push((msg_uid, body.to_vec()));
                        }
                    }
                } else {
                    let start = if exists > limit { exists - limit + 1 } else { 1 };
                    let seq_set = format!("{start}:{exists}");
                    let fetches: Vec<async_imap::types::Fetch> = $s
                        .fetch(&seq_set, "(UID BODY.PEEK[])")
                        .await
                        .map_err(|e| PebbleError::Network(format!("FETCH failed: {e}")))?
                        .try_collect()
                        .await
                        .map_err(|e| PebbleError::Network(format!("FETCH collect: {e}")))?;
                    for fetch in fetches {
                        let msg_uid = fetch.uid.unwrap_or(fetch.message);
                        if let Some(body) = fetch.body() {
                            results.push((msg_uid, body.to_vec()));
                        }
                    }
                }

                results
            }};
        }

        let results = match sess {
            ImapSession::Tls(s) => do_fetch!(s),
            ImapSession::Plain(s) => do_fetch!(s),
        };

        Ok(results)
    }

    /// Fetch flags for a set of UIDs. Returns `(uid, is_read, is_starred)`.
    pub async fn fetch_flags(
        &self,
        mailbox: &str,
        uids: &[u32],
    ) -> Result<Vec<(u32, bool, bool)>> {
        if uids.is_empty() {
            return Ok(Vec::new());
        }

        let uid_set: String = uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let mut guard = self.session.lock().await;
        let sess = guard
            .as_mut()
            .ok_or_else(|| PebbleError::Network("Not connected".to_string()))?;

        macro_rules! do_flags {
            ($s:expr) => {{
                $s.select(mailbox)
                    .await
                    .map_err(|e| PebbleError::Network(format!("SELECT failed: {e}")))?;

                let fetches: Vec<async_imap::types::Fetch> = $s
                    .uid_fetch(&uid_set, "FLAGS")
                    .await
                    .map_err(|e| PebbleError::Network(format!("UID FETCH FLAGS failed: {e}")))?
                    .try_collect()
                    .await
                    .map_err(|e| PebbleError::Network(format!("FLAGS collect: {e}")))?;

                fetches
                    .into_iter()
                    .map(|fetch| {
                        let uid = fetch.uid.unwrap_or(fetch.message);
                        let (is_read, is_starred) = parse_flags(fetch.flags());
                        (uid, is_read, is_starred)
                    })
                    .collect::<Vec<_>>()
            }};
        }

        let results = match sess {
            ImapSession::Tls(s) => do_flags!(s),
            ImapSession::Plain(s) => do_flags!(s),
        };

        Ok(results)
    }

    /// Set flags on a message identified by UID.
    pub async fn set_flags(
        &self,
        mailbox: &str,
        uid: u32,
        is_read: Option<bool>,
        is_starred: Option<bool>,
    ) -> Result<()> {
        let uid_str = uid.to_string();

        let mut guard = self.session.lock().await;
        let sess = guard
            .as_mut()
            .ok_or_else(|| PebbleError::Network("Not connected".to_string()))?;

        macro_rules! do_store {
            ($s:expr) => {{
                $s.select(mailbox)
                    .await
                    .map_err(|e| PebbleError::Network(format!("SELECT failed: {e}")))?;

                if let Some(read) = is_read {
                    let flag_cmd = if read {
                        format!("+FLAGS (\\Seen)")
                    } else {
                        format!("-FLAGS (\\Seen)")
                    };
                    let _: Vec<async_imap::types::Fetch> = $s
                        .uid_store(&uid_str, &flag_cmd)
                        .await
                        .map_err(|e| {
                            PebbleError::Network(format!("STORE \\Seen failed: {e}"))
                        })?
                        .try_collect()
                        .await
                        .map_err(|e| {
                            PebbleError::Network(format!("STORE \\Seen collect: {e}"))
                        })?;
                }

                if let Some(starred) = is_starred {
                    let flag_cmd = if starred {
                        format!("+FLAGS (\\Flagged)")
                    } else {
                        format!("-FLAGS (\\Flagged)")
                    };
                    let _: Vec<async_imap::types::Fetch> = $s
                        .uid_store(&uid_str, &flag_cmd)
                        .await
                        .map_err(|e| {
                            PebbleError::Network(format!("STORE \\Flagged failed: {e}"))
                        })?
                        .try_collect()
                        .await
                        .map_err(|e| {
                            PebbleError::Network(format!("STORE \\Flagged collect: {e}"))
                        })?;
                }
            }};
        }

        match sess {
            ImapSession::Tls(s) => do_store!(s),
            ImapSession::Plain(s) => do_store!(s),
        }

        Ok(())
    }

    /// Disconnect from the IMAP server.
    pub async fn disconnect(&self) -> Result<()> {
        let mut guard = self.session.lock().await;
        if let Some(sess) = guard.as_mut() {
            match sess {
                ImapSession::Tls(s) => {
                    let _ = s.logout().await;
                }
                ImapSession::Plain(s) => {
                    let _ = s.logout().await;
                }
            }
            *guard = None;
        }
        Ok(())
    }
}

/// Parse flags from an iterator of `Flag` values.
fn parse_flags<'a>(flags: impl Iterator<Item = async_imap::types::Flag<'a>>) -> (bool, bool) {
    let mut is_read = false;
    let mut is_starred = false;
    for flag in flags {
        match flag {
            async_imap::types::Flag::Seen => is_read = true,
            async_imap::types::Flag::Flagged => is_starred = true,
            _ => {}
        }
    }
    (is_read, is_starred)
}

/// Detect a folder role based on its name.
pub fn detect_folder_role(name: &str) -> Option<FolderRole> {
    let lower = name.to_lowercase();
    // Check last component after hierarchy separator
    let leaf = lower.rsplit('/').next().unwrap_or(&lower);
    let leaf = leaf.rsplit('.').next().unwrap_or(leaf);

    if leaf == "inbox" {
        Some(FolderRole::Inbox)
    } else if leaf.contains("sent") {
        Some(FolderRole::Sent)
    } else if leaf.contains("draft") {
        Some(FolderRole::Drafts)
    } else if leaf.contains("trash") || leaf.contains("deleted") {
        Some(FolderRole::Trash)
    } else if leaf.contains("archive") {
        Some(FolderRole::Archive)
    } else if leaf.contains("spam") || leaf.contains("junk") {
        Some(FolderRole::Spam)
    } else {
        None
    }
}

/// Sort order for folder roles.
pub fn folder_sort_order(role: &Option<FolderRole>) -> i32 {
    match role {
        Some(FolderRole::Inbox) => 0,
        Some(FolderRole::Drafts) => 1,
        Some(FolderRole::Sent) => 2,
        Some(FolderRole::Archive) => 3,
        Some(FolderRole::Spam) => 4,
        Some(FolderRole::Trash) => 5,
        None => 100,
    }
}

// Satisfy the unused import of `now_timestamp`
#[allow(dead_code)]
fn _use_now() -> i64 {
    now_timestamp()
}
