import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const appsRoot = path.resolve(__dirname, "..");
const mainJs = fs.readFileSync(path.join(appsRoot, "src", "main.js"), "utf8");

test("main refresh flow uses centralized refresh helpers", () => {
  assert.ok(mainJs.includes("runRefreshTasks"), "main.js should use runRefreshTasks");
  assert.ok(mainJs.includes("ensureAutoRefreshTimer"), "main.js should use ensureAutoRefreshTimer");
  assert.ok(mainJs.includes("stopAutoRefreshTimer"), "main.js should use stopAutoRefreshTimer");
});
