import test from "node:test";
import assert from "node:assert/strict";

import {
  ensureAutoRefreshTimer,
  runRefreshTasks,
  stopAutoRefreshTimer,
} from "./refresh.js";

test("runRefreshTasks continues when one task fails", async () => {
  const errors = [];
  const results = await runRefreshTasks(
    [
      {
        name: "accounts",
        run: async () => "ok",
      },
      {
        name: "usage",
        run: async () => {
          throw new Error("usage failed");
        },
      },
      {
        name: "models",
        run: async () => "ok",
      },
    ],
    (name, err) => errors.push([name, err && err.message]),
  );

  assert.equal(results.length, 3);
  assert.equal(results[0].status, "fulfilled");
  assert.equal(results[1].status, "rejected");
  assert.equal(results[2].status, "fulfilled");
  assert.deepEqual(errors, [["usage", "usage failed"]]);
});

test("ensureAutoRefreshTimer creates one timer only", async () => {
  const state = { autoRefreshTimer: null };
  let tickCount = 0;

  const started = ensureAutoRefreshTimer(state, async () => {
    tickCount += 1;
  }, 10);
  assert.equal(started, true);
  assert.ok(state.autoRefreshTimer);

  const startedAgain = ensureAutoRefreshTimer(state, async () => {
    tickCount += 1;
  }, 10);
  assert.equal(startedAgain, false);

  await new Promise((resolve) => setTimeout(resolve, 35));
  const stopped = stopAutoRefreshTimer(state);
  assert.equal(stopped, true);
  assert.equal(state.autoRefreshTimer, null);
  assert.ok(tickCount >= 1);
});
