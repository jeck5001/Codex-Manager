import type { FreeProxyClearResult } from "../../types/index.ts";

export function describeFreeProxyClearResult(result: FreeProxyClearResult) {
  return `已清空网关代理池 ${result.clearedGatewayProxyCount} 个条目，注册代理池删除 ${result.deletedRegisterProxyCount} 个，失败 ${result.failedRegisterProxyCount} 个`;
}
