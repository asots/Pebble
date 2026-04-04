use pebble_core::{Message, new_id};

/// Strip common reply/forward prefixes from a subject line, recursively.
/// Handles Re:, Fwd:, Fw:, 回复:, 转发: (case-insensitive).
pub fn normalize_subject(subject: &str) -> String {
    let mut s = subject.trim().to_string();
    loop {
        let lower = s.to_lowercase();
        let stripped = lower
            .strip_prefix("re:")
            .or_else(|| lower.strip_prefix("fwd:"))
            .or_else(|| lower.strip_prefix("fw:"))
            .or_else(|| lower.strip_prefix("回复:"))
            .or_else(|| lower.strip_prefix("转发:"));

        match stripped {
            Some(rest) => {
                // Advance s by the bytes consumed by the prefix
                let consumed = s.len() - rest.len();
                s = s[consumed..].trim().to_string();
            }
            None => break,
        }
    }
    s
}

/// Compute the thread ID for a message.
///
/// `existing_threads` is a slice of `(message_id_header, thread_id)` pairs
/// representing messages already in the store.
///
/// Logic:
/// 1. Check `in_reply_to` against existing `message_id_header`s.
/// 2. Check each ID in `references_header` (space-separated) against existing.
/// 3. Fall back to the message's own `message_id_header`.
/// 4. If none, generate a new UUID.
pub fn compute_thread_id(message: &Message, existing_threads: &[(String, String)]) -> String {
    // Helper: look up a message-id in existing_threads
    let lookup = |mid: &str| -> Option<String> {
        let needle = mid.trim();
        existing_threads
            .iter()
            .find(|(msg_id, _)| msg_id.trim() == needle)
            .map(|(_, thread_id)| thread_id.clone())
    };

    // 1. Check In-Reply-To
    if let Some(irt) = &message.in_reply_to {
        // In-Reply-To may contain multiple IDs space-separated
        for id in irt.split_whitespace() {
            if let Some(tid) = lookup(id) {
                return tid;
            }
        }
    }

    // 2. Check References (space-separated, check last to first for closest ancestor)
    if let Some(refs) = &message.references_header {
        let ids: Vec<&str> = refs.split_whitespace().collect();
        for id in ids.iter().rev() {
            if let Some(tid) = lookup(id) {
                return tid;
            }
        }
    }

    // 3. Use the message's own message_id_header as thread root
    if let Some(mid) = &message.message_id_header {
        return mid.clone();
    }

    // 4. Generate new UUID
    new_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pebble_core::{EmailAddress, Message, new_id, now_timestamp};

    fn make_message(
        message_id_header: Option<&str>,
        in_reply_to: Option<&str>,
        references_header: Option<&str>,
    ) -> Message {
        let now = now_timestamp();
        Message {
            id: new_id(),
            account_id: "acct1".to_string(),
            remote_id: "1".to_string(),
            message_id_header: message_id_header.map(|s| s.to_string()),
            in_reply_to: in_reply_to.map(|s| s.to_string()),
            references_header: references_header.map(|s| s.to_string()),
            thread_id: None,
            subject: "Test".to_string(),
            snippet: "".to_string(),
            from_address: "a@b.com".to_string(),
            from_name: "A".to_string(),
            to_list: vec![EmailAddress {
                name: None,
                address: "b@c.com".to_string(),
            }],
            cc_list: vec![],
            bcc_list: vec![],
            body_text: "".to_string(),
            body_html_raw: "".to_string(),
            has_attachments: false,
            is_read: false,
            is_starred: false,
            is_draft: false,
            date: now,
            remote_version: None,
            is_deleted: false,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_normalize_subject() {
        assert_eq!(normalize_subject("Re: Hello"), "Hello");
        assert_eq!(normalize_subject("re: Hello"), "Hello");
        assert_eq!(normalize_subject("RE: Hello"), "Hello");
        assert_eq!(normalize_subject("Fwd: Hello"), "Hello");
        assert_eq!(normalize_subject("FWD: Hello"), "Hello");
        assert_eq!(normalize_subject("Fw: Hello"), "Hello");
        assert_eq!(normalize_subject("Re: Re: Hello"), "Hello");
        assert_eq!(normalize_subject("Re: Fwd: Hello"), "Hello");
        assert_eq!(normalize_subject("回复: Hello"), "Hello");
        assert_eq!(normalize_subject("转发: Hello"), "Hello");
        assert_eq!(normalize_subject("Hello"), "Hello");
        assert_eq!(normalize_subject("  Hello  "), "Hello");
    }

    #[test]
    fn test_compute_thread_id_new_thread() {
        let msg = make_message(Some("<new@example.com>"), None, None);
        let existing: Vec<(String, String)> = vec![];
        let tid = compute_thread_id(&msg, &existing);
        // No match → use own message_id_header
        assert_eq!(tid, "<new@example.com>");
    }

    #[test]
    fn test_compute_thread_id_reply() {
        let existing = vec![(
            "<original@example.com>".to_string(),
            "thread-abc".to_string(),
        )];
        let msg = make_message(
            Some("<reply@example.com>"),
            Some("<original@example.com>"),
            Some("<original@example.com>"),
        );
        let tid = compute_thread_id(&msg, &existing);
        assert_eq!(tid, "thread-abc");
    }

    #[test]
    fn test_compute_thread_id_via_references() {
        let existing = vec![("<root@example.com>".to_string(), "thread-xyz".to_string())];
        // in_reply_to doesn't match, but references does
        let msg = make_message(
            Some("<reply2@example.com>"),
            Some("<nonexistent@example.com>"),
            Some("<root@example.com> <intermediate@example.com>"),
        );
        let tid = compute_thread_id(&msg, &existing);
        assert_eq!(tid, "thread-xyz");
    }
}
