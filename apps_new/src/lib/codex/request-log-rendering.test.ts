import test from "node:test";
import assert from "node:assert/strict";

import {
  buildRequestLogIdentity,
  isAppendOnlyRequestLogResult,
  resolveNextRequestLogRenderedCount,
  resolveRequestLogVirtualWindow,
} from "./request-log-rendering.ts";

test("buildRequestLogIdentity prefers visible path fields and keeps row uniqueness", () => {
  const key = buildRequestLogIdentity(
    {
      id: "log-1",
      traceId: "trc-1",
      createdAt: 123,
      accountId: "acc-1",
      method: "POST",
      adaptedPath: "/v1/responses",
      statusCode: 200,
    },
    3,
  );

  assert.equal(key, "log-1|trc-1|123|acc-1|POST|/v1/responses|200|3");
});

test("isAppendOnlyRequestLogResult only accepts stable prefix growth", () => {
  assert.equal(isAppendOnlyRequestLogResult(["a", "b"], ["a", "b", "c"]), true);
  assert.equal(isAppendOnlyRequestLogResult(["a", "b"], ["a", "x", "c"]), false);
  assert.equal(isAppendOnlyRequestLogResult(["a", "b"], ["a"]), false);
});

test("resolveNextRequestLogRenderedCount preserves expanded count on unchanged refresh", () => {
  const nextRendered = resolveNextRequestLogRenderedCount({
    previousKeys: Array.from({ length: 300 }, (_, index) => `log-${index}`),
    nextKeys: Array.from({ length: 300 }, (_, index) => `log-${index}`),
    currentRenderedCount: 200,
    initialBatch: 120,
    appendBatch: 80,
    wasNearBottom: false,
  });

  assert.equal(nextRendered, 200);
});

test("resolveNextRequestLogRenderedCount appends one more batch when user is already near bottom", () => {
  const nextRendered = resolveNextRequestLogRenderedCount({
    previousKeys: Array.from({ length: 200 }, (_, index) => `log-${index}`),
    nextKeys: Array.from({ length: 300 }, (_, index) => `log-${index}`),
    currentRenderedCount: 200,
    initialBatch: 120,
    appendBatch: 80,
    wasNearBottom: true,
  });

  assert.equal(nextRendered, 280);
});

test("resolveNextRequestLogRenderedCount resets to initial batch for filtered or reordered results", () => {
  const nextRendered = resolveNextRequestLogRenderedCount({
    previousKeys: ["a", "b", "c"],
    nextKeys: ["b", "c"],
    currentRenderedCount: 200,
    initialBatch: 120,
    appendBatch: 80,
    wasNearBottom: true,
  });

  assert.equal(nextRendered, 2);
});

test("resolveRequestLogVirtualWindow limits rendered rows and returns spacer heights", () => {
  assert.deepEqual(
    resolveRequestLogVirtualWindow({
      renderedCount: 200,
      scrollTop: 1080,
      viewportHeight: 648,
      estimatedRowHeight: 54,
      overscanRows: 12,
    }),
    {
      startIndex: 8,
      endIndex: 44,
      topSpacerHeight: 432,
      bottomSpacerHeight: 8424,
    },
  );
});

test("resolveRequestLogVirtualWindow falls back to empty-safe state", () => {
  assert.deepEqual(
    resolveRequestLogVirtualWindow({
      renderedCount: 0,
      scrollTop: 100,
      viewportHeight: 0,
      estimatedRowHeight: 54,
      overscanRows: 12,
    }),
    {
      startIndex: 0,
      endIndex: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
    },
  );
});
