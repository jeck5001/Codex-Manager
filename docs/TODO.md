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
  - `cargo check -p codexmanager-service` 通过（F15 重试策略配置）
  - `cargo test -p codexmanager-service gateway_retry_policy_rpc_supports_get_set_and_snapshot -- --nocapture` 通过
  - `cargo test -p codexmanager-service status_429_respects_retry_policy_status_list -- --nocapture` 通过
  - `cargo test -p codexmanager-core storage -- --nocapture` 通过（SQLite WAL 降级兼容 bind mount）
  - `cargo check -p codexmanager-service` 通过（F14 Web 2FA 服务端）
  - `cargo check -p codexmanager-web` 通过（F14 Web 登录两段式验证）
  - `cargo test -p codexmanager-service web_auth_two_factor_rpc_supports_setup_verify_recovery_and_disable -- --nocapture` 通过
  - `cargo test -p codexmanager-service clearing_web_access_password_also_clears_two_factor_state -- --nocapture` 通过
  - `cargo test -p codexmanager-web -- --nocapture` 通过（5 passed）
  - `cargo test -p codexmanager-web login_submit_ -- --nocapture` 通过（覆盖密码后进入 2FA 页面、pending cookie 换正式 cookie）
  - `cargo test -p codexmanager-web login_submit_ -- --nocapture` 通过（新增 recovery code 登录并扣减剩余次数校验）
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 通过（F15 Tauri 命令接入）
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 通过（F14 桌面端 2FA 入口；并补齐 `apps/src-tauri` 的 `vendor-rust/` 离线依赖）
  - `pnpm exec tsc --noEmit` 通过（F15 设置页重试策略配置）
  - `pnpm exec tsc --noEmit` 通过（F14 桌面端 2FA 类型与 modal）
  - `pnpm run build:desktop` 通过（F15 设置页静态构建）
  - `pnpm run build:desktop` 通过（F14 桌面端 2FA 入口）
  - `docker compose -f docker/docker-compose.localbuild.yml up -d --build` 通过（基于本地源码 + `vendor-rust/` 离线构建 `codexmanager-*-local` 镜像）
  - `docker compose -f docker/docker-compose.localbuild.yml ps` 通过（`codexmanager-service` / `codexmanager-web` healthy）
  - `curl -i --max-time 10 http://127.0.0.1:48760/health` => `200 OK`
  - `curl -i --max-time 10 http://127.0.0.1:48761/__auth_status` => `200 OK`
  - `curl -s -X POST http://127.0.0.1:48761/api/rpc ... method=audit/list` => 返回分页结果
  - `curl -s -X POST http://127.0.0.1:48761/api/rpc ... method=gateway/retryPolicy/get` => 返回默认策略
  - `curl -s -X POST http://127.0.0.1:48761/api/rpc ... method=gateway/retryPolicy/set` => 成功写入 `maxRetries=5 / backoffStrategy=fixed / retryableStatusCodes=[429,502]`
  - `curl -s -X POST http://127.0.0.1:48761/api/rpc ... method=appSettings/get` => 返回 `retryPolicyMaxRetries / retryPolicyBackoffStrategy / retryPolicyRetryableStatusCodes`
  - `curl -s -X POST http://127.0.0.1:48761/api/rpc ... method=audit/list params={page:1,pageSize:10}` => 可查询到 `gateway/retryPolicy/set` 审计记录
  - `curl -s -X POST http://127.0.0.1:48761/api/rpc ... method=healthcheck/config/set` 后再次查询 `audit/list` => 成功写入 `operator=web-ui` 审计记录
  - 本轮尝试重新执行 Docker 本地源码复验时，`docker info` / `docker compose` 在本机 `orbstack` context 下无输出卡住；属于本地 Docker daemon/CLI 环境阻塞，待守护恢复后复跑
  - `cargo test -p codexmanager-service healthcheck_config_rpc_supports_get_and_set -- --nocapture` 通过
  - `cargo test -p codexmanager-service healthcheck_run_rpc_returns_empty_summary_without_probe_candidates -- --nocapture` 通过
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 通过（F11 巡检 RPC / Tauri 接入）
  - `pnpm exec tsc --noEmit` 通过（F11 设置页 / 仪表盘展示）
  - `pnpm run build:desktop` 通过（F11 设置页 / 仪表盘展示）
  - `pnpm run build` 通过（F11 Web 静态产物）
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 通过（F06 设置页缓存入口）
  - `pnpm run build:desktop` 通过（F06 设置页缓存入口）
  - `pnpm exec tsc --noEmit` 通过（先执行 build 生成 `.next/types` 后复跑）
  - `cargo check -p codexmanager-service` 通过（F06 响应缓存后端）
  - `cargo test -p codexmanager-service --lib gateway_cache_rpc_supports_get_set_stats_and_clear -- --nocapture --test-threads=1` 通过
  - `cargo test -p codexmanager-service --test gateway_logs gateway_response_cache_hits_second_non_stream_request -- --nocapture --test-threads=1` 通过
  - `pnpm exec tsc --noEmit` 通过（费用统计页图表）
  - `pnpm run build:desktop` 通过（费用统计页图表）
  - `pnpm run build:desktop` 通过
  - `pnpm lint` 通过（前端 warning 已清零）
  - `pnpm exec tsc --noEmit` 通过（费用统计页草稿态 / 告警设置草稿同步改造）
  - `pnpm run build:desktop` 通过（费用统计页草稿态 / 告警设置草稿同步改造）
  - `pnpm exec tsc --noEmit` 通过（register / add-account / web-password lint 收口）
  - `pnpm run build:desktop` 通过（register / add-account / web-password lint 收口）
  - `cargo test -p codexmanager-web login_submit_ -- --nocapture` 通过（新增错误 2FA 验证码失败路径）
  - `cargo test -p codexmanager-web -- --nocapture` 通过（9 passed）
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
  - `cargo test -p codexmanager-service healthcheck_run_triggers_token_refresh_fail_alerts_when_probe_fails -- --nocapture` 通过
  - `cargo test -p codexmanager-service -- --nocapture` 再次整包通过（528 passed）
  - `cargo test -p codexmanager-core storage_can_summarize_request_trends_models_and_heatmap -- --nocapture` 通过
  - `cargo test -p codexmanager-service stats_trends_rpc_returns_requests_models_and_heatmap -- --nocapture` 通过
  - `cargo check -p codexmanager-service` 通过（F09 趋势聚合 / RPC）
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 通过（F09 Tauri 命令接入）
  - `pnpm exec tsc --noEmit` 通过（F09 用量分析页）
  - `pnpm run build:desktop` 通过（新增 `/analytics` 静态页面）
  - `cargo test -p codexmanager-service -- --nocapture` 再次整包通过（529 passed）

---

## Phase 1 — 运维基础 + 核心路由增强

### F01 实时健康仪表盘

- [x] **后端：健康快照聚合**
  - [x] 新增 RPC 方法 `dashboard/health`
  - [x] 按状态（online / cooldown / unavailable / disabled / quota_exhausted）聚合账号数量
  - [x] 计算滚动窗口（5 分钟）的 QPS、成功率、延迟 P50/P95/P99
  - [x] 内存中维护 ring buffer 存储最近请求的延迟采样
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
  - [x] 已评估：暂不替换为 `uPlot`，继续复用现有轻量 SVG 图表以降低桌面静态导出与样式维护成本
  - 完成标准：最近 1 小时趋势可见，无数据空态正常

- [x] **验证与环境收口**
  - [x] 前端构建通过
  - [x] 前端 lint 无 error
  - [x] `cargo test -p codexmanager-service -- --nocapture` 整包通过

- [x] **前端 lint warning 收口**
  - [x] `apps/src/app/register/page.tsx`：稳定 `latestTasks` 依赖，消除 `react-hooks/exhaustive-deps`
  - [x] `apps/src/components/modals/add-account-modal.tsx`：收口 `completeLoginSuccess` / `invalidateLoginQueries` 的 hook 依赖 warning
  - [x] `apps/src/components/modals/web-password-modal.tsx`：2FA 二维码切换为 `next/image`
  - [x] `apps/src/hooks/useAccounts.ts`：清理未使用的 `scopeLabel`

---

### F02 告警通知系统

- [x] **数据库**
  - [x] 新增 migration：`alert_rules` 表（id, name, type, config_json, enabled, created_at）
  - [x] 新增 migration：`alert_channels` 表（id, name, type, config_json, enabled, created_at）
  - [x] 新增 migration：`alert_history` 表（id, rule_id, channel_id, status, message, created_at）
  - [x] 新增 storage CRUD：规则 / 渠道 / 历史记录
  - 完成标准：数据库初始化后可持久化规则、渠道、历史记录

- [x] **后端：规则引擎**
  - [x] 新增 `alert/` 模块目录结构
  - [x] 实现规则类型枚举：token_refresh_fail / usage_threshold / error_rate / all_unavailable
  - [x] 实现定时轮询检查器（后台线程轮询）
  - [x] 实现告警去重 + 静默期逻辑
  - [x] 实现恢复通知与运行态持久化

- [x] **后端：通知渠道**
  - [x] 定义 `AlertSender` trait
  - [x] 实现 Webhook 渠道（POST JSON）
  - [x] 实现 Bark 渠道
  - [x] 实现 Telegram Bot 渠道
  - [x] 实现企业微信机器人渠道
  - [x] 将通知渠道接入实际规则触发链路
  - [x] `cargo test -p codexmanager-service alert_engine_usage_threshold_dedupes_and_recovers -- --nocapture`
  - [x] `cargo test -p codexmanager-service alert_engine_all_unavailable_triggers_and_recovers -- --nocapture`

- [x] **后端：RPC 接口**
  - [x] `alert/rules/list`、`alert/rules/upsert`、`alert/rules/delete`
  - [x] `alert/channels/list`、`alert/channels/upsert`、`alert/channels/delete`、`alert/channels/test`
  - [x] `alert/history/list`
  - [x] `cargo test -p codexmanager-service alert_rpc_supports_rule_channel_history_and_channel_test -- --nocapture`

- [x] **前端：告警设置页**
  - [x] 设置页新增「告警通知」Tab
  - [x] 规则管理 CRUD 界面
  - [x] 渠道管理 CRUD 界面 + 测试发送按钮
  - [x] 告警历史列表
  - [x] `cd apps && pnpm exec tsc --noEmit`
  - [x] `cd apps && pnpm run build:desktop`
  - [x] `source "$HOME/.cargo/env" && cargo check --manifest-path apps/src-tauri/Cargo.toml`

- [x] **联调与回归**
  - [x] 设置页真实调用 `alert/*` RPC，桌面命令已注册
  - [x] `source "$HOME/.cargo/env" && cargo test -p codexmanager-service -- --nocapture`

---

### F03 智能路由策略增强

- [x] **后端：策略抽象**
  - [x] 将现有路由逻辑重构为 `RouteStrategy` trait
  - [x] 扩展现有策略解析与运行时切换，保留 `ordered` 和 `balanced`
  - 完成标准：设置页可切换并持久化，gateway 可识别新策略

- [x] **后端：加权轮询策略**
  - [x] 基于 `usage_snapshots.used_percent` 计算剩余额度权重
  - [x] 实现按 key/model 维度推进 ticket 的加权轮转
  - [x] 补跑 Rust 测试验证命中分布

- [x] **后端：最低延迟优先策略**
  - [x] 在内存中维护每账号 EMA 延迟统计
  - [x] 实现按最近延迟排序选择
  - [x] 补跑 Rust 测试验证排序生效

- [x] **后端：成本优先策略**
  - [x] 读取账号 plan 类型，按 free > plus > team/pro 排序
  - [x] 候选池为空或 free 不可路由时自动落到下一优先级
  - [x] 增加更细的 plan 类型兼容测试

- [x] **前端：策略选择**
  - [x] 设置页网关区域扩展策略下拉，增加 weighted / least-latency / cost-first
  - [x] 各策略增加简要说明文案
  - 完成标准：设置页可见新策略，前端构建通过

- [x] **日志增强**
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

- [x] **前端：费用统计页**
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

- [x] **后端：聚合查询**
  - [x] `stats/trends/requests`：按天/周/月的请求量 + 成功率
  - [x] `stats/trends/models`：模型使用分布
  - [x] `stats/trends/heatmap`：按 hour x weekday 的请求热力图

- [x] **前端：分析视图**
  - [x] 新增「用量分析」页面
  - [x] 请求量趋势折线图
  - [x] 模型分布面板
  - [x] 请求热力图（7x24 网格）
  - [x] 导航、Header 与桌面静态预热路由已接入 `/analytics`

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

- [-] **后端：巡检调度**
  - [x] 复用 `session_probe` 逻辑对启用账号执行探测，并保留轮询线程配置
  - [x] 失败账号自动标记 `unavailable` + 写入现有失败 event
  - [x] 巡检成功后自动恢复账号为 `active` 并清理 cooldown
  - [x] 内存维护最近一次巡检摘要（开始 / 结束时间、采样数、成功数、失败数、失败账号）
  - [x] 新增独立并发控制（最多 N 个并发探测）
  - [x] 评估默认巡检间隔是否调整到 PRD 约定的 30 分钟

- [x] **后端：RPC 接口**
  - [x] `healthcheck/config/get`、`healthcheck/config/set`
  - [x] `healthcheck/run`（手动触发）

- [-] **前端**
  - [x] 设置页复用现有巡检配置区，支持开关 / 间隔 / 抽样数配置
  - [x] 设置页增加「立即巡检」按钮与最近巡检结果摘要
  - [x] 仪表盘展示最近巡检时间、抽检通过率与采样结果概览

- [x] **集成**
  - [x] 巡检异常结果接入告警通知系统（F02）
  - [x] 新增回归测试：`healthcheck/run` 失败后触发 `token_refresh_fail` 告警 webhook 与历史记录

---

## Phase 4 — 安全与审计

### F12 API Key 模型访问控制

- [x] **数据库**
  - [x] `api_keys` 表新增 `allowed_models_json` 列（nullable TEXT）

- [x] **后端**
  - [x] gateway model_picker 阶段检查白名单
  - [x] 白名单外模型返回 403
  - [x] `apikey/allowedModels/get`、`apikey/allowedModels/set`

- [x] **前端**
  - [x] API Key 编辑页增加模型白名单多选组件

---

### F13 操作审计日志增强

- [x] **数据库**
  - [x] 新增 migration：`audit_logs` 表（id, action, object_type, object_id, operator, changes_json, created_at）

- [x] **后端**
  - [x] RPC dispatch 层植入审计拦截中间件
  - [x] 记录所有写操作的 before / after 变更
  - [x] `audit/list`、`audit/export`

- [x] **前端**
  - [x] 新增「审计日志」页面
  - [x] 按操作类型、对象、时间筛选
  - [x] 导出功能

- [x] **联调与交付**
  - [x] Web 代理转发 `X-CodexManager-Operator`
  - [x] 本地源码 Docker 镜像构建成功并通过健康检查
  - [x] Web RPC 写操作可落审计日志并被 `audit/list` 查询到

---

### F14 Web UI 二步验证 (2FA)

- [x] **后端**
  - [x] 引入 `totp-rs` crate
  - [x] 实现 TOTP secret 生成、二维码 URL 生成、验证码校验
  - [x] `app_settings` 存储加密 TOTP secret
  - [x] 生成一次性恢复码
  - [x] Web auth 流程扩展：密码 -> 2FA 验证

- [x] **后端：RPC 接口**
  - [x] `webAuth/2fa/setup`（返回 secret + QR URL + 恢复码）
  - [x] `webAuth/2fa/verify`（验证码校验）
  - [x] `webAuth/2fa/disable`

- [x] **前端**
  - [x] 设置页安全区域增加 2FA 绑定/解绑入口
  - [x] 展示 QR 码供扫描
  - [x] Web 登录页增加验证码输入步骤

- [-] **联调与验证**
  - [x] 服务端 / Web 登录页 / 桌面端设置入口编译通过
  - [x] 回归测试覆盖 setup / verify / recovery code / disable / clear password 清空 2FA
  - [x] Web handler 测试覆盖密码后进入 2FA 页面与 pending cookie -> 正式登录 cookie 交换
  - [x] Web handler 测试覆盖 recovery code 登录成功并消耗剩余次数
  - [x] Web handler 测试覆盖错误 2FA 验证码返回失败且保留 pending cookie
  - [x] `docs/API.md` 已补齐 Web auth/2FA、重试策略、巡检、导出与审计接口说明，并挂到 README / docs 索引
  - [!] Docker 本地源码镜像复验本轮受本机 `orbstack` daemon 卡住影响，待环境恢复后复跑

---

### F15 重试与降级策略配置

- [x] **后端**
  - [x] 将现有 failover 硬编码重构为可配置 retry policy
  - [x] 支持参数：max_retries / backoff_strategy / retryable_status_codes
  - [x] `gateway/retryPolicy/get`、`gateway/retryPolicy/set`

- [x] **前端**
  - [x] 设置页网关区域增加重试策略配置
  - [x] 最大重试次数、退避策略下拉、可重试状态码多选

- [x] **联调与 Docker**
  - [x] 本地源码 Docker 改为使用 `vendor-rust/` 离线编译，避免容器内 crates DNS 波动阻塞交付
  - [x] Web RPC `gateway/retryPolicy/get`、`gateway/retryPolicy/set` 联调通过
  - [x] `appSettings/get` 返回已持久化的 `retryPolicy*` 设置快照
  - [x] `audit/list` 可读取 `gateway/retryPolicy/set` 审计记录

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
