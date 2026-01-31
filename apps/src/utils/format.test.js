import test from "node:test";
import assert from "node:assert/strict";

import { calcAvailability } from "./format.js";

test("calcAvailability treats missing primary fields as unavailable", () => {
  const usage = {
    usedPercent: null,
    windowMinutes: 300,
    secondaryUsedPercent: 10,
    secondaryWindowMinutes: 10080,
  };
  const result = calcAvailability(usage);
  assert.equal(result.level, "bad");
});

test("calcAvailability treats missing secondary fields as unavailable", () => {
  const usage = {
    usedPercent: 10,
    windowMinutes: 300,
    secondaryUsedPercent: null,
    secondaryWindowMinutes: 10080,
  };
  const result = calcAvailability(usage);
  assert.equal(result.level, "bad");
});

test("calcAvailability keeps ok when both windows present and under limit", () => {
  const usage = {
    usedPercent: 10,
    windowMinutes: 300,
    secondaryUsedPercent: 5,
    secondaryWindowMinutes: 10080,
  };
  const result = calcAvailability(usage);
  assert.equal(result.level, "ok");
});
