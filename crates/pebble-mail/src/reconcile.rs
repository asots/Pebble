/// Compute flag differences between local and remote state.
///
/// local: Vec of (message_id, remote_id, is_read, is_starred, updated_at)
/// remote: Vec of (uid, is_read, is_starred) from IMAP FETCH FLAGS
///
/// Returns changes to apply: (message_id, Option<is_read>, Option<is_starred>)
/// Only includes entries where at least one flag differs.
///
/// Messages modified within the last 60 seconds are skipped to avoid
/// overwriting pending local flag changes (fire-and-forget writeback race).
pub fn compute_flag_diff(
    local: &[(String, String, bool, bool, i64)],
    remote: &[(u32, bool, bool)],
) -> Vec<(String, Option<bool>, Option<bool>)> {
    use std::collections::HashMap;

    let now = pebble_core::now_timestamp();
    let remote_map: HashMap<u32, (bool, bool)> = remote
        .iter()
        .map(|&(uid, read, starred)| (uid, (read, starred)))
        .collect();

    let mut changes = Vec::new();

    for (msg_id, remote_id, local_read, local_starred, updated_at) in local {
        // Skip messages modified in the last 60 seconds to avoid overwriting
        // pending local flag changes (fire-and-forget writeback race)
        if now - updated_at < 60 {
            continue;
        }

        let uid: u32 = match remote_id.parse() {
            Ok(u) => u,
            Err(_) => continue,
        };

        if let Some(&(remote_read, remote_starred)) = remote_map.get(&uid) {
            let read_change = if *local_read != remote_read {
                Some(remote_read)
            } else {
                None
            };
            let starred_change = if *local_starred != remote_starred {
                Some(remote_starred)
            } else {
                None
            };
            if read_change.is_some() || starred_change.is_some() {
                changes.push((msg_id.clone(), read_change, starred_change));
            }
        }
    }

    changes
}

/// Detect messages that exist locally but have been deleted on the server.
///
/// local_remote_ids: Vec of (message_id, remote_id) for local messages
/// server_uids: all UIDs currently on the server
///
/// Returns message_ids that should be soft-deleted locally.
pub fn detect_deletions(
    local_remote_ids: &[(String, String)],
    server_uids: &[u32],
) -> Vec<String> {
    use std::collections::HashSet;

    let server_set: HashSet<u32> = server_uids.iter().copied().collect();

    local_remote_ids
        .iter()
        .filter_map(|(msg_id, remote_id)| {
            let uid: u32 = remote_id.parse().ok()?;
            if server_set.contains(&uid) {
                None
            } else {
                Some(msg_id.clone())
            }
        })
        .collect()
}

/// Check if reconciliation can be skipped based on MODSEQ (RFC 4551 CONDSTORE).
/// Returns true if the stored modseq matches the server modseq, meaning no flag
/// modifications have occurred since the last reconcile.
/// Returns false if stored_modseq is 0 (first sync or no prior CONDSTORE data).
pub fn can_skip_reconcile(stored_modseq: u64, server_modseq: u64) -> bool {
    stored_modseq > 0 && stored_modseq == server_modseq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_diff_detects_read_change() {
        let local = vec![("msg1".to_string(), "100".to_string(), false, false, 0i64)];
        let remote = vec![(100, true, false)];
        let diff = compute_flag_diff(&local, &remote);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].0, "msg1");
        assert_eq!(diff[0].1, Some(true)); // is_read changed
        assert_eq!(diff[0].2, None); // is_starred unchanged
    }

    #[test]
    fn test_flag_diff_no_changes() {
        let local = vec![("msg1".to_string(), "100".to_string(), true, true, 0i64)];
        let remote = vec![(100, true, true)];
        let diff = compute_flag_diff(&local, &remote);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_flag_diff_multiple_changes() {
        let local = vec![
            ("msg1".to_string(), "100".to_string(), false, true, 0i64),
            ("msg2".to_string(), "101".to_string(), true, false, 0i64),
        ];
        let remote = vec![(100, true, false), (101, true, true)];
        let diff = compute_flag_diff(&local, &remote);
        assert_eq!(diff.len(), 2);
    }

    #[test]
    fn test_flag_diff_grace_period_skips_recent_message() {
        let now = pebble_core::now_timestamp();
        // updated_at = now - 30 seconds (within the 60s grace period)
        let local = vec![("msg1".to_string(), "100".to_string(), false, false, now - 30)];
        let remote = vec![(100, true, false)];
        let diff = compute_flag_diff(&local, &remote);
        // Should be skipped due to grace period
        assert!(diff.is_empty());
    }

    #[test]
    fn test_flag_diff_grace_period_allows_old_message() {
        let now = pebble_core::now_timestamp();
        // updated_at = now - 120 seconds (outside the 60s grace period)
        let local = vec![("msg1".to_string(), "100".to_string(), false, false, now - 120)];
        let remote = vec![(100, true, false)];
        let diff = compute_flag_diff(&local, &remote);
        // Should NOT be skipped — old enough to reconcile
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].1, Some(true));
    }

    #[test]
    fn test_detect_deletions_finds_missing() {
        let local = vec![
            ("msg1".to_string(), "100".to_string()),
            ("msg2".to_string(), "101".to_string()),
            ("msg3".to_string(), "102".to_string()),
        ];
        let server_uids = vec![100, 102];
        let deleted = detect_deletions(&local, &server_uids);
        assert_eq!(deleted, vec!["msg2".to_string()]);
    }

    #[test]
    fn test_detect_deletions_all_present() {
        let local = vec![("msg1".to_string(), "100".to_string())];
        let server_uids = vec![100, 101, 102];
        let deleted = detect_deletions(&local, &server_uids);
        assert!(deleted.is_empty());
    }

    #[test]
    fn test_can_skip_reconcile_same_modseq() {
        assert!(can_skip_reconcile(100, 100));
    }

    #[test]
    fn test_can_skip_reconcile_different_modseq() {
        assert!(!can_skip_reconcile(100, 101));
    }

    #[test]
    fn test_can_skip_reconcile_zero_stored() {
        assert!(!can_skip_reconcile(0, 100));
    }

    #[test]
    fn test_can_skip_reconcile_both_zero() {
        assert!(!can_skip_reconcile(0, 0));
    }
}
