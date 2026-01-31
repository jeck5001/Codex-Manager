use rusqlite::{Connection, Result};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub issuer: String,
    pub chatgpt_account_id: Option<String>,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
    pub note: Option<String>,
    pub tags: Option<String>,
    pub group_name: Option<String>,
    pub sort: i64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub account_id: String,
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub api_key_access_token: Option<String>,
    pub last_refresh: i64,
}

#[derive(Debug, Clone)]
pub struct LoginSession {
    pub login_id: String,
    pub code_verifier: String,
    pub state: String,
    pub status: String,
    pub error: Option<String>,
    pub note: Option<String>,
    pub tags: Option<String>,
    pub group_name: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct UsageSnapshotRecord {
    pub account_id: String,
    pub used_percent: Option<f64>,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<i64>,
    pub secondary_used_percent: Option<f64>,
    pub secondary_window_minutes: Option<i64>,
    pub secondary_resets_at: Option<i64>,
    pub credits_json: Option<String>,
    pub captured_at: i64,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub account_id: Option<String>,
    pub event_type: String,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: String,
    pub name: Option<String>,
    pub key_hash: String,
    pub status: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug)]
pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    pub fn init(&self) -> Result<()> {
        let sql = include_str!("../../migrations/001_init.sql");
        self.conn.execute_batch(sql)?;
        let sql_login = include_str!("../../migrations/002_login_sessions.sql");
        self.conn.execute_batch(sql_login)?;
        let sql_api_keys = include_str!("../../migrations/003_api_keys.sql");
        self.conn.execute_batch(sql_api_keys)?;
        self.ensure_account_meta_columns()?;
        self.ensure_usage_secondary_columns()?;
        self.ensure_token_api_key_column()
    }

    pub fn insert_account(&self, account: &Account) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO accounts (id, label, issuer, chatgpt_account_id, workspace_id, workspace_name, note, tags, group_name, sort, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            (
                &account.id,
                &account.label,
                &account.issuer,
                &account.chatgpt_account_id,
                &account.workspace_id,
                &account.workspace_name,
                &account.note,
                &account.tags,
                &account.group_name,
                account.sort,
                &account.status,
                account.created_at,
                account.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn insert_token(&self, token: &Token) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO tokens (account_id, id_token, access_token, refresh_token, api_key_access_token, last_refresh) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &token.account_id,
                &token.id_token,
                &token.access_token,
                &token.refresh_token,
                &token.api_key_access_token,
                token.last_refresh,
            ),
        )?;
        Ok(())
    }

    pub fn account_count(&self) -> Result<i64> {
        self.conn.query_row("SELECT COUNT(1) FROM accounts", [], |row| row.get(0))
    }

    pub fn token_count(&self) -> Result<i64> {
        self.conn.query_row("SELECT COUNT(1) FROM tokens", [], |row| row.get(0))
    }

    pub fn insert_usage_snapshot(&self, snap: &UsageSnapshotRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO usage_snapshots (account_id, used_percent, window_minutes, resets_at, secondary_used_percent, secondary_window_minutes, secondary_resets_at, credits_json, captured_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                &snap.account_id,
                snap.used_percent,
                snap.window_minutes,
                snap.resets_at,
                snap.secondary_used_percent,
                snap.secondary_window_minutes,
                snap.secondary_resets_at,
                &snap.credits_json,
                snap.captured_at,
            ),
        )?;
        Ok(())
    }

    pub fn latest_usage_snapshot(&self) -> Result<Option<UsageSnapshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, used_percent, window_minutes, resets_at, secondary_used_percent, secondary_window_minutes, secondary_resets_at, credits_json, captured_at FROM usage_snapshots ORDER BY captured_at DESC, id DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(UsageSnapshotRecord {
                account_id: row.get(0)?,
                used_percent: row.get(1)?,
                window_minutes: row.get(2)?,
                resets_at: row.get(3)?,
                secondary_used_percent: row.get(4)?,
                secondary_window_minutes: row.get(5)?,
                secondary_resets_at: row.get(6)?,
                credits_json: row.get(7)?,
                captured_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, issuer, chatgpt_account_id, workspace_id, workspace_name, note, tags, group_name, sort, status, created_at, updated_at FROM accounts ORDER BY sort ASC, updated_at DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Account {
                id: row.get(0)?,
                label: row.get(1)?,
                issuer: row.get(2)?,
                chatgpt_account_id: row.get(3)?,
                workspace_id: row.get(4)?,
                workspace_name: row.get(5)?,
                note: row.get(6)?,
                tags: row.get(7)?,
                group_name: row.get(8)?,
                sort: row.get(9)?,
                status: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            });
        }
        Ok(out)
    }

    pub fn list_tokens(&self) -> Result<Vec<Token>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, id_token, access_token, refresh_token, api_key_access_token, last_refresh FROM tokens",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Token {
                account_id: row.get(0)?,
                id_token: row.get(1)?,
                access_token: row.get(2)?,
                refresh_token: row.get(3)?,
                api_key_access_token: row.get(4)?,
                last_refresh: row.get(5)?,
            });
        }
        Ok(out)
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

    pub fn insert_api_key(&self, key: &ApiKey) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO api_keys (id, name, key_hash, status, created_at, last_used_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &key.id,
                &key.name,
                &key.key_hash,
                &key.status,
                key.created_at,
                &key.last_used_at,
            ),
        )?;
        Ok(())
    }

    pub fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, key_hash, status, created_at, last_used_at FROM api_keys ORDER BY created_at DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(ApiKey {
                id: row.get(0)?,
                name: row.get(1)?,
                key_hash: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                last_used_at: row.get(5)?,
            });
        }
        Ok(out)
    }

    pub fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, key_hash, status, created_at, last_used_at FROM api_keys WHERE key_hash = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query([key_hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ApiKey {
                id: row.get(0)?,
                name: row.get(1)?,
                key_hash: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                last_used_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_api_key_last_used(&self, key_hash: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE api_keys SET last_used_at = ?1 WHERE key_hash = ?2",
            (now_ts(), key_hash),
        )?;
        Ok(())
    }

    pub fn update_api_key_status(&self, key_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE api_keys SET status = ?1 WHERE id = ?2",
            (status, key_id),
        )?;
        Ok(())
    }

    pub fn delete_api_key(&self, key_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM api_keys WHERE id = ?1", [key_id])?;
        Ok(())
    }

    pub fn insert_event(&self, event: &Event) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (account_id, type, message, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                &event.account_id,
                &event.event_type,
                &event.message,
                event.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn event_count(&self) -> Result<i64> {
        self.conn.query_row("SELECT COUNT(1) FROM events", [], |row| row.get(0))
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

    pub fn latest_usage_snapshots_by_account(&self) -> Result<Vec<UsageSnapshotRecord>> {
        // Latest per account by captured_at; ties broken by highest id.
        let mut stmt = self.conn.prepare(
            "SELECT account_id, used_percent, window_minutes, resets_at, secondary_used_percent, secondary_window_minutes, secondary_resets_at, credits_json, captured_at FROM usage_snapshots WHERE id IN (SELECT MAX(s2.id) FROM usage_snapshots s2 JOIN (SELECT account_id, MAX(captured_at) AS max_captured FROM usage_snapshots GROUP BY account_id) latest ON latest.account_id = s2.account_id AND latest.max_captured = s2.captured_at GROUP BY s2.account_id) ORDER BY captured_at DESC, id DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(UsageSnapshotRecord {
                account_id: row.get(0)?,
                used_percent: row.get(1)?,
                window_minutes: row.get(2)?,
                resets_at: row.get(3)?,
                secondary_used_percent: row.get(4)?,
                secondary_window_minutes: row.get(5)?,
                secondary_resets_at: row.get(6)?,
                credits_json: row.get(7)?,
                captured_at: row.get(8)?,
            });
        }
        Ok(out)
    }

    pub fn insert_login_session(&self, session: &LoginSession) -> Result<()> {
        self.conn.execute(
            "INSERT INTO login_sessions (login_id, code_verifier, state, status, error, note, tags, group_name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            (
                &session.login_id,
                &session.code_verifier,
                &session.state,
                &session.status,
                &session.error,
                &session.note,
                &session.tags,
                &session.group_name,
                session.created_at,
                session.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn get_login_session(&self, login_id: &str) -> Result<Option<LoginSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT login_id, code_verifier, state, status, error, note, tags, group_name, created_at, updated_at FROM login_sessions WHERE login_id = ?1",
        )?;
        let mut rows = stmt.query([login_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(LoginSession {
                login_id: row.get(0)?,
                code_verifier: row.get(1)?,
                state: row.get(2)?,
                status: row.get(3)?,
                error: row.get(4)?,
                note: row.get(5)?,
                tags: row.get(6)?,
                group_name: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_login_session_status(
        &self,
        login_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE login_sessions SET status = ?1, error = ?2, updated_at = ?3 WHERE login_id = ?4",
            (status, error, now_ts(), login_id),
        )?;
        Ok(())
    }

    fn ensure_token_api_key_column(&self) -> Result<()> {
        if self.has_column("tokens", "api_key_access_token")? {
            return Ok(());
        }
        self.conn.execute(
            "ALTER TABLE tokens ADD COLUMN api_key_access_token TEXT",
            [],
        )?;
        Ok(())
    }

    fn ensure_account_meta_columns(&self) -> Result<()> {
        self.ensure_column("accounts", "chatgpt_account_id", "TEXT")?;
        self.ensure_column("accounts", "note", "TEXT")?;
        self.ensure_column("accounts", "tags", "TEXT")?;
        self.ensure_column("accounts", "group_name", "TEXT")?;
        self.ensure_column("accounts", "workspace_name", "TEXT")?;
        self.ensure_column("accounts", "sort", "INTEGER DEFAULT 0")?;
        self.ensure_column("login_sessions", "note", "TEXT")?;
        self.ensure_column("login_sessions", "tags", "TEXT")?;
        self.ensure_column("login_sessions", "group_name", "TEXT")?;
        Ok(())
    }

    fn ensure_usage_secondary_columns(&self) -> Result<()> {
        self.ensure_column("usage_snapshots", "secondary_used_percent", "REAL")?;
        self.ensure_column("usage_snapshots", "secondary_window_minutes", "INTEGER")?;
        self.ensure_column("usage_snapshots", "secondary_resets_at", "INTEGER")?;
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, column_type: &str) -> Result<()> {
        if self.has_column(table, column)? {
            return Ok(());
        }
        let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}");
        self.conn.execute(&sql, [])?;
        Ok(())
    }

    fn has_column(&self, table: &str, column: &str) -> Result<bool> {
        let sql = format!("PRAGMA table_info({table})");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
