# CodexManager Rename + Frontend Move Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename visible branding to CodexManager and move the frontend from `apps/gpttools-desktop` to `apps/`, deleting the old folder.

**Architecture:** Keep the Rust workspace `crates/` layout unchanged. Frontend root becomes `apps/` with `src/`, `src-tauri/`, `dist/`, and `package.json` directly under it. Update path references and Tauri config only.

**Tech Stack:** Rust (Tauri), Vite, plain HTML/CSS/JS, Node.js tests

---

### Task 1: Add regression test for new layout/branding

**Files:**
- Create: `apps/tests/codexmanager-layout.test.mjs`

**Step 1: Write the failing test**

Create `apps/tests/codexmanager-layout.test.mjs` with checks for:
- `apps/src-tauri/tauri.conf.json` exists and has `productName` + `app.windows[0].title` = `CodexManager`
- `apps/index.html` contains `<title>CodexManager</title>` and `<h1>CodexManager</h1>`
- `apps/dist/index.html` contains `<title>CodexManager</title>` and `<h1>CodexManager</h1>`

**Step 2: Run test to verify it fails**

Run:
```powershell
node --test apps\tests\codexmanager-layout.test.mjs
```
Expected: FAIL because `apps/src-tauri/tauri.conf.json` does not exist yet (still under `apps/gpttools-desktop`).

**Step 3: Commit**
```bash
git add apps/tests/codexmanager-layout.test.mjs
git commit -m "test: add layout/branding expectations"
```

---

### Task 2: Move frontend to `apps/` and update branding/paths

**Files:**
- Move: all contents of `apps/gpttools-desktop/*` → `apps/`
- Delete: `apps/gpttools-desktop/`
- Modify: `apps/index.html`
- Modify: `apps/dist/index.html`
- Modify: `apps/src-tauri/tauri.conf.json`
- Modify: `apps/src-tauri/Cargo.toml`
- Modify: `apps/src-tauri/src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `README.md`

**Step 1: Move contents**

Run:
```powershell
Get-ChildItem -Path .\apps\gpttools-desktop | ForEach-Object { Move-Item -Force $_.FullName .\apps }
```
Expected: `package.json`, `src`, `src-tauri`, `dist` now exist under `apps/`.

**Step 2: Remove old folder**

Run:
```powershell
Remove-Item -Force -Recurse .\apps\gpttools-desktop
```
Expected: folder removed.

**Step 3: Update branding**
- `apps/index.html`: change `<title>` + brand text from GPTTools → CodexManager.
- `apps/dist/index.html`: change `<title>` + brand text from GPTTools → CodexManager.
- `apps/src-tauri/tauri.conf.json`: change `productName` and `app.windows[0].title` to CodexManager.
- `README.md`: change project name to CodexManager.

**Step 4: Update path references**
- `Cargo.toml`: change exclude from `apps/gpttools-desktop/src-tauri` → `apps/src-tauri`.
- `apps/src-tauri/Cargo.toml`: update path dependencies from `../../../crates/...` → `../../crates/...`.
- `apps/src-tauri/src/lib.rs`: update `workspace_root` path traversal by one level (from three `..` to two).

**Step 5: Run test to verify it passes**

Run:
```powershell
node --test apps\tests\codexmanager-layout.test.mjs
```
Expected: PASS.

**Step 6: Commit**
```bash
git add apps Cargo.toml README.md
git commit -m "chore: move frontend to apps root and rename branding"
```

---

## Rollback Plan
- Move contents back into `apps/gpttools-desktop`.
- Restore `Cargo.toml` exclude path.
- Revert branding changes in HTML/Tauri config/README.
- Revert `apps/src-tauri` path updates.
