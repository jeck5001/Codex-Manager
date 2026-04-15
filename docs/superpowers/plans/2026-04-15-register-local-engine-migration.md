# Register Local Engine Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the external register-service execution path with a local CPA-style registration engine, starting with `generator_email`, while preserving the current RPC/UI shell.

**Architecture:** Keep the current `account/register/*` RPC contract and frontend modal, but refactor `crates/service/src/account/account_register.rs` from an HTTP proxy into a local orchestrator. Introduce focused Rust modules for runtime state, mailbox providers, HTTP/auth helpers, and the single-task CPA flow, then layer local batch scheduling on top.

**Tech Stack:** Rust service (`reqwest::blocking`, `serde_json`, `OnceLock`, threads/mutexes), existing account import/storage pipeline, Next.js/Tauri frontend, current RPC transport.

---

### Task 1: Add Local Register Runtime Skeleton

**Files:**
- Modify: `crates/service/src/account/mod.rs`
- Modify: `crates/service/src/account/account_register.rs`
- Create: `crates/service/src/account/register_runtime.rs`
- Create: `crates/service/src/account/tests/register_runtime_tests.rs`

- [ ] **Step 1: Write the failing runtime tests**

Add local-runtime tests in `crates/service/src/account/tests/register_runtime_tests.rs`:

```rust
#[test]
fn register_runtime_creates_pending_task_snapshot() {
    let task = super::create_local_register_task_for_test(super::LocalRegisterTaskInput {
        email_service_type: "generator_email".to_string(),
        register_mode: "standard".to_string(),
        proxy: None,
    });

    assert_eq!(task.status, "queued");
    assert_eq!(task.email_service_type, "generator_email");
    assert!(task.task_uuid.starts_with("reg-"));
}

#[test]
fn register_runtime_appends_logs_and_updates_status() {
    let task_uuid = super::create_local_register_task_for_test(super::LocalRegisterTaskInput {
        email_service_type: "generator_email".to_string(),
        register_mode: "standard".to_string(),
        proxy: None,
    })
    .task_uuid;

    super::append_register_task_log_for_test(&task_uuid, "signup email submitted");
    super::set_register_task_status_for_test(&task_uuid, "submitting_signup", None);

    let snapshot = super::read_local_register_task_for_test(&task_uuid).expect("task snapshot");
    assert_eq!(snapshot.status, "submitting_signup");
    assert!(snapshot.logs.iter().any(|line| line.contains("signup email submitted")));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p codexmanager-service register_runtime_ -- --nocapture`

Expected: FAIL because there is no local runtime module or task state yet.

- [ ] **Step 3: Add the local runtime module and wire it into `account/mod.rs`**

Create `crates/service/src/account/register_runtime.rs` with:

```rust
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub(crate) struct LocalRegisterTaskInput {
    pub email_service_type: String,
    pub register_mode: String,
    pub proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalRegisterTaskSnapshot {
    pub task_uuid: String,
    pub status: String,
    pub email_service_type: String,
    pub register_mode: String,
    pub proxy: Option<String>,
    pub failure_code: Option<String>,
    pub error_message: Option<String>,
    pub logs: Vec<String>,
}

#[derive(Default)]
struct LocalRegisterRuntime {
    tasks: HashMap<String, LocalRegisterTaskSnapshot>,
    seq: u64,
}

static LOCAL_REGISTER_RUNTIME: OnceLock<Mutex<LocalRegisterRuntime>> = OnceLock::new();
```

And expose focused helpers:

```rust
pub(crate) fn create_local_register_task(input: LocalRegisterTaskInput) -> LocalRegisterTaskSnapshot;
pub(crate) fn read_local_register_task(task_uuid: &str) -> Option<LocalRegisterTaskSnapshot>;
pub(crate) fn list_local_register_tasks() -> Vec<LocalRegisterTaskSnapshot>;
pub(crate) fn append_register_task_log(task_uuid: &str, line: &str);
pub(crate) fn set_register_task_status(
    task_uuid: &str,
    status: &str,
    failure_code: Option<&str>,
    error_message: Option<&str>,
);
```

Wire the module in `crates/service/src/account/mod.rs`:

```rust
#[path = "register_runtime.rs"]
pub(crate) mod register_runtime;
```

- [ ] **Step 4: Add test-only helpers and pass focused runtime tests**

Add under `#[cfg(test)]` in `register_runtime.rs`:

```rust
pub(crate) fn create_local_register_task_for_test(input: LocalRegisterTaskInput) -> LocalRegisterTaskSnapshot {
    create_local_register_task(input)
}

pub(crate) fn read_local_register_task_for_test(task_uuid: &str) -> Option<LocalRegisterTaskSnapshot> {
    read_local_register_task(task_uuid)
}

pub(crate) fn append_register_task_log_for_test(task_uuid: &str, line: &str) {
    append_register_task_log(task_uuid, line);
}

pub(crate) fn set_register_task_status_for_test(task_uuid: &str, status: &str, failure_code: Option<&str>) {
    set_register_task_status(task_uuid, status, failure_code, None);
}
```

Run: `cargo test -p codexmanager-service register_runtime_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/service/src/account/mod.rs crates/service/src/account/register_runtime.rs crates/service/src/account/tests/register_runtime_tests.rs
git commit -m "feat: add local register runtime skeleton"
```

### Task 2: Add Mailbox Provider Abstraction and `generator_email`

**Files:**
- Modify: `crates/service/src/account/mod.rs`
- Create: `crates/service/src/account/register_email/mod.rs`
- Create: `crates/service/src/account/register_email/generator_email.rs`
- Create: `crates/service/src/account/tests/register_email_generator_tests.rs`

- [ ] **Step 1: Write the failing provider tests**

Add tests in `crates/service/src/account/tests/register_email_generator_tests.rs`:

```rust
#[test]
fn generator_email_parses_address_from_homepage_html() {
    let html = r#"<span id="email_ch_text">alpha123@generator.email</span>"#;
    assert_eq!(
        super::parse_generator_email_address_for_test(html),
        Some("alpha123@generator.email".to_string())
    );
}

#[test]
fn generator_email_builds_surl_from_email() {
    assert_eq!(
        super::generator_email_surl_for_test("Alpha.123@generator.email"),
        Some("generator.email/alpha.123".to_string())
    );
}

#[test]
fn generator_email_extracts_openai_code_from_mailbox_html() {
    let html = "<html><body>Your ChatGPT code is 123456</body></html>";
    assert_eq!(super::extract_generator_email_code_for_test(html), Some("123456".to_string()));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p codexmanager-service generator_email_ -- --nocapture`

Expected: FAIL because the provider module does not exist yet.

- [ ] **Step 3: Create the provider abstraction**

Create `crates/service/src/account/register_email/mod.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterMailboxLease {
    pub email: String,
    pub credential: String,
}

pub(crate) trait RegisterEmailProvider {
    fn create_mailbox(&self) -> Result<RegisterMailboxLease, String>;
    fn fetch_code(&self, credential: &str) -> Result<Option<String>, String>;
}
```

And export:

```rust
pub(crate) mod generator_email;
pub(crate) use generator_email::GeneratorEmailProvider;
```

- [ ] **Step 4: Implement `generator_email` parsing and code extraction**

Create `crates/service/src/account/register_email/generator_email.rs` with pure helpers first:

```rust
pub(crate) fn parse_generator_email_address(html: &str) -> Option<String> { /* span/user/domain parsing */ }
pub(crate) fn build_generator_email_surl(email: &str) -> Option<String> { /* domain/user normalization */ }
pub(crate) fn extract_generator_email_code(html: &str) -> Option<String> { /* direct/contextual/generic 6-digit extraction */ }
```

Then add the provider shell:

```rust
pub(crate) struct GeneratorEmailProvider {
    client: reqwest::blocking::Client,
    base_url: String,
}

impl GeneratorEmailProvider {
    pub(crate) fn new() -> Result<Self, String> { /* generator.email defaults */ }
}

impl RegisterEmailProvider for GeneratorEmailProvider {
    fn create_mailbox(&self) -> Result<RegisterMailboxLease, String> { /* GET homepage + parse + surl */ }
    fn fetch_code(&self, credential: &str) -> Result<Option<String>, String> { /* GET mailbox page + parse code */ }
}
```

- [ ] **Step 5: Add test-only wrappers and pass focused provider tests**

Expose `parse_generator_email_address_for_test`, `generator_email_surl_for_test`, and `extract_generator_email_code_for_test`.

Run: `cargo test -p codexmanager-service generator_email_ -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/service/src/account/mod.rs crates/service/src/account/register_email/mod.rs crates/service/src/account/register_email/generator_email.rs crates/service/src/account/tests/register_email_generator_tests.rs
git commit -m "feat: add generator email register provider"
```

### Task 3: Add Local OpenAI/Auth HTTP Helpers

**Files:**
- Create: `crates/service/src/account/register_http.rs`
- Create: `crates/service/src/account/tests/register_http_tests.rs`
- Modify: `crates/service/src/account/mod.rs`

- [ ] **Step 1: Write the failing HTTP/auth helper tests**

Add tests in `crates/service/src/account/tests/register_http_tests.rs`:

```rust
#[test]
fn register_http_parses_callback_query_and_fragment() {
    let parsed = super::parse_register_callback_for_test(
        "http://localhost:1455/auth/callback?code=abc&state=xyz"
    );
    assert_eq!(parsed.code, "abc");
    assert_eq!(parsed.state, "xyz");
}

#[test]
fn register_http_builds_oauth_start_with_pkce() {
    let start = super::generate_register_oauth_start_for_test();
    assert!(start.auth_url.contains("code_challenge="));
    assert!(!start.state.is_empty());
    assert!(!start.code_verifier.is_empty());
}

#[test]
fn register_http_extracts_auth_claims_from_id_token_payload() {
    let claims = super::extract_id_token_claims_for_test(
        "header.eyJlbWFpbCI6InVzZXJAZXhhbXBsZS5jb20iLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjLTEifX0.sig"
    );
    assert_eq!(claims.email.as_deref(), Some("user@example.com"));
    assert_eq!(claims.account_id.as_deref(), Some("acc-1"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p codexmanager-service register_http_ -- --nocapture`

Expected: FAIL because `register_http.rs` does not exist yet.

- [ ] **Step 3: Implement focused helper types and parsers**

Create `crates/service/src/account/register_http.rs` with:

```rust
#[derive(Debug, Clone)]
pub(crate) struct RegisterOAuthStart {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RegisterCallbackParams {
    pub code: String,
    pub state: String,
    pub error: String,
    pub error_description: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RegisterIdTokenClaims {
    pub email: Option<String>,
    pub account_id: Option<String>,
}
```

Add helpers mirroring the CPA Python flow:

```rust
pub(crate) fn generate_register_oauth_start() -> RegisterOAuthStart;
pub(crate) fn parse_register_callback(callback_url: &str) -> RegisterCallbackParams;
pub(crate) fn extract_id_token_claims(id_token: &str) -> RegisterIdTokenClaims;
pub(crate) fn submit_register_callback(/* token endpoint args */) -> Result<String, String>;
```

- [ ] **Step 4: Add test-only wrappers and pass focused helper tests**

Run: `cargo test -p codexmanager-service register_http_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/service/src/account/mod.rs crates/service/src/account/register_http.rs crates/service/src/account/tests/register_http_tests.rs
git commit -m "feat: add local register http helpers"
```

### Task 4: Implement Single-Task CPA Flow and Replace `account/register/start`

**Files:**
- Modify: `crates/service/src/account/account_register.rs`
- Create: `crates/service/src/account/register_engine.rs`
- Create: `crates/service/src/account/tests/register_engine_tests.rs`
- Modify: `crates/service/src/rpc_dispatch/account.rs`
- Modify: `crates/service/tests/register_payload_forwarding.rs`

- [ ] **Step 1: Write the failing local-engine tests**

Add tests in `crates/service/src/account/tests/register_engine_tests.rs`:

```rust
#[test]
fn register_engine_runs_generator_email_flow_to_importable_result() {
    let result = super::run_local_register_flow_for_test(super::RegisterEngineTestScenario::success())
        .expect("register flow");

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.email.as_deref(), Some("alpha123@generator.email"));
    assert!(result.payload.contains("\"refresh_token\""));
}

#[test]
fn register_engine_marks_otp_timeout_when_code_never_arrives() {
    let err = super::run_local_register_flow_for_test(super::RegisterEngineTestScenario::otp_timeout())
        .expect_err("otp timeout");

    assert!(err.contains("otp_timeout"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p codexmanager-service register_engine_ -- --nocapture`

Expected: FAIL because the local engine module is not implemented.

- [ ] **Step 3: Implement the single-task state machine**

Create `crates/service/src/account/register_engine.rs` with:

```rust
#[derive(Debug, Clone)]
pub(crate) struct RegisterEngineResult {
    pub status: String,
    pub email: Option<String>,
    pub payload: String,
}

pub(crate) fn run_local_register_flow(
    task_uuid: &str,
    input: &crate::account::register_runtime::LocalRegisterTaskSnapshot,
) -> Result<RegisterEngineResult, String> {
    // queued -> preparing_email -> submitting_signup -> waiting_email_otp -> ...
}
```

Within the function:

- resolve `GeneratorEmailProvider`
- allocate mailbox and append task logs
- call `register_http` helpers for OAuth/callback utilities
- update runtime statuses at each step
- produce normalized token JSON compatible with `account_import::import_account_auth_json`

- [ ] **Step 4: Replace `start_register_task()` with local orchestration**

Refactor `crates/service/src/account/account_register.rs`:

```rust
pub(crate) fn start_register_task(/* existing args */) -> Result<Value, String> {
    let snapshot = crate::account::register_runtime::create_local_register_task(...);
    std::thread::spawn({
        let task_uuid = snapshot.task_uuid.clone();
        move || {
            let _ = crate::account::register_engine::run_local_register_flow(&task_uuid, &snapshot);
        }
    });

    Ok(json!({
        "taskUuid": snapshot.task_uuid,
        "status": snapshot.status,
        "emailServiceType": snapshot.email_service_type,
    }))
}
```

Also replace the old forwarding test in `crates/service/tests/register_payload_forwarding.rs` with an assertion that `account/register/start` no longer requires an external register-service URL for `generator_email`.

- [ ] **Step 5: Pass focused single-task tests**

Run:

```bash
cargo test -p codexmanager-service register_engine_ -- --nocapture
cargo test -p codexmanager-service rpc_register_ -- --nocapture
```

Expected: PASS for the new local-engine tests and updated RPC/register tests.

- [ ] **Step 6: Commit**

```bash
git add crates/service/src/account/account_register.rs crates/service/src/account/register_engine.rs crates/service/src/account/tests/register_engine_tests.rs crates/service/src/rpc_dispatch/account.rs crates/service/tests/register_payload_forwarding.rs
git commit -m "feat: run single register tasks locally"
```

### Task 5: Replace Task Read / Import / Cancel With Local Runtime State

**Files:**
- Modify: `crates/service/src/account/account_register.rs`
- Modify: `crates/service/src/account/register_runtime.rs`
- Create: `crates/service/src/account/tests/register_task_snapshot_tests.rs`
- Modify: `crates/service/tests/rpc.rs`

- [ ] **Step 1: Write the failing task snapshot tests**

Add tests in `crates/service/src/account/tests/register_task_snapshot_tests.rs`:

```rust
#[test]
fn read_register_task_returns_local_runtime_snapshot() {
    let task = super::seed_completed_local_register_task_for_test("user@example.com");
    let snapshot = super::read_register_task(&task.task_uuid).expect("task snapshot");

    assert_eq!(snapshot.status(), "completed");
    assert_eq!(snapshot.email(), Some("user@example.com"));
    assert!(snapshot.can_import());
}

#[test]
fn import_register_task_uses_local_payload_and_marks_imported() {
    let task = super::seed_completed_local_register_task_for_test("import@example.com");
    let imported = super::import_register_task(&task.task_uuid).expect("import task");
    assert!(imported.get("created").is_some() || imported.get("updated").is_some());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p codexmanager-service register_task_snapshot_ -- --nocapture`

Expected: FAIL because reads/imports still expect external-service payloads.

- [ ] **Step 3: Refactor task reads/imports to local data**

Update `account_register.rs` so:

- `read_register_task()` reads from `register_runtime`
- `list_register_tasks()` reads from `register_runtime`
- `cancel_register_task()` sets a local canceled flag
- `import_register_task()` uses the locally stored payload/result

Expected local result shape:

```rust
json!({
    "email": email,
    "payload": normalized_payload,
    "importedAccountId": imported_account_id,
    "isImported": true
})
```

- [ ] **Step 4: Add an RPC integration test for local task reads**

Add to `crates/service/tests/rpc.rs`:

```rust
#[test]
fn rpc_register_task_read_returns_local_status_and_logs() {
    let _ctx = RpcTestContext::new("rpc-register-task-read");
    let result = /* seed local task + call account/register/task/read */;
    assert!(result.get("status").is_some());
    assert!(result.get("logs").is_some());
}
```

- [ ] **Step 5: Pass focused task snapshot tests**

Run:

```bash
cargo test -p codexmanager-service register_task_snapshot_ -- --nocapture
cargo test -p codexmanager-service rpc_register_task_read_returns_local_status_and_logs -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/service/src/account/account_register.rs crates/service/src/account/register_runtime.rs crates/service/src/account/tests/register_task_snapshot_tests.rs crates/service/tests/rpc.rs
git commit -m "feat: read local register task snapshots"
```

### Task 6: Implement Local Batch Runtime and Replace `account/register/batch/start`

**Files:**
- Modify: `crates/service/src/account/account_register.rs`
- Modify: `crates/service/src/account/register_runtime.rs`
- Create: `crates/service/src/account/tests/register_batch_runtime_tests.rs`
- Modify: `crates/service/tests/register_payload_forwarding.rs`
- Modify: `apps/src/components/modals/add-account-modal.tsx`

- [ ] **Step 1: Write the failing batch runtime tests**

Add tests in `crates/service/src/account/tests/register_batch_runtime_tests.rs`:

```rust
#[test]
fn register_batch_runtime_creates_multiple_local_tasks() {
    let batch = super::start_local_register_batch_for_test(super::LocalRegisterBatchInput {
        email_service_type: "generator_email".to_string(),
        count: 3,
        interval_min: 0,
        interval_max: 0,
        concurrency: 1,
        mode: "pipeline".to_string(),
    })
    .expect("start batch");

    assert_eq!(batch.total, 3);
    assert_eq!(batch.task_uuids.len(), 3);
}

#[test]
fn register_batch_runtime_cancel_prevents_new_tasks_from_starting() {
    let batch = super::start_local_register_batch_for_test(/* same input */).expect("start batch");
    super::cancel_local_register_batch_for_test(&batch.batch_id).expect("cancel batch");
    let snapshot = super::read_local_register_batch_for_test(&batch.batch_id).expect("batch snapshot");
    assert_eq!(snapshot.status, "cancelled");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p codexmanager-service register_batch_runtime_ -- --nocapture`

Expected: FAIL because batch runtime is still external.

- [ ] **Step 3: Add local batch structures and scheduler**

Extend `register_runtime.rs` with:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalRegisterBatchSnapshot {
    pub batch_id: String,
    pub status: String,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub task_uuids: Vec<String>,
}
```

Add:

```rust
pub(crate) fn create_local_register_batch(/* input */) -> Result<LocalRegisterBatchSnapshot, String>;
pub(crate) fn read_local_register_batch(batch_id: &str) -> Option<LocalRegisterBatchSnapshot>;
pub(crate) fn cancel_local_register_batch(batch_id: &str) -> Result<(), String>;
```

Implement a small scheduler loop that:

- respects configured concurrency
- waits between launches according to `interval_min` / `interval_max`
- stops dispatching after cancellation

- [ ] **Step 4: Replace `start_register_batch()` and batch reads with local runtime**

Refactor `account_register.rs` so:

- `start_register_batch()` creates local batch + tasks
- `read_register_batch()` reads local batch snapshots
- `cancel_register_batch()` cancels local batch runtime

Update `apps/src/components/modals/add-account-modal.tsx` only if local batch status names need explicit labels beyond the current fallback behavior.

- [ ] **Step 5: Pass focused batch tests**

Run:

```bash
cargo test -p codexmanager-service register_batch_runtime_ -- --nocapture
cargo test -p codexmanager-service rpc_register_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/service/src/account/account_register.rs crates/service/src/account/register_runtime.rs crates/service/src/account/tests/register_batch_runtime_tests.rs crates/service/tests/register_payload_forwarding.rs apps/src/components/modals/add-account-modal.tsx
git commit -m "feat: add local register batch runtime"
```

### Task 7: Final Cleanup, Verification, and Legacy Proxy Reduction

**Files:**
- Modify: `crates/service/src/account/account_register.rs`
- Verify: `crates/service/src/account/register_runtime.rs`
- Verify: `crates/service/src/account/register_engine.rs`
- Verify: `crates/service/src/account/register_email/generator_email.rs`
- Verify: `crates/service/tests/register_payload_forwarding.rs`
- Verify: `crates/service/tests/rpc.rs`
- Verify: `apps/src/components/modals/add-account-modal.tsx`

- [ ] **Step 1: Remove local-engine blockers from the old proxy path**

Delete or fence the old external-only helpers that are no longer used by:

- `start_register_task()`
- `start_register_batch()`
- `read_register_task()`
- `list_register_tasks()`
- `cancel_register_task()`
- `read_register_batch()`
- `cancel_register_batch()`

Keep temporary fallback code only where explicitly required for not-yet-migrated scenarios.

- [ ] **Step 2: Run the register verification suite**

Run:

```bash
cargo test -p codexmanager-service register_runtime_ -- --nocapture
cargo test -p codexmanager-service generator_email_ -- --nocapture
cargo test -p codexmanager-service register_http_ -- --nocapture
cargo test -p codexmanager-service register_engine_ -- --nocapture
cargo test -p codexmanager-service register_task_snapshot_ -- --nocapture
cargo test -p codexmanager-service register_batch_runtime_ -- --nocapture
cargo test -p codexmanager-service rpc_register_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run desktop build verification**

Run: `pnpm run build:desktop`

Expected: PASS.

- [ ] **Step 4: Inspect final diff for scope control**

Run:

```bash
git diff --stat origin/codex/auto...HEAD
git diff -- crates/service/src/account/account_register.rs crates/service/src/account/register_runtime.rs crates/service/src/account/register_engine.rs crates/service/src/account/register_http.rs crates/service/src/account/register_email/mod.rs crates/service/src/account/register_email/generator_email.rs crates/service/tests/register_payload_forwarding.rs crates/service/tests/rpc.rs apps/src/components/modals/add-account-modal.tsx
```

Expected: only local register engine migration files changed.

- [ ] **Step 5: Commit any final cleanup**

```bash
git add crates/service/src/account/account_register.rs crates/service/src/account/register_runtime.rs crates/service/src/account/register_engine.rs crates/service/src/account/register_http.rs crates/service/src/account/register_email/mod.rs crates/service/src/account/register_email/generator_email.rs crates/service/tests/register_payload_forwarding.rs crates/service/tests/rpc.rs apps/src/components/modals/add-account-modal.tsx
git commit -m "fix: finalize local register engine migration"
```

- [ ] **Step 6: Prepare handoff summary**

Summarize:

```text
- Replaced external register-service execution with a local runtime
- Migrated generator_email mailbox creation and OTP extraction
- Rebuilt the CPA-style single-task registration flow locally
- Replaced local task/batch reads and cancellation with in-process runtime state
- Verified Rust register tests and pnpm build:desktop
```
