# Database and Model Extension Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add account metadata + secondary usage fields to schema, storage models, and RPC types.

**Architecture:** Add two migrations, update storage structs and SQL, and add runtime column checks for backward compatibility.

**Tech Stack:** Rust, rusqlite, SQL migrations

### Task 1: Add migrations for account metadata

**Files:**
- Create: `GPTTools/crates/gpttools-core/migrations/003_account_meta.sql`

**Step 1: Write migration SQL file**
```sql
ALTER TABLE accounts ADD COLUMN note TEXT;
ALTER TABLE accounts ADD COLUMN tags TEXT;
ALTER TABLE accounts ADD COLUMN group_name TEXT;

ALTER TABLE login_sessions ADD COLUMN note TEXT;
ALTER TABLE login_sessions ADD COLUMN tags TEXT;
ALTER TABLE login_sessions ADD COLUMN group_name TEXT;
```

**Step 2: Manual verification (no automated tests available)**
Run: N/A (manual inspection only)
Expected: Migration file exists with required columns.

### Task 2: Add migration for secondary usage window

**Files:**
- Create: `GPTTools/crates/gpttools-core/migrations/004_usage_secondary.sql`

**Step 1: Write migration SQL file**
```sql
ALTER TABLE usage_snapshots ADD COLUMN secondary_used_percent REAL;
ALTER TABLE usage_snapshots ADD COLUMN secondary_window_minutes INTEGER;
ALTER TABLE usage_snapshots ADD COLUMN secondary_resets_at INTEGER;
```

**Step 2: Manual verification (no automated tests available)**
Run: N/A (manual inspection only)
Expected: Migration file exists with required columns.

### Task 3: Update storage models and SQL

**Files:**
- Modify: `GPTTools/crates/gpttools-core/src/storage/mod.rs`

**Step 1: Update structs**
- Add `note`, `tags`, `group_name` to `Account` and `LoginSession`.
- Add `secondary_*` to `UsageSnapshotRecord`.

**Step 2: Update insert/select SQL**
- Update `insert_account`, `list_accounts`.
- Update `insert_login_session`, `get_login_session`.
- Update `insert_usage_snapshot`, `latest_usage_snapshot`, `latest_usage_snapshots_by_account`.

**Step 3: Add ensure_* column helpers**
- Add column checks + ALTER TABLE for new columns on `accounts`, `login_sessions`, `usage_snapshots`.
- Call helpers from `Storage::init` after running migrations.

**Step 4: Manual verification (no automated tests available)**
Run: N/A (manual inspection only)
Expected: SQL statements include new columns; ensure_* helpers exist.

### Task 4: Update RPC types

**Files:**
- Modify: `GPTTools/crates/gpttools-core/src/rpc/types.rs`

**Step 1: Add fields**
- Add `note`, `tags`, `group_name` to `AccountSummary`.
- Add `secondary_*` fields to `UsageSnapshotResult`.

**Step 2: Manual verification (no automated tests available)**
Run: N/A (manual inspection only)
Expected: RPC structs compile with new fields.
