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
  assert.equal(isInvalidAuthCleanupStatusReason("Refresh 刷新失败"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("登录态已失效"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("授权失效"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("Refresh 连续失效"), true);
  assert.equal(isInvalidAuthCleanupStatusReason("检测到账号已停用"), false);
  assert.equal(isInvalidAuthCleanupStatusReason(""), false);
  assert.equal(isInvalidAuthCleanupStatusReason(null), false);
});

test("collectInvalidAuthCleanupAccountIds only returns filtered invalid-auth accounts", () => {
  const ids = collectInvalidAuthCleanupAccountIds([
    { id: "acc-reused", lastStatusReason: "Refresh 已复用" },
    { id: "acc-invalidated", lastStatusReason: "登录态已失效" },
    { id: "acc-401", lastStatusReason: "授权失效" },
    { id: "acc-ok", lastStatusReason: "用量恢复正常" },
    { id: "acc-empty", lastStatusReason: null },
  ]);

  assert.deepEqual(ids, ["acc-reused", "acc-invalidated", "acc-401"]);
});
