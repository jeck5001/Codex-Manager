import test from "node:test";
import assert from "node:assert/strict";

import {
  describeRouteAccountScope,
  isRouteAccountSelected,
  mergeRouteAccountIds,
  normalizeRouteAccountIds,
  removeRouteAccountIds,
} from "./route-account-state.ts";

test("normalizeRouteAccountIds trims, dedupes, and drops empty ids", () => {
  assert.deepEqual(
    normalizeRouteAccountIds([" acc-a ", "", "acc-a", "acc-b"]),
    ["acc-a", "acc-b"],
  );
});

test("describeRouteAccountScope reports unrestricted routing for empty whitelist", () => {
  assert.equal(
    describeRouteAccountScope([], ["acc-a", "acc-b"]),
    "全部可用账号参与路由",
  );
});

test("describeRouteAccountScope counts only known route accounts", () => {
  assert.equal(
    describeRouteAccountScope(["acc-a", "acc-missing", "acc-b"], ["acc-a", "acc-b"]),
    "已限制为 2 个账号参与路由",
  );
});

test("isRouteAccountSelected matches exact account ids", () => {
  assert.equal(isRouteAccountSelected(["acc-a", "acc-b"], "acc-b"), true);
  assert.equal(isRouteAccountSelected(["acc-a", "acc-b"], "acc-c"), false);
});

test("describeRouteAccountScope falls back to unrestricted when all whitelisted accounts are unknown", () => {
  assert.equal(
    describeRouteAccountScope(["acc-missing"], ["acc-a", "acc-b"]),
    "全部可用账号参与路由",
  );
});

test("mergeRouteAccountIds preserves existing order and appends new unique ids", () => {
  assert.deepEqual(
    mergeRouteAccountIds([" acc-a ", "acc-b"], ["", "acc-b", "acc-c", "acc-a", "acc-d"]),
    ["acc-a", "acc-b", "acc-c", "acc-d"],
  );
});

test("removeRouteAccountIds removes normalized ids and preserves remaining order", () => {
  assert.deepEqual(
    removeRouteAccountIds(["acc-a", "acc-b", "acc-c"], [" acc-b ", "", "acc-x"]),
    ["acc-a", "acc-c"],
  );
});
