import test from "node:test";
import assert from "node:assert/strict";

import {
  getHotmailBatchProgress,
  mergeHotmailBatchArtifacts,
  shouldPollHotmailBatch,
} from "./hotmail-batch-state.ts";

test("getHotmailBatchProgress returns 0% when total is missing", () => {
  assert.equal(getHotmailBatchProgress({ total: 0, completed: 5 }), "0%");
});

test("getHotmailBatchProgress rounds down using completed over total", () => {
  assert.equal(getHotmailBatchProgress({ total: 8, completed: 3 }), "37%");
});

test("shouldPollHotmailBatch only polls unfinished active batches", () => {
  assert.equal(shouldPollHotmailBatch({ finished: false, cancelled: false }), true);
  assert.equal(shouldPollHotmailBatch({ finished: true, cancelled: false }), false);
  assert.equal(shouldPollHotmailBatch({ finished: false, cancelled: true }), false);
});

test("mergeHotmailBatchArtifacts keeps newest non-empty artifacts", () => {
  const previous = [{ filename: "batch-a.txt", path: "/tmp/batch-a.txt", size: 12 }];
  const next = [{ filename: "batch-b.txt", path: "/tmp/batch-b.txt", size: 20 }];
  assert.deepEqual(mergeHotmailBatchArtifacts(previous, next), next);
  assert.deepEqual(mergeHotmailBatchArtifacts(previous, []), previous);
});
