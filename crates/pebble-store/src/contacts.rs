use pebble_core::{KnownContact, Result};
use rusqlite::params;

use crate::Store;

impl Store {
    /// Query distinct contacts from the messages table matching a prefix.
    ///
    /// Searches `from_address`/`from_name` columns and also parses `to_list`
    /// JSON arrays to extract recipient contacts.  Results are deduplicated by
    /// email address (case-insensitive) and limited to `limit` rows.
    pub fn list_known_contacts(
        &self,
        account_id: &str,
        query: &str,
        limit: i64,
    ) -> Result<Vec<KnownContact>> {
        self.with_read(|conn| {
            let escaped = query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let pattern = format!("%{}%", escaped);

            // First: collect contacts from from_address / from_name columns
            let mut stmt = conn.prepare(
                "SELECT DISTINCT from_name, from_address
                     FROM messages
                     WHERE account_id = ?1
                       AND is_deleted = 0
                       AND (from_address LIKE ?2 ESCAPE '\\' OR from_name LIKE ?2 ESCAPE '\\')
                     LIMIT ?3",
            )?;

            let from_rows = stmt.query_map(params![account_id, pattern, limit], |row| {
                let name: String = row.get(0)?;
                let address: String = row.get(1)?;
                Ok(KnownContact {
                    name: if name.is_empty() { None } else { Some(name) },
                    address,
                })
            })?;

            let mut seen = std::collections::HashSet::new();
            let mut contacts = Vec::new();

            for row in from_rows {
                let contact = row?;
                let key = contact.address.to_lowercase();
                if seen.insert(key) {
                    contacts.push(contact);
                }
            }

            // Second: search inside to_list JSON for matching recipients
            if (contacts.len() as i64) < limit {
                let remaining = limit - contacts.len() as i64;
                let mut stmt2 = conn.prepare(
                    "SELECT DISTINCT to_list
                         FROM messages
                         WHERE account_id = ?1
                           AND is_deleted = 0
                           AND to_list LIKE ?2 ESCAPE '\\'
                         LIMIT ?3",
                )?;

                let to_rows = stmt2
                    .query_map(params![account_id, pattern, remaining * 5], |row| {
                        row.get::<_, String>(0)
                    })?;

                for row in to_rows {
                    if contacts.len() as i64 >= limit {
                        break;
                    }
                    let json_str = row?;
                    if let Ok(addrs) =
                        serde_json::from_str::<Vec<pebble_core::EmailAddress>>(&json_str)
                    {
                        let lower_query = query.to_lowercase();
                        for addr in addrs {
                            if contacts.len() as i64 >= limit {
                                break;
                            }
                            let matches = addr.address.to_lowercase().contains(&lower_query)
                                || addr
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&lower_query))
                                    .unwrap_or(false);
                            if matches {
                                let key = addr.address.to_lowercase();
                                if seen.insert(key) {
                                    contacts.push(KnownContact {
                                        name: addr.name,
                                        address: addr.address,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Ok(contacts)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;
    use pebble_core::*;

    fn setup_store_with_contacts() -> (Store, String) {
        let store = Store::open_in_memory().unwrap();
        let now = now_timestamp();
        let account = Account {
            id: new_id(),
            email: "me@example.com".to_string(),
            display_name: "Me".to_string(),
            color: None,
            provider: ProviderType::Imap,
            created_at: now,
            updated_at: now,
        };
        store.insert_account(&account).unwrap();

        let folder = Folder {
            id: new_id(),
            account_id: account.id.clone(),
            remote_id: "INBOX".to_string(),
            name: "Inbox".to_string(),
            folder_type: FolderType::Folder,
            role: Some(FolderRole::Inbox),
            parent_id: None,
            color: None,
            is_system: true,
            sort_order: 0,
        };
        store.insert_folder(&folder).unwrap();

        // Message from alice
        let msg1 = Message {
            id: new_id(),
            account_id: account.id.clone(),
            remote_id: "1".to_string(),
            message_id_header: None,
            in_reply_to: None,
            references_header: None,
            thread_id: None,
            subject: "Hello".to_string(),
            snippet: "hi".to_string(),
            from_address: "alice@example.com".to_string(),
            from_name: "Alice Smith".to_string(),
            to_list: vec![EmailAddress {
                name: Some("Bob Jones".to_string()),
                address: "bob@example.com".to_string(),
            }],
            cc_list: vec![],
            bcc_list: vec![],
            body_text: "hi".to_string(),
            body_html_raw: "<p>hi</p>".to_string(),
            has_attachments: false,
            is_read: true,
            is_starred: false,
            is_draft: false,
            date: now,
            remote_version: None,
            is_deleted: false,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        };
        store
            .insert_message(&msg1, std::slice::from_ref(&folder.id))
            .unwrap();

        // Message from charlie
        let msg2 = Message {
            id: new_id(),
            account_id: account.id.clone(),
            remote_id: "2".to_string(),
            message_id_header: None,
            in_reply_to: None,
            references_header: None,
            thread_id: None,
            subject: "Hey".to_string(),
            snippet: "hey".to_string(),
            from_address: "charlie@other.com".to_string(),
            from_name: "Charlie".to_string(),
            to_list: vec![EmailAddress {
                name: Some("Me".to_string()),
                address: "me@example.com".to_string(),
            }],
            cc_list: vec![],
            bcc_list: vec![],
            body_text: "hey".to_string(),
            body_html_raw: "<p>hey</p>".to_string(),
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
        store
            .insert_message(&msg2, std::slice::from_ref(&folder.id))
            .unwrap();

        (store, account.id)
    }

    #[test]
    fn test_list_known_contacts_by_from_address() {
        let (store, account_id) = setup_store_with_contacts();
        let results = store.list_known_contacts(&account_id, "alice", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].address, "alice@example.com");
        assert_eq!(results[0].name.as_deref(), Some("Alice Smith"));
    }

    #[test]
    fn test_list_known_contacts_by_to_list() {
        let (store, account_id) = setup_store_with_contacts();
        let results = store.list_known_contacts(&account_id, "bob", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].address, "bob@example.com");
        assert_eq!(results[0].name.as_deref(), Some("Bob Jones"));
    }

    #[test]
    fn test_list_known_contacts_broad_query() {
        let (store, account_id) = setup_store_with_contacts();
        let results = store
            .list_known_contacts(&account_id, "example", 10)
            .unwrap();
        // alice@example.com from from_address, bob@example.com from to_list, me@example.com from to_list
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_list_known_contacts_empty_query() {
        let (store, account_id) = setup_store_with_contacts();
        let results = store.list_known_contacts(&account_id, "", 10).unwrap();
        // Should return all known contacts
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_list_known_contacts_respects_limit() {
        let (store, account_id) = setup_store_with_contacts();
        let results = store.list_known_contacts(&account_id, "", 1).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_list_known_contacts_no_match() {
        let (store, account_id) = setup_store_with_contacts();
        let results = store
            .list_known_contacts(&account_id, "zzzznotfound", 10)
            .unwrap();
        assert!(results.is_empty());
    }
}
