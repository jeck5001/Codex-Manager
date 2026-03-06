use rusqlite::{params_from_iter, types::Value, Result, Row};

use super::{now_ts, Account, Storage};

impl Storage {
    pub fn insert_account(&self, account: &Account) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO accounts (id, label, issuer, chatgpt_account_id, workspace_id, group_name, sort, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            (
                &account.id,
                &account.label,
                &account.issuer,
                &account.chatgpt_account_id,
                &account.workspace_id,
                &account.group_name,
                account.sort,
                &account.status,
                account.created_at,
                account.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn account_count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(1) FROM accounts", [], |row| row.get(0))
    }

    pub fn account_count_filtered(
        &self,
        query: Option<&str>,
        group_name: Option<&str>,
    ) -> Result<i64> {
        let mut params = Vec::new();
        let where_clause = build_account_where_clause(query, group_name, &mut params);
        let sql = format!("SELECT COUNT(1) FROM accounts{where_clause}");
        self.conn
            .query_row(&sql, params_from_iter(params), |row| row.get(0))
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        self.list_accounts_filtered(None, None)
    }

    pub fn list_accounts_filtered(
        &self,
        query: Option<&str>,
        group_name: Option<&str>,
    ) -> Result<Vec<Account>> {
        self.query_accounts(query, group_name, None)
    }

    pub fn list_accounts_paginated(
        &self,
        query: Option<&str>,
        group_name: Option<&str>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Account>> {
        self.query_accounts(query, group_name, Some((offset, limit)))
    }

    pub fn find_account_by_id(&self, account_id: &str) -> Result<Option<Account>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, issuer, chatgpt_account_id, workspace_id, group_name, sort, status, created_at, updated_at
             FROM accounts
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([account_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_account_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn update_account_sort(&self, account_id: &str, sort: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET sort = ?1, updated_at = ?2 WHERE id = ?3",
            (sort, now_ts(), account_id),
        )?;
        Ok(())
    }

    pub fn update_account_status(&self, account_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET status = ?1, updated_at = ?2 WHERE id = ?3",
            (status, now_ts(), account_id),
        )?;
        Ok(())
    }

    pub fn update_account_status_if_changed(&self, account_id: &str, status: &str) -> Result<bool> {
        let updated = self.conn.execute(
            "UPDATE accounts SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status != ?1",
            (status, now_ts(), account_id),
        )?;
        Ok(updated > 0)
    }

    pub fn delete_account(&mut self, account_id: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM tokens WHERE account_id = ?1", [account_id])?;
        tx.execute(
            "DELETE FROM usage_snapshots WHERE account_id = ?1",
            [account_id],
        )?;
        tx.execute("DELETE FROM events WHERE account_id = ?1", [account_id])?;
        tx.execute("DELETE FROM accounts WHERE id = ?1", [account_id])?;
        tx.commit()?;
        Ok(())
    }

    pub(super) fn ensure_account_meta_columns(&self) -> Result<()> {
        self.ensure_column("accounts", "chatgpt_account_id", "TEXT")?;
        self.ensure_column("accounts", "group_name", "TEXT")?;
        self.ensure_column("accounts", "sort", "INTEGER DEFAULT 0")?;
        self.ensure_column("login_sessions", "note", "TEXT")?;
        self.ensure_column("login_sessions", "tags", "TEXT")?;
        self.ensure_column("login_sessions", "group_name", "TEXT")?;
        Ok(())
    }

    fn query_accounts(
        &self,
        query: Option<&str>,
        group_name: Option<&str>,
        pagination: Option<(i64, i64)>,
    ) -> Result<Vec<Account>> {
        let mut params = Vec::new();
        let where_clause = build_account_where_clause(query, group_name, &mut params);
        let mut sql = format!(
            "SELECT id, label, issuer, chatgpt_account_id, workspace_id, group_name, sort, status, created_at, updated_at FROM accounts{where_clause} ORDER BY sort ASC, updated_at DESC"
        );

        if let Some((offset, limit)) = pagination {
            sql.push_str(" LIMIT ? OFFSET ?");
            params.push(Value::Integer(limit.max(1)));
            params.push(Value::Integer(offset.max(0)));
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params_from_iter(params))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_account_row(row)?);
        }
        Ok(out)
    }
}

fn normalize_optional_filter(value: Option<&str>) -> Option<String> {
    let trimmed = value.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn build_account_where_clause(
    query: Option<&str>,
    group_name: Option<&str>,
    params: &mut Vec<Value>,
) -> String {
    let mut clauses = Vec::new();

    if let Some(keyword) = normalize_optional_filter(query) {
        let pattern = format!("%{keyword}%");
        clauses.push("(LOWER(label) LIKE LOWER(?) OR LOWER(id) LIKE LOWER(?))".to_string());
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern));
    }

    if let Some(group) = normalize_optional_filter(group_name) {
        clauses.push("group_name = ?".to_string());
        params.push(Value::Text(group));
    }

    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

fn map_account_row(row: &Row<'_>) -> Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        label: row.get(1)?,
        issuer: row.get(2)?,
        chatgpt_account_id: row.get(3)?,
        workspace_id: row.get(4)?,
        group_name: row.get(5)?,
        sort: row.get(6)?,
        status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
