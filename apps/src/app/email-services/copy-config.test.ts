import test from "node:test";
import assert from "node:assert/strict";

// @ts-ignore -- node:test 直接加载本地 ts 文件
import { buildEmailServiceCopyConfigJson } from "./copy-config.ts";

test("复制配置 JSON 只包含可复用字段并保留完整 config", () => {
  const actual = buildEmailServiceCopyConfigJson({
    id: 12,
    serviceType: "outlook",
    name: "主力 Outlook 池",
    enabled: true,
    priority: 10,
    config: {
      client_id: "client-id-1",
      refresh_token: "refresh-token-1",
      tenant: "common",
    },
    lastUsed: "2026-04-02T10:00:00Z",
    createdAt: "2026-04-01T10:00:00Z",
    updatedAt: "2026-04-02T11:00:00Z",
  });

  assert.equal(
    actual,
    JSON.stringify(
      {
        id: 12,
        name: "主力 Outlook 池",
        serviceType: "outlook",
        enabled: true,
        priority: 10,
        config: {
          client_id: "client-id-1",
          refresh_token: "refresh-token-1",
          tenant: "common",
        },
      },
      null,
      2
    )
  );
});

test("复制配置 JSON 在空 config 时仍输出固定骨架", () => {
  const actual = buildEmailServiceCopyConfigJson({
    id: 7,
    serviceType: "tempmail",
    name: "Tempmail",
    enabled: false,
    priority: 0,
    config: {},
    lastUsed: "",
    createdAt: "",
    updatedAt: "",
  });

  assert.equal(
    actual,
    JSON.stringify(
      {
        id: 7,
        name: "Tempmail",
        serviceType: "tempmail",
        enabled: false,
        priority: 0,
        config: {},
      },
      null,
      2
    )
  );
});
