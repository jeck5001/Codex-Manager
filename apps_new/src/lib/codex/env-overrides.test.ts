import assert from "node:assert/strict";
import test from "node:test";

import {
  buildEnvOverrideDescription,
  buildEnvOverrideHint,
  buildEnvOverrideOptionLabel,
  filterEnvOverrideCatalog,
  formatEnvOverrideApplyModeLabel,
  formatEnvOverrideDisplayValue,
  formatEnvOverrideScopeLabel,
  normalizeEnvOverrideCatalogItem,
} from "./env-overrides.ts";

test("normalizeEnvOverrideCatalogItem keeps service label scope apply mode and default value", () => {
  assert.deepEqual(
    normalizeEnvOverrideCatalogItem({
      key: "codexmanager_upstream_total_timeout_ms",
      label: "上游总超时",
      scope: "service",
      applyMode: "runtime",
      defaultValue: 120000,
    }),
    {
      key: "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS",
      name: "上游总超时",
      description: "控制单次上游请求允许持续的最长时间，单位毫秒；超过后会主动结束请求并返回超时错误。",
      scope: "service",
      applyMode: "runtime",
      defaultValue: "120000",
    },
  );
});

test("normalizeEnvOverrideCatalogItem falls back to key label and generated description", () => {
  const item = normalizeEnvOverrideCatalogItem({
    key: "codexmanager_web_root",
    scope: "web",
    applyMode: "restart",
    defaultValue: "",
  });

  assert.deepEqual(item, {
    key: "CODEXMANAGER_WEB_ROOT",
    name: "CODEXMANAGER_WEB_ROOT",
    description: "控制 Web 静态资源目录；适合自定义前端资源位置或部署目录。",
    scope: "web",
    applyMode: "restart",
    defaultValue: "",
  });
});

test("filterEnvOverrideCatalog supports label description and key search", () => {
  const catalog = [
    normalizeEnvOverrideCatalogItem({
      key: "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS",
      label: "上游总超时",
      scope: "service",
      applyMode: "runtime",
      defaultValue: "120000",
    }),
    normalizeEnvOverrideCatalogItem({
      key: "CODEXMANAGER_PROMPT_CACHE_TTL_SECS",
      label: "提示缓存 TTL",
      scope: "service",
      applyMode: "runtime",
      defaultValue: "3600",
    }),
  ].filter(Boolean);

  assert.deepEqual(
    filterEnvOverrideCatalog(catalog, "缓存").map((item) => item.key),
    ["CODEXMANAGER_PROMPT_CACHE_TTL_SECS"],
  );
  assert.deepEqual(
    filterEnvOverrideCatalog(catalog, "timeout").map((item) => item.key),
    ["CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS"],
  );
});

test("display helpers keep old env override semantics", () => {
  const item = normalizeEnvOverrideCatalogItem({
    key: "CODEXMANAGER_GITHUB_TOKEN",
    label: "GitHub 访问令牌",
    scope: "desktop",
    applyMode: "restart",
    defaultValue: "",
  });

  assert.equal(buildEnvOverrideOptionLabel(item), "GitHub 访问令牌");
  assert.match(buildEnvOverrideDescription(item), /访问令牌|限流|失败/);
  assert.equal(formatEnvOverrideDisplayValue(""), "空");
  assert.equal(formatEnvOverrideDisplayValue("  600 "), "600");
  assert.equal(formatEnvOverrideScopeLabel(item?.scope), "桌面端");
  assert.equal(formatEnvOverrideApplyModeLabel(item?.applyMode), "重启生效");
  assert.equal(
    buildEnvOverrideHint(item, "", "当前编辑内容仍是本地草稿，尚未写入服务端。"),
    "当前编辑内容仍是本地草稿，尚未写入服务端。；默认值：空；当前值：空；作用域：桌面端；重启生效",
  );
});
