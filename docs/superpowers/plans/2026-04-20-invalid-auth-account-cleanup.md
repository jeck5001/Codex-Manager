# Invalid Auth Account Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an accounts-page bulk cleanup action that removes accounts whose latest status reason shows an unrecoverable auth failure, using the current filtered account scope.

**Architecture:** Keep this feature frontend-only and reuse the existing `account/deleteMany` flow. Put the “is this reason unrecoverable?” logic in a small accounts-page helper module with its own node-test coverage, then have `apps/src/app/accounts/page.tsx` derive matching IDs from `filteredAccounts` and route the action through the existing bulk delete confirmation dialog.

**Tech Stack:** Next.js App Router client page, TypeScript strict mode, `node:test`, existing TanStack Query account mutations.

---

### Task 1: Add failing tests for invalid-auth cleanup matching

**Files:**
- Create: `apps/src/app/accounts/account-cleanup.test.ts`
- Create: `apps/src/app/accounts/account-cleanup.ts`

- [ ] **Step 1: Write the failing test**

```ts
import test from "node:test";
import assert from "node:assert/strict";

import {
  collectInvalidAuthCleanupAccountIds,
  isInvalidAuthCleanupStatusReason,
} from "./account-cleanup.ts";

test("isInvalidAuthCleanupStatusReason matches unrecoverable auth labels", () => {
  assert.equal(isInvalidAuthCleanupStatusReason("Refresh 已过期"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("Refresh 已复用"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("Refresh 已失效"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("登录态已失效"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("授权失效"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("检测到账号已停用"), false);
});

test("collectInvalidAuthCleanupAccountIds only returns filtered invalid-auth accounts", () => {
  const ids = collectInvalidAuthCleanupAccountIds([
    { id: "acc-reused", lastStatusReason: "Refresh 已复用" },
    { id: "acc-invalidated", lastStatusReason: "登录态已失效" },
    { id: "acc-ok", lastStatusReason: "用量恢复正常" },
    { id: "acc-empty", lastStatusReason: null },
  ]);

  assert.deepEqual(ids, ["acc-reused", "acc-invalidated"]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test apps/src/app/accounts/account-cleanup.test.ts`

Expected: FAIL with module-not-found or missing export errors because `account-cleanup.ts` does not exist yet.

- [ ] **Step 3: Write minimal helper implementation**

```ts
const INVALID_AUTH_STATUS_REASONS = new Set([
  "Refresh 已过期",
  "Refresh 已复用",
  "Refresh 已失效",
  "Refresh 刷新失败",
  "登录态已失效",
  "授权失效",
  "Refresh 连续失效",
]);

export function isInvalidAuthCleanupStatusReason(
  value: string | null | undefined,
): boolean {
  const label = String(value || "").trim();
  return label.length > 0 && INVALID_AUTH_STATUS_REASONS.has(label);
}

export function collectInvalidAuthCleanupAccountIds(
  accounts: Array<{ id: string; lastStatusReason: string | null | undefined }>,
): string[] {
  return accounts
    .filter((account) => isInvalidAuthCleanupStatusReason(account.lastStatusReason))
    .map((account) => account.id);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test apps/src/app/accounts/account-cleanup.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/src/app/accounts/account-cleanup.ts apps/src/app/accounts/account-cleanup.test.ts
git commit -m "test: cover invalid auth cleanup matching"
```

### Task 2: Wire the accounts-page bulk cleanup action

**Files:**
- Modify: `apps/src/app/accounts/page.tsx`
- Create: `apps/src/app/accounts/account-cleanup.ts`

- [ ] **Step 1: Extend the page with derived invalid-auth cleanup ids**

Add the helper import and a memo derived from the current filtered scope:

```ts
import { collectInvalidAuthCleanupAccountIds } from "./account-cleanup";

const filteredInvalidAuthCleanupIds = useMemo(
  () => collectInvalidAuthCleanupAccountIds(filteredAccounts),
  [filteredAccounts],
);
```

- [ ] **Step 2: Add the bulk cleanup handler**

Reuse the existing delete confirmation dialog instead of creating a new RPC or modal:

```ts
const handleDeleteInvalidAuthAccounts = () => {
  if (!filteredInvalidAuthCleanupIds.length) {
    toast.info("当前筛选范围内没有可清理的失效登录态账号");
    return;
  }
  setDeleteDialogState({
    kind: "selected",
    ids: [...filteredInvalidAuthCleanupIds],
    count: filteredInvalidAuthCleanupIds.length,
  });
};
```

- [ ] **Step 3: Add the cleanup menu entry**

Insert a destructive menu action alongside “一键清理封禁账号” and “一键清理不可用免费”:

```tsx
<DropdownMenuItem
  disabled={!filteredInvalidAuthCleanupIds.length || isDeletingMany}
  variant="destructive"
  className="h-9 rounded-lg px-2"
  onClick={handleDeleteInvalidAuthAccounts}
>
  <Trash2 className="mr-2 h-4 w-4" /> 一键清理失效登录态
  <DropdownMenuShortcut>
    {isDeletingMany ? "..." : filteredInvalidAuthCleanupIds.length || "-"}
  </DropdownMenuShortcut>
</DropdownMenuItem>
```

- [ ] **Step 4: Verify the delete confirmation still uses the shared bulk-delete path**

No new delete code should be added. Keep using:

```ts
deleteManyAccounts(deleteDialogState.ids);
```

and the existing confirmation dialog copy:

```ts
`确定删除选中的 ${deleteDialogState?.count || 0} 个账号吗？删除后不可恢复。`
```

- [ ] **Step 5: Run the focused frontend test and desktop build**

Run:

```bash
node --test apps/src/app/accounts/account-cleanup.test.ts
pnpm run build:desktop
```

Expected:
- the new node test passes
- `next build` succeeds from `apps/`

- [ ] **Step 6: Commit**

```bash
git add apps/src/app/accounts/account-cleanup.ts apps/src/app/accounts/account-cleanup.test.ts apps/src/app/accounts/page.tsx
git commit -m "feat: add invalid auth account cleanup action"
```
