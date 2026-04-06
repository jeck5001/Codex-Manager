import test from "node:test";
import assert from "node:assert/strict";

import {
  canShowTempMailAutoCreateToggle,
  deriveTempMailAutoCreateSubmitFlag,
  shouldBypassTempMailServicePickerOnSubmit,
  shouldDisableTempMailServicePicker,
} from "./register-temp-mail-auto-create.ts";

test("toggle visibility is limited to temp_mail", () => {
  assert.equal(canShowTempMailAutoCreateToggle("temp_mail"), true);
  assert.equal(canShowTempMailAutoCreateToggle("outlook"), false);
  assert.equal(canShowTempMailAutoCreateToggle("custom_domain"), false);
});

test("derived submit flag ignores stale toggle state for non-temp services", () => {
  assert.equal(deriveTempMailAutoCreateSubmitFlag("temp_mail", true), true);
  assert.equal(deriveTempMailAutoCreateSubmitFlag("temp_mail", false), false);
  assert.equal(deriveTempMailAutoCreateSubmitFlag("outlook", true), false);
  assert.equal(deriveTempMailAutoCreateSubmitFlag("custom_domain", true), false);
});

test("picker disable and submit bypass align when auto-create temp_mail is active", () => {
  assert.equal(shouldDisableTempMailServicePicker("temp_mail", true), true);
  assert.equal(shouldDisableTempMailServicePicker("temp_mail", false), false);
  assert.equal(shouldDisableTempMailServicePicker("outlook", true), false);
  assert.equal(shouldBypassTempMailServicePickerOnSubmit("temp_mail", true), true);
  assert.equal(shouldBypassTempMailServicePickerOnSubmit("outlook", true), false);
});
