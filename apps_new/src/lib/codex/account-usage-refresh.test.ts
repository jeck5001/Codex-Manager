import assert from "node:assert/strict";
import test from "node:test";

import {
  buildAccountUsageRefreshText,
  EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS,
  normalizeAccountUsageRefreshProgress,
} from "./account-usage-refresh.ts";

test("normalizeAccountUsageRefreshProgress clamps counters and derives remaining", () => {
  assert.deepEqual(
    normalizeAccountUsageRefreshProgress({
      active: true,
      total: 5,
      completed: 9,
      remaining: -1,
      lastAccountLabel: " demo ",
    }),
    {
      active: true,
      total: 5,
      completed: 5,
      remaining: 0,
      lastAccountLabel: "demo",
    },
  );
});

test("normalizeAccountUsageRefreshProgress returns empty state for invalid totals", () => {
  assert.deepEqual(normalizeAccountUsageRefreshProgress({ active: true, total: 0, completed: 1 }), {
    ...EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS,
  });
});

test("buildAccountUsageRefreshText keeps old progress wording", () => {
  assert.equal(
    buildAccountUsageRefreshText({
      active: true,
      total: 5,
      completed: 2,
      remaining: 3,
      lastAccountLabel: "AmyMoore9344@outlook.com",
    }),
    "刷新进度 2/5，剩余 3 项 · 最近完成：AmyMoore9344@outlook.com",
  );
  assert.equal(buildAccountUsageRefreshText(EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS), "");
});
