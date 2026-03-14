import assert from "node:assert/strict";
import test from "node:test";

import { formatResetLabel } from "./format.ts";

test("formatResetLabel returns placeholder for empty timestamp", () => {
  assert.equal(formatResetLabel(null), "重置：--");
});

test("formatResetLabel formats same-day reset as hh:mm", () => {
  const now = new Date();
  const sameDay = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate(),
    13,
    45,
    0,
    0,
  );
  assert.equal(formatResetLabel(Math.floor(sameDay.getTime() / 1000)), "重置：13:45");
});

test("formatResetLabel formats cross-day reset with month and day", () => {
  const now = new Date();
  const nextDay = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate() + 1,
    9,
    30,
    0,
    0,
  );
  assert.equal(
    formatResetLabel(Math.floor(nextDay.getTime() / 1000)),
    `重置：${nextDay.getMonth() + 1}月${nextDay.getDate()}日 09:30`,
  );
});
