import test from "node:test";
import assert from "node:assert/strict";

import { describeFreeProxyClearResult } from "./freeproxy-clear-state.ts";

test("describeFreeProxyClearResult summarizes cleared gateway and register pools", () => {
  assert.equal(
    describeFreeProxyClearResult({
      previousProxyListValue: "http://1.1.1.1:80,http://2.2.2.2:80",
      previousProxyListCount: 2,
      clearedGatewayProxyCount: 2,
      deletedRegisterProxyCount: 5,
      failedRegisterProxyCount: 1,
      remainingRegisterProxyCount: 1,
    }),
    "已清空网关代理池 2 个条目，注册代理池删除 5 个，失败 1 个"
  );
});
