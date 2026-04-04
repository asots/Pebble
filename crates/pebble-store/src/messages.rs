use pebble_core::{Message, PebbleError, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::Store;

/// Maps a row to a Message. Column order must match the SELECT lists used below.
///
/// Expected column indices:
/// 0=id, 1=account_id, 2=remote_id, 3=message_id_header, 4=in_reply_to,
/// 5=references_header, 6=thread_id, 7=subject, 8=snippet, 9=from_address,
/// 10=from_name, 11=to_list, 12=cc_list, 13=bcc_list,
/// 14=body_text, 15=body_html_raw,
/// 16=has_attachments, 17=is_read, 18=is_starred, 19=is_draft,
/// 20=date, 21=remote_version, 22=is_deleted, 23=deleted_at, 24=created_at, 25=updated_at
fn row_to_message(row: &Row) -> rusqlite::Result<Message> {
    let to_json: String = row.get(11)?;
    let cc_json: String = row.get(12)?;
    let bcc_json: String = row.get(13)?;
    let has_attachments: i32 = row.get(16)?;
    let is_read: i32 = row.get(17)?;
    let is_starred: i32 = row.get(18)?;
    let is_draft: i32 = row.get(19)?;
    let is_deleted: i32 = row.get(22)?;

    Ok(Message {
        id: row.get(0)?,
        account_id: row.get(1)?,
        remote_id: row.get(2)?,
        message_id_header: row.get(3)?,
        in_reply_to: row.get(4)?,
        references_header: row.get(5)?,
        thread_id: row.get(6)?,
        subject: row.get(7)?,
        snippet: row.get(8)?,
        from_address: row.get(9)?,
        from_name: row.get(10)?,
        to_list: serde_json::from_str(&to_json).unwrap_or_default(),
        cc_list: serde_json::from_str(&cc_json).unwrap_or_default(),
        bcc_list: serde_json::from_str(&bcc_json).unwrap_or_default(),
        body_text: row.get(14)?,
        body_html_raw: row.get(15)?,
        has_attachments: has_attachments != 0,
        is_read: is_read != 0,
        is_starred: is_starred != 0,
        is_draft: is_draft != 0,
        date: row.get(20)?,
        remote_version: row.get(21)?,
        is_deleted: is_deleted != 0,
        deleted_at: row.get(23)?,
        created_at: row.get(24)?,
        updated_at: row.get(25)?,
    })
}

const MSG_SELECT: &str =
    "id, account_id, remote_id, message_id_header, in_reply_to, \
     references_header, thread_id, subject, snippet, from_address, \
     from_name, to_list, cc_list, bcc_list, \
     body_text, body_html_raw, \
     has_attachments, is_read, is_starred, is_draft, \
     date, remote_version, is_deleted, deleted_at, created_at, updated_at";

impl Store {
    pub fn insert_message(&self, msg: &Message, folder_ids: &[String]) -> Result<()> {
        self.with_conn(|conn| {
            let to_json = serde_json::to_string(&msg.to_list)
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let cc_json = serde_json::to_string(&msg.cc_list)
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let bcc_json = serde_json::to_string(&msg.bcc_list)
                .map_err(|e| PebbleError::Storage(e.to_string()))?;

            conn.execute_batch("BEGIN")
                .map_err(|e| PebbleError::Storage(e.to_string()))?;

            let result = (|| -> Result<()> {
                conn.execute(
                    "INSERT INTO messages (id, account_id, remote_id, message_id_header, in_reply_to,
                     references_header, thread_id, subject, snippet, from_address, from_name,
                     to_list, cc_list, bcc_list, body_text, body_html_raw,
                     has_attachments, is_read, is_starred, is_draft,
                     date, remote_version, is_deleted, deleted_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                             ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                             ?21, ?22, ?23, ?24, ?25, ?26)",
                    params![
                        msg.id,
                        msg.account_id,
                        msg.remote_id,
                        msg.message_id_header,
                        msg.in_reply_to,
                        msg.references_header,
                        msg.thread_id,
                        msg.subject,
                        msg.snippet,
                        msg.from_address,
                        msg.from_name,
                        to_json,
                        cc_json,
                        bcc_json,
                        msg.body_text,
                        msg.body_html_raw,
                        msg.has_attachments as i32,
                        msg.is_read as i32,
                        msg.is_starred as i32,
                        msg.is_draft as i32,
                        msg.date,
                        msg.remote_version,
                        msg.is_deleted as i32,
                        msg.deleted_at,
                        msg.created_at,
                        msg.updated_at,
                    ],
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;

                for folder_id in folder_ids {
                    conn.execute(
                        "INSERT INTO message_folders (message_id, folder_id) VALUES (?1, ?2)",
                        params![msg.id, folder_id],
                    )
                    .map_err(|e| PebbleError::Storage(e.to_string()))?;
                }

                Ok(())
            })();

            match result {
                Ok(()) => {
                    conn.execute_batch("COMMIT")
                        .map_err(|e| PebbleError::Storage(e.to_string()))?;
                    Ok(())
                }
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    Err(e)
                }
            }
        })
    }

    pub fn list_messages_by_folder(
        &self,
        folder_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Message>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT m.id, m.account_id, m.remote_id, m.message_id_header, m.in_reply_to,
                     m.references_header, m.thread_id, m.subject, m.snippet, m.from_address,
                     m.from_name, m.to_list, m.cc_list, m.bcc_list,
                     m.body_text, m.body_html_raw,
                     m.has_attachments, m.is_read, m.is_starred, m.is_draft,
                     m.date, m.remote_version, m.is_deleted, m.deleted_at, m.created_at, m.updated_at
                     FROM messages m
                     JOIN message_folders mf ON m.id = mf.message_id
                     WHERE mf.folder_id = ?1 AND m.is_deleted = 0
                     ORDER BY m.date DESC
                     LIMIT ?2 OFFSET ?3",
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![folder_id, limit, offset], row_to_message)
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let mut messages = Vec::new();
            for row in rows {
                messages.push(row.map_err(|e| PebbleError::Storage(e.to_string()))?);
            }
            Ok(messages)
        })
    }

    pub fn get_message(&self, id: &str) -> Result<Option<Message>> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {MSG_SELECT} FROM messages WHERE id = ?1");
            let result = conn
                .query_row(&sql, params![id], row_to_message)
                .optional()
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(result)
        })
    }

    pub fn update_message_flags(
        &self,
        id: &str,
        is_read: Option<bool>,
        is_starred: Option<bool>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            let mut sets = Vec::new();
            let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(read) = is_read {
                sets.push(format!("is_read = ?{}", values.len() + 1));
                values.push(Box::new(read as i32));
            }
            if let Some(starred) = is_starred {
                sets.push(format!("is_starred = ?{}", values.len() + 1));
                values.push(Box::new(starred as i32));
            }

            if sets.is_empty() {
                return Ok(());
            }

            let now = pebble_core::now_timestamp();
            sets.push(format!("updated_at = ?{}", values.len() + 1));
            values.push(Box::new(now));

            let id_idx = values.len() + 1;
            values.push(Box::new(id.to_string()));

            let sql = format!("UPDATE messages SET {} WHERE id = ?{}", sets.join(", "), id_idx);
            let params: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            conn.execute(&sql, params.as_slice())
                .map_err(|e| PebbleError::Storage(e.to_string()))?;

            Ok(())
        })
    }

    pub fn soft_delete_message(&self, id: &str) -> Result<()> {
        self.with_conn(|conn| {
            let now = pebble_core::now_timestamp();
            conn.execute(
                "UPDATE messages SET is_deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
                params![now, id],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    /// Check whether a message with the given `remote_id` exists for this account.
    pub fn has_message_by_remote_id(&self, account_id: &str, remote_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM messages WHERE account_id = ?1 AND remote_id = ?2",
                    params![account_id, remote_id],
                    |row| row.get(0),
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(count > 0)
        })
    }

    /// Get the maximum remote_id (interpreted as integer) for messages in a folder.
    pub fn get_max_remote_id(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let result: Option<i64> = conn
                .query_row(
                    "SELECT MAX(CAST(m.remote_id AS INTEGER))
                     FROM messages m
                     JOIN message_folders mf ON m.id = mf.message_id
                     WHERE m.account_id = ?1 AND mf.folder_id = ?2 AND m.is_deleted = 0",
                    params![account_id, folder_id],
                    |row| row.get(0),
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(result.map(|v| v.to_string()))
        })
    }

    /// Get all (message_id_header, thread_id) pairs for an account where both are present.
    pub fn get_thread_mappings(
        &self,
        account_id: &str,
    ) -> Result<Vec<(String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT message_id_header, thread_id
                     FROM messages
                     WHERE account_id = ?1
                       AND message_id_header IS NOT NULL
                       AND thread_id IS NOT NULL
                       AND is_deleted = 0",
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![account_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| PebbleError::Storage(e.to_string()))?);
            }
            Ok(results)
        })
    }
}
