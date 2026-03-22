# CodexManager 功能增强 TODO

> 对应 PRD：[docs/PRD.md](PRD.md) | 验收标准：[docs/ACCEPTANCE.md](ACCEPTANCE.md)

状态说明：`[ ]` 未开始 | `[-]` 进行中 | `[x]` 已完成 | `[!]` 阻塞

---

## 当前基线（2026-03-22）

- [x] **项目结构识别**
  - 前端：`apps/`（Next.js 16 + Tauri v2）
  - 后端：`crates/service`
  - 共享类型 / 存储：`crates/core`
  - Web / 启动器：`crates/web`、`crates/start`
  - 数据库 schema / migration：`crates/core/src/storage/`、`crates/core/migrations/`

- [x] **当前进度判断**
  - 已完成：账号管理、平台 Key、日志、设置、请求链路、用量聚合、启动快照等基础能力
  - 已发现缺口：PRD 增强项大多未落地，其中 F01 是最适合先打通的 P0 首页主链路
  - 文档差异：`docs/ACCEPTANCE.md` 覆盖范围大于原 TODO，后续继续按验收项补齐

- [-] **本轮验证结果**
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 通过（F06 设置页缓存入口）
  - `pnpm run build:desktop` 通过（F06 设置页缓存入口）
  - `pnpm exec tsc --noEmit` 通过（先执行 build 生成 `.next/types` 后复跑）
  - `cargo check -p codexmanager-service` 通过（F06 响应缓存后端）
  - `cargo test -p codexmanager-service --lib gateway_cache_rpc_supports_get_set_stats_and_clear -- --nocapture --test-threads=1` 通过
  - `cargo test -p codexmanager-service --test gateway_logs gateway_response_cache_hits_second_non_stream_request -- --nocapture --test-threads=1` 通过
  - `pnpm exec tsc --noEmit` 通过（费用统计页图表）
  - `pnpm run build:desktop` 通过（费用统计页图表）
  - `pnpm run build:desktop` 通过
  - `pnpm lint` 无 error，剩余 4 条 warning（历史问题，未阻塞构建）
  - `pnpm exec tsc --noEmit` 通过
  - `cargo test -p codexmanager-core storage_api_keys_include_profile_fields -- --nocapture` 通过
  - `cargo test -p codexmanager-core storage_can_roundtrip_api_key_rate_limit_config -- --nocapture` 通过
  - `cargo test -p codexmanager-service load_active_api_key_rejects_expired_key_with_401_and_marks_status -- --nocapture` 通过
  - `cargo test -p codexmanager-service load_active_api_key_allows_unexpired_active_key -- --nocapture` 通过
  - `cargo test -p codexmanager-service apikey_rpc_supports_expires_at_and_renew -- --nocapture` 通过
  - `cargo test -p codexmanager-service apikey_rpc_supports_rate_limit_get_and_set -- --nocapture` 通过
  - `cargo test -p codexmanager-core storage_can_roundtrip_api_key_response_cache_config -- --nocapture` 通过
  - `cargo test -p codexmanager-service apikey_response_cache_rpc_supports_get_and_set -- --nocapture` 通过
  - `cargo test -p codexmanager-service --test gateway_logs cache:: -- --nocapture` 通过
  - `cargo test -p codexmanager-service build_response_cache_key_skips_stream_requests -- --nocapture` 通过
  - `cargo test -p codexmanager-service response_cache_entry_expires_after_ttl -- --nocapture` 通过
  - `cargo test -p codexmanager-service response_cache_evicts_oldest_entry_when_capacity_is_exceeded -- --nocapture` 通过
  - `cargo test -p codexmanager-service requestlog_export_rpc_returns_filtered_csv_content -- --nocapture` 通过
  - `cargo test -p codexmanager-service requestlog_export_rpc_supports_key_model_and_time_filters -- --nocapture` 通过
  - `cargo test -p codexmanager-service requestlog_list_and_summary_support_extended_filters -- --nocapture` 通过
  - `cargo test -p codexmanager-service export_response_sets_download_headers -- --nocapture` 通过
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 通过（API Key 缓存与日志导出）
  - `pnpm run build:desktop` 通过（API Key 缓存与日志导出）
  - `pnpm exec tsc --noEmit` 通过（API Key 缓存与日志导出）
  - `cargo check -p codexmanager-service` 通过（F10 HTTP 导出端点）
  - `cargo check -p codexmanager-web` 通过（F10 Web 导出代理）
  - `cargo build -p codexmanager-service -p codexmanager-web` 通过（本地二进制联调）
  - `pnpm run build` 通过（Web / Docker 静态产物）
  - `curl -I http://localhost:48861/` => `200 OK`
  - `curl -i http://localhost:48861/api/export/requestlogs?format=csv&statusFilter=all` => `200 OK`，返回 `content-disposition: attachment`
  - `curl -i http://localhost:48861/api/export/requestlogs?format=json&statusFilter=all` => `200 OK`，返回 `transfer-encoding: chunked`
  - `docker compose -f docker/docker-compose.yml up -d --build` 本轮失败：容器内 `static.crates.io` DNS 解析失败，非代码问题，待网络恢复后复跑
  - `cargo test -p codexmanager-service rate_limit_check_enforces_rpm_limit -- --nocapture` 通过
  - `cargo test -p codexmanager-service rate_limit_check_enforces_tpm_limit -- --nocapture` 通过
  - `cargo test -p codexmanager-service rate_limit_check_enforces_daily_limit -- --nocapture` 通过
  - `cargo test -p codexmanager-service with_retry_after_header_appends_retry_after_header -- --nocapture` 通过
  - `cargo test -p codexmanager-core request_logs -- --nocapture` 通过
  - `cargo test -p codexmanager-core request_logs_support_prefixed_query_filters -- --nocapture` 通过
  - `cargo test -p codexmanager-service rpc_requestlog_list_and_summary_support_pagination -- --nocapture` 通过
  - `cargo test -p codexmanager-service trend_builds_full_hour_and_groups_by_minute -- --nocapture` 通过
  - `cargo test -p codexmanager-service gateway_metrics_calculates_percentiles_and_success_rate -- --nocapture` 通过
  - `cargo test -p codexmanager-service weighted_rotation_prefers_higher_weight_candidate -- --nocapture` 通过
  - `cargo test -p codexmanager-service least_latency_prefers_account_with_lower_recent_latency -- --nocapture` 通过
  - `cargo test -p codexmanager-service plan_priority -- --nocapture` 通过
  - `cargo test -p codexmanager-service backend_send_failure_returns_502 -- --nocapture` 通过
  - `cargo test -p codexmanager-service local_backend_client_builds_without_system_proxy -- --nocapture` 通过
  - `cargo test -p codexmanager-service request_without_content_length_over_limit_returns_413 -- --nocapture` 通过
  - `cargo test -p codexmanager-service classify_register_failure_reason_detects_proxy_error -- --nocapture` 通过
  - `cargo test -p codexmanager-service classify_register_failure_reason_detects_phone_required -- --nocapture` 通过
  - `cargo test -p codexmanager-service usage_refresh_failure_throttle_splits_401_reason_classes -- --nocapture` 通过
  - `cargo test -p codexmanager-service failure_event_throttle_isolated_by_error_class -- --nocapture` 通过
  - `cargo test -p codexmanager-service refresh_token_auth_error_reason_from_message_tracks_canonical_messages -- --nocapture` 通过
  - `cargo test -p codexmanager-service --lib -- --nocapture` 通过（488 passed）
  - `cargo test -p codexmanager-service --test gateway_logs -- --nocapture` 通过（29 passed）
  - `cargo test -p codexmanager-service -- --nocapture` 整包通过

---

## Phase 1 — 运维基础 + 核心路由增强

### F01 实时健康仪表盘

- [x] **后端：健康快照聚合**
  - [x] 新增 RPC 方法 `dashboard/health`
  - [x] 按状态（online / cooldown / unavailable / disabled / quota_exhausted）聚合账号数量
  - [x] 计算滚动窗口（5 分钟）的 QPS、成功率、延迟 P50/P95/P99
  - [ ] 内存中维护 ring buffer 存储最近请求的延迟采样
  - 完成标准：首页可稳定消费，空数据不报错

- [x] **后端：趋势数据接口**
  - [x] 新增 RPC 方法 `dashboard/trend`
  - [x] 返回最近 1 小时的分钟级请求量与错误率序列
  - 完成标准：返回完整分钟桶，支持无请求补零

- [x] **前端：健康总览卡片**
  - [x] 仪表盘顶部增加账号状态分布卡片（数量 + 百分比）
  - [x] 增加网关实时指标卡片（QPS / 成功率 / P95 延迟）
  - [x] 接入真实接口并开启 30 秒刷新
  - 完成标准：首页可见实时健康分布与网关指标

- [x] **前端：mini 趋势图**
  - [x] 使用轻量 SVG 趋势图渲染请求量 + 错误率折线图
  - [x] 自动刷新（30 秒间隔）
  - [ ] 评估是否替换为独立图表库（uPlot）以复用到 F08/F09
  - 完成标准：最近 1 小时趋势可见，无数据空态正常

- [-] **验证与环境收口**
  - [x] 前端构建通过
  - [x] 前端 lint 无 error
  - [!] Rust 编译与测试待工具链恢复后补跑

---

### F02 告警通知系统

- [ ] **数据库**
  - [ ] 新增 migration：`alert_rules` 表（id, name, type, config_json, enabled, created_at）
  - [ ] 新增 migration：`alert_channels` 表（id, name, type, config_json, enabled, created_at）
  - [ ] 新增 migration：`alert_history` 表（id, rule_id, channel_id, status, message, created_at）

- [ ] **后端：规则引擎**
  - [ ] 新增 `alert/` 模块目录结构
  - [ ] 实现规则类型枚举：token_refresh_fail / usage_threshold / error_rate / all_unavailable
  - [ ] 实现定时轮询检查器（复用 tokio interval）
  - [ ] 实现告警去重 + 静默期逻辑

- [ ] **后端：通知渠道**
  - [ ] 定义 `AlertSender` trait
  - [ ] 实现 Webhook 渠道（POST JSON）
  - [ ] 实现 Bark 渠道
  - [ ] 实现 Telegram Bot 渠道
  - [ ] 实现企业微信机器人渠道

- [ ] **后端：RPC 接口**
  - [ ] `alert/rules/list`、`alert/rules/upsert`、`alert/rules/delete`
  - [ ] `alert/channels/list`、`alert/channels/upsert`、`alert/channels/delete`、`alert/channels/test`
  - [ ] `alert/history/list`

- [ ] **前端：告警设置页**
  - [ ] 设置页新增「告警通知」Tab
  - [ ] 规则管理 CRUD 界面
  - [ ] 渠道管理 CRUD 界面 + 测试发送按钮
  - [ ] 告警历史列表

---

### F03 智能路由策略增强

- [-] **后端：策略抽象**
  - [ ] 将现有路由逻辑重构为 `RouteStrategy` trait
  - [x] 扩展现有策略解析与运行时切换，保留 `ordered` 和 `balanced`
  - 完成标准：设置页可切换并持久化，gateway 可识别新策略

- [-] **后端：加权轮询策略**
  - [x] 基于 `usage_snapshots.used_percent` 计算剩余额度权重
  - [x] 实现按 key/model 维度推进 ticket 的加权轮转
  - [x] 补跑 Rust 测试验证命中分布

- [-] **后端：最低延迟优先策略**
  - [x] 在内存中维护每账号 EMA 延迟统计
  - [x] 实现按最近延迟排序选择
  - [x] 补跑 Rust 测试验证排序生效

- [-] **后端：成本优先策略**
  - [x] 读取账号 plan 类型，按 free > plus > team/pro 排序
  - [x] 候选池为空或 free 不可路由时自动落到下一优先级
  - [x] 增加更细的 plan 类型兼容测试

- [x] **前端：策略选择**
  - [x] 设置页网关区域扩展策略下拉，增加 weighted / least-latency / cost-first
  - [x] 各策略增加简要说明文案
  - 完成标准：设置页可见新策略，前端构建通过

- [ ] **日志增强**
  - [x] `request_logs` 中记录实际使用的路由策略名称
  - [x] 日志列表 / 搜索 / 前端详情展示 `routeStrategy`

---

## Phase 2 — 网关管控 + 费用可视

### F04 请求限流

- [x] **数据库**
  - [x] 新增 migration：`api_key_rate_limits` 表（key_id, rpm, tpm, daily_limit）

- [x] **后端：限流引擎**
  - [x] 实现令牌桶算法（per API Key，内存维护）
  - [x] RPM 桶 + TPM 桶 + 日计数器
  - [x] 超限返回 429 + `Retry-After` 头

- [x] **后端：RPC 接口**
  - [x] `apikey/rateLimit/get`、`apikey/rateLimit/set`

- [x] **网关集成**
  - [x] 在 gateway auth 层之后、route 之前插入限流检查

- [x] **前端**
  - [x] API Key 编辑弹窗增加限流配置字段（RPM / TPM / 日上限）

---

### F05 模型降级链

- [x] **数据库**
  - [x] 新增 migration：`api_key_model_fallbacks` 表（key_id, model_chain_json）

- [x] **后端：降级逻辑**
  - [x] 扩展 gateway 上游执行链，支持按降级链顺序尝试
  - [x] 降级时设置响应头 `X-CodexManager-Actual-Model`
  - [x] 已验证：主模型可用时保持首选模型
  - [x] 已验证：主模型失败后自动尝试下一个模型
  - [x] 已验证：支持多级降级直到最终可用模型
  - [x] 已验证：未配置降级链时保持原有 failover 行为

- [x] **后端：RPC 接口**
  - [x] `apikey/modelFallback/get`、`apikey/modelFallback/set`

- [x] **前端**
  - [x] API Key 编辑页增加模型降级链配置（顺序多行输入）

- [x] **日志增强**
  - [x] `request_logs` 记录请求模型、实际模型与降级路径
  - [x] 日志页展示请求模型、实际模型与降级路径

- [x] **验证**
  - [x] `cargo test -p codexmanager-service --test gateway_logs fallback:: -- --nocapture`
  - [x] `cargo test -p codexmanager-service status_500_with_more_candidates_triggers_failover -- --nocapture`
  - [x] `pnpm exec tsc --noEmit`
  - [x] `pnpm run build:desktop`

---

### F07 API Key 过期与临时分享

- [x] **数据库**
  - [x] `api_keys` 表新增 `expires_at` 列（nullable DATETIME）

- [x] **后端**
  - [x] gateway auth 鉴权时检查 `expires_at`
  - [x] 过期返回 `401` + 明确错误信息
  - [x] `apikey/create` RPC 增加 `expires_at` 可选参数
  - [x] 新增 `apikey/renew` RPC 方法

- [x] **前端**
  - [x] 创建 API Key 弹窗增加过期时间选择器（可选）
  - [x] API Key 列表展示过期时间 / 倒计时
  - [x] 增加「续期」操作按钮

---

### F08 费用统计与报表

- [x] **数据库**
  - [x] 新增 migration：`model_pricing` 表（model_slug, input_price_per_1k, output_price_per_1k, updated_at）

- [-] **后端：聚合查询**
  - [x] 按 API Key / 模型 / 日期多维聚合 token 消耗与费用
  - [x] 支持时间范围过滤（today / week / month / custom）

- [-] **后端：RPC 接口**
  - [x] `stats/cost/summary`
  - [x] `stats/cost/export`（返回 CSV 内容）
  - [x] `stats/cost/modelPricing/get`、`stats/cost/modelPricing/set`

- [ ] **前端：费用统计页**
  - [x] 新增导航入口「费用统计」
  - [x] 页面可访问并支持模型单价配置读写
  - [x] 时间范围选择器（今日 / 本周 / 本月 / 自定义）
  - [x] 每日费用柱状图
  - [x] 模型分布饼图
  - [x] 汇总表格
  - [x] CSV 导出按钮

- [x] **阶段验证**
  - [x] `cargo test -p codexmanager-core storage_can_roundtrip_model_pricing_config -- --nocapture`
  - [x] `cargo test -p codexmanager-core model_pricing_item_serialization_uses_camel_case -- --nocapture`
  - [x] `cargo test -p codexmanager-service stats_cost_model_pricing_rpc_supports_get_and_set -- --nocapture`
  - [x] `cargo test -p codexmanager-core storage_can_summarize_cost_usage_by_key_model_and_day -- --nocapture`
  - [x] `cargo test -p codexmanager-core cost_summary_params_serialization_uses_camel_case -- --nocapture`
  - [x] `cargo test -p codexmanager-service stats_cost_summary_rpc_aggregates_custom_range -- --nocapture`
  - [x] `cargo test -p codexmanager-core cost_export_result_serialization_uses_camel_case -- --nocapture`
  - [x] `cargo test -p codexmanager-service stats_cost_export_rpc_returns_csv_content -- --nocapture`
  - [x] `pnpm exec tsc --noEmit`
  - [x] `pnpm run build:desktop`
  - [x] `cargo check --manifest-path apps/src-tauri/Cargo.toml`

---

## Phase 3 — 缓存 + 分析 + 巡检

### F06 响应缓存

- [-] **后端：缓存层**
  - [x] 实现内存 LRU 风格响应缓存
  - [x] 缓存 key = 路径 + 请求体规范化后的 SHA256
  - [x] 支持配置 TTL 和最大条目数

- [-] **后端：网关集成**
  - [x] 非流式请求在路由前查缓存
  - [x] 缓存命中时直接返回，标注 `X-CodexManager-Cache: HIT`
  - [x] 缓存未命中且请求成功后写入缓存
  - [x] 首次非流式响应标注 `X-CodexManager-Cache: MISS`
  - [x] 流式请求不缓存回归用例
  - [x] TTL 过期与容量淘汰回归用例

- [-] **后端：RPC 接口**
  - [x] `gateway/cache/config/get`、`gateway/cache/config/set`
  - [x] `gateway/cache/stats`（命中率、条目数、内存占用估算）
  - [x] `gateway/cache/clear`

- [x] **前端**
  - [x] 设置页网关区域增加缓存开关、TTL、最大条目数配置
  - [x] 展示缓存命中率统计
  - [x] 清空缓存操作入口

- [x] **API Key 维度控制**
  - [x] 新增 API Key 级别缓存开关配置
  - [x] 网关请求按 Key 判断是否允许命中 / 写入缓存
  - [x] API Key 编辑弹窗增加缓存开关入口
  - [x] 联调验证：已开启 Key 二次命中缓存，未开启 Key 持续绕过缓存

---

### F09 用量趋势分析

- [ ] **后端：聚合查询**
  - [ ] `stats/trends/requests`：按天/周/月的请求量 + 成功率
  - [ ] `stats/trends/models`：模型使用分布
  - [ ] `stats/trends/heatmap`：按 hour x weekday 的请求热力图

- [ ] **前端：分析视图**
  - [ ] 新增「用量分析」页面或仪表盘 Tab
  - [ ] 请求量趋势折线图
  - [ ] 模型分布饼图
  - [ ] 请求热力图（7x24 网格）

---

### F10 请求日志导出

- [-] **后端**
  - [x] 新增 HTTP 端点 `GET /export/requestlogs`
  - [x] 先补 RPC 导出链路，支持 `format` 参数（csv / json）
  - [x] 支持当前日志页筛选参数（query / statusFilter）
  - [x] 导出接口额外支持 `timeFrom / timeTo / model / keyId` 筛选
  - [x] HTTP 导出改为分批流式响应，避免大数据量整块占用内存

- [-] **前端**
  - [x] 请求日志页增加「导出」按钮
  - [x] 格式选择下拉（CSV / JSON）
  - [x] 导出使用当前页面的筛选条件
  - [x] Web / Docker 版优先走 `/api/export/requestlogs` 直接下载
  - [x] 日志页补充 `keyId / model / timeFrom / timeTo` 筛选并与列表、摘要、导出联动

---

### F11 账号自动巡检

- [ ] **后端：巡检调度**
  - [ ] 新增定时任务，可配置间隔（默认 30 分钟）
  - [ ] 复用 session_probe 逻辑对启用账号执行探测
  - [ ] 并发控制（最多 N 个并发探测）
  - [ ] 失败账号自动标记 unavailable + 写入 event
  - [ ] 恢复账号自动标记 enabled

- [ ] **后端：RPC 接口**
  - [ ] `healthcheck/config/get`、`healthcheck/config/set`
  - [ ] `healthcheck/run`（手动触发）

- [ ] **前端**
  - [ ] 设置页增加巡检开关 + 间隔配置
  - [ ] 仪表盘展示最近巡检时间和结果概要

- [ ] **集成**
  - [ ] 巡检异常结果接入告警通知系统（F02）

---

## Phase 4 — 安全与审计

### F12 API Key 模型访问控制

- [ ] **数据库**
  - [ ] `api_keys` 表新增 `allowed_models_json` 列（nullable TEXT）

- [ ] **后端**
  - [ ] gateway model_picker 阶段检查白名单
  - [ ] 白名单外模型返回 403
  - [ ] `apikey/allowedModels/get`、`apikey/allowedModels/set`

- [ ] **前端**
  - [ ] API Key 编辑页增加模型白名单多选组件

---

### F13 操作审计日志增强

- [ ] **数据库**
  - [ ] 新增 migration：`audit_logs` 表（id, action, object_type, object_id, operator, changes_json, created_at）

- [ ] **后端**
  - [ ] RPC dispatch 层植入审计拦截中间件
  - [ ] 记录所有写操作的 before / after 变更
  - [ ] `audit/list`、`audit/export`

- [ ] **前端**
  - [ ] 新增「审计日志」页面
  - [ ] 按操作类型、对象、时间筛选
  - [ ] 导出功能

---

### F14 Web UI 二步验证 (2FA)

- [ ] **后端**
  - [ ] 引入 `totp-rs` crate
  - [ ] 实现 TOTP secret 生成、二维码 URL 生成、验证码校验
  - [ ] `app_settings` 存储加密 TOTP secret
  - [ ] 生成一次性恢复码
  - [ ] Web auth 流程扩展：密码 -> 2FA 验证

- [ ] **后端：RPC 接口**
  - [ ] `webAuth/2fa/setup`（返回 secret + QR URL + 恢复码）
  - [ ] `webAuth/2fa/verify`（验证码校验）
  - [ ] `webAuth/2fa/disable`

- [ ] **前端**
  - [ ] 设置页安全区域增加 2FA 绑定/解绑入口
  - [ ] 展示 QR 码供扫描
  - [ ] Web 登录页增加验证码输入步骤

---

### F15 重试与降级策略配置

- [ ] **后端**
  - [ ] 将现有 failover 硬编码重构为可配置 retry policy
  - [ ] 支持参数：max_retries / backoff_strategy / retryable_status_codes
  - [ ] `gateway/retryPolicy/get`、`gateway/retryPolicy/set`

- [ ] **前端**
  - [ ] 设置页网关区域增加重试策略配置
  - [ ] 最大重试次数、退避策略下拉、可重试状态码多选

---

## Phase 5 — 生态集成（可选）

### F16 MCP Server 模式

- [ ] **后端**
  - [ ] 评估 MCP 协议实现方案（rmcp vs 手写）
  - [ ] 实现 MCP Tools：chat_completion / list_models / list_accounts / get_usage
  - [ ] 支持 stdio + HTTP SSE 传输
  - [ ] 以 feature flag 编译

- [ ] **前端**
  - [ ] 设置页增加 MCP Server 开关与端口配置

- [ ] **文档**
  - [ ] 编写 MCP 接入指南（Claude Code / Cursor 配置示例）

---

### F17 插件 / Hook 系统

- [ ] **后端**
  - [ ] 引入 `mlua` crate
  - [ ] 定义钩子点：pre_route / post_route / post_response
  - [ ] 实现 Lua 脚本加载、沙箱执行、超时保护
  - [ ] 插件管理 CRUD

- [ ] **前端**
  - [ ] 设置页增加插件管理区域
  - [ ] 插件上传、启用/禁用、编辑、删除
  - [ ] 内置插件模板

- [ ] **文档**
  - [ ] 编写插件开发指南与 API 参考
