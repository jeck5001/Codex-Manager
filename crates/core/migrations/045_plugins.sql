CREATE TABLE IF NOT EXISTS plugins (
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

CREATE INDEX IF NOT EXISTS idx_plugins_enabled
    ON plugins(enabled, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_plugins_runtime
    ON plugins(runtime, updated_at DESC);
