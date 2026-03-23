use rusqlite::{params, OptionalExtension, Result};

use super::{PluginRecord, Storage};

impl Storage {
    pub fn list_plugins(&self) -> Result<Vec<PluginRecord>> {
        self.ensure_plugins_table()?;
        let mut stmt = self.conn.prepare(
            "SELECT
                id,
                name,
                description,
                runtime,
                hook_points_json,
                script_content,
                enabled,
                timeout_ms,
                created_at,
                updated_at
             FROM plugins
             ORDER BY updated_at DESC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PluginRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                runtime: row.get(3)?,
                hook_points_json: row.get(4)?,
                script_content: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
                timeout_ms: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn find_plugin_by_id(&self, plugin_id: &str) -> Result<Option<PluginRecord>> {
        self.ensure_plugins_table()?;
        self.conn
            .query_row(
                "SELECT
                    id,
                    name,
                    description,
                    runtime,
                    hook_points_json,
                    script_content,
                    enabled,
                    timeout_ms,
                    created_at,
                    updated_at
                 FROM plugins
                 WHERE id = ?1
                 LIMIT 1",
                [plugin_id],
                |row| {
                    Ok(PluginRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        runtime: row.get(3)?,
                        hook_points_json: row.get(4)?,
                        script_content: row.get(5)?,
                        enabled: row.get::<_, i64>(6)? != 0,
                        timeout_ms: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
            .optional()
    }

    pub fn upsert_plugin(&self, plugin: &PluginRecord) -> Result<()> {
        self.ensure_plugins_table()?;
        self.conn.execute(
            "INSERT INTO plugins (
                id,
                name,
                description,
                runtime,
                hook_points_json,
                script_content,
                enabled,
                timeout_ms,
                created_at,
                updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                runtime = excluded.runtime,
                hook_points_json = excluded.hook_points_json,
                script_content = excluded.script_content,
                enabled = excluded.enabled,
                timeout_ms = excluded.timeout_ms,
                updated_at = excluded.updated_at",
            params![
                plugin.id,
                plugin.name,
                plugin.description,
                plugin.runtime,
                plugin.hook_points_json,
                plugin.script_content,
                if plugin.enabled { 1 } else { 0 },
                plugin.timeout_ms,
                plugin.created_at,
                plugin.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn delete_plugin(&self, plugin_id: &str) -> Result<()> {
        self.ensure_plugins_table()?;
        self.conn
            .execute("DELETE FROM plugins WHERE id = ?1", [plugin_id])?;
        Ok(())
    }

    pub(super) fn ensure_plugins_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                runtime TEXT NOT NULL DEFAULT 'lua',
                hook_points_json TEXT NOT NULL DEFAULT '[]',
                script_content TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                timeout_ms INTEGER NOT NULL DEFAULT 100,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_plugins_enabled ON plugins(enabled, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_plugins_runtime ON plugins(runtime, updated_at DESC);",
        )?;
        Ok(())
    }
}
