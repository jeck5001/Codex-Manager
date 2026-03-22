CREATE TABLE IF NOT EXISTS model_pricing (
  model_slug TEXT PRIMARY KEY,
  input_price_per_1k REAL NOT NULL,
  output_price_per_1k REAL NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_pricing_updated_at
  ON model_pricing(updated_at DESC);
