import test from "node:test";
import assert from "node:assert/strict";

import { buildAccountsFilterUrl } from "./accounts-filter-query.ts";

test("buildAccountsFilterUrl omits default filters", () => {
  assert.equal(
    buildAccountsFilterUrl({
      search: "",
      groupFilter: "all",
      statusFilter: "all",
      governanceFilter: "all",
      statusReasonFilter: "all",
      cooldownReasonFilter: "all",
      tagFilter: "all",
    }),
    "/accounts",
  );
});

test("buildAccountsFilterUrl preserves active filters in query string", () => {
  assert.equal(
    buildAccountsFilterUrl({
      search: "acc-a ",
      groupFilter: "默认",
      statusFilter: "available",
      governanceFilter: "all",
      statusReasonFilter: "授权失效",
      cooldownReasonFilter: "all",
      tagFilter: "vip",
    }),
    "/accounts?status=available&statusReason=%E6%8E%88%E6%9D%83%E5%A4%B1%E6%95%88&query=acc-a&group=%E9%BB%98%E8%AE%A4&tag=vip",
  );
});
