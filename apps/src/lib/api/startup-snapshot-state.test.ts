import test from "node:test";
import assert from "node:assert/strict";

import {
  hasStartupSnapshotSignal,
  pickCurrentAccountId,
} from "./startup-snapshot-state.ts";

test("hasStartupSnapshotSignal no longer depends on raw usageSnapshots/requestLogs arrays", () => {
  assert.equal(
    hasStartupSnapshotSignal({
      accounts: [],
      usageAggregateSummary: {
        primaryBucketCount: 1,
        primaryKnownCount: 1,
        secondaryBucketCount: 0,
        secondaryKnownCount: 0,
      },
      requestLogTodaySummary: {
        todayTokens: 0,
      },
      recentRequestLogCount: 0,
    }),
    true,
  );
});

test("pickCurrentAccountId prefers latest request account id before scanning account list", () => {
  const accountId = pickCurrentAccountId(
    [
      { id: "acc-a", availabilityLevel: "warn" },
      { id: "acc-b", availabilityLevel: "ok" },
    ],
    "acc-b",
    undefined,
  );

  assert.equal(accountId, "acc-b");
});

test("pickCurrentAccountId scopes selection to route whitelist before using latest request", () => {
  const accountId = pickCurrentAccountId(
    [
      { id: "acc-a", availabilityLevel: "ok" },
      { id: "acc-b", availabilityLevel: "ok" },
      { id: "acc-c", availabilityLevel: "ok" },
    ],
    "acc-a",
    ["acc-b", "acc-c"],
  );

  assert.equal(accountId, "acc-b");
});

test("pickCurrentAccountId falls back to all accounts when route whitelist is empty", () => {
  const accountId = pickCurrentAccountId(
    [
      { id: "acc-a", availabilityLevel: "ok" },
      { id: "acc-b", availabilityLevel: "ok" },
    ],
    "acc-b",
    [],
  );

  assert.equal(accountId, "acc-b");
});
