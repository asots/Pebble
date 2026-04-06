use pebble_core::{PebbleError, Result};
use rusqlite::{params, OptionalExtension};

use crate::Store;

impl Store {
    /// Store encrypted auth data for an account.
    pub fn set_auth_data(&self, account_id: &str, encrypted: &[u8]) -> Result<()> {
        self.with_write(|conn| {
            let now = pebble_core::now_timestamp();
            let rows_affected = conn.execute(
                "UPDATE accounts SET auth_data = ?1, updated_at = ?2 WHERE id = ?3",
                params![encrypted, now, account_id],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            if rows_affected == 0 {
                return Err(PebbleError::Storage(format!(
                    "account not found: {account_id}"
                )));
            }
            Ok(())
        })
    }

    /// Retrieve encrypted auth data for an account.
    pub fn get_auth_data(&self, account_id: &str) -> Result<Option<Vec<u8>>> {
        self.with_read(|conn| {
            let result: Option<Option<Vec<u8>>> = conn
                .query_row(
                    "SELECT auth_data FROM accounts WHERE id = ?1",
                    params![account_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(result.flatten())
        })
    }

    /// Clear auth data for an account.
    pub fn clear_auth_data(&self, account_id: &str) -> Result<()> {
        self.with_write(|conn| {
            let now = pebble_core::now_timestamp();
            conn.execute(
                "UPDATE accounts SET auth_data = NULL, updated_at = ?1 WHERE id = ?2",
                params![now, account_id],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use pebble_core::{Account, ProviderType, new_id, now_timestamp};

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
    fn test_set_and_get_auth_data() {
        let store = crate::Store::open_in_memory().unwrap();
        let account = test_account();
        store.insert_account(&account).unwrap();

        let data = b"encrypted credentials here";
        store.set_auth_data(&account.id, data).unwrap();

        let fetched = store.get_auth_data(&account.id).unwrap();
        assert_eq!(fetched, Some(data.to_vec()));
    }

    #[test]
    fn test_get_auth_data_returns_none() {
        let store = crate::Store::open_in_memory().unwrap();
        let account = test_account();
        store.insert_account(&account).unwrap();

        let fetched = store.get_auth_data(&account.id).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_clear_auth_data() {
        let store = crate::Store::open_in_memory().unwrap();
        let account = test_account();
        store.insert_account(&account).unwrap();

        store.set_auth_data(&account.id, b"secret").unwrap();
        store.clear_auth_data(&account.id).unwrap();

        let fetched = store.get_auth_data(&account.id).unwrap();
        assert!(fetched.is_none());
    }
}
