import test from "node:test";
import assert from "node:assert/strict";

import {
  formatEnvOverridesText,
  normalizeEnvOverrideCatalog,
  normalizeEnvOverrides,
  normalizeStringList,
  parseEnvOverridesText,
} from "../env-overrides.js";

test("parseEnvOverridesText supports comments, clear lines and key normalization", () => {
  const parsed = parseEnvOverridesText(`
# comment
codexmanager_upstream_total_timeout_ms=120000
CODEXMANAGER_TRACE_QUEUE_CAPACITY=64
CODEXMANAGER_TRACE_QUEUE_CAPACITY=
`);

  assert.equal(parsed.ok, true);
  assert.deepEqual(parsed.overrides, {
    CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS: "120000",
  });
});

test("parseEnvOverridesText rejects invalid lines", () => {
  const parsed = parseEnvOverridesText("NOT_VALID");
  assert.equal(parsed.ok, false);
  assert.match(parsed.error, /KEY=VALUE/);
});

test("formatEnvOverridesText sorts normalized keys", () => {
  const text = formatEnvOverridesText({
    CODEXMANAGER_UPSTREAM_COOKIE: "cookie=1",
    codexmanager_upstream_total_timeout_ms: "321000",
  });
  assert.equal(
    text,
    "CODEXMANAGER_UPSTREAM_COOKIE=cookie=1\nCODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS=321000",
  );
});

test("normalize helpers keep deterministic arrays", () => {
  assert.deepEqual(normalizeStringList([" b ", "a", "", "a"]), ["a", "b"]);
  assert.deepEqual(
    normalizeEnvOverrides({
      foo: "bar",
      CODEXMANAGER_UPSTREAM_BASE_URL: "https://chatgpt.com",
      CODEXMANAGER_UPSTREAM_COOKIE: "  ",
    }),
    {
      CODEXMANAGER_UPSTREAM_BASE_URL: "https://chatgpt.com",
    },
  );
  assert.deepEqual(
    normalizeEnvOverrideCatalog([
      { key: "CODEXMANAGER_WEB_ROOT", scope: "web", applyMode: "restart" },
      { key: "codexmanager_upstream_total_timeout_ms", scope: "service", applyMode: "runtime" },
    ]),
    [
      { key: "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS", scope: "service", applyMode: "runtime" },
      { key: "CODEXMANAGER_WEB_ROOT", scope: "web", applyMode: "restart" },
    ],
  );
});
