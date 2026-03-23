CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    object_type TEXT NOT NULL,
    object_id TEXT,
    operator TEXT NOT NULL,
    changes_json TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at
    ON audit_logs(created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_action_created_at
    ON audit_logs(action, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_object_type_created_at
    ON audit_logs(object_type, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_object_id_created_at
    ON audit_logs(object_id, created_at DESC, id DESC);
