import assert from "node:assert/strict";
import test from "node:test";

import { buildRequestRouteMeta, resolveRequestDisplayPath } from "./app-helpers.ts";

test("resolveRequestDisplayPath prefers original path over adapted path", () => {
  assert.equal(
    resolveRequestDisplayPath({
      originalPath: "/v1/chat/completions",
      requestPath: "/v1/chat/completions",
      adaptedPath: "/v1/responses",
      responseAdapter: "OpenAIChatCompletionsJson",
    }),
    "/v1/chat/completions",
  );
});

test("buildRequestRouteMeta keeps adapted path as route metadata when display path uses original path", () => {
  assert.deepEqual(
    buildRequestRouteMeta({
      originalPath: "/v1/chat/completions",
      requestPath: "/v1/chat/completions",
      adaptedPath: "/v1/responses",
      responseAdapter: "OpenAIChatCompletionsJson",
      upstreamUrl: "https://api.openai.com/v1",
    }),
    ["转发 /v1/responses", "适配 OpenAIChatCompletionsJson", "上游 默认"],
  );
});
