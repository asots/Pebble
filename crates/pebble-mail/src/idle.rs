use pebble_core::Result;
use tracing::{debug, warn};

/// Result of checking for new mail activity.
#[derive(Debug)]
pub enum IdleEvent {
    /// New mail may be available.
    NewMail,
    /// The wait timed out with no changes.
    Timeout,
    /// An error occurred during the check.
    Error(String),
}

/// Check if a mailbox has new messages by comparing UID count.
///
/// This is a lightweight fallback for servers that do not advertise the
/// IDLE capability. It does a quick UID SEARCH ALL and compares the
/// message count against the previously observed value.
pub async fn check_for_changes(
    provider: &super::imap::ImapProvider,
    mailbox: &str,
    last_exists: &mut u32,
) -> Result<IdleEvent> {
    match provider.fetch_all_uids(mailbox).await {
        Ok(uids) => {
            let current_count = uids.len() as u32;
            if *last_exists == 0 {
                // First check — record the baseline without triggering a sync.
                *last_exists = current_count;
                Ok(IdleEvent::Timeout)
            } else if current_count != *last_exists {
                debug!(
                    "Mailbox {} count changed: {} -> {}",
                    mailbox, *last_exists, current_count
                );
                *last_exists = current_count;
                Ok(IdleEvent::NewMail)
            } else {
                Ok(IdleEvent::Timeout)
            }
        }
        Err(e) => Ok(IdleEvent::Error(e.to_string())),
    }
}

/// Check for changes using native IDLE if supported, falling back to
/// UID-count comparison when IDLE is unavailable or fails.
pub async fn check_for_changes_with_idle(
    provider: &super::imap::ImapProvider,
    mailbox: &str,
    last_exists: &mut u32,
    use_idle: bool,
) -> Result<IdleEvent> {
    if use_idle {
        // Use native IMAP IDLE with a 60-second timeout to match the poll interval.
        match provider
            .idle_wait(mailbox, std::time::Duration::from_secs(60))
            .await
        {
            Ok(event) => Ok(event),
            Err(e) => {
                warn!("IDLE failed, attempting reconnect before fallback poll: {e}");
                // The IDLE failure may have left the session as None (e.g.
                // when done() fails to recover it). Reconnect so the
                // fallback poll has a usable session.
                if let Err(reconn_err) = provider.connect().await {
                    warn!("Reconnect after IDLE failure also failed: {reconn_err}");
                    return Ok(IdleEvent::Error(format!("IDLE failed and reconnect failed: {e}; {reconn_err}")));
                }
                check_for_changes(provider, mailbox, last_exists).await
            }
        }
    } else {
        check_for_changes(provider, mailbox, last_exists).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_event_variants() {
        let new_mail = IdleEvent::NewMail;
        assert!(matches!(new_mail, IdleEvent::NewMail));

        let timeout = IdleEvent::Timeout;
        assert!(matches!(timeout, IdleEvent::Timeout));

        let error = IdleEvent::Error("test error".to_string());
        assert!(matches!(error, IdleEvent::Error(ref s) if s == "test error"));
    }

    #[test]
    fn test_idle_event_debug() {
        let event = IdleEvent::NewMail;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("NewMail"));
    }
}
