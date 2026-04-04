# Temp Mail Cloudflare Domain Sync Design

## Goal

在创建 `Temp-Mail（自部署）` 邮箱服务时，系统自动为该服务生成一个固定子域名，并同步完成 Cloudflare Email Routing `Subdomains` 与 `temp-email` Worker `DOMAINS` 环境变量更新，只有全部成功后才落库为可用服务。

## Current Context

- 当前 `temp_mail` 服务配置创建入口位于 `vendor/codex-register/src/web/routes/email_services.py` 的 `create_email_service`。
- 当前 `TempMailService` 实现在 `vendor/codex-register/src/services/temp_mail.py`，`create_email()` 只会基于已存在的 `config.domain` 生成邮箱地址名，不会编排 Cloudflare 资源。
- 当前邮箱服务管理页位于 `vendor/codex-register/templates/email_services.html` 与 `vendor/codex-register/static/js/email_services.js`。
- 当前系统没有 Cloudflare 相关全局配置，也没有 Email Routing / Worker 的 API 封装。

## Recommended Approach

采用“服务创建时同步编排”的同步式方案：

1. 用户在邮箱服务页新增 `Temp-Mail（自部署）` 服务。
2. 后端读取全局 Cloudflare 配置与域名生成规则。
3. 后端生成一个固定子域名，例如 `tm-ab12cd.mail.example.com`。
4. 后端调用 Cloudflare Email Routing，同步新增该子域名到 `Subdomains`。
5. 后端读取并更新 `temp-email` Worker 的 `DOMAINS` 环境变量，追加该域名并保存部署。
6. 只有上述 Cloudflare 两步全部成功，才把 `temp_mail` 服务配置写入数据库。

后续真正注册时，`TempMailService.create_email()` 仍只负责在这个固定域名下创建随机邮箱地址，不再触碰 Cloudflare。

## Why This Approach

- 保持 `TempMailService` 的职责边界清晰：运行时只创建邮箱地址，不做外部基础设施编排。
- 避免每次注册都修改 Cloudflare，减少高频路径的外部依赖、延迟与失败率。
- 一个服务绑定一个固定域名，排障、审计和后续手工清理都更直接。
- 与当前代码结构最契合，改动集中在邮箱服务配置创建链路。

## Global Settings

新增一组全局 Cloudflare 设置，放在现有设置页中统一管理，而不是保存在单条 `Temp-Mail` 服务配置里。

### Credentials

- `cloudflare_api_token`
- `cloudflare_account_id`
- `cloudflare_zone_id`
- `cloudflare_worker_name`

其中 `cloudflare_worker_name` 默认值为 `temp-email`。

### Domain Generation Rules

- `temp_mail_domain_base`
- `temp_mail_subdomain_mode`
- `temp_mail_subdomain_length`
- `temp_mail_subdomain_prefix`

含义：

- `temp_mail_domain_base` 是固定域名后缀，例如 `mail.example.com` 或 `example.com`
- `temp_mail_subdomain_mode` 第一版支持：
  - `random`
  - `sequence`
- `temp_mail_subdomain_length` 仅对 `random` 生效
- `temp_mail_subdomain_prefix` 为可选固定前缀，例如 `tm`

第一版推荐默认：

- `mode = random`
- `length = 6`
- `prefix = tm`

### Sync Behavior

- `temp_mail_sync_cloudflare_enabled`
- `temp_mail_require_cloudflare_sync`

默认策略：

- 启用 Cloudflare 自动同步
- 要求同步成功才允许创建服务

## Domain Allocation Rules

每创建一个 `Temp-Mail（自部署）` 服务，只生成一次固定域名，之后该服务长期绑定此域名。

例如：

- 基底为 `mail.example.com`
- 前缀为 `tm`
- 随机段为 `ab12cd`

最终域名为：

- `tm-ab12cd.mail.example.com`

约束：

- 生成结果必须先检查数据库 `email_services.config.domain` 是否已存在
- 如果冲突则重新生成，直到命中唯一值或达到最大重试次数
- 编辑服务时不允许修改 `domain`
- 如果想使用新域名，用户应新建一个服务

## Cloudflare Orchestration

新增一个独立的 Cloudflare 编排服务模块，例如：

- `vendor/codex-register/src/services/cloudflare_temp_mail.py`

该模块负责：

1. 生成候选域名
2. 幂等确保 Email Routing Subdomain 已存在
3. 幂等确保 Worker `DOMAINS` 已包含目标域名
4. 在半失败场景下执行有限回滚

路由层 `email_services.py` 只负责：

- 识别 `service_type == "temp_mail"`
- 调用编排服务
- 根据结果决定是否落库

## Worker DOMAINS Format

系统内部统一把 Worker `DOMAINS` 视为“JSON 数组字符串”，例如：

```json
["tm-ab12cd.mail.example.com", "tm-ef34gh.mail.example.com"]
```

更新逻辑需要兼容历史格式：

1. 优先按 JSON 数组解析
2. 如果不是 JSON 数组，则尝试按逗号分隔字符串解析
3. 解析后做去重
4. 最终统一写回 JSON 数组字符串

这可以兼容已有旧值，又让后续行为稳定可预期。

## Create Flow

创建 `Temp-Mail（自部署）` 服务时，后端流程固定为：

1. 校验请求中的 `base_url` 与 `admin_password`
2. 读取全局 Cloudflare 设置
3. 校验 Cloudflare 设置完整性
4. 生成固定子域名
5. 检查数据库域名冲突
6. 调用 Email Routing 同步子域名
7. 调用 Worker `DOMAINS` 更新并保存部署
8. 将最终 `domain` 写入服务配置
9. 落库创建 `email_services` 记录

如果第 6 或第 7 步失败，则整个创建接口返回错误，不创建服务记录。

## Failure Handling And Rollback

失败策略以“数据库与 Cloudflare 尽量一致”为目标。

### Email Routing Failed

- 直接返回错误
- 不落库

### Email Routing Succeeded But Worker Update Failed

- 尝试回滚刚刚新增的 Email Routing Subdomain
- 回滚成功：返回错误，不落库
- 回滚失败：返回明确错误，不落库，并记录日志，提示存在残留 Cloudflare 资源需要手工处理

### Database Write Failed After Cloudflare Succeeded

- 这是低概率但必须考虑的场景
- 处理方式与上一类一致：
  - 先尝试回滚 Worker `DOMAINS`
  - 再尝试回滚 Email Routing Subdomain
- 如果回滚链路失败，返回带残留资源说明的错误日志

## Edit And Delete Rules

### Edit

允许编辑：

- 服务名称
- `base_url`
- `admin_password`
- `enabled`
- `priority`

不允许编辑：

- `domain`

原因：该域名已与 Cloudflare Worker 和 Email Routing 建立绑定关系，修改会引入额外资源迁移与一致性问题。

### Delete

第一版删除服务时：

- 只删除本地服务记录
- 不自动删除 Cloudflare Email Routing Subdomain
- 不自动从 Worker `DOMAINS` 中移除该域名

原因：

- 自动清理远端资源存在误删风险
- 当前没有域名引用计数与安全确认流程
- 用户可以后续手工复用或手工清理该域名

## UI Changes

### Settings Page

新增 Cloudflare Temp Mail 设置区块，用于维护：

- API Token
- Account ID
- Zone ID
- Worker Name
- 域名基底
- 生成模式
- 随机长度
- 固定前缀
- 同步开关

### Email Service Create Form

创建 `Temp-Mail（自部署）` 服务时，表单不再要求用户填写最终 `domain`。

替代方式：

- 表单只保留 `Worker 地址` 与 `Admin 密码`
- 域名字段改为只读提示，说明“创建时自动生成固定域名”

如果需要兼容现有表单结构，也可以保留字段但改为：

- 不可编辑
- 仅展示“将自动生成”

推荐直接移除手填 `domain` 输入，避免用户误解。

## Non-Goals

- 不实现“每次创建邮箱地址都动态生成并同步子域名”
- 不在第一版实现 Cloudflare 资源自动清理
- 不重写 `TempMailService.create_email()` 的邮箱名前缀生成策略
- 不扩展到 Outlook 或 `custom_domain` 服务

## Files Likely To Change

- `vendor/codex-register/src/config/settings.py`
- `vendor/codex-register/src/web/routes/settings.py`
- `vendor/codex-register/src/web/routes/email_services.py`
- `vendor/codex-register/src/services/temp_mail.py`
- `vendor/codex-register/src/services/__init__.py`
- `vendor/codex-register/src/services/cloudflare_temp_mail.py`
- `vendor/codex-register/static/js/email_services.js`
- `vendor/codex-register/templates/email_services.html`
- `vendor/codex-register/tests/test_custom_domain_email_service.py`
- 新增 Cloudflare 编排相关测试文件

## Testing Requirements

至少覆盖以下场景：

1. `temp_mail` 服务创建时会先执行 Cloudflare 编排，再落库
2. Cloudflare 配置缺失时创建失败，且不会落库
3. 域名生成逻辑会跳过数据库内已存在域名
4. Email Routing 成功、Worker 更新失败时，会尝试回滚且不会落库
5. Worker `DOMAINS` 更新对已存在域名是幂等的
6. 历史 `DOMAINS` 逗号分隔格式可被兼容解析
7. 编辑 `temp_mail` 服务时不能修改 `domain`
8. 删除 `temp_mail` 服务时不触发 Cloudflare 清理
9. 非 `temp_mail` 服务创建路径保持不变

验证命令应至少包含：

- `python -m pytest vendor/codex-register/tests/...`
- `pnpm run build:desktop`

## Expected Outcome

- 新建 `Temp-Mail（自部署）` 服务后，服务记录中的 `domain` 已经是可用且稳定的固定域名
- 注册运行时不再依赖 Cloudflare 资源动态变更
- Cloudflare Email Routing 与 Worker `DOMAINS` 能与本地服务配置保持高一致性
- 失败时接口能给出明确错误，并尽量避免留下半完成状态
