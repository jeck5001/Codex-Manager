# CPA Account Sync Design

## Goal

在当前 `Codex-Manager` 项目里新增一套“CLIProxyAPI / CPA 账号同步源”，允许用户在设置页配置 CPA 地址与凭据，并把 CPA 中已经登录好的 Codex 认证文件同步导入到本项目号池。

## Current Context

- 当前项目已经有成熟的“账号导入”链路，入口在 Web UI 的批量导入与文件导入，后端核心逻辑在 `crates/service/src/account/account_import.rs`。
- 当前项目中的 “CPA” 只表示一组网关兼容模式，例如 `cpaNoCookieHeaderModeEnabled`，并不代表“可连接外部 CLIProxyAPI 拉账号”的同步功能。
- 当前设置页已经存在一类成熟的外部系统集成模式：`Team Manager`。它提供 `启用 / API URL / API Key / 测试连接` 的完整配置结构，适合作为本次 CPA 同步功能的 UI 和存储参考。
- CLIProxyAPI 对外管理是通过 Management API 完成，认证使用 `Management Key`，不是网页登录密码。

## External Constraint

根据 CLIProxyAPI 官方文档与仓库说明：

- 管理接口基路径为 `/v0/management`
- 认证使用：
  - `Authorization: Bearer <management-key>`
  - 或 `X-Management-Key: <management-key>`
- 账号文件相关接口包括：
  - `GET /auth-files`
  - `GET /auth-files/download`
  - `POST /auth-files`

因此本轮设计不支持“只填网页登录密码后直接同步账号”。真正可行的配置项必须是：

- `CPA API URL`
- `CPA Management Key`

如果用户只有网页登录密码，没有 Management Key，则只能先到 CLIProxyAPI 侧生成或找到 Management Key，再回到本项目配置同步。

## Scope

本轮只做：

- 在设置页新增 CPA 同步源配置
- 测试 CPA Management API 连通性
- 手动触发一次“立即同步”
- 将 CPA 返回的认证文件转换后导入现有号池
- 在同步结果中明确新增、更新、失败数量

本轮不做：

- 不支持仅凭网页登录密码直接同步
- 不做双向同步
- 不回删 CPA 中的账号文件
- 不自动定时同步
- 不新增新的账号存储模型

## User Experience

### Settings Page

在设置页新增一张独立卡片：`CLIProxyAPI / CPA`

字段：

- `启用同步`
- `CPA API URL`
  - 例：`https://your-cpa.example.com`
- `Management Key`
  - 密码输入框
  - 如果已经保存，则显示“留空表示保持不变”

按钮：

- `保存 CPA 设置`
- `测试连接`
- `立即同步`

说明文案需要明确提示：

- 这里填写的是 CLIProxyAPI 的 Management Key，不是网页登录密码
- 同步会把 CPA 中已登录的 Codex auth 文件导入当前号池
- 当前是单向同步，不会删除 CPA 侧账号

### Sync Result

点击“立即同步”后，前端展示：

- 总文件数
- 成功导入数
- 新增账号数
- 更新账号数
- 失败数
- 最多前若干条失败原因

成功后刷新账号列表。

## Architecture

### 1. Reuse Existing Import Pipeline

本轮不单独实现第二套“CPA 账号入库器”。

后端同步流程统一为：

1. 读取应用设置中的 CPA URL / Management Key
2. 请求 CPA Management API，获取 auth file 列表
3. 下载每个可识别的 auth 文件内容
4. 将 CPA auth 内容转换为当前 `account/import` 支持的 JSON 格式
5. 调用已有导入逻辑完成去重、新增、更新、cookies/session_token 写入

这样可以复用已有：

- token 提取与兼容逻辑
- `chatgpt_account_id` / `workspace_id` 去重逻辑
- cookies / session token 合并逻辑
- 账号与 token 存储逻辑

### 2. Dedicated CPA Sync Module

为了不把 `account_import.rs` 和 `account_payment.rs` 继续堆大，本轮新增独立模块，例如：

- `crates/service/src/account/account_cpa_sync.rs`

职责：

- 读取 CPA 设置
- 发送 CPA Management API 请求
- 解析 auth file 列表
- 下载 auth file 内容
- 转换为导入器可消费的字符串列表
- 调用 `import_account_auth_json`
- 输出同步摘要

`account_import.rs` 继续只负责“导入内容 -> 入库”，不感知 CPA HTTP 细节。

### 3. Settings Storage

应用设置新增以下字段：

- `cpaSyncEnabled: boolean`
- `cpaSyncApiUrl: string`
- `cpaSyncHasManagementKey: boolean`
- `cpaSyncManagementKey?: string`

其中：

- 明文 key 不直接在快照接口里回显
- 只返回 `hasManagementKey`
- 写入时采用和 Team Manager API Key 相同的“留空不覆盖、显式输入才更新”的策略

## Data Flow

### Test Connection

`测试连接` 的后端流程：

1. 解析前端传入或已保存的 `apiUrl`
2. 解析前端传入或已保存的 `managementKey`
3. 请求 `GET /v0/management/auth-files`
4. 如果返回成功，则显示：
   - 连接成功
   - 可见 auth file 数量
5. 如果失败，则返回清晰错误：
   - URL 未配置
   - Key 未配置
   - 401/403 认证失败
   - 网络连接失败
   - 响应结构不兼容

### Manual Sync

`立即同步` 的后端流程：

1. 拉取 `auth-files` 列表
2. 仅处理可判定为 Codex / OpenAI / ChatGPT auth 的记录
3. 下载每个目标 auth file
4. 将下载内容映射为现有导入器可接受的对象或 JSON 流
5. 调用导入器
6. 聚合同步结果并返回前端

返回结构至少包含：

- `totalFiles`
- `eligibleFiles`
- `downloadedFiles`
- `created`
- `updated`
- `failed`
- `importedAccountIds`
- `errors`

## CPA Auth File Compatibility

本轮设计要求同步器对以下几类输入做兼容判断：

1. 已经接近当前项目导入格式的 JSON
2. 扁平 token 结构
3. `tokens` 包裹结构
4. 含 `session_token` / `cookies` 的结构

如果某个 auth file 不是 OpenAI / Codex 账号，或者缺少最小可导入字段，则跳过并记录原因，不中断整批同步。

## Error Handling

需要显式处理：

- `CPA API URL` 未配置
- `Management Key` 未配置
- URL 格式非法
- HTTP 超时 / DNS / TLS 异常
- `401` / `403`
- `auth-files` 返回非 JSON
- 下载单个 auth file 失败
- 单个 auth file 无法转换
- 导入器拒绝该文件

原则：

- 批量同步是“尽量导入其余可用项”
- 单个文件失败不应导致整批失败
- 前端结果页只展示前几条失败项，完整内容写日志

## Security

- Management Key 视为敏感凭据，存储与展示策略对齐 Team Manager API Key
- 设置快照不回传明文 key
- 日志中不打印完整 key
- 请求失败日志不打印完整 Authorization 头

## Testing

至少覆盖以下测试：

1. 设置快照与保存逻辑
   - 保存 CPA URL
   - 保存 Management Key
   - 二次保存时留空不覆盖原 key

2. CPA 测试连接
   - 成功
   - 未配置 URL / Key
   - 401
   - 非法响应

3. CPA 同步
   - 同步一个可导入文件并新增账号
   - 同步已有账号并更新
   - 混合同步：部分成功、部分失败
   - 非 Codex 文件被跳过

4. 前端设置页
   - 渲染新卡片
   - 保存按钮调用设置接口
   - 测试连接按钮调用测试接口
   - 立即同步按钮调用同步接口并显示结果

## File Impact

预计涉及：

- `apps/src/app/settings/page.tsx`
- `apps/src/types/index.ts`
- `apps/src/lib/api/account-client.ts`
- `apps/src/lib/api/normalize.ts`
- `crates/service/src/account/account_cpa_sync.rs`
- `crates/service/src/account/mod.rs`
- `crates/service/src/rpc_dispatch/account.rs`
- `crates/service/src/app_settings/api/current.rs`
- `crates/service/src/app_settings/api/patch.rs`
- `crates/service/src/app_settings/shared.rs`
- `crates/service/src/account/tests/...`

## Recommendation

采用“单向手动同步 + 复用现有导入器”的最小可用方案。

原因：

- 与现有项目结构最契合
- 可以最快交付“配置 URL + Key 后直接同步号池”的用户价值
- 不会把 CPA HTTP 协议细节扩散到账号存储和网关逻辑
- 后续若需要定时同步，只需在该模块外再挂后台任务调度，不必推翻本轮设计
