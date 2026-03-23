use rusqlite::{params, params_from_iter, types::Value, Result};

use super::{AuditLog, AuditLogFilterInput, Storage};

impl Storage {
    pub fn insert_audit_log(&self, item: &AuditLog) -> Result<i64> {
        self.ensure_audit_logs_table()?;
        self.conn.execute(
            "INSERT INTO audit_logs (action, object_type, object_id, operator, changes_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                item.action,
                item.object_type,
                item.object_id,
                item.operator,
                item.changes_json,
                item.created_at
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn count_audit_logs_filtered(&self, filters: AuditLogFilterInput<'_>) -> Result<i64> {
        self.ensure_audit_logs_table()?;
        let mut params = Vec::new();
        let where_clause = build_audit_log_where_clause(&filters, &mut params);
        let sql = format!("SELECT COUNT(1) FROM audit_logs{where_clause}");
        self.conn
            .query_row(&sql, params_from_iter(params), |row| row.get(0))
    }

    pub fn list_audit_logs_paginated_filtered(
        &self,
        filters: AuditLogFilterInput<'_>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<AuditLog>> {
        self.ensure_audit_logs_table()?;
        let mut params = Vec::new();
        let where_clause = build_audit_log_where_clause(&filters, &mut params);
        let sql = format!(
            "SELECT id, action, object_type, object_id, operator, changes_json, created_at
             FROM audit_logs
             {where_clause}
             ORDER BY created_at DESC, id DESC
             LIMIT ?{} OFFSET ?{}",
            params.len() + 1,
            params.len() + 2
        );
        params.push(Value::Integer(limit.max(1)));
        params.push(Value::Integer(offset.max(0)));

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(AuditLog {
                id: row.get(0)?,
                action: row.get(1)?,
                object_type: row.get(2)?,
                object_id: row.get(3)?,
                operator: row.get(4)?,
                changes_json: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub(super) fn ensure_audit_logs_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                object_type TEXT NOT NULL,
                object_id TEXT,
                operator TEXT NOT NULL,
                changes_json TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at
                ON audit_logs(created_at DESC, id DESC);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_action_created_at
                ON audit_logs(action, created_at DESC, id DESC);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_object_type_created_at
                ON audit_logs(object_type, created_at DESC, id DESC);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_object_id_created_at
                ON audit_logs(object_id, created_at DESC, id DESC);",
        )?;
        Ok(())
    }
}

fn push_optional_text_filter(
    clauses: &mut Vec<&'static str>,
    params: &mut Vec<Value>,
    sql: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        clauses.push(sql);
        params.push(Value::Text(value.to_string()));
    }
}

fn build_audit_log_where_clause(
    filters: &AuditLogFilterInput<'_>,
    params: &mut Vec<Value>,
) -> String {
    let mut clauses = Vec::new();
    push_optional_text_filter(&mut clauses, params, "action = ?", filters.action);
    push_optional_text_filter(&mut clauses, params, "object_type = ?", filters.object_type);
    push_optional_text_filter(&mut clauses, params, "object_id = ?", filters.object_id);

    if let Some(value) = filters.time_from {
        clauses.push("created_at >= ?");
        params.push(Value::Integer(value));
    }
    if let Some(value) = filters.time_to {
        clauses.push("created_at <= ?");
        params.push(Value::Integer(value));
    }

    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{now_ts, Storage};

    #[test]
    fn audit_logs_support_insert_filter_and_pagination() {
        let storage = Storage::open_in_memory().expect("open");
        storage.init().expect("init");

        let now = now_ts();
        storage
            .insert_audit_log(&AuditLog {
                id: 0,
                action: "update".to_string(),
                object_type: "account".to_string(),
                object_id: Some("acc-1".to_string()),
                operator: "desktop-app".to_string(),
                changes_json: "{\"after\":{\"status\":\"active\"}}".to_string(),
                created_at: now,
            })
            .expect("insert first audit");
        storage
            .insert_audit_log(&AuditLog {
                id: 0,
                action: "delete".to_string(),
                object_type: "apikey".to_string(),
                object_id: Some("gk_1".to_string()),
                operator: "web-ui".to_string(),
                changes_json: "{\"before\":{\"status\":\"active\"}}".to_string(),
                created_at: now + 1,
            })
            .expect("insert second audit");

        let count = storage
            .count_audit_logs_filtered(AuditLogFilterInput {
                action: Some("update"),
                object_type: Some("account"),
                object_id: Some("acc-1"),
                time_from: Some(now),
                time_to: Some(now),
            })
            .expect("count audit logs");
        assert_eq!(count, 1);

        let rows = storage
            .list_audit_logs_paginated_filtered(
                AuditLogFilterInput {
                    action: None,
                    object_type: None,
                    object_id: None,
                    time_from: None,
                    time_to: None,
                },
                0,
                1,
            )
            .expect("list audit logs");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "delete");
        assert_eq!(rows[0].object_type, "apikey");
    }
}
