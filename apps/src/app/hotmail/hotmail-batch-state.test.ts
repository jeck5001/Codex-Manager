import test from "node:test";
import assert from "node:assert/strict";

import {
  buildHotmailHandoffAccessUrl,
  classifyHotmailLogLine,
  formatHotmailBatchStatus,
  getHotmailBatchProgress,
  hasHotmailPendingHandoff,
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

test("classifyHotmailLogLine marks human verification logs as challenge", () => {
  assert.equal(
    classifyHotmailLogLine("微软要求人工验证（Press and hold the button）"),
    "challenge",
  );
  assert.equal(
    classifyHotmailLogLine("Hotmail signup failed: unsupported_challenge | title=Let's prove you're human"),
    "challenge",
  );
  assert.equal(classifyHotmailLogLine("phone required"), "default");
});

test("formatHotmailBatchStatus marks action required batches", () => {
  assert.deepEqual(
    formatHotmailBatchStatus({
      finished: false,
      cancelled: false,
      status: "action_required",
      actionRequiredReason: "unsupported_challenge",
    }),
    {
      label: "等待人工处理",
      className: "border-amber-500/20 bg-amber-500/10 text-amber-600 dark:text-amber-300",
    },
  );
});

test("hasHotmailPendingHandoff only returns true for action-required batches with handoff ids", () => {
  assert.equal(
    hasHotmailPendingHandoff({
      status: "action_required",
      actionRequiredReason: "unsupported_challenge",
      handoffId: "handoff-1",
    }),
    true,
  );
  assert.equal(
    hasHotmailPendingHandoff({
      status: "action_required",
      actionRequiredReason: "unsupported_challenge",
      handoffId: "",
    }),
    false,
  );
});

test("buildHotmailHandoffAccessUrl falls back to current hostname with default noVNC port", () => {
  assert.equal(
    buildHotmailHandoffAccessUrl(
      {
        status: "action_required",
        actionRequiredReason: "unsupported_challenge",
        handoffId: "handoff-1",
        handoffUrl: "",
      },
      "http://192.168.5.35:48761/hotmail",
    ),
    "http://192.168.5.35:7900/vnc.html?autoconnect=1&resize=scale",
  );
});
