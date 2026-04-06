use pebble_core::{Account, PebbleError, ProviderType, Result};
use rusqlite::{self, OptionalExtension};

use crate::Store;

fn provider_to_str(p: &ProviderType) -> &'static str {
    match p {
        ProviderType::Imap => "imap",
        ProviderType::Gmail => "gmail",
        ProviderType::Outlook => "outlook",
    }
}

fn str_to_provider(s: &str) -> ProviderType {
    match s {
        "gmail" => ProviderType::Gmail,
        "outlook" => ProviderType::Outlook,
        _ => ProviderType::Imap,
    }
}

impl Store {
    pub fn insert_account(&self, account: &Account) -> Result<()> {
        self.with_write(|conn| {
            conn.execute(
                "INSERT INTO accounts (id, email, display_name, provider, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    account.id,
                    account.email,
                    account.display_name,
                    provider_to_str(&account.provider),
                    account.created_at,
                    account.updated_at,
                ],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    pub fn update_account(&self, id: &str, email: &str, display_name: &str) -> Result<()> {
        self.with_write(|conn| {
            let now = pebble_core::now_timestamp();
            conn.execute(
                "UPDATE accounts SET email = ?1, display_name = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![email, display_name, now, id],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    pub fn get_account(&self, id: &str) -> Result<Option<Account>> {
        self.with_read(|conn| {
            let result = conn
                .query_row(
                    "SELECT id, email, display_name, provider, created_at, updated_at
                     FROM accounts WHERE id = ?1",
                    rusqlite::params![id],
                    |row| {
                        Ok(Account {
                            id: row.get(0)?,
                            email: row.get(1)?,
                            display_name: row.get(2)?,
                            provider: str_to_provider(&row.get::<_, String>(3)?),
                            created_at: row.get(4)?,
                            updated_at: row.get(5)?,
                        })
                    },
                )
                .optional()
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(result)
        })
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        self.with_read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, email, display_name, provider, created_at, updated_at
                     FROM accounts ORDER BY created_at ASC",
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(Account {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        display_name: row.get(2)?,
                        provider: str_to_provider(&row.get::<_, String>(3)?),
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                })
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let mut accounts = Vec::new();
            for row in rows {
                accounts.push(row.map_err(|e| PebbleError::Storage(e.to_string()))?);
            }
            Ok(accounts)
        })
    }

    pub fn delete_account(&self, id: &str) -> Result<()> {
        self.with_write(|conn| {
            conn.execute("DELETE FROM accounts WHERE id = ?1", rusqlite::params![id])
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    pub fn update_account_sync_state(&self, account_id: &str, sync_state: &str) -> Result<()> {
        self.with_write(|conn| {
            conn.execute(
                "UPDATE accounts SET sync_state = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![sync_state, pebble_core::now_timestamp(), account_id],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    pub fn get_account_sync_state(&self, account_id: &str) -> Result<Option<String>> {
        self.with_read(|conn| {
            let result = conn
                .query_row(
                    "SELECT sync_state FROM accounts WHERE id = ?1",
                    rusqlite::params![account_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(result)
        })
    }

    /// Get the sync cursor for an account from the sync_state JSON.
    pub fn get_sync_cursor(&self, account_id: &str) -> Result<Option<String>> {
        self.with_read(|conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT sync_state FROM accounts WHERE id = ?1",
                    rusqlite::params![account_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| PebbleError::Storage(e.to_string()))?
                .flatten();

            if let Some(json_str) = result {
                let value: serde_json::Value = serde_json::from_str(&json_str)
                    .map_err(|e| PebbleError::Storage(format!("Invalid sync_state JSON: {e}")))?;
                Ok(value
                    .get("last_sync_cursor")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()))
            } else {
                Ok(None)
            }
        })
    }

    /// Set the sync cursor in the sync_state JSON without clobbering other fields.
    pub fn set_sync_cursor(&self, account_id: &str, cursor: &str) -> Result<()> {
        self.with_write(|conn| {
            // Read current sync_state
            let current: Option<String> = conn
                .query_row(
                    "SELECT sync_state FROM accounts WHERE id = ?1",
                    rusqlite::params![account_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| PebbleError::Storage(e.to_string()))?
                .flatten();

            let mut value: serde_json::Value = if let Some(json_str) = current {
                serde_json::from_str(&json_str).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            value["last_sync_cursor"] = serde_json::Value::String(cursor.to_string());

            let new_json = serde_json::to_string(&value)
                .map_err(|e| PebbleError::Storage(format!("Failed to serialize sync_state: {e}")))?;

            let now = pebble_core::now_timestamp();
            conn.execute(
                "UPDATE accounts SET sync_state = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![new_json, now, account_id],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod cursor_tests {
    use crate::Store;
    use pebble_core::*;

    fn test_account() -> Account {
        Account {
            id: new_id(),
            email: "test@example.com".to_string(),
            display_name: "Test".to_string(),
            provider: ProviderType::Imap,
            created_at: now_timestamp(),
            updated_at: now_timestamp(),
        }
    }

    #[test]
    fn test_set_and_get_sync_cursor() {
        let store = Store::open_in_memory().unwrap();
        let account = test_account();
        store.insert_account(&account).unwrap();

        store.set_sync_cursor(&account.id, "12345").unwrap();
        let cursor = store.get_sync_cursor(&account.id).unwrap();
        assert_eq!(cursor, Some("12345".to_string()));
    }

    #[test]
    fn test_get_sync_cursor_returns_none() {
        let store = Store::open_in_memory().unwrap();
        let account = test_account();
        store.insert_account(&account).unwrap();

        let cursor = store.get_sync_cursor(&account.id).unwrap();
        assert!(cursor.is_none());
    }

    #[test]
    fn test_set_cursor_preserves_other_fields() {
        let store = Store::open_in_memory().unwrap();
        let account = test_account();
        store.insert_account(&account).unwrap();

        // Set initial sync_state with some data
        store
            .update_account_sync_state(&account.id, r#"{"provider":"imap","foo":"bar"}"#)
            .unwrap();

        // Set cursor
        store.set_sync_cursor(&account.id, "999").unwrap();

        // Verify cursor is set
        let cursor = store.get_sync_cursor(&account.id).unwrap();
        assert_eq!(cursor, Some("999".to_string()));

        // Verify other fields preserved
        let state = store.get_account_sync_state(&account.id).unwrap().unwrap();
        let value: serde_json::Value = serde_json::from_str(&state).unwrap();
        assert_eq!(value["foo"], "bar");
        assert_eq!(value["provider"], "imap");
    }
}
