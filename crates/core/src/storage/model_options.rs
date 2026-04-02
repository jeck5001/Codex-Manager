use rusqlite::params;

use super::{ModelOptionsCacheRecord, Storage};

impl Storage {
    /// 函数 `upsert_model_options_cache`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - scope: 参数 scope
    /// - items_json: 参数 items_json
    /// - updated_at: 参数 updated_at
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn upsert_model_options_cache(
        &self,
        scope: &str,
        items_json: &str,
        updated_at: i64,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO model_options_cache (scope, items_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(scope) DO UPDATE SET
               items_json = excluded.items_json,
               updated_at = excluded.updated_at",
            params![scope, items_json, updated_at],
        )?;
        Ok(())
    }

    /// 函数 `get_model_options_cache`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - scope: 参数 scope
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn get_model_options_cache(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Option<ModelOptionsCacheRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT scope, items_json, updated_at
             FROM model_options_cache
             WHERE scope = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([scope])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(ModelOptionsCacheRecord {
                scope: row.get(0)?,
                items_json: row.get(1)?,
                updated_at: row.get(2)?,
            }));
        }
        Ok(None)
    }
}
