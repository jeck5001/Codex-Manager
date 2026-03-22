CREATE TABLE IF NOT EXISTS api_key_response_cache_configs (
    key_id TEXT PRIMARY KEY REFERENCES api_keys(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
