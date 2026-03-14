import assert from "node:assert/strict";
import test from "node:test";

import { resolveBusinessErrorMessage, unwrapRpcError, unwrapRpcResult } from "./rpc-payload.ts";

test("unwrapRpcResult returns nested result from tauri rpc wrapper", () => {
  const snapshot = {
    accounts: [{ id: "acc-1", label: "demo@example.com" }],
    apiKeys: [{ id: "key-1", name: "codex" }],
    requestLogs: [{ id: "log-1", path: "/v1/responses", status: 200 }],
  };

  assert.deepEqual(
    unwrapRpcResult({
      jsonrpc: "2.0",
      id: 7,
      result: snapshot,
    }),
    snapshot,
  );
});

test("unwrapRpcResult throws rpc error messages from wrapper", () => {
  assert.throws(
    () =>
      unwrapRpcResult({
        jsonrpc: "2.0",
        id: 7,
        error: {
          code: -32000,
          message: "service unavailable",
        },
      }),
    /service unavailable/,
  );
});

test("unwrapRpcResult throws business error messages from nested result", () => {
  assert.throws(
    () =>
      unwrapRpcResult({
        jsonrpc: "2.0",
        id: 7,
        result: {
          ok: false,
          error: "invalid token",
        },
      }),
    /invalid token/,
  );
});

test("resolveBusinessErrorMessage only treats explicit failures as business errors", () => {
  assert.equal(resolveBusinessErrorMessage({ ok: true, error: "" }), "");
  assert.equal(resolveBusinessErrorMessage({ ok: false }), "操作失败");
  assert.equal(resolveBusinessErrorMessage({ error: "bad request" }), "bad request");
});

test("unwrapRpcError supports string and object-shaped rpc errors", () => {
  assert.equal(unwrapRpcError({ error: "broken pipe" }), "broken pipe");
  assert.equal(
    unwrapRpcError({
      error: {
        code: -32001,
        message: "timeout",
      },
    }),
    "timeout",
  );
});
