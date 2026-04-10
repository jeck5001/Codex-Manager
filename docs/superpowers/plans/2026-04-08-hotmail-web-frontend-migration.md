# Hotmail Web Frontend Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the existing Hotmail batch registration flow inside the real Next.js frontend used at port `48761`.

**Architecture:** Reuse the existing `vendor/codex-register` Hotmail backend and bridge it through `crates/service` RPC, then add a dedicated `/hotmail` page in `apps/src` that follows the current register/email-service UI patterns. Keep the feature scoped to one trackable active batch to minimize new state complexity.

**Tech Stack:** Rust (`crates/service`), Next.js App Router, TypeScript strict mode, TanStack Query, shadcn/ui

---

### Task 1: Lock the UI entry points

**Files:**
- Modify: `apps/src/lib/navigation.ts`
- Modify: `apps/src/components/layout/header.tsx`
- Test: `apps/src/app/hotmail/navigation.test.ts`

- [ ] **Step 1: Write the failing navigation test**

```ts
import { APP_NAV_ITEMS } from "@/lib/navigation";

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

const hotmail = APP_NAV_ITEMS.find((item) => item.id === "hotmail");
assert(hotmail, "hotmail nav item should exist");
assert(hotmail?.href === "/hotmail/", "hotmail nav item should point to /hotmail/");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps exec tsx src/app/hotmail/navigation.test.ts`
Expected: FAIL with `hotmail nav item should exist`

- [ ] **Step 3: Add the navigation/header implementation**

```ts
{ id: "hotmail", name: "Hotmail", href: "/hotmail/" }
```

```ts
case "/hotmail":
  return "Hotmail";
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps exec tsx src/app/hotmail/navigation.test.ts`
Expected: PASS with no output

- [ ] **Step 5: Commit**

```bash
git add apps/src/lib/navigation.ts apps/src/components/layout/header.tsx apps/src/app/hotmail/navigation.test.ts
git commit -m "test: add hotmail navigation entry coverage"
```

### Task 2: Bridge Hotmail batch APIs through service RPC

**Files:**
- Modify: `crates/service/src/account/account_register.rs`
- Modify: `crates/service/src/rpc_dispatch/account.rs`

- [ ] **Step 1: Write the failing RPC path expectations**

```rust
// Add assertions in the existing account register tests (or create a focused test)
// that calling the new handlers with empty batch ids returns the same validation
// errors as other batch handlers.
assert_eq!(
    account_register::read_register_hotmail_batch(""),
    Err("batchId is required".to_string())
);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p codexmanager-service hotmail_batch -- --nocapture`
Expected: FAIL because `read_register_hotmail_batch` does not exist yet

- [ ] **Step 3: Implement the proxy functions and RPC routes**

```rust
register_post_json("/api/hotmail/batches", &payload)
register_get_json(&format!("/api/hotmail/batches/{batch_id}"))
register_post_json(&format!("/api/hotmail/batches/{batch_id}/cancel"), &json!({}))
register_get_json(&format!("/api/hotmail/batches/{batch_id}/artifacts"))
```

```rust
"account/register/hotmailBatch/start" => { ... }
"account/register/hotmailBatch/read" => { ... }
"account/register/hotmailBatch/cancel" => { ... }
"account/register/hotmailBatch/artifacts" => { ... }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p codexmanager-service hotmail_batch -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/service/src/account/account_register.rs crates/service/src/rpc_dispatch/account.rs
git commit -m "feat: bridge hotmail batch rpc endpoints"
```

### Task 3: Add typed frontend client coverage

**Files:**
- Modify: `apps/src/lib/api/transport.ts`
- Modify: `apps/src/lib/api/account-client.ts`
- Modify: `apps/src/types/index.ts`
- Test: `apps/src/app/hotmail/account-client-normalize.test.ts`

- [ ] **Step 1: Write the failing normalize test**

```ts
import { accountClient } from "@/lib/api/account-client";

void accountClient;
throw new Error("replace with hotmail normalize coverage");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps exec tsx src/app/hotmail/account-client-normalize.test.ts`
Expected: FAIL

- [ ] **Step 3: Add transport mappings, types, and client methods**

```ts
service_account_register_hotmail_batch_start
service_account_register_hotmail_batch_read
service_account_register_hotmail_batch_cancel
service_account_register_hotmail_batch_artifacts
```

```ts
async startRegisterHotmailBatch(...)
async getRegisterHotmailBatch(...)
async cancelRegisterHotmailBatch(...)
async getRegisterHotmailBatchArtifacts(...)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps exec tsx src/app/hotmail/account-client-normalize.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/src/lib/api/transport.ts apps/src/lib/api/account-client.ts apps/src/types/index.ts apps/src/app/hotmail/account-client-normalize.test.ts
git commit -m "feat: add hotmail frontend api client"
```

### Task 4: Build the Hotmail page

**Files:**
- Create: `apps/src/app/hotmail/page.tsx`

- [ ] **Step 1: Write a failing page-state smoke test**

```ts
throw new Error("replace with hotmail page helper coverage");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps exec tsx src/app/hotmail/page-state.test.ts`
Expected: FAIL

- [ ] **Step 3: Implement the page**

```tsx
export default function HotmailPage() {
  // form state
  // create batch
  // poll active batch
  // cancel batch
  // render logs and artifacts
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps exec tsx src/app/hotmail/page-state.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/src/app/hotmail/page.tsx apps/src/app/hotmail/page-state.test.ts
git commit -m "feat: add hotmail page to web frontend"
```

### Task 5: Final verification

**Files:**
- Modify: `docs/superpowers/specs/2026-04-08-hotmail-web-frontend-migration-design.md`
- Modify: `docs/superpowers/plans/2026-04-08-hotmail-web-frontend-migration.md`

- [ ] **Step 1: Run focused frontend smoke tests**

Run: `pnpm --dir apps exec tsx src/app/hotmail/navigation.test.ts`
Expected: PASS

Run: `pnpm --dir apps exec tsx src/app/hotmail/account-client-normalize.test.ts`
Expected: PASS

- [ ] **Step 2: Run service tests**

Run: `cargo test -p codexmanager-service hotmail_batch -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run desktop build verification**

Run: `pnpm --dir apps run build:desktop`
Expected: PASS

- [ ] **Step 4: Commit the completed migration**

```bash
git add apps/src crates/service/src docs/superpowers/specs/2026-04-08-hotmail-web-frontend-migration-design.md docs/superpowers/plans/2026-04-08-hotmail-web-frontend-migration.md
git commit -m "feat: expose hotmail flow in web frontend"
```
