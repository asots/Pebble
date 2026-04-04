use pebble_core::{PebbleError, Result, SnoozedMessage};
use rusqlite::params;

use crate::Store;

fn row_to_snoozed(row: &rusqlite::Row) -> rusqlite::Result<SnoozedMessage> {
    Ok(SnoozedMessage {
        message_id: row.get(0)?,
        snoozed_at: row.get(1)?,
        unsnoozed_at: row.get(2)?,
        return_to: row.get(3)?,
    })
}

impl Store {
    pub fn snooze_message(&self, snooze: &SnoozedMessage) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO snoozed_messages (message_id, snoozed_at, unsnoozed_at, return_to)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    snooze.message_id,
                    snooze.snoozed_at,
                    snooze.unsnoozed_at,
                    snooze.return_to,
                ],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    pub fn list_snoozed_messages(&self) -> Result<Vec<SnoozedMessage>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT message_id, snoozed_at, unsnoozed_at, return_to
                     FROM snoozed_messages",
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map([], row_to_snoozed)
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| PebbleError::Storage(e.to_string()))?);
            }
            Ok(results)
        })
    }

    pub fn get_due_snoozed(&self, now: i64) -> Result<Vec<SnoozedMessage>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT message_id, snoozed_at, unsnoozed_at, return_to
                     FROM snoozed_messages WHERE unsnoozed_at <= ?1",
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![now], row_to_snoozed)
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| PebbleError::Storage(e.to_string()))?);
            }
            Ok(results)
        })
    }

    pub fn unsnooze_message(&self, message_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM snoozed_messages WHERE message_id = ?1",
                params![message_id],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    fn setup_store_with_message() -> (Store, String) {
        let store = Store::open_in_memory().unwrap();
        let now = pebble_core::now_timestamp();
        let account = pebble_core::Account {
            id: pebble_core::new_id(),
            email: "test@example.com".to_string(),
            display_name: "Test".to_string(),
            provider: pebble_core::ProviderType::Imap,
            created_at: now,
            updated_at: now,
        };
        store.insert_account(&account).unwrap();
        let folder = pebble_core::Folder {
            id: pebble_core::new_id(),
            account_id: account.id.clone(),
            remote_id: "INBOX".to_string(),
            name: "Inbox".to_string(),
            folder_type: pebble_core::FolderType::Folder,
            role: Some(pebble_core::FolderRole::Inbox),
            parent_id: None,
            color: None,
            is_system: true,
            sort_order: 0,
        };
        store.insert_folder(&folder).unwrap();
        let msg_id = pebble_core::new_id();
        let msg = pebble_core::Message {
            id: msg_id.clone(),
            account_id: account.id.clone(),
            remote_id: "1".to_string(),
            message_id_header: None,
            in_reply_to: None,
            references_header: None,
            thread_id: None,
            subject: "Test".to_string(),
            snippet: "Test snippet".to_string(),
            from_address: "sender@example.com".to_string(),
            from_name: "Sender".to_string(),
            to_list: vec![],
            cc_list: vec![],
            bcc_list: vec![],
            body_text: "body".to_string(),
            body_html_raw: "<p>body</p>".to_string(),
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
        };
        store.insert_message(&msg, &[folder.id.clone()]).unwrap();
        (store, msg_id)
    }

    #[test]
    fn test_snooze_and_list() {
        let (store, msg_id) = setup_store_with_message();
        let now = pebble_core::now_timestamp();
        let snooze = SnoozedMessage {
            message_id: msg_id.clone(),
            snoozed_at: now,
            unsnoozed_at: now + 3600,
            return_to: "inbox".to_string(),
        };
        store.snooze_message(&snooze).unwrap();

        let all = store.list_snoozed_messages().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].message_id, msg_id);
        assert_eq!(all[0].return_to, "inbox");
    }

    #[test]
    fn test_due_snoozed() {
        let (store, msg_id) = setup_store_with_message();
        let now = pebble_core::now_timestamp();
        let snooze = SnoozedMessage {
            message_id: msg_id.clone(),
            snoozed_at: now,
            unsnoozed_at: now + 3600,
            return_to: "inbox".to_string(),
        };
        store.snooze_message(&snooze).unwrap();

        // Not due yet
        let due = store.get_due_snoozed(now).unwrap();
        assert_eq!(due.len(), 0);

        // Now due
        let due = store.get_due_snoozed(now + 3600).unwrap();
        assert_eq!(due.len(), 1);

        // Also due if past the time
        let due = store.get_due_snoozed(now + 7200).unwrap();
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn test_unsnooze() {
        let (store, msg_id) = setup_store_with_message();
        let now = pebble_core::now_timestamp();
        let snooze = SnoozedMessage {
            message_id: msg_id.clone(),
            snoozed_at: now,
            unsnoozed_at: now + 3600,
            return_to: "inbox".to_string(),
        };
        store.snooze_message(&snooze).unwrap();
        store.unsnooze_message(&msg_id).unwrap();

        let all = store.list_snoozed_messages().unwrap();
        assert_eq!(all.len(), 0);
    }
}
