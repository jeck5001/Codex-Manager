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
