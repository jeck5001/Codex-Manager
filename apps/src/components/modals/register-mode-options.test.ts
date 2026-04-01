import test from "node:test";
import assert from "node:assert/strict";

// @ts-ignore -- node:test 直接加载本地 ts 文件
import {
  canUseOutlookBatchRegisterMode,
  getRegisterChannelLabel,
  sanitizeRegisterModeForChannel,
} from "./register-mode-options.ts";

test("标准模式展示标准注册标签", () => {
  assert.equal(getRegisterChannelLabel("standard"), "标准注册");
});

test("browserbase 模式展示 Browserbase-DDG 标签", () => {
  assert.equal(getRegisterChannelLabel("browserbase_ddg"), "Browserbase-DDG 注册");
});

test("any_auto 模式展示 Any-Auto 标签", () => {
  assert.equal(getRegisterChannelLabel("any_auto"), "Any-Auto 注册");
});

test("browserbase 模式不支持 outlook 批量", () => {
  assert.equal(canUseOutlookBatchRegisterMode("browserbase_ddg"), false);
});

test("any_auto 模式支持 outlook 批量", () => {
  assert.equal(canUseOutlookBatchRegisterMode("any_auto"), true);
});

test("browserbase 通道下 outlook 批量会回退到单个注册", () => {
  assert.equal(sanitizeRegisterModeForChannel("browserbase_ddg", "outlook-batch"), "single");
});

test("标准通道保留当前注册模式", () => {
  assert.equal(sanitizeRegisterModeForChannel("standard", "batch"), "batch");
});
