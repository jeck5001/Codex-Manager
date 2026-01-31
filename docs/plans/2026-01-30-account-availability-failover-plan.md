# Account Availability Failover Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在网关上游出错时即时刷新用量并切换账号，同时后台轮询根据“用量为空/用尽”标记账号可用性。

**Architecture:** 在 gpttools-core 增加账号状态更新接口；在 gpttools-service 增加“用量可用性判定”与“账号状态更新”小工具。后台轮询在写入快照后更新账号状态；网关在上游非 2xx 时刷新用量并按可用性决定是否切换到下一个账号重试。

**Tech Stack:** Rust, rusqlite, reqwest, tiny_http

---

### Task 1: 增加账号状态更新存储接口

**Files:**
- Modify: `crates/gpttools-core/src/storage/mod.rs`
- Modify: `crates/gpttools-core/tests/storage.rs`

**Step 1: Write the failing test**

Add to `crates/gpttools-core/tests/storage.rs`:

```rust
#[test]
fn storage_can_update_account_status() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let account = Account {
        id: "acc-1".to_string(),
        label: "main".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: Some("acct_123".to_string()),
        workspace_id: Some("org_123".to_string()),
        workspace_name: None,
        note: None,
        tags: None,
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");

    storage
        .update_account_status("acc-1", "inactive")
        .expect("update status");

    let loaded = storage
        .list_accounts()
        .expect("list accounts")
        .into_iter()
        .find(|acc| acc.id == "acc-1")
        .expect("account exists");

    assert_eq!(loaded.status, "inactive");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gpttools-core storage_can_update_account_status`
Expected: FAIL with "no method named `update_account_status`".

**Step 3: Write minimal implementation**

Add to `crates/gpttools-core/src/storage/mod.rs` (near other update methods):

```rust
    pub fn update_account_status(&self, account_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET status = ?1, updated_at = ?2 WHERE id = ?3",
            (status, now_ts(), account_id),
        )?;
        Ok(())
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p gpttools-core storage_can_update_account_status`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/gpttools-core/src/storage/mod.rs crates/gpttools-core/tests/storage.rs
git commit -m "feat(core): add account status update"
```

---

### Task 2: 新增用量可用性判定与账号状态更新工具

**Files:**
- Create: `crates/gpttools-service/src/account_availability.rs`
- Create: `crates/gpttools-service/src/account_status.rs`
- Modify: `crates/gpttools-service/src/lib.rs`

**Step 1: Write the failing test**

Add in `crates/gpttools-service/src/account_availability.rs` (test module):

```rust
#[cfg(test)]
mod tests {
    use super::{evaluate_snapshot, Availability};
    use gpttools_core::storage::UsageSnapshotRecord;

    fn snap(primary_used: Option<f64>, primary_window: Option<i64>, secondary_used: Option<f64>, secondary_window: Option<i64>) -> UsageSnapshotRecord {
        UsageSnapshotRecord {
            account_id: "acc-1".to_string(),
            used_percent: primary_used,
            window_minutes: primary_window,
            resets_at: None,
            secondary_used_percent: secondary_used,
            secondary_window_minutes: secondary_window,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: 0,
        }
    }

    #[test]
    fn availability_marks_missing_primary_unavailable() {
        let record = snap(None, Some(300), Some(10.0), Some(10080));
        assert!(matches!(evaluate_snapshot(&record), Availability::Unavailable(_)));
    }

    #[test]
    fn availability_marks_exhausted_secondary_unavailable() {
        let record = snap(Some(10.0), Some(300), Some(100.0), Some(10080));
        assert!(matches!(evaluate_snapshot(&record), Availability::Unavailable(_)));
    }

    #[test]
    fn availability_marks_ok_available() {
        let record = snap(Some(10.0), Some(300), Some(20.0), Some(10080));
        assert!(matches!(evaluate_snapshot(&record), Availability::Available));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gpttools-service availability_marks_missing_primary_unavailable`
Expected: FAIL with missing module/func symbols.

**Step 3: Write minimal implementation**

Create `crates/gpttools-service/src/account_availability.rs`:

```rust
use gpttools_core::storage::UsageSnapshotRecord;

pub(crate) enum Availability {
    Available,
    Unavailable(&'static str),
}

pub(crate) fn evaluate_snapshot(snap: &UsageSnapshotRecord) -> Availability {
    let primary_missing = snap.used_percent.is_none() || snap.window_minutes.is_none();
    let secondary_missing =
        snap.secondary_used_percent.is_none() || snap.secondary_window_minutes.is_none();
    if primary_missing {
        return Availability::Unavailable("usage_missing_primary");
    }
    if secondary_missing {
        return Availability::Unavailable("usage_missing_secondary");
    }
    if let Some(value) = snap.used_percent {
        if value >= 100.0 {
            return Availability::Unavailable("usage_exhausted_primary");
        }
    }
    if let Some(value) = snap.secondary_used_percent {
        if value >= 100.0 {
            return Availability::Unavailable("usage_exhausted_secondary");
        }
    }
    Availability::Available
}

pub(crate) fn is_available(snap: Option<&UsageSnapshotRecord>) -> bool {
    match snap {
        None => true,
        Some(record) => matches!(evaluate_snapshot(record), Availability::Available),
    }
}
```

Create `crates/gpttools-service/src/account_status.rs`:

```rust
use gpttools_core::storage::{now_ts, Event, Storage};

pub(crate) fn set_account_status(
    storage: &Storage,
    account_id: &str,
    status: &str,
    reason: &str,
) {
    let _ = storage.update_account_status(account_id, status);
    let _ = storage.insert_event(&Event {
        account_id: Some(account_id.to_string()),
        event_type: "account_status_update".to_string(),
        message: format!("status={status} reason={reason}"),
        created_at: now_ts(),
    });
}
```

Update `crates/gpttools-service/src/lib.rs` module list:

```rust
mod account_availability;
mod account_status;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p gpttools-service availability_marks_missing_primary_unavailable`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/gpttools-service/src/account_availability.rs crates/gpttools-service/src/account_status.rs crates/gpttools-service/src/lib.rs
git commit -m "feat(service): add usage availability helpers"
```

---

### Task 3: 轮询刷新后更新账号状态

**Files:**
- Modify: `crates/gpttools-service/src/usage_refresh.rs`

**Step 1: Write the failing test**

Add in `crates/gpttools-service/src/usage_refresh.rs` (test module):

```rust
#[cfg(test)]
mod status_tests {
    use super::apply_status_from_snapshot;
    use crate::account_availability::Availability;
    use gpttools_core::storage::{now_ts, Account, Storage, UsageSnapshotRecord};

    #[test]
    fn apply_status_marks_inactive_on_missing() {
        let storage = Storage::open_in_memory().expect("open");
        storage.init().expect("init");
        let account = Account {
            id: "acc-1".to_string(),
            label: "main".to_string(),
            issuer: "issuer".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            workspace_name: None,
            note: None,
            tags: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now_ts(),
            updated_at: now_ts(),
        };
        storage.insert_account(&account).expect("insert");

        let record = UsageSnapshotRecord {
            account_id: "acc-1".to_string(),
            used_percent: None,
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: Some(10.0),
            secondary_window_minutes: Some(10080),
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now_ts(),
        };

        let availability = apply_status_from_snapshot(&storage, &record);
        assert!(matches!(availability, Availability::Unavailable(_)));
        let loaded = storage
            .list_accounts()
            .expect("list")
            .into_iter()
            .find(|acc| acc.id == "acc-1")
            .expect("exists");
        assert_eq!(loaded.status, "inactive");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gpttools-service apply_status_marks_inactive_on_missing`
Expected: FAIL with missing `apply_status_from_snapshot`.

**Step 3: Write minimal implementation**

Update `crates/gpttools-service/src/usage_refresh.rs`:

```rust
use crate::account_availability::{evaluate_snapshot, Availability};
use crate::account_status::set_account_status;

fn apply_status_from_snapshot(storage: &Storage, record: &UsageSnapshotRecord) -> Availability {
    let availability = evaluate_snapshot(record);
    match availability {
        Availability::Available => {
            set_account_status(storage, &record.account_id, "active", "usage_ok");
        }
        Availability::Unavailable(reason) => {
            set_account_status(storage, &record.account_id, "inactive", reason);
        }
    }
    availability
}
```

Then call it after writing snapshot in `store_usage_snapshot`:

```rust
    storage
        .insert_usage_snapshot(&record)
        .map_err(|e| e.to_string())?;
    let _ = apply_status_from_snapshot(storage, &record);
    Ok(())
```

And in `refresh_usage_for_token`, when `fetch_usage_snapshot` returns an error string starting with `"usage endpoint status"`, mark inactive before returning:

```rust
        Err(err) if err.starts_with("usage endpoint status") => {
            set_account_status(storage, &current.account_id, "inactive", "usage_unreachable");
            return Err(err);
        }
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p gpttools-service apply_status_marks_inactive_on_missing`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/gpttools-service/src/usage_refresh.rs
git commit -m "feat(service): mark account status from usage refresh"
```

---

### Task 4: 网关出错时刷新用量并切换账号

**Files:**
- Modify: `crates/gpttools-service/src/gateway.rs`

**Step 1: Write the failing test**

Add in `crates/gpttools-service/src/gateway.rs` (test module):

```rust
#[cfg(test)]
mod availability_tests {
    use super::should_failover_after_refresh;
    use gpttools_core::storage::{now_ts, Account, Storage, UsageSnapshotRecord};

    #[test]
    fn failover_on_missing_usage() {
        let storage = Storage::open_in_memory().expect("open");
        storage.init().expect("init");
        let account = Account {
            id: "acc-1".to_string(),
            label: "main".to_string(),
            issuer: "issuer".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            workspace_name: None,
            note: None,
            tags: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now_ts(),
            updated_at: now_ts(),
        };
        storage.insert_account(&account).expect("insert");
        let record = UsageSnapshotRecord {
            account_id: "acc-1".to_string(),
            used_percent: None,
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: Some(10.0),
            secondary_window_minutes: Some(10080),
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now_ts(),
        };
        storage.insert_usage_snapshot(&record).expect("insert usage");

        let should_failover = should_failover_after_refresh(&storage, "acc-1", Ok(()));
        assert!(should_failover);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gpttools-service failover_on_missing_usage`
Expected: FAIL with missing `should_failover_after_refresh`.

**Step 3: Write minimal implementation**

Update `crates/gpttools-service/src/gateway.rs`:

- Add helpers near bottom:

```rust
use crate::account_availability::{evaluate_snapshot, Availability, is_available};
use crate::account_status::set_account_status;
use crate::usage_refresh;

fn should_failover_after_refresh(
    storage: &Storage,
    account_id: &str,
    refresh_result: Result<(), String>,
) -> bool {
    match refresh_result {
        Ok(_) => {
            let snap = storage
                .latest_usage_snapshots_by_account()
                .ok()
                .and_then(|snaps| snaps.into_iter().find(|s| s.account_id == account_id));
            match snap.as_ref().map(evaluate_snapshot) {
                Some(Availability::Unavailable(reason)) => {
                    set_account_status(storage, account_id, "inactive", reason);
                    true
                }
                Some(Availability::Available) => false,
                None => {
                    set_account_status(storage, account_id, "inactive", "usage_missing_snapshot");
                    true
                }
            }
        }
        Err(err) => {
            if err.starts_with("usage endpoint status") {
                set_account_status(storage, account_id, "inactive", "usage_unreachable");
                true
            } else {
                false
            }
        }
    }
}
```

- Refactor account selection to collect candidates using availability:

```rust
fn collect_gateway_candidates(storage: &Storage) -> Result<Vec<(Account, Token)>, String> {
    let accounts = storage.list_accounts().map_err(|e| e.to_string())?;
    let tokens = storage.list_tokens().map_err(|e| e.to_string())?;
    let snaps = storage
        .latest_usage_snapshots_by_account()
        .map_err(|e| e.to_string())?;
    let mut token_map = HashMap::new();
    for token in tokens {
        token_map.insert(token.account_id.clone(), token);
    }
    let mut snap_map = HashMap::new();
    for snap in snaps {
        snap_map.insert(snap.account_id.clone(), snap);
    }

    let mut out = Vec::new();
    for account in accounts {
        if account.status != "active" {
            continue;
        }
        let token = match token_map.get(&account.id) {
            Some(token) => token.clone(),
            None => continue,
        };
        let usage = snap_map.get(&account.id);
        if !is_available(usage) {
            continue;
        }
        out.push((account, token));
    }
    Ok(out)
}
```

- In `handle_gateway_request`, replace `select_account_for_gateway` with:

```rust
    let candidates = collect_gateway_candidates(&storage)?;
    if candidates.is_empty() {
        return Err("no available account".to_string());
    }

    for (idx, (account, token)) in candidates.into_iter().enumerate() {
        let upstream = match send_upstream_request(&request, &account, &token, &url, &method, &body, &cookie, has_user_agent) { /* ... */ };
        let status = upstream.status();
        if status.is_success() {
            return respond_with_upstream(request, upstream, status);
        }
        let refresh_result = usage_refresh::refresh_usage_for_account(&account.id);
        let should_failover = should_failover_after_refresh(&storage, &account.id, refresh_result);
        if should_failover && idx + 1 < candidates.len() {
            continue;
        }
        return respond_with_upstream(request, upstream, status);
    }
```

Also extract `respond_with_upstream` helper to reuse existing response-building code.

**Step 4: Run test to verify it passes**

Run: `cargo test -p gpttools-service failover_on_missing_usage`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/gpttools-service/src/gateway.rs
git commit -m "feat(service): failover on usage missing or exhausted"
```

---

### Task 5: 验证全量测试

**Files:**
- None

**Step 1: Verify tooling**

Run: `Get-Command cargo -ErrorAction SilentlyContinue`
Expected: Output includes `cargo` path.

**Step 2: Run tests**

Run: `cargo test -p gpttools-core`
Expected: PASS.

Run: `cargo test -p gpttools-service`
Expected: PASS.

**Step 3: Commit (if any pending)**

```bash
git status -sb
```
Expected: clean working tree.

---

## Rollback Plan
- Revert the commits from Task 4 → Task 1 in reverse order.
- If only partial changes were applied, `git checkout -- <file>` for the modified files.
