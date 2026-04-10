import test from "node:test";
import assert from "node:assert/strict";

import {
  buildHotmailLocalHelperUrl,
  hotmailLocalHelperClient,
} from "./hotmail-local-helper.ts";

test("buildHotmailLocalHelperUrl uses localhost helper endpoint", () => {
  assert.equal(buildHotmailLocalHelperUrl("/health"), "http://127.0.0.1:16788/health");
});

test("hotmailLocalHelperClient.health requests helper health endpoint", async () => {
  const originalFetch = globalThis.fetch;
  const calls: Array<{ input: string; init?: RequestInit }> = [];
  globalThis.fetch = (async (input: URL | RequestInfo, init?: RequestInit) => {
    calls.push({ input: String(input), init });
    return new Response(
      JSON.stringify({
        ok: true,
        service: "hotmail-local-helper",
        version: "1",
        playwright_ready: true,
      }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" },
      },
    );
  }) as typeof fetch;

  try {
    const result = await hotmailLocalHelperClient.health();

    assert.equal(calls[0]?.input, "http://127.0.0.1:16788/health");
    assert.equal(result.ok, true);
    assert.equal(result.playwrightReady, true);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("hotmailLocalHelperClient.startTask posts task payload to helper endpoint", async () => {
  const originalFetch = globalThis.fetch;
  const calls: Array<{ input: string; init?: RequestInit }> = [];
  globalThis.fetch = (async (input: URL | RequestInfo, init?: RequestInit) => {
    calls.push({ input: String(input), init });
    return new Response(
      JSON.stringify({
        ok: true,
        task_id: "task-1",
        message: "task accepted",
      }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" },
      },
    );
  }) as typeof fetch;

  try {
    const result = await hotmailLocalHelperClient.startTask({
      batchId: "batch-1",
      taskId: "task-1",
      profile: { first_name: "Alice" },
      targetDomains: ["hotmail.com"],
      proxy: "",
      verificationMailbox: { email: "verify@example.com", service_id: "svc-1" },
      backendCallbackBase: "http://192.168.5.35:9000/api/hotmail",
      backendCallbackToken: "",
    });

    assert.equal(calls[0]?.input, "http://127.0.0.1:16788/hotmail/start-task");
    assert.equal(result.taskId, "task-1");
  } finally {
    globalThis.fetch = originalFetch;
  }
});
