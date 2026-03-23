use rusqlite::{params, OptionalExtension, Result};

use super::{now_ts, AlertChannel, AlertHistoryEntry, AlertRule, Storage};

impl Storage {
    pub fn list_alert_rules(&self) -> Result<Vec<AlertRule>> {
        self.ensure_alerting_tables()?;
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, config_json, enabled, created_at, updated_at
             FROM alert_rules
             ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AlertRule {
                id: row.get(0)?,
                name: row.get(1)?,
                rule_type: row.get(2)?,
                config_json: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn find_alert_rule_by_id(&self, rule_id: &str) -> Result<Option<AlertRule>> {
        self.ensure_alerting_tables()?;
        self.conn
            .query_row(
                "SELECT id, name, type, config_json, enabled, created_at, updated_at
                 FROM alert_rules
                 WHERE id = ?1
                 LIMIT 1",
                [rule_id],
                |row| {
                    Ok(AlertRule {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        rule_type: row.get(2)?,
                        config_json: row.get(3)?,
                        enabled: row.get::<_, i64>(4)? != 0,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
    }

    pub fn upsert_alert_rule(&self, rule: &AlertRule) -> Result<()> {
        self.ensure_alerting_tables()?;
        self.conn.execute(
            "INSERT INTO alert_rules (id, name, type, config_json, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               type = excluded.type,
               config_json = excluded.config_json,
               enabled = excluded.enabled,
               updated_at = excluded.updated_at",
            params![
                rule.id,
                rule.name,
                rule.rule_type,
                rule.config_json,
                if rule.enabled { 1 } else { 0 },
                rule.created_at,
                rule.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn delete_alert_rule(&self, rule_id: &str) -> Result<()> {
        self.ensure_alerting_tables()?;
        self.conn
            .execute("DELETE FROM alert_rules WHERE id = ?1", [rule_id])?;
        Ok(())
    }

    pub fn list_alert_channels(&self) -> Result<Vec<AlertChannel>> {
        self.ensure_alerting_tables()?;
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, config_json, enabled, created_at, updated_at
             FROM alert_channels
             ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AlertChannel {
                id: row.get(0)?,
                name: row.get(1)?,
                channel_type: row.get(2)?,
                config_json: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn find_alert_channel_by_id(&self, channel_id: &str) -> Result<Option<AlertChannel>> {
        self.ensure_alerting_tables()?;
        self.conn
            .query_row(
                "SELECT id, name, type, config_json, enabled, created_at, updated_at
                 FROM alert_channels
                 WHERE id = ?1
                 LIMIT 1",
                [channel_id],
                |row| {
                    Ok(AlertChannel {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        channel_type: row.get(2)?,
                        config_json: row.get(3)?,
                        enabled: row.get::<_, i64>(4)? != 0,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
    }

    pub fn upsert_alert_channel(&self, channel: &AlertChannel) -> Result<()> {
        self.ensure_alerting_tables()?;
        self.conn.execute(
            "INSERT INTO alert_channels (id, name, type, config_json, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               type = excluded.type,
               config_json = excluded.config_json,
               enabled = excluded.enabled,
               updated_at = excluded.updated_at",
            params![
                channel.id,
                channel.name,
                channel.channel_type,
                channel.config_json,
                if channel.enabled { 1 } else { 0 },
                channel.created_at,
                channel.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn delete_alert_channel(&self, channel_id: &str) -> Result<()> {
        self.ensure_alerting_tables()?;
        self.conn
            .execute("DELETE FROM alert_channels WHERE id = ?1", [channel_id])?;
        Ok(())
    }

    pub fn insert_alert_history(
        &self,
        rule_id: Option<&str>,
        channel_id: Option<&str>,
        status: &str,
        message: &str,
    ) -> Result<i64> {
        self.ensure_alerting_tables()?;
        self.conn.execute(
            "INSERT INTO alert_history (rule_id, channel_id, status, message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rule_id, channel_id, status, message, now_ts()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_alert_history(&self, limit: i64) -> Result<Vec<AlertHistoryEntry>> {
        self.ensure_alerting_tables()?;
        let mut stmt = self.conn.prepare(
            "SELECT
                h.id,
                h.rule_id,
                r.name,
                h.channel_id,
                c.name,
                h.status,
                h.message,
                h.created_at
             FROM alert_history h
             LEFT JOIN alert_rules r ON r.id = h.rule_id
             LEFT JOIN alert_channels c ON c.id = h.channel_id
             ORDER BY h.created_at DESC, h.id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |row| {
            Ok(AlertHistoryEntry {
                id: row.get(0)?,
                rule_id: row.get(1)?,
                rule_name: row.get(2)?,
                channel_id: row.get(3)?,
                channel_name: row.get(4)?,
                status: row.get(5)?,
                message: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub(super) fn ensure_alerting_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS alert_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                config_json TEXT NOT NULL DEFAULT '{}',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled ON alert_rules(enabled, updated_at DESC);

            CREATE TABLE IF NOT EXISTS alert_channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                config_json TEXT NOT NULL DEFAULT '{}',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_alert_channels_enabled ON alert_channels(enabled, updated_at DESC);

            CREATE TABLE IF NOT EXISTS alert_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                rule_id TEXT,
                channel_id TEXT,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_alert_history_created_at ON alert_history(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_alert_history_rule_id ON alert_history(rule_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_alert_history_channel_id ON alert_history(channel_id, created_at DESC);",
        )?;
        Ok(())
    }
}
