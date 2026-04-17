import test from "node:test";
import assert from "node:assert/strict";

import { normalizeAccount } from "./normalize.ts";

test("normalizeAccount does not mark unavailable accounts as low quota", () => {
  const account = normalizeAccount(
    {
      id: "acc-unavailable-refresh-reused",
      label: "acc-unavailable-refresh-reused",
      status: "unavailable",
      healthScore: 100,
    },
    {
      accountId: "acc-unavailable-refresh-reused",
      availabilityStatus: "bad",
      usedPercent: 88,
      windowMinutes: 300,
      resetsAt: null,
      secondaryUsedPercent: 92,
      secondaryWindowMinutes: 10080,
      secondaryResetsAt: null,
      creditsJson: null,
      capturedAt: 1_700_000_000,
    }
  );

  assert.ok(account);
  assert.equal(account.isLowQuota, false);
  assert.equal(account.isAvailable, false);
});
