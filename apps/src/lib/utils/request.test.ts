import test from "node:test";
import assert from "node:assert/strict";

// @ts-ignore node:test 直接加载本地 ts
import { fetchWithRetry } from "./request.ts";

test("fetchWithRetry returns friendly timeout message instead of raw abort text", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = ((_: string, init?: RequestInit) => {
    return new Promise<Response>((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => {
        reject(new DOMException("signal is aborted without reason", "AbortError"));
      });
    });
  }) as typeof fetch;

  try {
    await assert.rejects(
      () =>
        fetchWithRetry("/api/rpc", undefined, {
          timeoutMs: 5,
          retries: 0,
        }),
      (error: unknown) => {
        assert.match(String(error), /请求超时|timeout/i);
        assert.doesNotMatch(String(error), /signal is aborted without reason/i);
        return true;
      }
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("fetchWithRetry stops retrying when outer signal aborts", async () => {
  const originalFetch = globalThis.fetch;
  let callCount = 0;
  globalThis.fetch = ((_: string, init?: RequestInit) => {
    callCount += 1;
    return new Promise<Response>((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => {
        reject(new DOMException("signal is aborted without reason", "AbortError"));
      });
    });
  }) as typeof fetch;

  const controller = new AbortController();
  controller.abort();

  try {
    await assert.rejects(
      () =>
        fetchWithRetry("/api/rpc", undefined, {
          signal: controller.signal,
          retries: 3,
          timeoutMs: 1000,
        }),
      (error: unknown) => {
        assert.match(String(error), /请求已取消|aborted|cancel/i);
        return true;
      }
    );
    assert.equal(callCount, 1);
  } finally {
    globalThis.fetch = originalFetch;
  }
});
