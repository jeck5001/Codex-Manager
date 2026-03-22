CREATE TABLE IF NOT EXISTS api_key_model_fallbacks (
    key_id TEXT PRIMARY KEY REFERENCES api_keys(id) ON DELETE CASCADE,
    model_chain_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_api_key_model_fallbacks_updated_at
    ON api_key_model_fallbacks(updated_at);
