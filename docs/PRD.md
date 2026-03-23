# CodexManager 功能增强 PRD

> 版本：v1.0 | 日期：2026-03-22 | 基线版本：v0.1.10

---

## 一、背景与目标

CodexManager 已具备账号池管理、用量追踪、本地网关、协议适配、请求日志等核心能力。为进一步提升产品的运维可观测性、网关智能化、安全性与生态集成能力，规划以下六大方向的功能增强。

---

## 二、功能清单

### F01 实时健康仪表盘 (Health Dashboard)

**优先级**：P0 | **复杂度**：中 | **涉及层**：前端 + service

**背景**：当前仪表盘以用量数值为主，缺少对账号池整体健康状态的直观感知。

**需求描述**：
- 在仪表盘首页增加账号健康总览卡片：按状态（在线 / 冷却中 / 不可用 / 已禁用 / 额度耗尽）分组统计，展示数量与占比。
- 增加网关实时指标面板：当前 QPS、请求成功率、延迟 P50 / P95 / P99，基于滚动窗口（最近 5 分钟）。
- 增加 mini 时序图：最近 1 小时的请求量与错误率趋势折线。
- 数据来源：复用现有 `usage_snapshots` + `request_logs` + 内存中的 gateway metrics。

**技术要点**：
- service 层新增 RPC 方法 `dashboard/health` 返回聚合后的健康快照。
- 延迟百分位基于 `request_logs.duration_ms` 在内存中维护滑动窗口（ring buffer，5 分钟粒度）。
- 前端使用轻量图表库（如 uPlot）渲染趋势图，避免引入重依赖。

---

### F02 告警通知系统 (Alert & Notification)

**优先级**：P0 | **复杂度**：高 | **涉及层**：service + 前端设置页

**背景**：账号失效、额度耗尽等异常需要用户主动查看才能发现，缺少主动推送机制。

**需求描述**：
- 支持定义告警规则，初期内置以下规则：
  - 账号 token 刷新连续失败 N 次
  - 账号额度使用率超过阈值（如 90%）
  - 最近 M 分钟内网关错误率超过阈值
  - 所有账号均不可用
- 支持通知渠道：
  - Webhook（通用，POST JSON payload）
  - Bark（iOS 推送）
  - Telegram Bot
  - 企业微信机器人
- 设置页新增「告警通知」Tab，配置渠道与规则。
- 告警去重：同一规则在恢复前不重复触发，支持配置静默期。

**技术要点**：
- 新增 `alert_rules` 与 `alert_channels` 表。
- service 层新增 `alert/` 模块，包含规则引擎（定时轮询式）和渠道分发器。
- 渠道分发器设计为 trait `AlertSender`，便于后续扩展。
- RPC 方法：`alert/rules/list`、`alert/rules/upsert`、`alert/rules/delete`、`alert/channels/list`、`alert/channels/upsert`、`alert/channels/delete`、`alert/channels/test`、`alert/history/list`。

---

### F03 智能路由策略增强

**优先级**：P0 | **复杂度**：中 | **涉及层**：service gateway

**背景**：当前路由策略仅支持 `ordered` 和 `balanced`，无法根据账号实际状态做更精细的调度。

**需求描述**：
- 新增路由策略：
  - **加权轮询 (weighted)**：按账号剩余额度比例分配权重，额度越多权重越高。
  - **最低延迟优先 (least-latency)**：根据最近 N 次请求的平均延迟选择最快的账号。
  - **成本优先 (cost-first)**：优先消耗 free 账号额度，不足时再切换到 Plus/Pro 账号。
- 路由策略可在设置页切换，保存到 `app_settings`。
- 请求日志中展示实际命中的路由策略名称，便于排查。

**技术要点**：
- 扩展 `gateway/routing/` 模块，将策略抽象为 `RouteStrategy` trait。
- 加权轮询：基于 `usage_snapshots.used_percent` 计算权重。
- 最低延迟：在 gateway 内存中维护每账号的滑动窗口延迟统计。
- 成本优先：读取账号 plan 类型（free / plus / pro），按优先级排序。

---

### F04 请求限流 (Rate Limiting per API Key)

**优先级**：P1 | **复杂度**：中 | **涉及层**：service gateway + 前端

**背景**：多人共用时，单个 API Key 的下游消费者可能耗尽所有账号额度，缺少流量管控手段。

**需求描述**：
- 对每个 API Key 支持配置：
  - RPM（Requests Per Minute）上限
  - TPM（Tokens Per Minute）上限（基于 input_tokens 估算）
  - 每日请求总量上限
- 超限时返回标准 `429 Too Many Requests` 响应，附带 `Retry-After` 头。
- API Key 管理页中增加限流配置入口。
- 仪表盘展示当前各 Key 的消耗速率与剩余配额。

**技术要点**：
- 限流使用令牌桶算法，在内存中维护（per API Key）。
- 新增 `api_key_rate_limits` 表持久化配置。
- gateway auth 层在鉴权通过后、路由前执行限流检查。
- RPC 方法：`apikey/rateLimit/get`、`apikey/rateLimit/set`。

---

### F05 模型降级链 (Model Fallback Chain)

**优先级**：P1 | **复杂度**：中 | **涉及层**：service gateway

**背景**：当前 failover 仅在账号维度切换，不支持模型维度的降级。高负载时主力模型不可用，应能自动降级到备选模型。

**需求描述**：
- 支持为每个 API Key 配置模型降级链，例如：`o3 -> o4-mini -> gpt-4o`。
- 当主模型在所有可用账号上均失败时，自动尝试降级链中的下一个模型。
- 降级触发时在响应头中标注实际使用的模型：`X-CodexManager-Actual-Model`。
- 请求日志记录降级过程。

**技术要点**：
- 新增 `api_key_model_fallbacks` 表，存储降级链配置。
- 扩展 gateway 的 model_picker 模块，在路由失败时查询降级链。
- RPC 方法：`apikey/modelFallback/get`、`apikey/modelFallback/set`。

---

### F06 响应缓存 (Response Cache)

**优先级**：P1 | **复杂度**：高 | **涉及层**：service gateway

**背景**：重复或高度相似的请求（如 embedding、固定 system prompt 的查询）会浪费额度。

**需求描述**：
- 支持对非流式请求做精确匹配缓存（按 model + messages hash）。
- 可按 API Key 粒度开关缓存功能。
- 支持配置全局 TTL（默认 1 小时）和最大缓存条目数。
- 设置页新增「响应缓存」配置区域，展示缓存命中率。
- 缓存命中时响应头标注 `X-CodexManager-Cache: HIT`。

**技术要点**：
- 使用内存 LRU 缓存（`mini-moka` 或手写 LRU）。
- 缓存 key = SHA256(model + messages JSON 序列化)。
- embedding 请求天然适合缓存；chat 请求默认不缓存，需用户显式开启。
- 流式请求不走缓存。
- RPC 方法：`gateway/cache/config/get`、`gateway/cache/config/set`、`gateway/cache/stats`、`gateway/cache/clear`。

---

### F07 API Key 过期与临时分享

**优先级**：P1 | **复杂度**：低 | **涉及层**：service + 前端

**背景**：临时分享 API Key 给他人后，无法自动回收，需手动删除或禁用。

**需求描述**：
- 创建 API Key 时可设置过期时间（可选）。
- 到期后自动标记为 `expired` 状态，拒绝请求。
- API Key 列表展示过期倒计时。
- 支持续期操作。

**技术要点**：
- `api_keys` 表新增 `expires_at` 列（nullable）。
- gateway auth 层鉴权时检查过期时间。
- 可复用现有 usage scheduler 做定期清扫，或在请求时惰性检查。
- RPC 方法：`apikey/create` 增加 `expires_at` 参数、`apikey/renew`。

---

### F08 费用统计与报表

**优先级**：P1 | **复杂度**：中 | **涉及层**：service + 前端

**背景**：当前有 `request_token_stats` 和 `estimated_cost_usd`，但缺少汇总视图和导出能力。

**需求描述**：
- 新增「费用统计」页面：
  - 按 API Key / 模型 / 日期维度汇总 token 消耗与估算费用。
  - 支持选择时间范围（今日 / 本周 / 本月 / 自定义）。
  - 柱状图展示每日费用趋势，饼图展示模型分布。
- 支持导出 CSV 报表。
- 费用估算公式可在设置页配置（每模型单价）。

**技术要点**：
- 基于 `request_logs` 和 `request_token_stats` 做聚合查询。
- 新增 `model_pricing` 表存储模型单价配置。
- RPC 方法：`stats/cost/summary`、`stats/cost/export`、`stats/cost/modelPricing/get`、`stats/cost/modelPricing/set`。

---

### F09 用量趋势分析

**优先级**：P2 | **复杂度**：中 | **涉及层**：service + 前端

**背景**：当前用量数据以快照形式存在，缺少趋势视图帮助用户了解使用模式。

**需求描述**：
- 新增「用量分析」Tab（可挂在仪表盘下或独立页面）：
  - 按天 / 周 / 月的请求量趋势折线图。
  - 模型使用分布饼图。
  - 请求高峰时段热力图（按小时 x 星期几）。
  - 成功率趋势折线图。
- 数据范围：最近 30 天 / 90 天。

**技术要点**：
- 基于 `request_logs.created_at` 做时间桶聚合。
- 热力图数据：`SELECT strftime('%w', created_at) as dow, strftime('%H', created_at) as hour, COUNT(*) ...`
- RPC 方法：`stats/trends/requests`、`stats/trends/models`、`stats/trends/heatmap`。

---

### F10 请求日志导出

**优先级**：P2 | **复杂度**：低 | **涉及层**：service + 前端

**背景**：当前日志仅支持页面查看，无法离线分析。

**需求描述**：
- 在请求日志页增加「导出」按钮。
- 支持导出格式：CSV、JSON。
- 支持按当前筛选条件导出（时间范围、状态码、模型、API Key）。
- 大数据量时采用流式导出，避免内存溢出。

**技术要点**：
- service 层新增 HTTP 端点 `GET /export/requestlogs?format=csv&...filters`，返回流式响应。
- 前端通过 `<a download>` 或 `fetch` + `Blob` 触发下载。
- CSV 使用 Rust `csv` crate 序列化。

---

### F11 账号自动巡检 (Scheduled Health Check)

**优先级**：P2 | **复杂度**：中 | **涉及层**：service

**背景**：账号可能因 token 过期、封禁等原因变为不可用，当前依赖用户手动刷新或请求时才发现。

**需求描述**：
- 支持配置定时巡检间隔（默认每 30 分钟）。
- 巡检内容：对每个启用状态的账号执行 session validity probe。
- 巡检失败的账号自动标记为 `unavailable`，并记录 event。
- 巡检恢复的账号自动恢复为 `active`（对应系统现有可用态枚举）。
- 设置页可开关巡检、调整间隔。
- 巡检结果接入告警通知系统（F02）。

**技术要点**：
- 复用现有 `session_probe` 逻辑，封装为定时任务。
- 挂载到 `usage_scheduler` 或独立 tokio 定时任务。
- 并发控制：每次巡检最多 N 个并发探测。
- RPC 方法：`healthcheck/config/get`、`healthcheck/config/set`、`healthcheck/run`（手动触发）。

---

### F12 API Key 模型访问控制

**优先级**：P2 | **复杂度**：低 | **涉及层**：service gateway + 前端

**背景**：当前 API Key 的 model profile 用于绑定默认模型，但无法限制 Key 可访问的模型范围。

**需求描述**：
- 对每个 API Key 支持配置模型白名单（允许列表）。
- 未配置白名单时，默认允许所有模型（向后兼容）。
- 请求指定了白名单外的模型时，返回 `403 Forbidden`。
- API Key 编辑页增加模型白名单多选。

**技术要点**：
- `api_keys` 表新增 `allowed_models_json` 列（nullable TEXT，JSON 数组）。
- gateway 在 model_picker 阶段检查白名单。
- RPC 方法：`apikey/allowedModels/get`、`apikey/allowedModels/set`。

---

### F13 操作审计日志增强

**优先级**：P2 | **复杂度**：中 | **涉及层**：service + 前端

**背景**：当前有 lightweight audit，但缺少完整的操作追踪与查询能力。

**需求描述**：
- 记录所有管理操作（创建/删除/修改账号、API Key、设置变更等）。
- 审计日志字段：操作时间、操作类型、操作对象、操作者（API Key / Web session）、变更详情（before/after）。
- 新增「审计日志」页面，支持按时间、操作类型、对象筛选。
- 支持导出审计日志。

**技术要点**：
- 新增 `audit_logs` 表，替代或扩展现有 `events` 表。
- 在 RPC dispatch 层统一植入审计拦截器（middleware 模式）。
- RPC 方法：`audit/list`、`audit/export`。

---

### F14 Web UI 二步验证 (2FA)

**优先级**：P2 | **复杂度**：中 | **涉及层**：service web + 前端

**背景**：Web 模式暴露在网络上时，仅密码保护安全性不足。

**需求描述**：
- 支持 TOTP 二步验证（兼容 Google Authenticator / Authy）。
- 设置页可绑定/解绑 2FA，展示二维码供扫描。
- 启用 2FA 后，Web 登录需输入密码 + 验证码。
- 提供恢复码，用于 2FA 设备丢失时的紧急恢复。

**技术要点**：
- 使用 `totp-rs` crate 实现 TOTP 生成与验证。
- `app_settings` 中存储加密后的 TOTP secret。
- Web auth 流程扩展：密码验证通过后，若启用 2FA 则要求输入 TOTP。
- RPC 方法：`webAuth/2fa/setup`、`webAuth/2fa/verify`、`webAuth/2fa/disable`。

---

### F15 重试与降级策略配置

**优先级**：P2 | **复杂度**：中 | **涉及层**：service gateway + 前端

**背景**：当前 failover 逻辑硬编码在 gateway 中，缺少用户可配置的重试策略。

**需求描述**：
- 支持配置：
  - 最大重试次数（默认 3）。
  - 重试退避策略：立即重试 / 固定间隔 / 指数退避。
  - 可重试的错误类型（如 429、500、502、503）。
- 设置页「网关」区域增加重试策略配置。

**技术要点**：
- 将现有 failover 逻辑重构为可配置的 retry policy。
- 配置持久化到 `app_settings`。
- RPC 方法：`gateway/retryPolicy/get`、`gateway/retryPolicy/set`。

---

### F16 MCP Server 模式

**优先级**：P3 | **复杂度**：高 | **涉及层**：新增 crate 或 service 扩展

**背景**：MCP (Model Context Protocol) 正成为 AI 工具集成的标准协议，Claude Code / Cursor 等工具原生支持 MCP。

**需求描述**：
- 将 CodexManager 的网关能力暴露为 MCP Server。
- 支持的 MCP Tools：
  - `chat_completion`：发送聊天补全请求。
  - `list_models`：列出可用模型。
  - `list_accounts`：查看账号状态概览。
  - `get_usage`：查看用量信息。
- 支持 stdio 和 HTTP SSE 两种 MCP 传输方式。
- 可在设置页开关 MCP Server。

**技术要点**：
- 评估 `rmcp` 或手写 MCP 协议实现。
- MCP Server 复用 gateway 的路由和协议适配能力。
- 以独立 feature flag 编译，不影响现有二进制体积。

---

### F17 插件 / Hook 系统

**优先级**：P3 | **复杂度**：高 | **涉及层**：service gateway

**背景**：不同用户有定制化需求（内容过滤、自定义审计、请求改写），硬编码无法满足。

**需求描述**：
- 在请求生命周期中提供钩子点：
  - `pre_route`：路由前，可修改请求或拒绝。
  - `post_route`：路由后、发送前，可修改上游请求。
  - `post_response`：收到响应后，可修改或记录。
- 初期支持 Lua 脚本作为插件语言（轻量、沙箱安全）。
- 插件通过设置页上传和管理。

**技术要点**：
- 使用 `mlua` crate 嵌入 Lua 运行时。
- 每个钩子点传入请求/响应的 JSON 表示，脚本返回修改后的版本或 `nil` 表示不修改。
- 脚本执行超时保护（默认 100ms）。
- 插件存储在 `plugins/` 目录或数据库中。

---

## 三、非功能性要求

| 维度 | 要求 |
|------|------|
| 兼容性 | 所有新功能须兼容桌面端（Tauri）和 Web/Docker 两种运行模式 |
| 性能 | 网关链路新增逻辑（限流、缓存查找、策略计算）单次开销 < 1ms |
| 数据库 | 继续使用 SQLite，新增表须提供 migration 文件 |
| 前端 | 沿用现有技术栈（原生 JS + Vite），不引入新框架 |
| 配置 | 所有新功能须支持通过环境变量覆盖默认值 |
| 向后兼容 | 升级不丢失现有数据，新字段使用 nullable 或默认值 |

---

## 四、排期建议

| 阶段 | 功能 | 说明 |
|------|------|------|
| Phase 1 | F01、F02、F03 | 运维基础 + 核心路由增强 |
| Phase 2 | F04、F05、F07、F08 | 网关管控 + 费用可视 |
| Phase 3 | F06、F09、F10、F11 | 缓存 + 分析 + 巡检 |
| Phase 4 | F12、F13、F14、F15 | 安全与审计 |
| Phase 5 | F16、F17 | 生态集成（可选） |
