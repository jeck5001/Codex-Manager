use rusqlite::Result;

use super::{
    ApiKeyTokenUsageSummary, CacheSummaryDayRow, CacheSummaryKeyRow, CacheSummaryModelRow,
    CacheSummaryRow, ConsumerDayRow, ConsumerModelRow, ConsumerOverviewRow, CostSummaryDayRow,
    CostSummaryKeyRow, CostSummaryModelRow, CostUsageSummary, RequestLogTodaySummary,
    RequestTokenStat, Storage,
};

const NON_NEGATIVE_TOTAL_TOKENS_SQL: &str = "IFNULL(
    SUM(
        CASE
            WHEN total_tokens IS NOT NULL THEN
                CASE WHEN total_tokens > 0 THEN total_tokens ELSE 0 END
            ELSE
                CASE
                    WHEN IFNULL(input_tokens, 0) - IFNULL(cached_input_tokens, 0) + IFNULL(output_tokens, 0) > 0
                        THEN IFNULL(input_tokens, 0) - IFNULL(cached_input_tokens, 0) + IFNULL(output_tokens, 0)
                    ELSE 0
                END
        END
    ),
    0
)";

impl Storage {
    pub fn insert_request_token_stat(&self, stat: &RequestTokenStat) -> Result<()> {
        self.conn.execute(
            "INSERT INTO request_token_stats (
                request_log_id, key_id, account_id, model,
                input_tokens, cached_input_tokens, output_tokens, total_tokens, reasoning_output_tokens,
                estimated_cost_usd, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            (
                stat.request_log_id,
                &stat.key_id,
                &stat.account_id,
                &stat.model,
                stat.input_tokens,
                stat.cached_input_tokens,
                stat.output_tokens,
                stat.total_tokens,
                stat.reasoning_output_tokens,
                stat.estimated_cost_usd,
                stat.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn summarize_request_token_stats_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<RequestLogTodaySummary> {
        let mut stmt = self.conn.prepare(
            "SELECT
                IFNULL(SUM(input_tokens), 0),
                IFNULL(SUM(cached_input_tokens), 0),
                IFNULL(SUM(output_tokens), 0),
                IFNULL(SUM(reasoning_output_tokens), 0),
                IFNULL(SUM(estimated_cost_usd), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2",
        )?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        if let Some(row) = rows.next()? {
            return Ok(RequestLogTodaySummary {
                input_tokens: row.get(0)?,
                cached_input_tokens: row.get(1)?,
                output_tokens: row.get(2)?,
                reasoning_output_tokens: row.get(3)?,
                estimated_cost_usd: row.get(4)?,
            });
        }
        Ok(RequestLogTodaySummary {
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 0,
            reasoning_output_tokens: 0,
            estimated_cost_usd: 0.0,
        })
    }

    pub fn summarize_request_token_stats_by_key(&self) -> Result<Vec<ApiKeyTokenUsageSummary>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                    key_id,
                    {NON_NEGATIVE_TOTAL_TOKENS_SQL} AS total_tokens
             FROM request_token_stats
             WHERE key_id IS NOT NULL AND TRIM(key_id) <> ''
             GROUP BY key_id
             ORDER BY total_tokens DESC, key_id ASC"
        ))?;
        let mut rows = stmt.query([])?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(ApiKeyTokenUsageSummary {
                key_id: row.get(0)?,
                total_tokens: row.get(1)?,
            });
        }
        Ok(items)
    }

    pub fn summarize_cost_usage_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<CostUsageSummary> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(output_tokens, 0)), 0),
                {NON_NEGATIVE_TOTAL_TOKENS_SQL},
                IFNULL(SUM(IFNULL(estimated_cost_usd, 0.0)), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2"
        ))?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        if let Some(row) = rows.next()? {
            return Ok(CostUsageSummary {
                request_count: row.get(0)?,
                input_tokens: row.get(1)?,
                cached_input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                total_tokens: row.get(4)?,
                estimated_cost_usd: row.get(5)?,
            });
        }
        Ok(CostUsageSummary {
            request_count: 0,
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            estimated_cost_usd: 0.0,
        })
    }

    pub fn summarize_cost_usage_by_key_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CostSummaryKeyRow>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                key_id,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(output_tokens, 0)), 0),
                {NON_NEGATIVE_TOTAL_TOKENS_SQL},
                IFNULL(SUM(IFNULL(estimated_cost_usd, 0.0)), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
               AND key_id IS NOT NULL AND TRIM(key_id) <> ''
             GROUP BY key_id
             ORDER BY estimated_cost_usd DESC, key_id ASC"
        ))?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(CostSummaryKeyRow {
                key_id: row.get(0)?,
                request_count: row.get(1)?,
                input_tokens: row.get(2)?,
                cached_input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                total_tokens: row.get(5)?,
                estimated_cost_usd: row.get(6)?,
            });
        }
        Ok(items)
    }

    pub fn summarize_cost_usage_by_model_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CostSummaryModelRow>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                model,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(output_tokens, 0)), 0),
                {NON_NEGATIVE_TOTAL_TOKENS_SQL},
                IFNULL(SUM(IFNULL(estimated_cost_usd, 0.0)), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
               AND model IS NOT NULL AND TRIM(model) <> ''
             GROUP BY model
             ORDER BY estimated_cost_usd DESC, model ASC"
        ))?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(CostSummaryModelRow {
                model: row.get(0)?,
                request_count: row.get(1)?,
                input_tokens: row.get(2)?,
                cached_input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                total_tokens: row.get(5)?,
                estimated_cost_usd: row.get(6)?,
            });
        }
        Ok(items)
    }

    pub fn summarize_cost_usage_by_day_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CostSummaryDayRow>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                strftime('%Y-%m-%d', datetime(created_at, 'unixepoch', 'localtime')) AS bucket_day,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(output_tokens, 0)), 0),
                {NON_NEGATIVE_TOTAL_TOKENS_SQL},
                IFNULL(SUM(IFNULL(estimated_cost_usd, 0.0)), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
             GROUP BY bucket_day
             ORDER BY bucket_day ASC"
        ))?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(CostSummaryDayRow {
                day: row.get(0)?,
                request_count: row.get(1)?,
                input_tokens: row.get(2)?,
                cached_input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                total_tokens: row.get(5)?,
                estimated_cost_usd: row.get(6)?,
            });
        }
        Ok(items)
    }

    // -- Consumer Analytics queries --

    pub fn summarize_consumer_overview_between(
        &self,
        key_id: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<ConsumerOverviewRow> {
        // 中文注释：JOIN 查询需要限定表别名，NON_NEGATIVE_TOTAL_TOKENS_SQL 只用于单表，这里手动展开。
        let mut stmt = self.conn.prepare(
            "SELECT
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(ts.input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(ts.cached_input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(ts.output_tokens, 0)), 0),
                IFNULL(
                    SUM(
                        CASE
                            WHEN ts.total_tokens IS NOT NULL THEN
                                CASE WHEN ts.total_tokens > 0 THEN ts.total_tokens ELSE 0 END
                            ELSE
                                CASE
                                    WHEN IFNULL(ts.input_tokens, 0) - IFNULL(ts.cached_input_tokens, 0) + IFNULL(ts.output_tokens, 0) > 0
                                        THEN IFNULL(ts.input_tokens, 0) - IFNULL(ts.cached_input_tokens, 0) + IFNULL(ts.output_tokens, 0)
                                    ELSE 0
                                END
                        END
                    ),
                    0
                ),
                IFNULL(SUM(IFNULL(ts.estimated_cost_usd, 0.0)), 0.0),
                IFNULL(SUM(CASE WHEN r.status_code >= 200 AND r.status_code <= 299 THEN 1 ELSE 0 END), 0),
                AVG(r.duration_ms)
             FROM request_token_stats ts
             LEFT JOIN request_logs r ON r.id = ts.request_log_id
             WHERE ts.created_at >= ?1 AND ts.created_at < ?2
               AND ts.key_id = ?3",
        )?;
        let mut rows = stmt.query((start_ts, end_ts, key_id))?;
        if let Some(row) = rows.next()? {
            return Ok(ConsumerOverviewRow {
                request_count: row.get(0)?,
                input_tokens: row.get(1)?,
                cached_input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                total_tokens: row.get(4)?,
                estimated_cost_usd: row.get(5)?,
                success_count: row.get(6)?,
                avg_duration_ms: row.get(7)?,
            });
        }
        Ok(ConsumerOverviewRow {
            request_count: 0,
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            estimated_cost_usd: 0.0,
            success_count: 0,
            avg_duration_ms: None,
        })
    }

    pub fn summarize_consumer_trend_between(
        &self,
        key_id: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<ConsumerDayRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                strftime('%Y-%m-%d', datetime(created_at, 'unixepoch', 'localtime')) AS bucket_day,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(output_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(estimated_cost_usd, 0.0)), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
               AND key_id = ?3
             GROUP BY bucket_day
             ORDER BY bucket_day ASC",
        )?;
        let mut rows = stmt.query((start_ts, end_ts, key_id))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(ConsumerDayRow {
                day: row.get(0)?,
                request_count: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                estimated_cost_usd: row.get(4)?,
            });
        }
        Ok(items)
    }

    pub fn summarize_consumer_model_breakdown_between(
        &self,
        key_id: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<ConsumerModelRow>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                model,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(output_tokens, 0)), 0),
                {NON_NEGATIVE_TOTAL_TOKENS_SQL},
                IFNULL(SUM(IFNULL(estimated_cost_usd, 0.0)), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
               AND key_id = ?3
               AND model IS NOT NULL AND TRIM(model) <> ''
             GROUP BY model
             ORDER BY estimated_cost_usd DESC, model ASC"
        ))?;
        let mut rows = stmt.query((start_ts, end_ts, key_id))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(ConsumerModelRow {
                model: row.get(0)?,
                request_count: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                total_tokens: row.get(4)?,
                estimated_cost_usd: row.get(5)?,
            });
        }
        Ok(items)
    }

    pub fn summarize_consumer_ranking_between(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: i64,
    ) -> Result<Vec<CostSummaryKeyRow>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                key_id,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(output_tokens, 0)), 0),
                {NON_NEGATIVE_TOTAL_TOKENS_SQL},
                IFNULL(SUM(IFNULL(estimated_cost_usd, 0.0)), 0.0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
               AND key_id IS NOT NULL AND TRIM(key_id) <> ''
             GROUP BY key_id
             ORDER BY estimated_cost_usd DESC, key_id ASC
             LIMIT ?3"
        ))?;
        let mut rows = stmt.query((start_ts, end_ts, limit))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(CostSummaryKeyRow {
                key_id: row.get(0)?,
                request_count: row.get(1)?,
                input_tokens: row.get(2)?,
                cached_input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                total_tokens: row.get(5)?,
                estimated_cost_usd: row.get(6)?,
            });
        }
        Ok(items)
    }

    // -- Cache Analytics queries --

    pub fn summarize_cache_analytics_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<CacheSummaryRow> {
        // 中文注释：cached_input_tokens > 0 表示该请求命中了 prompt cache，按此统计命中率。
        // estimated_savings 按 cached_input_tokens 的 90% 输入价格估算（缓存通常 10% 价格）。
        let mut stmt = self.conn.prepare(
            "SELECT
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(CASE WHEN IFNULL(cached_input_tokens, 0) > 0 THEN 1 ELSE 0 END), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(
                    SUM(
                        CASE WHEN IFNULL(cached_input_tokens, 0) > 0 AND IFNULL(input_tokens, 0) > 0
                             AND IFNULL(estimated_cost_usd, 0.0) > 0
                        THEN (CAST(cached_input_tokens AS REAL) / CAST(input_tokens AS REAL))
                             * estimated_cost_usd * 0.9
                        ELSE 0 END
                    ),
                    0.0
                )
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2",
        )?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        if let Some(row) = rows.next()? {
            return Ok(CacheSummaryRow {
                total_requests: row.get(0)?,
                cached_requests: row.get(1)?,
                total_input_tokens: row.get(2)?,
                cached_input_tokens: row.get(3)?,
                estimated_savings_usd: row.get(4)?,
            });
        }
        Ok(CacheSummaryRow {
            total_requests: 0,
            cached_requests: 0,
            total_input_tokens: 0,
            cached_input_tokens: 0,
            estimated_savings_usd: 0.0,
        })
    }

    pub fn summarize_cache_analytics_trend_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CacheSummaryDayRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                strftime('%Y-%m-%d', datetime(created_at, 'unixepoch', 'localtime')) AS bucket_day,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(CASE WHEN IFNULL(cached_input_tokens, 0) > 0 THEN 1 ELSE 0 END), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0)
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
             GROUP BY bucket_day
             ORDER BY bucket_day ASC",
        )?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(CacheSummaryDayRow {
                day: row.get(0)?,
                total_requests: row.get(1)?,
                cached_requests: row.get(2)?,
                total_input_tokens: row.get(3)?,
                cached_input_tokens: row.get(4)?,
            });
        }
        Ok(items)
    }

    pub fn summarize_cache_analytics_by_model_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CacheSummaryModelRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                model,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(CASE WHEN IFNULL(cached_input_tokens, 0) > 0 THEN 1 ELSE 0 END), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(
                    SUM(
                        CASE WHEN IFNULL(cached_input_tokens, 0) > 0 AND IFNULL(input_tokens, 0) > 0
                             AND IFNULL(estimated_cost_usd, 0.0) > 0
                        THEN (CAST(cached_input_tokens AS REAL) / CAST(input_tokens AS REAL))
                             * estimated_cost_usd * 0.9
                        ELSE 0 END
                    ),
                    0.0
                )
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
               AND model IS NOT NULL AND TRIM(model) <> ''
             GROUP BY model
             ORDER BY cached_input_tokens DESC, model ASC",
        )?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(CacheSummaryModelRow {
                model: row.get(0)?,
                total_requests: row.get(1)?,
                cached_requests: row.get(2)?,
                total_input_tokens: row.get(3)?,
                cached_input_tokens: row.get(4)?,
                estimated_savings_usd: row.get(5)?,
            });
        }
        Ok(items)
    }

    pub fn summarize_cache_analytics_by_key_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CacheSummaryKeyRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                key_id,
                IFNULL(COUNT(*), 0),
                IFNULL(SUM(CASE WHEN IFNULL(cached_input_tokens, 0) > 0 THEN 1 ELSE 0 END), 0),
                IFNULL(SUM(IFNULL(input_tokens, 0)), 0),
                IFNULL(SUM(IFNULL(cached_input_tokens, 0)), 0),
                IFNULL(
                    SUM(
                        CASE WHEN IFNULL(cached_input_tokens, 0) > 0 AND IFNULL(input_tokens, 0) > 0
                             AND IFNULL(estimated_cost_usd, 0.0) > 0
                        THEN (CAST(cached_input_tokens AS REAL) / CAST(input_tokens AS REAL))
                             * estimated_cost_usd * 0.9
                        ELSE 0 END
                    ),
                    0.0
                )
             FROM request_token_stats
             WHERE created_at >= ?1 AND created_at < ?2
               AND key_id IS NOT NULL AND TRIM(key_id) <> ''
             GROUP BY key_id
             ORDER BY cached_input_tokens DESC, key_id ASC",
        )?;
        let mut rows = stmt.query((start_ts, end_ts))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(CacheSummaryKeyRow {
                key_id: row.get(0)?,
                total_requests: row.get(1)?,
                cached_requests: row.get(2)?,
                total_input_tokens: row.get(3)?,
                cached_input_tokens: row.get(4)?,
                estimated_savings_usd: row.get(5)?,
            });
        }
        Ok(items)
    }

    pub(super) fn ensure_request_token_stats_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS request_token_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_log_id INTEGER NOT NULL,
                key_id TEXT,
                account_id TEXT,
                model TEXT,
                input_tokens INTEGER,
                cached_input_tokens INTEGER,
                output_tokens INTEGER,
                total_tokens INTEGER,
                reasoning_output_tokens INTEGER,
                estimated_cost_usd REAL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_request_token_stats_request_log_id
             ON request_token_stats(request_log_id)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_token_stats_created_at
             ON request_token_stats(created_at DESC)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_token_stats_account_id_created_at
             ON request_token_stats(account_id, created_at DESC)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_token_stats_key_id_created_at
             ON request_token_stats(key_id, created_at DESC)",
            [],
        )?;
        self.ensure_column("request_token_stats", "total_tokens", "INTEGER")?;

        if self.has_column("request_logs", "input_tokens")? {
            // 中文注释：迁移历史 request_logs 里的 token 字段，避免升级后今日统计突然归零。
            self.conn.execute(
                "INSERT OR IGNORE INTO request_token_stats (
                    request_log_id, key_id, account_id, model,
                    input_tokens, cached_input_tokens, output_tokens, total_tokens, reasoning_output_tokens,
                    estimated_cost_usd, created_at
                 )
                 SELECT
                    id, key_id, account_id, model,
                    input_tokens, cached_input_tokens, output_tokens, NULL, reasoning_output_tokens,
                    estimated_cost_usd, created_at
                 FROM request_logs
                 WHERE input_tokens IS NOT NULL
                    OR cached_input_tokens IS NOT NULL
                    OR output_tokens IS NOT NULL
                    OR reasoning_output_tokens IS NOT NULL
                    OR estimated_cost_usd IS NOT NULL",
                [],
            )?;
        }
        Ok(())
    }
}
