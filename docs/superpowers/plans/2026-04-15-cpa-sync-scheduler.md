# CPA Scheduled Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a service-side scheduled sync loop for CLIProxyAPI / CPA accounts, with an enable switch, fixed-minute interval, runtime status reporting, and frontend controls in the existing settings card.

**Architecture:** Reuse the existing CPA manual sync pipeline in `crates/service/src/account/account_cpa_sync.rs` as the single execution path, then wrap it with an in-process scheduler and runtime status snapshot. Persist schedule settings via the existing app settings flow, expose status through a new RPC endpoint, and extend the existing settings page card to configure and display the scheduler state.

**Tech Stack:** Rust service (`serde_json`, existing blocking HTTP sync pipeline, std thread/sync primitives), Next.js App Router frontend, TypeScript strict mode, TanStack Query, centralized transport/invoke helpers.

---

### Task 1: Persist Scheduled Sync Settings

**Files:**
- Modify: `crates/service/src/app_settings/shared.rs`
- Modify: `crates/service/src/app_settings/mod.rs`
- Modify: `crates/service/src/app_settings/api/patch.rs`
- Modify: `crates/service/src/app_settings/api/current.rs`
- Modify: `crates/service/tests/app_settings.rs`
- Modify: `apps/src/types/index.ts`
- Modify: `apps/src/lib/api/normalize.ts`
- Modify: `apps/src/lib/store/useAppStore.ts`

- [ ] **Step 1: Write the failing backend settings test**

Add a persistence test near the existing CPA settings tests in `crates/service/tests/app_settings.rs`:

```rust
#[test]
fn app_settings_set_persists_cpa_schedule_snapshot() {
    let _guard = test_env_guard();

    let snapshot = codexmanager_service::app_settings_set(Some(&json!({
        "cpaSyncEnabled": true,
        "cpaSyncApiUrl": "https://cpa.example.com",
        "cpaSyncScheduleEnabled": true,
        "cpaSyncScheduleIntervalMinutes": 30
    })))
    .expect("set cpa schedule");

    assert_eq!(
        snapshot.get("cpaSyncScheduleEnabled").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        snapshot
            .get("cpaSyncScheduleIntervalMinutes")
            .and_then(|v| v.as_u64()),
        Some(30)
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p codexmanager-service app_settings_set_persists_cpa_schedule_snapshot -- --nocapture`

Expected: FAIL because the snapshot does not yet contain `cpaSyncScheduleEnabled` / `cpaSyncScheduleIntervalMinutes`.

- [ ] **Step 3: Add the new persisted setting keys**

Extend `crates/service/src/app_settings/shared.rs` and re-export through `crates/service/src/app_settings/mod.rs` and `crates/service/src/lib.rs` with:

```rust
pub const APP_SETTING_CPA_SYNC_SCHEDULE_ENABLED_KEY: &str = "cpa_sync.schedule_enabled";
pub const APP_SETTING_CPA_SYNC_SCHEDULE_INTERVAL_MINUTES_KEY: &str =
    "cpa_sync.schedule_interval_minutes";
```

And include them in the re-export lists alongside:

```rust
APP_SETTING_CPA_SYNC_API_URL_KEY,
APP_SETTING_CPA_SYNC_ENABLED_KEY,
APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY,
APP_SETTING_CPA_SYNC_SCHEDULE_ENABLED_KEY,
APP_SETTING_CPA_SYNC_SCHEDULE_INTERVAL_MINUTES_KEY,
```

- [ ] **Step 4: Wire the new settings through patch + snapshot**

Update `crates/service/src/app_settings/api/patch.rs`:

```rust
#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AppSettingsPatch {
    cpa_sync_schedule_enabled: Option<bool>,
    cpa_sync_schedule_interval_minutes: Option<u64>,
}
```

And persist them in `apply_app_settings_patch`:

```rust
if let Some(enabled) = patch.cpa_sync_schedule_enabled {
    save_persisted_bool_setting(APP_SETTING_CPA_SYNC_SCHEDULE_ENABLED_KEY, enabled)?;
}
if let Some(interval_minutes) = patch.cpa_sync_schedule_interval_minutes {
    save_persisted_app_setting(
        APP_SETTING_CPA_SYNC_SCHEDULE_INTERVAL_MINUTES_KEY,
        Some(interval_minutes.to_string().as_str()),
    )?;
}
```

Update `crates/service/src/app_settings/api/current.rs` to read and expose:

```rust
let cpa_sync_schedule_enabled = get_persisted_app_setting(
    APP_SETTING_CPA_SYNC_SCHEDULE_ENABLED_KEY,
)
.map(|raw| parse_bool_with_default(&raw, false))
.unwrap_or(false);

let cpa_sync_schedule_interval_minutes = get_persisted_app_setting(
    APP_SETTING_CPA_SYNC_SCHEDULE_INTERVAL_MINUTES_KEY,
)
.and_then(|raw| raw.trim().parse::<u64>().ok())
.map(|value| value.max(1))
.unwrap_or(30);
```

And return:

```rust
"cpaSyncScheduleEnabled": cpa_sync_schedule_enabled,
"cpaSyncScheduleIntervalMinutes": cpa_sync_schedule_interval_minutes,
```

- [ ] **Step 5: Extend frontend snapshot typing and normalization**

Update `apps/src/types/index.ts`:

```ts
export interface AppSettingsSnapshot {
  cpaSyncScheduleEnabled: boolean;
  cpaSyncScheduleIntervalMinutes: number;
}
```

Update `apps/src/lib/api/normalize.ts`:

```ts
cpaSyncScheduleEnabled: asBoolean(source.cpaSyncScheduleEnabled, false),
cpaSyncScheduleIntervalMinutes: asInteger(
  source.cpaSyncScheduleIntervalMinutes,
  30,
  1
),
```

Update `apps/src/lib/store/useAppStore.ts` defaults:

```ts
cpaSyncScheduleEnabled: false,
cpaSyncScheduleIntervalMinutes: 30,
```

- [ ] **Step 6: Run the focused settings test to verify it passes**

Run: `cargo test -p codexmanager-service app_settings_set_persists_cpa_schedule_snapshot -- --nocapture`

Expected: PASS with the new snapshot fields persisted and returned.

- [ ] **Step 7: Commit**

```bash
git add crates/service/src/app_settings/shared.rs crates/service/src/app_settings/mod.rs crates/service/src/app_settings/api/patch.rs crates/service/src/app_settings/api/current.rs crates/service/tests/app_settings.rs apps/src/types/index.ts apps/src/lib/api/normalize.ts apps/src/lib/store/useAppStore.ts
git commit -m "feat: persist cpa sync schedule settings"
```

### Task 2: Add CPA Sync Runtime Status and Single-Run Guard

**Files:**
- Modify: `crates/service/src/account/account_cpa_sync.rs`
- Modify: `crates/service/src/account/tests/account_cpa_sync_tests.rs`
- Modify: `crates/service/src/rpc_dispatch/account.rs`
- Modify: `apps/src/lib/api/transport.ts`
- Modify: `apps/src/lib/api/account-client.ts`

- [ ] **Step 1: Write the failing runtime status tests**

Add tests in `crates/service/src/account/tests/account_cpa_sync_tests.rs`:

```rust
#[test]
fn cpa_sync_status_defaults_to_disabled_snapshot() {
    let status = super::cpa_sync_status_for_test();
    assert_eq!(status.status, "disabled");
    assert!(!status.is_running);
    assert_eq!(status.interval_minutes, 30);
}

#[test]
fn cpa_sync_run_guard_rejects_overlapping_runs() {
    let _guard = super::begin_cpa_sync_run_for_test("manual").expect("first lock");
    let err = super::begin_cpa_sync_run_for_test("scheduled").expect_err("second run should fail");
    assert!(err.contains("正在执行中"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p codexmanager-service cpa_sync_status_defaults_to_disabled_snapshot -- --nocapture`

Expected: FAIL because there is no runtime status or run guard API yet.

- [ ] **Step 3: Add runtime status structs and guard helpers**

In `crates/service/src/account/account_cpa_sync.rs`, add:

```rust
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CpaSyncStatus {
    status: String,
    schedule_enabled: bool,
    interval_minutes: u64,
    is_running: bool,
    last_trigger: String,
    last_started_at: Option<i64>,
    last_finished_at: Option<i64>,
    last_success_at: Option<i64>,
    last_summary: String,
    last_error: String,
    next_run_at: Option<i64>,
}
```

Plus in-memory state:

```rust
#[derive(Debug, Clone, Default)]
struct CpaSyncRuntimeState { /* same fields, mutable */ }

static CPA_SYNC_RUNTIME: OnceLock<Mutex<CpaSyncRuntimeState>> = OnceLock::new();
```

And a run guard:

```rust
pub(crate) struct CpaSyncRunGuard;

fn begin_cpa_sync_run(trigger: &str) -> Result<CpaSyncRunGuard, String> { /* set is_running or reject */ }
impl Drop for CpaSyncRunGuard { /* clear is_running */ }
```

- [ ] **Step 4: Refactor manual sync into a reusable single-run executor**

Split current `sync_cpa_accounts` into:

```rust
fn sync_cpa_accounts_once(params: Option<&Value>, trigger: &str) -> Result<CpaSyncResult, String> { /* current body + runtime status updates */ }

pub(crate) fn sync_cpa_accounts(params: Option<&Value>) -> Result<CpaSyncResult, String> {
    sync_cpa_accounts_once(params, "manual")
}
```

Status updates should do:

```rust
state.last_trigger = trigger.to_string();
state.last_started_at = Some(now_ts());
state.last_error.clear();
state.status = "running".to_string();
```

And on finish:

```rust
state.last_finished_at = Some(now_ts());
state.last_summary = format!(
    "总文件 {}，可导入 {}，新增 {}，更新 {}，失败 {}",
    result.total_files, result.eligible_files, result.created, result.updated, result.failed
);
state.last_success_at = Some(now_ts());
state.status = "idle".to_string();
```

On error:

```rust
state.last_finished_at = Some(now_ts());
state.last_error = err.clone();
state.status = "error".to_string();
```

- [ ] **Step 5: Expose read-only status through RPC**

Add to `crates/service/src/account/account_cpa_sync.rs`:

```rust
pub(crate) fn cpa_sync_status(_params: Option<&Value>) -> Result<CpaSyncStatus, String> {
    Ok(read_cpa_sync_status())
}
```

Add route in `crates/service/src/rpc_dispatch/account.rs`:

```rust
"account/cpa/syncStatus" => {
    super::value_or_error(account_cpa_sync::cpa_sync_status(req.params.as_ref()))
}
```

Add transport mapping in `apps/src/lib/api/transport.ts`:

```ts
service_account_cpa_sync_status: {
  rpcMethod: "account/cpa/syncStatus",
},
```

Add client method in `apps/src/lib/api/account-client.ts`:

```ts
  getCpaSyncStatus: () =>
    invoke<AccountCpaSyncStatusResult>(
      "service_account_cpa_sync_status",
      withAddr()
    ),
```

- [ ] **Step 6: Add test-only helpers and pass focused tests**

Add test-only exports in `account_cpa_sync.rs`:

```rust
#[cfg(test)]
pub(crate) fn cpa_sync_status_for_test() -> CpaSyncStatus { read_cpa_sync_status() }

#[cfg(test)]
pub(crate) fn begin_cpa_sync_run_for_test(trigger: &str) -> Result<CpaSyncRunGuard, String> {
    begin_cpa_sync_run(trigger)
}
```

Run: `cargo test -p codexmanager-service cpa_sync_ -- --nocapture`

Expected: PASS for the new status + overlap guard coverage.

- [ ] **Step 7: Commit**

```bash
git add crates/service/src/account/account_cpa_sync.rs crates/service/src/account/tests/account_cpa_sync_tests.rs crates/service/src/rpc_dispatch/account.rs apps/src/lib/api/transport.ts apps/src/lib/api/account-client.ts
git commit -m "feat: add cpa sync runtime status"
```

### Task 3: Add the Service-Side Scheduler and Settings Refresh Hook

**Files:**
- Modify: `crates/service/src/account/account_cpa_sync.rs`
- Modify: `crates/service/src/lib.rs`
- Modify: `crates/service/src/app_settings/api/patch.rs`
- Modify: `crates/service/tests/rpc.rs`

- [ ] **Step 1: Write the failing scheduler tests**

Add tests in `crates/service/src/account/tests/account_cpa_sync_tests.rs`:

```rust
#[test]
fn cpa_schedule_status_marks_misconfigured_when_enabled_without_credentials() {
    super::refresh_cpa_sync_schedule_for_test(Some(false), true, 15, "", false);
    let status = super::cpa_sync_status_for_test();
    assert_eq!(status.status, "misconfigured");
    assert_eq!(status.last_error, "CPA API URL 或 Management Key 未配置");
}

#[test]
fn cpa_schedule_status_sets_next_run_when_enabled_and_configured() {
    super::refresh_cpa_sync_schedule_for_test(Some(true), true, 15, "https://cpa.example.com", true);
    let status = super::cpa_sync_status_for_test();
    assert_eq!(status.status, "idle");
    assert_eq!(status.interval_minutes, 15);
    assert!(status.next_run_at.is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p codexmanager-service cpa_schedule_status_ -- --nocapture`

Expected: FAIL because there is no scheduler refresh API.

- [ ] **Step 3: Add schedule config loader and refresh function**

In `crates/service/src/account/account_cpa_sync.rs`, add:

```rust
#[derive(Debug, Clone, Default)]
struct CpaSyncScheduleConfig {
    source_enabled: bool,
    schedule_enabled: bool,
    interval_minutes: u64,
    api_url: String,
    has_management_key: bool,
}

fn load_cpa_sync_schedule_config() -> CpaSyncScheduleConfig { /* read persisted app settings */ }
pub(crate) fn refresh_cpa_sync_schedule() -> Result<(), String> { /* update runtime status + next_run_at */ }
```

Status refresh logic should produce:

```rust
if !config.source_enabled || !config.schedule_enabled {
    state.status = "disabled".to_string();
    state.next_run_at = None;
} else if config.api_url.trim().is_empty() || !config.has_management_key {
    state.status = "misconfigured".to_string();
    state.last_error = "CPA API URL 或 Management Key 未配置".to_string();
    state.next_run_at = None;
} else {
    state.status = if state.is_running { "running".to_string() } else { "idle".to_string() };
    state.next_run_at = Some(now_ts() + (config.interval_minutes.max(1) as i64 * 60));
}
```

- [ ] **Step 4: Start the scheduler loop from service startup**

In `crates/service/src/account/account_cpa_sync.rs`, add:

```rust
pub(crate) fn ensure_cpa_sync_scheduler_started() {
    static START: Once = Once::new();
    START.call_once(|| {
        std::thread::Builder::new()
            .name("cpa-sync-scheduler".to_string())
            .spawn(|| loop {
                let _ = refresh_cpa_sync_schedule();
                let should_run = should_trigger_scheduled_sync();
                if should_run {
                    let _ = sync_cpa_accounts_once(None, "scheduled");
                }
                std::thread::sleep(Duration::from_secs(1));
            })
            .expect("spawn cpa sync scheduler");
    });
}
```

Then call it from `crates/service/src/lib.rs` in startup/init path, right after service bootstraps storage/runtime:

```rust
pub fn initialize_process_logging() { /* unchanged */ }

pub fn initialize_service_runtime() {
    account::cpa_sync::ensure_cpa_sync_scheduler_started();
}
```

If there is an existing initialization hook, attach there instead of inventing a second bootstrap path.

- [ ] **Step 5: Refresh scheduler immediately after settings save**

At the end of `apply_app_settings_patch` in `crates/service/src/app_settings/api/patch.rs`, trigger:

```rust
let _ = crate::account::cpa_sync::refresh_cpa_sync_schedule();
```

This keeps schedule changes hot-reloaded after every save.

- [ ] **Step 6: Add an RPC regression test for the status route**

Add to `crates/service/tests/rpc.rs`:

```rust
#[test]
fn rpc_account_cpa_sync_status_returns_structured_snapshot() {
    let result = rpc_call("account/cpa/syncStatus", None).expect("rpc status");
    assert!(result.get("status").is_some());
    assert!(result.get("intervalMinutes").is_some());
    assert!(result.get("isRunning").is_some());
}
```

- [ ] **Step 7: Run focused scheduler tests**

Run:

```bash
cargo test -p codexmanager-service cpa_schedule_status_ -- --nocapture
cargo test -p codexmanager-service rpc_account_cpa_sync_status_returns_structured_snapshot -- --nocapture
```

Expected: PASS for both the scheduler status tests and the RPC route test.

- [ ] **Step 8: Commit**

```bash
git add crates/service/src/account/account_cpa_sync.rs crates/service/src/lib.rs crates/service/src/app_settings/api/patch.rs crates/service/tests/rpc.rs
git commit -m "feat: add cpa sync scheduler runtime"
```

### Task 4: Wire the Settings Page Controls and Status Display

**Files:**
- Modify: `apps/src/app/settings/page.tsx`
- Modify: `apps/src/lib/api/account-client.ts`
- Modify: `apps/src/types/index.ts`

- [ ] **Step 1: Write the failing frontend normalization test**

Add a small assertion in an existing frontend normalization test file or create one if absent. If reusing an existing app-settings normalization test is impractical, add a minimal unit test under `apps/src/lib/api`:

```ts
test("normalize app settings snapshot includes cpa schedule fields", () => {
  const snapshot = normalizeAppSettingsSnapshot({
    cpaSyncScheduleEnabled: true,
    cpaSyncScheduleIntervalMinutes: 45,
  });
  assert.equal(snapshot.cpaSyncScheduleEnabled, true);
  assert.equal(snapshot.cpaSyncScheduleIntervalMinutes, 45);
});
```

- [ ] **Step 2: Run test to verify it fails if the test file is new**

Run: `pnpm exec vitest run apps/src/app/settings/cpa-sync-scheduler-state.test.ts`

Expected: FAIL before the new fields and status type are fully wired.

- [ ] **Step 3: Add the status result type and API method**

In `apps/src/types/index.ts`, add:

```ts
export interface AccountCpaSyncStatusResult {
  status: string;
  scheduleEnabled: boolean;
  intervalMinutes: number;
  isRunning: boolean;
  lastTrigger: string;
  lastStartedAt?: number | null;
  lastFinishedAt?: number | null;
  lastSuccessAt?: number | null;
  lastSummary: string;
  lastError: string;
  nextRunAt?: number | null;
}
```

And in `apps/src/lib/api/account-client.ts`:

```ts
  getCpaSyncStatus: () =>
    invoke<AccountCpaSyncStatusResult>(
      "service_account_cpa_sync_status",
      withAddr()
    ),
```

- [ ] **Step 4: Extend the settings page state**

In `apps/src/app/settings/page.tsx`, add:

```tsx
const [cpaSyncScheduleEnabledDraft, setCpaSyncScheduleEnabledDraft] = useState<boolean | null>(null);
const [cpaSyncScheduleIntervalDraft, setCpaSyncScheduleIntervalDraft] = useState<string | null>(null);
const { data: cpaSyncStatus } = useQuery({
  queryKey: ["cpa-sync-status"],
  queryFn: () => accountClient.getCpaSyncStatus(),
  refetchInterval: 5000,
});
```

And derive the input values from snapshot defaults:

```tsx
const cpaSyncScheduleEnabled =
  cpaSyncScheduleEnabledDraft ?? snapshot?.cpaSyncScheduleEnabled ?? false;
const cpaSyncScheduleIntervalInput =
  cpaSyncScheduleIntervalDraft ??
  String(snapshot?.cpaSyncScheduleIntervalMinutes ?? 30);
```

- [ ] **Step 5: Add validation and save payload**

Extend `handleSaveCpaSync` in `apps/src/app/settings/page.tsx`:

```tsx
const intervalMinutes = Math.max(1, Number.parseInt(cpaSyncScheduleIntervalInput.trim(), 10) || 0);
if (cpaSyncScheduleEnabled && intervalMinutes < 1) {
  toast.error("同步间隔必须是大于 0 的整数分钟");
  return;
}

updateSettings.mutate({
  cpaSyncEnabled: snapshot?.cpaSyncEnabled ?? false,
  cpaSyncApiUrl: cpaSyncApiUrlInput,
  cpaSyncManagementKey: cpaSyncManagementKeyDraft,
  cpaSyncScheduleEnabled,
  cpaSyncScheduleIntervalMinutes: intervalMinutes,
});
```

- [ ] **Step 6: Render the new switch, interval input, and runtime summary**

Add to the CPA card in `apps/src/app/settings/page.tsx`:

```tsx
<div className="flex items-center justify-between">
  <div className="space-y-0.5">
    <Label>启用定时同步</Label>
    <p className="text-xs text-muted-foreground">服务端常驻运行，适合 Docker / NAS 部署</p>
  </div>
  <Switch
    checked={cpaSyncScheduleEnabled}
    onCheckedChange={(value) => setCpaSyncScheduleEnabledDraft(value)}
  />
</div>

<div className="grid gap-2">
  <Label htmlFor="cpa-sync-interval">同步间隔（分钟）</Label>
  <Input
    id="cpa-sync-interval"
    inputMode="numeric"
    disabled={!cpaSyncScheduleEnabled}
    value={cpaSyncScheduleIntervalInput}
    onChange={(event) => setCpaSyncScheduleIntervalDraft(event.target.value)}
  />
</div>
```

And a summary panel:

```tsx
<div className="rounded-xl border border-border/60 bg-background/40 p-4 text-xs">
  <p>状态：{renderCpaSyncStatus(cpaSyncStatus?.status)}</p>
  <p>间隔：每 {cpaSyncStatus?.intervalMinutes ?? 30} 分钟</p>
  <p>下次计划：{formatTimestamp(cpaSyncStatus?.nextRunAt)}</p>
  <p>最近开始：{formatTimestamp(cpaSyncStatus?.lastStartedAt)}</p>
  <p>最近结束：{formatTimestamp(cpaSyncStatus?.lastFinishedAt)}</p>
  <p>最近结果：{cpaSyncStatus?.lastSummary || "--"}</p>
  <p>最近错误：{cpaSyncStatus?.lastError || "--"}</p>
</div>
```

- [ ] **Step 7: Re-run frontend checks**

Run:

```bash
pnpm exec vitest run apps/src/app/settings/cpa-sync-scheduler-state.test.ts
pnpm run build:desktop
```

Expected: tests pass and desktop build succeeds with the updated settings UI.

- [ ] **Step 8: Commit**

```bash
git add apps/src/app/settings/page.tsx apps/src/lib/api/account-client.ts apps/src/types/index.ts
git commit -m "feat: add cpa sync schedule settings ui"
```

### Task 5: Full Verification and Cleanup

**Files:**
- Modify: `CHANGELOG.md` (only if this repo expects release notes for feature work)
- Verify: `crates/service/src/account/account_cpa_sync.rs`
- Verify: `crates/service/tests/app_settings.rs`
- Verify: `crates/service/tests/rpc.rs`
- Verify: `apps/src/app/settings/page.tsx`

- [ ] **Step 1: Run the full CPA/backend verification suite**

Run:

```bash
cargo test -p codexmanager-service cpa_ -- --nocapture
cargo test -p codexmanager-service app_settings -- --nocapture
cargo test -p codexmanager-service rpc_account_cpa_sync_status_returns_structured_snapshot -- --nocapture
```

Expected: PASS, including new CPA schedule coverage and app settings snapshot persistence.

- [ ] **Step 2: Run the desktop build verification**

Run: `pnpm run build:desktop`

Expected: PASS with static export completing successfully.

- [ ] **Step 3: Inspect final diff for accidental scope creep**

Run:

```bash
git diff --stat origin/codex/auto...HEAD
git diff -- crates/service/src/account/account_cpa_sync.rs crates/service/src/app_settings/api/patch.rs crates/service/src/app_settings/api/current.rs crates/service/src/rpc_dispatch/account.rs apps/src/app/settings/page.tsx apps/src/lib/api/account-client.ts apps/src/types/index.ts
```

Expected: only the planned CPA scheduler, settings, RPC, and UI files changed.

- [ ] **Step 4: Commit any final verification-only fixes**

If verification required no code changes, skip this step. If a final small fix was needed:

```bash
git add crates/service/src/account/account_cpa_sync.rs crates/service/src/lib.rs crates/service/src/app_settings/api/patch.rs crates/service/src/app_settings/api/current.rs crates/service/src/rpc_dispatch/account.rs crates/service/tests/app_settings.rs crates/service/tests/rpc.rs apps/src/app/settings/page.tsx apps/src/lib/api/account-client.ts apps/src/lib/api/transport.ts apps/src/lib/api/normalize.ts apps/src/lib/store/useAppStore.ts apps/src/types/index.ts
git commit -m "fix: finalize cpa sync scheduler verification"
```

- [ ] **Step 5: Prepare handoff summary**

Summarize:

```text
- Added CPA schedule settings and persistence
- Added in-process scheduler and runtime status snapshot
- Added CPA sync status RPC and frontend card status display
- Verified cargo CPA/app-settings tests and pnpm build:desktop
```
