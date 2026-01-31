# Service Toggle Button + No CMD Window Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Start/Stop with a single toggle button (with loading state) and eliminate any CMD window when starting/stopping the service.

**Architecture:** Frontend uses a single toggle button with busy state. Backend service binary is built as Windows subsystem to avoid console window. Keep existing RPC flow; add retry/timeout only where needed.

**Tech Stack:** Tauri (Rust), plain HTML/CSS/JS, Node test runner, Cargo tests.

---

### Task 1: Add failing UI test for single toggle button

**Files:**
- Create: `apps/tests/service-toggle.test.mjs`

**Step 1: Write failing test**

Test should assert:
- `apps/index.html` has `id="serviceToggle"`.
- `apps/index.html` does **not** have `serviceStart` or `serviceStop`.
- `apps/src/ui/dom.js` maps `serviceToggle`.
- `apps/src/main.js` references `serviceToggle`.
- `apps/src/state.js` includes `serviceBusy`.

**Step 2: Run test to verify it fails**

Run:
```powershell
node --test apps\tests\service-toggle.test.mjs
```
Expected: FAIL because old start/stop IDs still exist.

---

### Task 2: Implement single toggle button + loading state

**Files:**
- Modify: `apps/index.html`
- Modify: `apps/src/ui/dom.js`
- Modify: `apps/src/state.js`
- Modify: `apps/src/main.js`
- Modify: `apps/src/styles/components.css`

**Step 1: Update HTML**
- Replace start/stop buttons with one `#serviceToggle`.

**Step 2: Update DOM mapping + state**
- Replace `serviceStartBtn`/`serviceStopBtn` with `serviceToggleBtn`.
- Add `serviceBusy` to state.

**Step 3: Update JS logic**
- Implement `updateServiceToggle()` and `setServiceBusy()`.
- Hook toggle button to start/stop depending on connection.
- Show loading text and disable button while busy.
- Clear auto-refresh timer on stop.

**Step 4: Add loading styles**
- Add `.is-loading` style and spinner keyframes.

**Step 5: Run test to verify it passes**

Run:
```powershell
node --test apps\tests\service-toggle.test.mjs
```
Expected: PASS.

---

### Task 3: Add failing service default/console test

**Files:**
- Create: `crates/gpttools-service/tests/default_addr.rs`

**Step 1: Write failing test**
- Assert `gpttools_service::DEFAULT_ADDR == "localhost:5050"`.

**Step 2: Run test to verify it fails**

Run:
```powershell
cargo test -p gpttools-service --test default_addr
```
Expected: FAIL because `DEFAULT_ADDR` is missing or different.

---

### Task 4: Update service binary to avoid CMD window

**Files:**
- Modify: `crates/gpttools-service/src/lib.rs`
- Modify: `crates/gpttools-service/src/main.rs`

**Step 1: Implement DEFAULT_ADDR constant**
- Add `pub const DEFAULT_ADDR: &str = "localhost:5050";`
- Update `main.rs` to use it.

**Step 2: Hide console window**
- Add `#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]` to `main.rs`.

**Step 3: Run test to verify it passes**

Run:
```powershell
cargo test -p gpttools-service --test default_addr
```
Expected: PASS.

---

### Task 5: Verification

**Step 1: Run UI tests**
```powershell
node --test apps\tests\service-toggle.test.mjs
node --test apps\src\services\connection.test.js
```
Expected: PASS.

**Step 2: Run service test**
```powershell
cargo test -p gpttools-service --test default_addr
```
Expected: PASS.

---

## Rollback Plan
- Restore `apps/index.html` start/stop buttons and DOM bindings.
- Remove `serviceBusy` and toggle logic.
- Remove `DEFAULT_ADDR` constant and Windows subsystem attribute.
- Delete tests added in this plan.
