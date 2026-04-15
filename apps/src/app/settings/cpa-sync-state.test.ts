import test from "node:test";
import assert from "node:assert/strict";

import {
  formatCpaSyncStatusLabel,
  parseCpaSyncScheduleInterval,
  shouldPollCpaSyncStatus,
} from "./cpa-sync-state.ts";

test("parseCpaSyncScheduleInterval requires a positive integer minute value", () => {
  assert.equal(parseCpaSyncScheduleInterval("15"), 15);
  assert.equal(parseCpaSyncScheduleInterval("0"), null);
  assert.equal(parseCpaSyncScheduleInterval("abc"), null);
});

test("formatCpaSyncStatusLabel maps runtime states to readable labels", () => {
  assert.equal(formatCpaSyncStatusLabel({ status: "running" } as never), "同步中");
  assert.equal(formatCpaSyncStatusLabel({ status: "misconfigured" } as never), "待补配置");
  assert.equal(formatCpaSyncStatusLabel({ status: "disabled" } as never), "已关闭");
});

test("shouldPollCpaSyncStatus only polls active scheduler states", () => {
  assert.equal(
    shouldPollCpaSyncStatus({ scheduleEnabled: true, isRunning: false } as never),
    true
  );
  assert.equal(
    shouldPollCpaSyncStatus({ scheduleEnabled: false, isRunning: true } as never),
    true
  );
  assert.equal(
    shouldPollCpaSyncStatus({ scheduleEnabled: false, isRunning: false } as never),
    false
  );
});
