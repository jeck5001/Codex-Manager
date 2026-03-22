use rusqlite::Result;

use super::{now_ts, ModelPricing, Storage};

impl Storage {
    pub fn list_model_pricing(&self) -> Result<Vec<ModelPricing>> {
        let mut stmt = self.conn.prepare(
            "SELECT model_slug, input_price_per_1k, output_price_per_1k, updated_at
             FROM model_pricing
             ORDER BY model_slug ASC",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(ModelPricing {
                model_slug: row.get(0)?,
                input_price_per_1k: row.get(1)?,
                output_price_per_1k: row.get(2)?,
                updated_at: row.get(3)?,
            });
        }
        Ok(out)
    }

    pub fn replace_model_pricing(&self, items: &[ModelPricing]) -> Result<()> {
        self.ensure_model_pricing_table()?;
        self.conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
        let result = (|| -> Result<()> {
            self.conn.execute("DELETE FROM model_pricing", [])?;
            for item in items {
                self.conn.execute(
                    "INSERT INTO model_pricing (
                        model_slug,
                        input_price_per_1k,
                        output_price_per_1k,
                        updated_at
                    ) VALUES (?1, ?2, ?3, ?4)",
                    (
                        &item.model_slug,
                        item.input_price_per_1k,
                        item.output_price_per_1k,
                        item.updated_at,
                    ),
                )?;
            }
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(err) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(err)
            }
        }
    }

    pub fn upsert_model_pricing(
        &self,
        model_slug: &str,
        input_price_per_1k: f64,
        output_price_per_1k: f64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO model_pricing (
                model_slug,
                input_price_per_1k,
                output_price_per_1k,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(model_slug) DO UPDATE SET
                input_price_per_1k = excluded.input_price_per_1k,
                output_price_per_1k = excluded.output_price_per_1k,
                updated_at = excluded.updated_at",
            (
                model_slug,
                input_price_per_1k,
                output_price_per_1k,
                now_ts(),
            ),
        )?;
        Ok(())
    }

    pub(super) fn ensure_model_pricing_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS model_pricing (
                model_slug TEXT PRIMARY KEY,
                input_price_per_1k REAL NOT NULL,
                output_price_per_1k REAL NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_model_pricing_updated_at ON model_pricing(updated_at DESC)",
            [],
        )?;
        Ok(())
    }
}
