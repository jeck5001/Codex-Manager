import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildManualImportSummary,
  getRegisterSubmitLabel,
} from './register-auto-import.ts';

test('默认开启自动入池时按钮文案为开始注册并导入', () => {
  assert.equal(getRegisterSubmitLabel(true), '开始注册并导入');
});

test('关闭自动入池时按钮文案为开始注册', () => {
  assert.equal(getRegisterSubmitLabel(false), '开始注册');
});

test('单个注册关闭自动入池时给出手动入池提示', () => {
  assert.equal(
    buildManualImportSummary('注册', 1),
    '注册已完成，未自动入池，可在注册中心手动加入号池',
  );
});

test('批量注册关闭自动入池时给出数量提示', () => {
  assert.equal(
    buildManualImportSummary('批量注册', 3),
    '批量注册已完成，共 3 个账号未自动入池，可在注册中心手动加入号池',
  );
});
