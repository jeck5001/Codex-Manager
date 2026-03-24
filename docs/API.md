# CodexManager API 说明

> 对应需求：[docs/PRD.md](PRD.md) | 开发清单：[docs/TODO.md](TODO.md) | 验收标准：[docs/ACCEPTANCE.md](ACCEPTANCE.md)

本文档只记录当前仓库里已经实现并进入主链路的接口，优先覆盖最近交付的 Web 安全、重试策略、巡检、导出与审计能力。

## 1. 访问入口

### Service 直连
- RPC 地址：`http://<service_addr>/rpc`
- 默认地址：`http://127.0.0.1:48760/rpc`
- Header：
  - `content-type: application/json`
  - `x-codexmanager-rpc-token: <rpc_token>`
  - `x-codexmanager-operator: <operator>`，可选，Web 侧默认写 `web-ui`

### Web 代理
- Web 地址：`http://127.0.0.1:48761`
- 代理 RPC：`POST /api/rpc`
- 远程管理状态：`GET /api/management/status`
- 远程管理 RPC：`POST /api/management/rpc`
  - Header：`x-codexmanager-management-secret: <secret>`，也支持 `Authorization: Bearer <secret>`
  - 仅在设置页启用“远程管理 API”且已配置访问密钥后可用
  - `GET /api/management/status` 与 `POST /api/management/rpc` 使用相同密钥鉴权
- 导出代理：
  - `GET /api/export/requestlogs`
  - `GET /api/export/auditlogs`

远程管理状态示例：

```bash
curl -s http://127.0.0.1:48761/api/management/status \
  -H "x-codexmanager-management-secret: <secret>"
```

返回字段示例：
- `enabled: boolean`
- `secretConfigured: boolean`
- `serviceAddr: string`
- `serviceReachable: boolean`
- `webAccessPasswordConfigured: boolean`
- `webAccessTwoFactorEnabled: boolean`

### Web 登录与安全
- 登录页：`GET /__login`
- 登录提交：`POST /__login`
- 鉴权状态：`GET /__auth_status`
- 退出登录：`GET /__logout` / `POST /__logout`

## 2. JSON-RPC 约定

请求示例：

```json
{
  "id": 1,
  "method": "webAuth/2fa/setup",
  "params": null
}
```

响应示例：

```json
{
  "id": 1,
  "result": {
    "enabled": false
  }
}
```

说明：
- `id` 由调用方自带，服务端原样返回。
- `method` 使用字符串路由。
- `params` 为对象或 `null`。
- 业务失败时，当前实现仍通过 `result.error` 或字符串错误返回，不统一使用 JSON-RPC 标准 `error` 对象。

## 3. Web 安全与 2FA

### `webAuth/status`

用途：读取当前 Web 访问安全状态。

返回字段：
- `passwordConfigured: boolean`
- `twoFactorEnabled: boolean`
- `recoveryCodesRemaining: number`

### `webAuth/password/set`

用途：设置或覆盖 Web 访问密码。

参数：
- `password: string`

返回字段：
- `passwordConfigured: boolean`

### `webAuth/password/clear`

用途：清空 Web 访问密码，同时清除已绑定的 2FA secret 和恢复码。

参数：无

返回字段：
- `passwordConfigured: boolean`

### `webAuth/2fa/setup`

用途：生成新的 TOTP secret、二维码与恢复码。要求已先设置 Web 访问密码。

参数：无

返回字段：
- `enabled: boolean`
- `secret: string`
- `otpAuthUrl: string`
- `qrCodeDataUrl: string`
- `recoveryCodes: string[]`
- `setupToken: string`

说明：
- `qrCodeDataUrl` 为可直接展示的 `data:image/png;base64,...`
- `setupToken` 用于后续 `webAuth/2fa/verify` 首次绑定确认

### `webAuth/2fa/verify`

用途：
- 首次绑定时校验 TOTP 验证码并正式启用 2FA
- 已启用后，也可用来校验当前 TOTP 或恢复码

参数：
- 首次绑定：
  - `setupToken: string`
  - `code: string`
- 已启用状态校验：
  - `code?: string`
  - `recoveryCode?: string`

返回字段：
- `enabled: boolean`
- `recoveryCodesRemaining: number`
- `method: "totp" | "recovery_code"`

### `webAuth/2fa/disable`

用途：用当前 TOTP 或恢复码停用 2FA。

参数：
- `code?: string`
- `recoveryCode?: string`

返回字段：
- `enabled: boolean`
- `recoveryCodesRemaining: number`
- `method: "disabled"`

## 4. App Settings 快照

### `appSettings/get`

用途：读取前端设置页和桌面端共享的当前配置快照。

近期相关字段：
- `webAccessPasswordConfigured: boolean`
- `webAccessTwoFactorEnabled: boolean`
- `webAccessRecoveryCodesRemaining: number`
- `remoteManagementEnabled: boolean`
- `remoteManagementSecretConfigured: boolean`
- `responseCacheEnabled: boolean`
- `responseCacheTtlSecs: number`
- `responseCacheMaxEntries: number`
- `payloadRewriteRulesJson: string`
- `retryPolicyMaxRetries: number`
- `retryPolicyBackoffStrategy: string`
- `retryPolicyRetryableStatusCodes: number[]`
- `backgroundTasks: object`

### `appSettings/set`

用途：更新前端设置页使用的配置项。

当前前端已消费的近期字段：
- `webAccessPassword`
- `remoteManagementEnabled`
- `remoteManagementSecret`
- `responseCacheEnabled`
- `responseCacheTtlSecs`
- `responseCacheMaxEntries`
- `payloadRewriteRulesJson`
- `retryPolicyMaxRetries`
- `retryPolicyBackoffStrategy`
- `retryPolicyRetryableStatusCodes`

补充说明：
- `payloadRewriteRulesJson` 当前是网关声明式 Payload Rewrite 的后端第一版入口，内容为 JSON 数组字符串。
- 第一版仅支持顶层字段改写，规则形如 `[{ "path": "/v1/responses", "field": "service_tier", "mode": "set_if_missing", "value": "flex" }]`。
- `mode` 目前支持 `set` / `set_if_missing`，`path` 支持精确路径或 `*`。
- 为避免绕过现有 API Key 模型白名单，第一版明确禁止改写 `model` 字段。
- 可通过环境变量 `CODEXMANAGER_PAYLOAD_REWRITE_RULES` 覆盖。

### 插件管理（实验）

当前已落地的后端 RPC。

补充说明：
- 更完整的插件脚本约定、模板和后续 Hook 契约见 [插件管理与 Lua 开发指南](report/20260323193000000_插件管理与Lua开发指南.md)。
- 已接入 `mlua` 运行时：启用插件会在 gateway 请求链路中执行。
- `pre_route` 可读取请求与 API Key 信息，并支持拒绝请求、改写请求体或修改 `model`。
- `post_route` 可在选定账号后继续基于账号 / 路由信息改写上游请求，或直接拒绝。
- `post_response` 可读取响应状态码与响应头，并记录 `annotations` 到 trace / 日志；当前不改写响应体。
- Lua 运行时已启用沙箱与超时保护：默认超时 `100ms`，可通过 `timeoutMs` 调整。
- 无启用插件时，网关仅做一次进程内缓存判断，不会继续扫描插件表。

### `plugin/list`

用途：列出当前插件注册表中的全部插件。

返回字段：
- `items: PluginItem[]`

`PluginItem` 关键字段：
- `id: string`
- `name: string`
- `description?: string`
- `runtime: "lua"`
- `hookPoints: ("pre_route" | "post_route" | "post_response")[]`
- `scriptContent: string`
- `enabled: boolean`
- `timeoutMs: number`

### `plugin/upsert`

用途：创建或更新插件元数据与脚本内容。

参数：
- `id?: string`
- `name: string`
- `description?: string`
- `runtime?: "lua"`
- `hookPoints: string[]`
- `scriptContent: string`
- `enabled?: boolean`
- `timeoutMs?: number`

说明：
- 当前仅接受 `lua` runtime。
- `hookPoints` 当前仅接受 `pre_route`、`post_route`、`post_response`。
- `scriptContent` 不能为空；`hookPoints` 会在写入前去重并保持输入顺序。
- 保存前会执行 Lua 加载校验，脚本必须暴露 `handle(ctx)` 函数。
- 运行时会禁用 `io` / `os` / `package` / `debug` / `require` / `loadfile` 等高风险能力。

### `plugin/delete`

用途：按 `id` 删除插件。

参数：
- `id: string`

审计说明：
- `plugin/upsert`、`plugin/delete` 都会写入审计日志。

## 5. 响应缓存

### `gateway/cache/config/get`

用途：读取全局响应缓存配置。

返回字段：
- `enabled: boolean`
- `ttlSecs: number`
- `maxEntries: number`

### `gateway/cache/config/set`

用途：更新全局响应缓存配置。

参数：
- `enabled?: boolean`
- `ttlSecs?: number`
- `maxEntries?: number`

返回字段同 `gateway/cache/config/get`。

说明：
- 写入后会同步更新 `appSettings/get` 的 `responseCache*` 快照字段。
- 环境变量 `CODEXMANAGER_RESPONSE_CACHE_ENABLED`、`CODEXMANAGER_RESPONSE_CACHE_TTL_SECS`、`CODEXMANAGER_RESPONSE_CACHE_MAX_ENTRIES` 可覆盖默认运行时配置，对齐通用验收 G4。

### `gateway/cache/stats`

用途：读取响应缓存运行时统计。

返回字段：
- `enabled: boolean`
- `ttlSecs: number`
- `maxEntries: number`
- `entryCount: number`
- `estimatedBytes: number`
- `hitCount: number`
- `missCount: number`
- `hitRatePercent: number`

### `gateway/cache/clear`

用途：清空响应缓存并返回清空后的统计快照。

### `apikey/responseCache/get`

用途：读取单个 API Key 是否允许命中 / 写入响应缓存。

参数：
- `id: string`

返回字段：
- `enabled: boolean`

### `apikey/responseCache/set`

用途：设置单个 API Key 的响应缓存开关。

参数：
- `id: string`
- `enabled: boolean`

返回字段同 `apikey/responseCache/get`。

## 6. 重试策略

### `gateway/retryPolicy/get`

返回字段：
- `maxRetries: number`
- `backoffStrategy: "fixed" | "exponential"`
- `retryableStatusCodes: number[]`

### `gateway/retryPolicy/set`

参数：
- `maxRetries?: number`
- `backoffStrategy?: string`
- `retryableStatusCodes?: number[]`

返回字段同 `get`。

说明：
- 当前 `appSettings/get` 会同步返回持久化后的 `retryPolicy*` 快照
- 写操作会进入 `audit/list`

## 7. 仪表盘与巡检

### `dashboard/health`

用途：读取首页健康仪表盘的聚合数据。

返回字段：
- `generatedAt: number`
- `accountStatusBuckets: array`
- `gatewayMetrics: object`
- `recentHealthcheck: object | null`

说明：
- `recentHealthcheck` 与 `healthcheck/run` 的结果结构一致，首页「最近巡检」卡片直接消费这一字段。
- 当服务尚未执行过巡检时，`recentHealthcheck` 返回 `null`。

### `healthcheck/config/get`

用途：读取自动巡检配置。

返回字段：
- `enabled: boolean`
- `intervalSecs: number`
- `sampleSize: number`
- `recentRun: object | null`

### `healthcheck/config/set`

参数：
- `enabled?: boolean`
- `intervalSecs?: number`
- `sampleSize?: number`

### `healthcheck/run`

用途：立即触发一次巡检。

返回字段示例：
- `startedAt`
- `finishedAt`
- `sampledAccounts`
- `successCount`
- `failureCount`
- `failedAccounts`

## 8. 审计日志

### `audit/list`

用途：分页查询审计日志。

常用参数：
- `action?: string`
- `objectType?: string`
- `objectId?: string`
- `timeFrom?: number | null`
- `timeTo?: number | null`
- `page?: number`
- `pageSize?: number`

返回字段：
- `items`
- `total`
- `page`
- `pageSize`

### `GET /api/export/auditlogs`

用途：通过 Web 代理下载审计日志导出文件。

转发目标：
- `GET http://<service_addr>/export/auditlogs?...`

## 9. 请求日志导出

### `GET /api/export/requestlogs`

用途：通过 Web 代理下载请求日志导出结果。

常用 query：
- `format=csv|json`
- `query`
- `statusFilter`
- `timeFrom`
- `timeTo`
- `model`
- `keyId`
- `keyIds`（可重复传多个，例如 `...&keyIds=gk-a&keyIds=gk-b`）

转发目标：
- `GET http://<service_addr>/export/requestlogs?...`

说明：
- Service 端导出为流式响应
- Web 端会透传 `content-type`、`content-disposition`、`cache-control`
- 当日志页按平台密钥名称模糊匹配到多个 Key 时，前端会展开成多个 `keyIds` 一并导出

## 9. Web 登录流程

### 单密码模式
1. `GET /__auth_status` 判断是否已设置 Web 密码。
2. `POST /__login` 提交密码。
3. 成功后写入 `codexmanager_web_auth` Cookie。

### 密码 + 2FA 模式
1. `POST /__login` 先校验密码。
2. 成功后服务端返回 2FA 页，并写入 `codexmanager_web_auth_pending` Cookie。
3. 再次 `POST /__login` 提交 `code` 或恢复码。
4. 成功后清掉 pending cookie，写入正式 `codexmanager_web_auth` Cookie。

## 10. 当前已验证的主链路

已通过仓库内回归验证：
- 2FA setup / verify / disable / recovery code 消费
- 清空 Web 密码时自动清除 2FA
- Web 登录从密码页进入验证码页
- pending cookie 成功交换为正式登录 cookie
- 桌面端设置弹窗可生成二维码、展示恢复码并调用真实 RPC

## 11. 实验性 MCP Server

当前仓库已落地一个 feature-gated 的实验二进制：
- 启动方式：`cargo run -p codexmanager-service --features mcp --bin codexmanager-mcp`
- 传输方式：
  - `stdio`：默认模式，使用 `Content-Length` framed JSON-RPC
  - `http-sse`：`cargo run -p codexmanager-service --features mcp --bin codexmanager-mcp -- http-sse`
- 当前已处理的方法：`initialize`、`ping`、`tools/list`、`tools/call`
- `tools/list` 当前会暴露规划中的 4 个工具定义：`chat_completion`、`list_models`、`list_accounts`、`get_usage`
- `tools/call` 已接通 4 个工具：
  - `chat_completion`：通过进程内一次性 backend server 复用真实网关 `/v1/chat/completions` 链路；需在参数中传 `apiKey`，或预先设置环境变量 `CODEXMANAGER_MCP_API_KEY`
    - 当前仅支持 `stream=false`，返回 `response`（网关 JSON）和 `gateway` 元数据（status / actualModel / cache / traceId）
  - `list_models`：读取当前模型缓存
  - `list_accounts`：返回账号状态概览（不含敏感 token）
  - `get_usage`：返回聚合后的用量汇总

说明：
- 首轮实现先手写最小协议层，避免在仅需 stdio 骨架阶段就引入新的 MCP 依赖并扩大二进制影响面
- 设置页与 `appSettings/get|set` 已新增 `mcpEnabled` / `mcpPort`；当 `mcpEnabled=false` 时，`initialize`、`tools/list`、`tools/call` 会直接返回禁用错误
- 新增环境变量覆盖：`CODEXMANAGER_MCP_ENABLED`、`CODEXMANAGER_MCP_PORT`
- HTTP SSE 传输已接通：
  - `GET /sse` 建立会话并返回一次性 `messageUrl`
  - `POST /message?sessionId=...` 复用同一套 `initialize` / `tools/*` 会话处理
  - 监听端口默认取 `mcpPort`（默认 `48762`）
- 当前仓库内已覆盖 `stdio` 主链路与 `http_sse_*` 单测；受 automation 沙箱限制，本轮未额外执行真实端口监听验收
- Claude Code / Cursor 的接入示例见 [MCP 接入指南](report/20260323161000000_MCP接入指南.md)
