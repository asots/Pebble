use pebble_core::{PebbleError, Result, Rule};
use rusqlite::params;

use crate::Store;

fn row_to_rule(row: &rusqlite::Row) -> rusqlite::Result<Rule> {
    let is_enabled: i32 = row.get(5)?;
    Ok(Rule {
        id: row.get(0)?,
        name: row.get(1)?,
        priority: row.get(2)?,
        conditions: row.get(3)?,
        actions: row.get(4)?,
        is_enabled: is_enabled != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

impl Store {
    pub fn insert_rule(&self, rule: &Rule) -> Result<()> {
        self.with_write(|conn| {
            conn.execute(
                "INSERT INTO rules (id, name, priority, conditions, actions, is_enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    rule.id,
                    rule.name,
                    rule.priority,
                    rule.conditions,
                    rule.actions,
                    rule.is_enabled as i32,
                    rule.created_at,
                    rule.updated_at,
                ],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    pub fn list_rules(&self) -> Result<Vec<Rule>> {
        self.with_read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, priority, conditions, actions, is_enabled, created_at, updated_at
                     FROM rules ORDER BY priority ASC",
                )
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map([], row_to_rule)
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            let mut rules = Vec::new();
            for row in rows {
                rules.push(row.map_err(|e| PebbleError::Storage(e.to_string()))?);
            }
            Ok(rules)
        })
    }

    pub fn update_rule(&self, rule: &Rule) -> Result<()> {
        self.with_write(|conn| {
            conn.execute(
                "UPDATE rules SET name = ?1, priority = ?2, conditions = ?3, actions = ?4,
                 is_enabled = ?5, updated_at = ?6 WHERE id = ?7",
                params![
                    rule.name,
                    rule.priority,
                    rule.conditions,
                    rule.actions,
                    rule.is_enabled as i32,
                    rule.updated_at,
                    rule.id,
                ],
            )
            .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    pub fn delete_rule(&self, id: &str) -> Result<()> {
        self.with_write(|conn| {
            conn.execute("DELETE FROM rules WHERE id = ?1", params![id])
                .map_err(|e| PebbleError::Storage(e.to_string()))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    #[test]
    fn test_rule_crud() {
        let store = Store::open_in_memory().unwrap();
        let now = pebble_core::now_timestamp();

        let rule = Rule {
            id: pebble_core::new_id(),
            name: "Auto-archive".to_string(),
            priority: 10,
            conditions: r#"{"from": "noreply@example.com"}"#.to_string(),
            actions: r#"["archive"]"#.to_string(),
            is_enabled: true,
            created_at: now,
            updated_at: now,
        };
        store.insert_rule(&rule).unwrap();

        let rules = store.list_rules().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "Auto-archive");
        assert!(rules[0].is_enabled);

        // Insert a second rule with higher priority (lower number = higher priority)
        let rule2 = Rule {
            id: pebble_core::new_id(),
            name: "Urgent".to_string(),
            priority: 1,
            conditions: r#"{"subject": "URGENT"}"#.to_string(),
            actions: r#"["star"]"#.to_string(),
            is_enabled: true,
            created_at: now,
            updated_at: now,
        };
        store.insert_rule(&rule2).unwrap();

        let rules = store.list_rules().unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "Urgent"); // priority 1 first
        assert_eq!(rules[1].name, "Auto-archive"); // priority 10 second

        // Update
        let mut updated = rule.clone();
        updated.name = "Auto-archive v2".to_string();
        updated.is_enabled = false;
        updated.updated_at = now + 1;
        store.update_rule(&updated).unwrap();

        let rules = store.list_rules().unwrap();
        let found = rules.iter().find(|r| r.id == rule.id).unwrap();
        assert_eq!(found.name, "Auto-archive v2");
        assert!(!found.is_enabled);

        // Delete
        store.delete_rule(&rule.id).unwrap();
        let rules = store.list_rules().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "Urgent");
    }
}
