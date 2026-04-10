import test from "node:test";
import assert from "node:assert/strict";

import {
  addDomainConfig,
  duplicateDomainConfig,
  removeDomainConfig,
  selectInitialDomainConfigId,
} from "./temp-mail-domain-config-state.ts";

const baseConfig = {
  id: "cfg-1",
  name: "主域名",
  zoneId: "zone-1",
  domainBase: "a.example.com",
  subdomainMode: "random",
  subdomainLength: "6",
  subdomainPrefix: "tm",
  syncCloudflareEnabled: true,
  requireCloudflareSync: true,
};

test("selectInitialDomainConfigId picks first config when selection is missing", () => {
  const configs = [baseConfig, { ...baseConfig, id: "cfg-2" }];
  assert.equal(selectInitialDomainConfigId(configs, null), "cfg-1");
});

test("selectInitialDomainConfigId keeps an existing valid selection", () => {
  const configs = [baseConfig, { ...baseConfig, id: "cfg-2" }];
  assert.equal(selectInitialDomainConfigId(configs, "cfg-2"), "cfg-2");
});

test("selectInitialDomainConfigId falls back to the first config when selection is stale", () => {
  const configs = [baseConfig, { ...baseConfig, id: "cfg-2" }];
  assert.equal(selectInitialDomainConfigId(configs, "missing"), "cfg-1");
});

test("addDomainConfig appends an empty config and selects it", () => {
  const result = addDomainConfig([baseConfig], () => "cfg-new");
  assert.equal(result.selectedId, "cfg-new");
  assert.equal(result.domainConfigs.length, 2);
  assert.equal(result.domainConfigs[1]?.name, "");
});

test("duplicateDomainConfig clones editable fields and appends suffix", () => {
  const result = duplicateDomainConfig([baseConfig], "cfg-1", () => "cfg-copy");
  assert.equal(result.selectedId, "cfg-copy");
  assert.deepEqual(result.domainConfigs[1], {
    ...baseConfig,
    id: "cfg-copy",
    name: "主域名-副本",
  });
});

test("duplicateDomainConfig preserves whitespace in the name when duplicating", () => {
  const configs = [{ ...baseConfig, name: "  主域名  " }];
  const result = duplicateDomainConfig(configs, "cfg-1", () => "cfg-copy");
  assert.equal(result.selectedId, "cfg-copy");
  assert.equal(result.domainConfigs[1]?.name, "  主域名  -副本");
});

test("duplicateDomainConfig keeps empty name empty when duplicating", () => {
  const configs = [{ ...baseConfig, name: "" }];
  const result = duplicateDomainConfig(configs, "cfg-1", () => "cfg-copy");
  assert.equal(result.selectedId, "cfg-copy");
  assert.equal(result.domainConfigs[1]?.name, "");
});

test("duplicateDomainConfig preserves selection when source is missing", () => {
  const configs = [baseConfig, { ...baseConfig, id: "cfg-2", name: "备用域名" }];
  const result = duplicateDomainConfig(configs, "missing", () => "cfg-copy", "cfg-2");
  assert.equal(result.selectedId, "cfg-2");
  assert.deepEqual(result.domainConfigs, configs);
});

test("removeDomainConfig falls forward when deleting active item", () => {
  const configs = [
    baseConfig,
    { ...baseConfig, id: "cfg-2", name: "备用域名" },
    { ...baseConfig, id: "cfg-3", name: "第三域名" },
  ];
  const result = removeDomainConfig(configs, "cfg-2", "cfg-2");
  assert.equal(result.selectedId, "cfg-3");
  assert.deepEqual(result.domainConfigs.map((item) => item.id), ["cfg-1", "cfg-3"]);
});

test("removeDomainConfig falls back to previous when deleting the last entry", () => {
  const configs = [baseConfig, { ...baseConfig, id: "cfg-2", name: "备用域名" }];
  const result = removeDomainConfig(configs, "cfg-2", "cfg-2");
  assert.equal(result.selectedId, "cfg-1");
});

test("removeDomainConfig selects null when no configs remain", () => {
  const result = removeDomainConfig([baseConfig], "cfg-1", "cfg-1");
  assert.equal(result.selectedId, null);
  assert.deepEqual(result.domainConfigs, []);
});

test("removeDomainConfig keeps current selection when removing another entry", () => {
  const configs = [
    baseConfig,
    { ...baseConfig, id: "cfg-2", name: "备用域名" },
    { ...baseConfig, id: "cfg-3", name: "第三域名" },
  ];
  const result = removeDomainConfig(configs, "cfg-3", "cfg-2");
  assert.equal(result.selectedId, "cfg-2");
});
