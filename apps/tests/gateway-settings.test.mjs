import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";
import ts from "../node_modules/typescript/lib/typescript.js";

const appsRoot = path.resolve(import.meta.dirname, "..");
const sourcePath = path.join(
  appsRoot,
  "src",
  "lib",
  "api",
  "gateway-settings.ts"
);

async function loadGatewaySettingsModule() {
  const source = await fs.readFile(sourcePath, "utf8");
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: sourcePath,
  });

  const tempDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "codexmanager-gateway-settings-")
  );
  const tempFile = path.join(tempDir, "gateway-settings.mjs");
  await fs.writeFile(tempFile, compiled.outputText, "utf8");
  return import(pathToFileURL(tempFile).href);
}

const gatewaySettings = await loadGatewaySettingsModule();

test("readGatewayTransportSettings 读取真实传输配置并补齐默认值", () => {
  const settings = gatewaySettings.readGatewayTransportSettings({
    sseKeepaliveIntervalMs: 5000,
    upstreamStreamTimeoutMs: "120000",
  });

  assert.equal(settings.sseKeepaliveIntervalMs, 5000);
  assert.equal(settings.upstreamStreamTimeoutMs, 120000);
  assert.deepEqual(settings.envKeys, [
    "CODEXMANAGER_SSE_KEEPALIVE_INTERVAL_MS",
    "CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS",
  ]);
  assert.equal(settings.requiresRestart, false);
});

test("readGatewayUpstreamProxySettings 与 readGatewayRouteStrategySettings 对齐对象返回", () => {
  const proxy = gatewaySettings.readGatewayUpstreamProxySettings({
    proxyUrl: "http://127.0.0.1:7890",
  });
  assert.equal(proxy.proxyUrl, "http://127.0.0.1:7890");
  assert.equal(proxy.envKey, "CODEXMANAGER_UPSTREAM_PROXY_URL");

  const route = gatewaySettings.readGatewayRouteStrategySettings({
    strategy: "balanced",
    manualPreferredAccountId: "acc-1",
  });
  assert.equal(route.strategy, "balanced");
  assert.deepEqual(route.options, ["ordered", "balanced"]);
  assert.equal(route.manualPreferredAccountId, "acc-1");
});
