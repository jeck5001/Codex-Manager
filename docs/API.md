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
- 导出代理：
  - `GET /api/export/requestlogs`
  - `GET /api/export/auditlogs`

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
- `retryPolicyMaxRetries: number`
- `retryPolicyBackoffStrategy: string`
- `retryPolicyRetryableStatusCodes: number[]`
- `backgroundTasks: object`

### `appSettings/set`

用途：更新前端设置页使用的配置项。

当前前端已消费的近期字段：
- `webAccessPassword`
- `retryPolicyMaxRetries`
- `retryPolicyBackoffStrategy`
- `retryPolicyRetryableStatusCodes`

## 5. 重试策略

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

## 6. 巡检与健康检查

### `healthcheck/config/get`

用途：读取自动巡检配置。

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

## 7. 审计日志

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

## 8. 请求日志导出

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

转发目标：
- `GET http://<service_addr>/export/requestlogs?...`

说明：
- Service 端导出为流式响应
- Web 端会透传 `content-type`、`content-disposition`、`cache-control`

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
