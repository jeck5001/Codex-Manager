# Account Auth Recovery Register Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make refresh-token-expired recovery fully automatic by falling back to the register service instead of returning an interactive browser-login flow.

**Architecture:** Keep the existing silent refresh path as the first attempt in `auth_login`. When it fails, recover the account by locating the matching register-service account by email label, triggering register-service token refresh, importing the refreshed tokens/cookies back into local storage, and returning a recovered status to the frontend. The frontend will treat recovery as non-interactive and stop opening browser windows for this path.

**Tech Stack:** Rust service RPC/auth/account modules, TypeScript React hook/client API, tiny_http RPC integration tests, reqwest blocking register-service client helpers.

---

### Task 1: Add failing integration test for register fallback recovery

**Files:**
- Modify: `crates/service/tests/rpc.rs`

- [ ] **Step 1: Write the failing test**

Add a new RPC integration test that seeds a local account with an expired refresh token, configures a mock register service, expects `/api/accounts`, `/api/accounts/{id}/refresh`, and `/api/accounts/{id}/tokens` to be called, and asserts `account/auth/recover` returns `status = "recovered"` instead of `pending_login`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p codexmanager-service --test rpc rpc_account_auth_recover_falls_back_to_register_service_refresh -- --nocapture`

Expected: FAIL because `account/auth/recover` still returns `pending_login` and never hits the register-service fallback.

### Task 2: Implement register-service recovery fallback in the backend

**Files:**
- Modify: `crates/service/src/account/account_register.rs`
- Modify: `crates/service/src/auth/auth_login.rs`

- [ ] **Step 1: Add register-service refresh/import helper**

Expose a helper in `account_register.rs` that:
1. resolves a remote account by email
2. POSTs `/api/accounts/{id}/refresh`
3. validates the response `success == true`
4. imports the refreshed account tokens/cookies back into local storage

- [ ] **Step 2: Use the helper from auth recovery**

Update `recover_account_auth` in `auth_login.rs` so the order becomes:
1. local silent refresh
2. register-service refresh/import using resolved email
3. only error out if both fail

Do not create a login session or return `pending_login` for this automatic recovery flow.

- [ ] **Step 3: Run targeted tests**

Run: `cargo test -p codexmanager-service --test rpc rpc_account_auth_recover -- --nocapture`

Expected: PASS with both the silent-refresh test and the register-fallback test green.

### Task 3: Remove frontend interactive fallback for auth recovery

**Files:**
- Modify: `apps/src/hooks/useAccounts.ts`

- [ ] **Step 1: Simplify recovery handling**

Update `recoverAccountAfterRefreshFailure` to assume the backend recovery is non-interactive. Remove browser-opening and login-status polling from this path, and treat any non-`recovered` response as a backend recovery failure.

- [ ] **Step 2: Run frontend build verification**

Run: `cd apps && pnpm run build:desktop`

Expected: PASS with no TypeScript or static export errors.

### Task 4: Final verification

**Files:**
- Modify: `crates/service/tests/rpc.rs`
- Modify: `crates/service/src/account/account_register.rs`
- Modify: `crates/service/src/auth/auth_login.rs`
- Modify: `apps/src/hooks/useAccounts.ts`

- [ ] **Step 1: Run backend regression verification**

Run: `cargo test -p codexmanager-service --test rpc rpc_account_auth_recover -- --nocapture`

Expected: PASS

- [ ] **Step 2: Run desktop build verification**

Run: `cd apps && pnpm run build:desktop`

Expected: PASS
