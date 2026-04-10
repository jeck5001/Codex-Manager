import test from "node:test";
import assert from "node:assert/strict";

import {
  canShowTempMailAutoCreateToggle,
  canShowTempMailDomainConfigPicker,
  deriveTempMailAutoCreateSubmitFlag,
  getDefaultTempMailAutoCreateState,
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

test("temp_mail defaults auto-create to enabled", () => {
  assert.equal(getDefaultTempMailAutoCreateState("temp_mail"), true);
  assert.equal(getDefaultTempMailAutoCreateState("outlook"), false);
});

test("domain config picker is only visible for auto-created temp_mail", () => {
  assert.equal(canShowTempMailDomainConfigPicker("temp_mail", true), true);
  assert.equal(canShowTempMailDomainConfigPicker("temp_mail", false), false);
  assert.equal(canShowTempMailDomainConfigPicker("outlook", true), false);
});

test("picker disable and submit bypass align when auto-create temp_mail is active", () => {
  assert.equal(shouldDisableTempMailServicePicker("temp_mail", true), true);
  assert.equal(shouldDisableTempMailServicePicker("temp_mail", false), false);
  assert.equal(shouldDisableTempMailServicePicker("outlook", true), false);
  assert.equal(shouldBypassTempMailServicePickerOnSubmit("temp_mail", true), true);
  assert.equal(shouldBypassTempMailServicePickerOnSubmit("outlook", true), false);
});
