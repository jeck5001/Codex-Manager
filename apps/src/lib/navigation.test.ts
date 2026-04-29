import test from "node:test";
import assert from "node:assert/strict";

import { normalizeVisibleMenuItems } from "./navigation.ts";

test("legacy full menu upgrades to include hotmail and models", () => {
  const actual = normalizeVisibleMenuItems([
    "dashboard",
    "accounts",
    "register",
    "payment",
    "emailServices",
    "apiKeys",
    "logs",
    "audit",
    "costs",
    "analytics",
    "settings",
  ]);

  assert.deepEqual(actual, [
    "dashboard",
    "accounts",
    "register",
    "payment",
    "emailServices",
    "hotmail",
    "apiKeys",
    "models",
    "logs",
    "audit",
    "costs",
    "analytics",
    "settings",
  ]);
});

test("hotmail-era full menu upgrades to include models", () => {
  const actual = normalizeVisibleMenuItems([
    "dashboard",
    "accounts",
    "register",
    "payment",
    "emailServices",
    "hotmail",
    "apiKeys",
    "logs",
    "audit",
    "costs",
    "analytics",
    "settings",
  ]);

  assert.deepEqual(actual, [
    "dashboard",
    "accounts",
    "register",
    "payment",
    "emailServices",
    "hotmail",
    "apiKeys",
    "models",
    "logs",
    "audit",
    "costs",
    "analytics",
    "settings",
  ]);
});
