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
  - 文档导航：根 `README.md` 已收口为项目首页，深度说明继续以 `docs/README.md` 与 `CHANGELOG.md` 为准
  - 本轮补齐账号治理小收口：`workspace_deactivated` 已纳入封禁识别，账号页支持“一键清理封禁账号”，5 小时 / 7 天额度列补充重置时间展示

- [x] **通用验收缺口**
  - G7 `cargo clippy`（`codexmanager-service`）已收口：`cargo clippy -p codexmanager-service --tests -- -D warnings` 当前通过，本轮继续清空 `account/account_register.rs`、`app_settings/api/current.rs` 与测试层历史 warning
  - 本轮已先收敛 4 条低风险 warning：`app_settings/api/patch.rs` 的 `let_unit_value`、`auth/web_access_2fa.rs` 的 `manual_is_multiple_of`、`requestlog/requestlog_export.rs` 的 `vec_init_then_push`、`gateway/model_picker/request.rs` 的 `unnecessary_lazy_evaluations`
  - 本轮继续收敛结构类 warning：`gateway/observability/http_bridge/aggregate/sse_frame.rs` 与 `gateway/observability/metrics.rs` 的 `items_after_test_module` 已清零
  - 本轮收敛 `gateway/observability/http_bridge/delivery.rs`：以 `RespondWithUpstreamArgs` / `CompactDebugMeta` 合并桥接参数，并修正 SSE 适配分支布尔表达式；整包 `clippy` 错误数由 23 降到 18
  - 本轮收敛 `gateway/upstream/proxy_pipeline/request_setup.rs`：以 `PrepareRequestSetupInput` 收束散参数并改为切片输入；`clippy` 剩余错误数由 18 降到 16
  - 本轮收敛 `gateway/request/local_models.rs` 与 `gateway/request/local_count_tokens.rs`：以 `LocalModelsRequestContext` / `LocalCountTokensRequestContext` 收束本地短路响应参数；`clippy` 剩余错误数由 16 降到 14
  - 本轮收敛 `gateway/observability/request_log.rs` 与 `gateway/observability/trace_log.rs`：以 `RequestLogEntry` / `RequestLogRouteMeta` / `RequestStartLog` 合并观测埋点参数；`clippy` 剩余错误数由 14 降到 12，`gateway/observability` 不再出现在剩余清单中
  - 本轮收敛 `usage/usage_scheduler.rs` 与 `usage/usage_refresh.rs`：以 `BlockingPollLoopConfig` 收束轮询调度参数，并将 `refresh` 内部实现模块改名为 `refresh_impl`；`clippy` 剩余错误数由 12 降到 10，`usage` 不再出现在剩余清单中
  - 本轮收敛 `gateway/upstream/attempt_flow/*` 与 `gateway/upstream/proxy_pipeline/*`：以 `SendUpstreamRequestArgs` / `OpenAiBaseAttemptArgs` / `FinalResultLogArgs` / `TerminalCandidateArgs` / `FinalizeUpstreamResponseArgs` 收束散参数，并将 `CandidateExecutionResult::Exhausted.request` 改为 `Box<Request>`；`clippy` 剩余错误数由 10 降到 5，`gateway/upstream` 不再出现在剩余清单中
  - 本轮收敛 `gateway/auth/openai_fallback.rs`：以 `TryOpenAiFallbackArgs` 收束 fallback 请求参数；`clippy` 剩余错误数由 5 降到 4，`gateway/auth` 不再出现在剩余清单中
  - 本轮收敛 `account/account_payment.rs`：以 `CheckoutLinkArgs` 收束支付下单参数；`clippy` 剩余错误数由 4 降到 3，`account_payment` 不再出现在剩余清单中
  - 本轮收敛 `account/account_register.rs`、`usage/refresh/autofill.rs`、`rpc_dispatch/account.rs` 与 `gateway/freeproxy.rs`：以 `StartRegisterBatchInput` / `CreateRegisterProxyInput` 收束注册批量与代理创建参数；同步将 `app_settings/api/current.rs` 的快照持久化改为 `PersistCurrentSnapshotInput`
  - 本轮继续收敛测试层 warning：`tests/shutdown_flag.rs` 改为布尔断言，`tests/gateway_logs/cache.rs` 改为 `contains_key` 判定；`clippy` 最终清零

- [-] **本轮验证结果**
  - `pnpm exec tsc --noEmit` 通过（F17 插件管理前端：插件类型、RPC normalize、设置页 CRUD 与模板入口）
  - `pnpm run build:desktop` 失败：Next 16 Turbopack 在当前 automation 沙箱内处理 `src/app/globals.css` 时尝试创建子进程并绑定端口，触发 `Operation not permitted`
  - `pnpm exec next build --webpack` 通过（作为当前环境下的等价前端构建复验，静态路由继续包含 `/settings`，插件管理页签可参与静态构建）
  - `cargo test -p codexmanager-service session_handler_ --features mcp -- --nocapture` 通过（本轮按防重叠规则复验 F16 `mcp::session`：parse error / initialized notification 仍通过）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（本轮确认现有 MCP 脏改在当前工作区仍可编译）
  - `pnpm exec tsc --noEmit` 通过（本轮复验设置页 MCP 开关与端口配置的前端类型）
  - `cargo test -p codexmanager-service session_handler_ --features mcp -- --nocapture` 通过（F16 MCP 会话层：新增 transport-agnostic JSON-RPC 处理回归，覆盖 parse error / initialized notification）
  - `cargo test -p codexmanager-service http_sse_ --features mcp -- --nocapture` 通过（F16 MCP HTTP SSE：覆盖 endpoint 握手、initialize 响应回推、禁用开关拒绝）
  - `cargo test -p codexmanager-service stdio_server_ --features mcp -- --nocapture` 通过（F16 MCP stdio：抽离 `mcp::session` 后 `initialize` / `tools/list` / `tools/call` 回归仍通过，并显式隔离测试 DB 的 `mcpEnabled` 状态）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（F16 MCP 会话层抽离后 feature-gated MCP 二进制仍可编译）
  - `cargo test -p codexmanager-service stdio_server_rejects_ --features mcp -- --nocapture` 通过（F16 MCP 开关关闭后，`initialize`、`tools/list`、`tools/call` 均返回禁用错误）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（F16 feature-gated MCP 二进制本轮改动后仍可编译）
  - `cargo test -p codexmanager-service --test app_settings app_settings_set_persists_snapshot_and_password_hash -- --nocapture` 通过（F16 MCP 设置项持久化：`mcpEnabled` / `mcpPort`）
  - `cargo test -p codexmanager-service --test app_settings app_settings_get_loads_env_backed_dedicated_settings_when_storage_missing -- --nocapture` 通过（F16 MCP 设置项环境变量覆盖：`CODEXMANAGER_MCP_ENABLED` / `CODEXMANAGER_MCP_PORT`）
  - `cargo test -p codexmanager-service stdio_server_ --features mcp -- --nocapture` 通过（F16 MCP stdio：新增设置禁用时拒绝 `initialize` 回归）
  - `pnpm exec tsc --noEmit` 通过（F16 设置页 MCP 开关与端口配置）
  - `cargo test -p codexmanager-service stdio_server_ --features mcp -- --nocapture` 通过（F16 MCP 接入指南对应的 stdio 主链路与 4 个工具回归）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（F16 文档中的 feature-gated 启动方式可编译）
  - `cargo test -p codexmanager-service stdio_server_ --features mcp -- --nocapture` 通过（F16 MCP stdio：新增 `chat_completion` 工具真实调用闭环，覆盖 success / missing api key / unknown tool）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（确认 feature-gated MCP 二进制在接入 `chat_completion` 后仍可编译）
  - `cargo check -p codexmanager-service` 通过（确认本轮 MCP 改造未影响默认 feature 主服务编译）
  - `cargo test -p codexmanager-service stdio_server_ --features mcp -- --nocapture` 通过（F16 MCP stdio：`tools/call` 已接通 `list_models` / `list_accounts` / `get_usage` 真实执行，并保留 `chat_completion` 未接线错误回归）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（F16 feature-gated MCP 二进制继续可编译）
  - `cargo check -p codexmanager-service` 通过（确认本轮 MCP 改造未影响默认 feature 主服务编译）
  - `cargo test -p codexmanager-service stdio_server_ --features mcp -- --nocapture` 通过（F16 MCP stdio 骨架：覆盖 initialize / tools/list / tools/call stub / unknown method）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（F16 feature-gated 实验 MCP 二进制）
  - `cargo check -p codexmanager-service` 通过（确认默认未开启 `mcp` feature 时主服务编译未回退）
  - `cargo test -p codexmanager-service audit_list_read_operation_does_not_create_audit_log -- --nocapture` 通过（F13 验收补齐：`audit/list` 只读查询不会新增审计记录）
  - `cargo check -p codexmanager-service` 通过（本轮仅补 F13 审计只读回归测试，无编译回归）
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
  - 本轮尝试重新执行 Docker 本地源码复验时，`docker info` 访问 `unix:///Users/jfwang/.orbstack/run/docker.sock` 直接返回 `operation not permitted`；属于当前 automation 沙箱无法访问 Docker daemon 的环境阻塞，待具备 daemon 权限后复跑
  - `cargo test -p codexmanager-service healthcheck_config_rpc_supports_get_and_set -- --nocapture` 通过
  - `cargo test -p codexmanager-service healthcheck_run_rpc_returns_empty_summary_without_probe_candidates -- --nocapture` 通过
  - `cargo test -p codexmanager-service dashboard_health_rpc_includes_recent_healthcheck_after_run -- --nocapture` 通过（F11 首页最近巡检卡片与 `healthcheck/run` 同源数据回归）
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
  - `cargo test -p codexmanager-service build_session_probe_tasks_skips_disabled_accounts -- --nocapture` 通过
  - `cargo test -p codexmanager-service recover_account_after_success_restores_unavailable_but_keeps_disabled -- --nocapture` 通过
  - `cargo test -p codexmanager-service healthcheck_run_triggers_token_refresh_fail_alerts_when_probe_fails -- --nocapture` 通过（改为内存 mock probe + webhook，沙箱内可稳定复验）
  - `cargo test -p codexmanager-service healthcheck_ -- --nocapture` 通过（覆盖巡检配置、空候选摘要、巡检失败触发告警）
  - `cargo test -p codexmanager-service reload_background_tasks_runtime_from_env_applies_session_probe_settings -- --nocapture` 通过
  - `cargo test -p codexmanager-service reload_background_tasks_runtime_from_env_restores_session_probe_defaults -- --nocapture` 通过
  - `cargo test -p codexmanager-service reload_background_tasks_runtime_from_env_ -- --nocapture` 通过（修复 env 覆盖测试并发污染后复验）
  - `cargo test -p codexmanager-service healthcheck_ -- --nocapture` 通过（复验 F11 巡检 RPC / 告警链路）
  - `cargo test -p codexmanager-service healthcheck_run_triggers_token_refresh_fail_alerts_when_probe_fails -- --nocapture` 通过（补充 RAII 清理 guard 后复验）
  - `cargo test -p codexmanager-service reload_background_tasks_runtime_from_env_ -- --nocapture` 通过（本轮复验 F11 环境变量覆盖回归）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（存在 80+ 条仓库级历史 warning，G7 待单独收口）
  - `cargo test -p codexmanager-service requestlog_export_rpc_returns_filtered_csv_content -- --nocapture` 通过（本轮复验导出链路）
  - `cargo test -p codexmanager-service web_auth_two_factor_rpc_supports_setup_verify_recovery_and_disable -- --nocapture` 通过（本轮复验 2FA 链路）
  - `cargo check -p codexmanager-service` 通过（本轮收敛 `app_settings` / `2FA` / `requestlog export` / `model_picker` 的 clippy 兼容改动后编译正常）
  - `cargo test -p codexmanager-service --test app_settings app_settings_get_loads_env_backed_dedicated_settings_when_storage_missing -- --nocapture` 通过（修复 `app_settings/get` 读取环境覆盖时误触发 reqwest client 初始化，macOS 下不再触发 `system-configuration` NULL object panic）
  - `cargo test -p codexmanager-service --test app_settings sync_runtime_settings_from_storage_ -- --nocapture` 通过（复验环境覆盖同步仍能保留显式进程变量并应用持久化运行时设置）
  - `cargo check -p codexmanager-service` 通过（本轮继续收敛 `account` / `gateway` / `http` / `usage` 相关 clippy warning）
  - `cargo test -p codexmanager-service gateway_latency_ring_buffer_returns_recent_samples_only -- --nocapture` 通过（复验 `metrics.rs` 调整后环形缓冲测试）
  - `cargo test -p codexmanager-service inspect_sse_frame_keeps_last_event_type -- --nocapture` 通过（复验 `sse_frame.rs` 调整后 SSE 事件类型保留逻辑）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（本轮确认 `items_after_test_module` 已不再出现；剩余主要为 `too_many_arguments`、`large_enum_variant`、`ptr_arg`、`module_inception`）
  - `cargo check -p codexmanager-service` 通过（本轮收敛 `http_bridge` 的响应桥接参数与 SSE 分支判断）
  - `cargo test -p codexmanager-service compact_header_only_ -- --nocapture` 通过（覆盖 compact 非成功体归一化分类单测）
  - `cargo test -p codexmanager-service --test gateway_logs gateway_openai_compact_invalid_success_body_is_mapped_to_502 -- --nocapture` 失败：沙箱内绑定本地端口 `41000` 被拒绝 `Operation not permitted`
  - `cargo test -p codexmanager-service --test gateway_logs gateway_openai_compact_html_non_success_is_mapped_to_structured_403 -- --nocapture` 失败：沙箱内绑定本地端口 `41000` 被拒绝 `Operation not permitted`
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（复验后 `http_bridge/delivery.rs` 与 `http_bridge/mod.rs` 不再报错，剩余 18 条集中在 `account`、`app_settings`、`gateway/request`、`gateway/upstream`、`usage`）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（本轮收敛 `gateway/upstream/proxy_pipeline/request_setup.rs` 的 `too_many_arguments` / `ptr_arg` 后，剩余 16 条，集中在 `account`、`app_settings`、`gateway/request`、`gateway/upstream`、`usage`）
  - `cargo check -p codexmanager-service` 通过（确认 `PrepareRequestSetupInput` 重构未引入编译回归）
  - `cargo check -p codexmanager-service` 通过（本轮收敛 `gateway/request` 本地短路响应参数后编译正常）
  - `cargo test -p codexmanager-service --lib build_openai_models_list_outputs_expected_shape -- --nocapture` 通过（复验本地 `/v1/models` 响应输出）
  - `cargo test -p codexmanager-service --lib estimate_input_tokens_ -- --nocapture` 通过（复验本地 `count_tokens` 估算逻辑）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（本轮收敛 `gateway/request/local_models.rs` 与 `gateway/request/local_count_tokens.rs` 后，剩余 14 条，已不再包含 `gateway/request`）
  - `cargo check -p codexmanager-service` 通过（本轮收敛 `gateway/observability/request_log.rs` / `trace_log.rs` 参数对象后编译正常）
  - `cargo test -p codexmanager-service gateway_latency_ring_buffer_returns_recent_samples_only -- --nocapture` 通过（复验 observability latency ring buffer）
  - `cargo test -p codexmanager-service --lib build_openai_models_list_outputs_expected_shape -- --nocapture` 通过（复验 `RequestLogEntry` 改造未影响本地 models 响应）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（本轮收敛 `gateway/observability/request_log.rs` 与 `gateway/observability/trace_log.rs` 后，剩余 12 条，已不再包含 `gateway/observability`）
  - `cargo test -p codexmanager-core storage_can_summarize_cost_usage_by_key_model_and_day -- --nocapture` 通过（本轮复验 F08 聚合查询）
  - `cargo test -p codexmanager-service stats_cost_model_pricing_rpc_supports_get_and_set -- --nocapture` 通过（本轮复验 F08 模型单价 RPC）
  - `cargo test -p codexmanager-service stats_cost_summary_rpc_aggregates_custom_range -- --nocapture` 通过（本轮复验 F08 汇总 RPC）
  - `cargo test -p codexmanager-service stats_cost_export_rpc_returns_csv_content -- --nocapture` 通过（本轮复验 F08 CSV 导出）
  - `cargo test -p codexmanager-service request_log_export_streams_json_in_multiple_chunks -- --nocapture` 通过（本轮补齐 F10 流式 JSON 导出回归）
  - `cargo test -p codexmanager-service export_response_sets_json_headers -- --nocapture` 通过（本轮补齐 F10 HTTP JSON 下载头回归）
  - `pnpm exec tsc --noEmit` 通过（本轮复验 F08 前端类型）
  - `pnpm run build:desktop` 失败：Next 16 默认 Turbopack 在当前沙箱内处理 `src/app/globals.css` 时尝试创建子进程并绑定端口，触发 `Operation not permitted`
  - `pnpm exec next build --webpack` 通过（作为当前环境下的等价前端构建复验，静态路由包含 `/costs`）
  - `cargo test -p codexmanager-service blocking_poll_loop_ -- --nocapture` 通过（本轮复验 usage scheduler 的轮询、退避与 jitter 单测）
  - `cargo test -p codexmanager-service parse_interval_secs_falls_back_and_applies_minimum -- --nocapture` 通过（本轮复验 usage scheduler 的最小间隔夹紧）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（本轮清空 `usage` 相关 warning 后，剩余 10 条集中在 `account`、`app_settings`、`gateway/auth`、`gateway/upstream`）
  - `cargo check -p codexmanager-service` 通过（本轮收敛 `gateway/upstream` attempt flow / proxy pipeline 的参数对象化与 `Box<Request>` 后编译正常）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（本轮清空 `gateway/upstream` 相关 warning 后，剩余 5 条集中在 `account`、`app_settings`、`gateway/auth`）
  - `cargo test -p codexmanager-service request_compression_only_applies_to_streaming_chatgpt_responses -- --nocapture` 通过（复验 `SendUpstreamRequestArgs` 改造后请求压缩判定）
  - `cargo test -p codexmanager-service encode_request_body_adds_zstd_content_encoding -- --nocapture` 通过（复验请求压缩头与 zstd 编码写入）
  - `cargo check -p codexmanager-service` 通过（本轮收敛 `gateway/auth/openai_fallback.rs` 参数对象化后编译正常）
  - `cargo test -p codexmanager-service request_affinity_uses_thread_anchor_for_fallback_headers -- --nocapture` 通过（复验 fallback 线程锚点请求亲和逻辑）
  - `cargo test -p codexmanager-service gateway_model_fallback_keeps_primary_model_when_first_attempt_succeeds -- --nocapture` 失败：测试 mock upstream 固定绑定本地端口 `41000`，当前 automation 沙箱返回 `Operation not permitted`
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（`gateway/auth/openai_fallback.rs` 已清零；剩余 4 条集中在 `account/account_payment.rs`、`account/account_register.rs`、`app_settings/api/current.rs`）
  - `cargo check -p codexmanager-service` 通过（本轮收敛 `account/account_payment.rs` 的下单参数对象化后编译正常）
  - `cargo clippy -p codexmanager-service --tests -- -D warnings` 失败（`account/account_payment.rs` 已清零；剩余 3 条集中在 `account/account_register.rs` 与 `app_settings/api/current.rs`）
  - `cargo fmt --all` 通过（本轮参数对象化与测试断言收口）
  - `cargo clippy -p codexmanager-service --tests --message-format short -- -D warnings` 通过（本轮清空剩余 3 条库 warning 与 5 条测试 warning）
  - `cargo test -p codexmanager-service send_test_alert_supports_bark_telegram_and_wecom_mock_transports -- --nocapture` 通过（F02 补齐 Bark / Telegram / 企业微信渠道发送格式回归）
  - `cargo test -p codexmanager-service alert_rpc_supports_rule_channel_history_and_channel_test -- --nocapture` 通过（F02 `alert/channels/test` 改为内存 webhook mock，沙箱内不再依赖端口绑定）
  - `cargo test -p codexmanager-service alert_engine_usage_threshold_dedupes_and_recovers -- --nocapture` 通过（F02 额度阈值告警触发 / 去重 / 恢复回归）
  - `cargo test -p codexmanager-service alert_engine_all_unavailable_triggers_and_recovers -- --nocapture` 通过（F02 全部账号不可用告警触发 / 恢复回归）
  - `cargo test -p codexmanager-service healthcheck_run_triggers_token_refresh_fail_alerts_when_probe_fails -- --nocapture` 通过（复验 F11 巡检接入 F02 告警链路未回退）
  - `cargo test -p codexmanager-service validate_api_key_allowed_model -- --nocapture` 通过（F12 白名单校验改为先拦截显式请求模型，再校验默认模型覆盖）
  - `cargo test -p codexmanager-service gateway_cache_rpc_supports_get_set_stats_and_clear -- --nocapture` 通过（本轮复验 F06 全局缓存配置 / 统计 / 清空 RPC）
  - `cargo test -p codexmanager-service response_cache_entry_expires_after_ttl -- --nocapture` 通过（本轮复验 F06 TTL 过期清理）
  - `cargo test -p codexmanager-service response_cache_evicts_oldest_entry_when_capacity_is_exceeded -- --nocapture` 通过（本轮复验 F06 容量淘汰）
  - `cargo test -p codexmanager-service apikey_response_cache_rpc_supports_get_and_set -- --nocapture` 通过（本轮复验 F06 API Key 级缓存 RPC）
  - `cargo test -p codexmanager-core storage_can_roundtrip_api_key_response_cache_config -- --nocapture` 通过（本轮复验 F06 API Key 级缓存持久化）
  - `cargo test -p codexmanager-service --test gateway_logs gateway_response_cache_hits_second_non_stream_request -- --nocapture --test-threads=1` 失败：当前 automation 沙箱禁止测试内 `TcpListener::bind("127.0.0.1:*")`，即使 `gateway_logs` mock upstream 已改为系统分配端口仍返回 `Operation not permitted`
  - `cargo test -p codexmanager-service --test gateway_logs gateway_response_cache_skips_requests_when_api_key_cache_disabled -- --nocapture --test-threads=1` 失败：同上，属于当前运行环境 listener 权限限制，待切回允许本地 socket 的环境后复跑
  - `cargo test -p codexmanager-service parse_transport_ --features mcp --bin codexmanager-mcp -- --nocapture` 通过（F16 MCP 二进制 transport 入口：默认 stdio、`http-sse`、`--transport=http-sse` 均可识别）
  - `cargo test -p codexmanager-service http_sse_ --features mcp -- --nocapture` 通过（F16 MCP HTTP SSE：覆盖会话初始化、`messageUrl` 生成与禁用态拒绝）
  - `cargo check -p codexmanager-service --features mcp --bin codexmanager-mcp` 通过（F16 feature-gated MCP 二进制在接入 HTTP SSE 文档收口后仍可编译）
  - `cargo test -p codexmanager-service plugin_rpc_ -- --nocapture` 通过（F17 插件管理 CRUD：覆盖 `plugin/upsert` / `plugin/list` / `plugin/delete` 与非法 runtime / hook point 校验）
  - `cargo check -p codexmanager-service` 通过（F17 service 插件 RPC、审计快照与 core RPC 类型扩展编译通过）
  - `cargo test -p codexmanager-service plugin_rpc_ -- --nocapture` 通过（本轮复验 F17 插件 CRUD 与校验逻辑仍可用）
  - `pnpm exec tsc --noEmit` 通过（本轮复验 F17 设置页插件管理 Tab 与模板草稿类型）
  - `cargo test -p codexmanager-service resolve_existing_imported_account_id_ -- --nocapture` 通过（本轮补齐注册任务“已入池 / 待入池”判断，覆盖 identity hint 与邮箱回退匹配）
  - `pnpm exec tsc --noEmit` 通过（本轮补齐注册中心“待入池”标识与手动加入号池按钮）
  - `cargo test -p codexmanager-service requestlog_list_and_summary_support_extended_filters -- --nocapture` 通过（本轮复验请求日志列表 / 摘要扩展筛选，并覆盖 `keyIds` 多密钥过滤）
  - `cargo test -p codexmanager-service requestlog_export_rpc_supports_key_model_and_time_filters -- --nocapture` 通过（本轮复验请求日志导出扩展筛选，并覆盖 `keyIds` 多密钥过滤）
  - `pnpm exec tsc --noEmit` 通过（本轮复验平台密钥页完整 ID 展示、复制兜底与请求日志按密钥名称筛选）
  - `pnpm run build:desktop` 通过（本轮复验平台密钥页与请求日志页桌面静态构建）
  - `pnpm exec tsc --noEmit` 通过（本轮复验请求日志快捷日期筛选：昨天 / 今天 / 本周 / 本月）
  - `pnpm run build:desktop` 通过（本轮复验请求日志快捷日期筛选桌面静态构建）
  - `cargo check -p codexmanager-service` 通过（本轮复验 F17 Lua 插件运行时接入后默认 feature 服务编译）
  - `cargo test -p codexmanager-service plugin_ -- --nocapture` 通过（本轮复验 F17 Lua 插件 CRUD / runtime 校验与 hook 执行回归）
  - `pnpm run build:desktop` 通过（本轮复验 F17 运行时落地后桌面端前端静态构建）
  - `docker build --no-cache -f docker/Dockerfile.service.local .` 通过（本轮复验 service 本地源码离线镜像构建）
  - `docker build --no-cache -f docker/Dockerfile.web.local .` 通过（本轮复验 web 本地源码离线镜像构建）
  - `docker compose -f docker/docker-compose.localbuild.yml build --no-cache codexmanager-service codexmanager-web` 失败：BuildKit/Compose 路径在复制 `vendor-rust/erased-serde/.cargo_vcs_info.json` 时出现校验和不一致；直接 `docker build` 可稳定通过，当前判定为 compose/build-cache 侧异常，待后续单独排查
  - `cargo test -p codexmanager-service usage_refresh_error_class_catches_ -- --nocapture` 通过（本轮补齐 `workspace_deactivated` 封禁识别，覆盖账号 / 工作区停用错误归类）
  - `cargo test -p codexmanager-service classify_failure_reason_detects_deactivated_ -- --nocapture` 通过（本轮补齐失败摘要中的工作区停用分类）
  - `cargo test -p codexmanager-service banned_cleanup_matches_deactivated_accounts_only -- --nocapture` 通过（本轮补齐“一键清理封禁账号”后端判定）
  - `cargo check -p codexmanager-service` 通过（本轮复验封禁清理 RPC、状态原因映射与账号页额度重置时间所依赖的 service 改动）
  - `pnpm exec tsc --noEmit` 通过（本轮复验账号页“一键清理封禁账号”入口与 5 小时 / 7 天重置时间展示）
  - `cargo check --manifest-path apps/src-tauri/Cargo.toml` 失败：当前 `vendor-rust/` 离线源缺少 `rfd`，编译在解析依赖阶段即中断，属于现有 workspace vendor 问题，待补齐依赖后复验桌面端命令桥编译

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
  - [x] 验收补齐：Bark / Telegram / 企业微信渠道发送改为内存 mock 回归测试，当前 automation 沙箱内无需真实端口或外网即可稳定验证
  - [x] 沙箱兼容：`alert/channels/test`、额度阈值告警、全部账号不可用告警均已移除 `TcpListener` 依赖，避免重复出现 `Operation not permitted`

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

- [x] **后端：聚合查询**
  - [x] 按 API Key / 模型 / 日期多维聚合 token 消耗与费用
  - [x] 支持时间范围过滤（today / week / month / custom）

- [x] **后端：RPC 接口**
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
  - [x] 按 Key 汇总与最高费用 Key 卡片补充平台密钥名称 + ID 展示，避免仅靠短 ID 难以区分
  - [x] 排序稳定性收口：最高费用 Key / 模型卡片、模型分布和汇总表统一基于费用降序数据渲染，避免依赖原始返回顺序

- [x] **阶段验证**
  - [x] `cargo test -p codexmanager-core storage_can_roundtrip_model_pricing_config -- --nocapture`
  - [x] `cargo test -p codexmanager-core model_pricing_item_serialization_uses_camel_case -- --nocapture`
  - [x] `cargo test -p codexmanager-service stats_cost_model_pricing_rpc_supports_get_and_set -- --nocapture`
  - [x] `cargo test -p codexmanager-core storage_can_summarize_cost_usage_by_key_model_and_day -- --nocapture`
  - [x] `cargo test -p codexmanager-core cost_summary_params_serialization_uses_camel_case -- --nocapture`
  - [x] `cargo test -p codexmanager-service stats_cost_summary_rpc_aggregates_custom_range -- --nocapture`
  - [x] 本轮补强 `stats_cost_summary_rpc_aggregates_custom_range`，显式校验按费用降序返回 `byKey` / `byModel`
  - [x] `cargo test -p codexmanager-core cost_export_result_serialization_uses_camel_case -- --nocapture`
  - [x] `cargo test -p codexmanager-service stats_cost_export_rpc_returns_csv_content -- --nocapture`
  - [x] `pnpm exec tsc --noEmit`
  - [x] `pnpm run build:desktop`
  - [x] `cargo check --manifest-path apps/src-tauri/Cargo.toml`

---

## Phase 3 — 缓存 + 分析 + 巡检

### F06 响应缓存

- [x] **后端：缓存层**
  - [x] 实现内存 LRU 风格响应缓存
  - [x] 缓存 key = 路径 + 请求体规范化后的 SHA256
  - [x] 支持配置 TTL 和最大条目数

- [x] **后端：网关集成**
  - [x] 非流式请求在路由前查缓存
  - [x] 缓存命中时直接返回，标注 `X-CodexManager-Cache: HIT`
  - [x] 缓存未命中且请求成功后写入缓存
  - [x] 首次非流式响应标注 `X-CodexManager-Cache: MISS`
  - [x] 流式请求不缓存回归用例
  - [x] TTL 过期与容量淘汰回归用例

- [x] **后端：RPC 接口**
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

- [x] **文档与验收收口**
  - [x] `docs/API.md` 已补齐响应缓存 RPC、API Key 缓存开关与 `appSettings` 快照字段说明
  - [x] 本轮重新复验全局配置 / TTL / 容量淘汰 / API Key 级持久化；HIT/MISS 集成回归仍受当前 automation 沙箱的本地 listener 权限限制

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

- [x] **后端**
  - [x] 新增 HTTP 端点 `GET /export/requestlogs`
  - [x] 先补 RPC 导出链路，支持 `format` 参数（csv / json）
  - [x] 支持当前日志页筛选参数（query / statusFilter）
  - [x] 导出接口额外支持 `timeFrom / timeTo / model / keyId` 筛选
  - [x] HTTP 导出改为分批流式响应，避免大数据量整块占用内存

- [x] **前端**
  - [x] 请求日志页增加「导出」按钮
  - [x] 格式选择下拉（CSV / JSON）
  - [x] 导出使用当前页面的筛选条件
  - [x] Web / Docker 版优先走 `/api/export/requestlogs` 直接下载
  - [x] 日志页补充 `keyId / model / timeFrom / timeTo` 筛选并与列表、摘要、导出联动
  - [x] 日志页时间筛选补充快捷日期按钮：昨天 / 今天 / 本周 / 本月，并同步当前 URL 查询参数
  - [x] 验收补齐：平台密钥筛选输入支持密钥名称或 ID 模糊匹配，并在多匹配时联动列表、摘要与导出走 `keyIds`
  - [x] 验收补齐：补充流式 JSON 导出与 HTTP JSON 下载头回归测试，确认 CSV / JSON 两种格式均可下载
  - [x] 日志页平台密钥列、详情浮层与筛选标签补充平台密钥名称 + ID 展示，便于直接区分不同 Key
  - [x] 平台密钥页显示完整密钥 ID，并为复制操作补齐非 Clipboard API 兜底，避免浏览器环境差异导致复制报错

---

### F11 账号自动巡检

- [x] **后端：巡检调度**
  - [x] 复用 `session_probe` 逻辑对启用账号执行探测，并保留轮询线程配置
  - [x] 失败账号自动标记 `unavailable` + 写入现有失败 event
  - [x] 巡检成功后自动恢复账号为 `active` 并清理 cooldown
  - [x] 内存维护最近一次巡检摘要（开始 / 结束时间、采样数、成功数、失败数、失败账号）
  - [x] 新增独立并发控制（最多 N 个并发探测）
  - [x] 评估默认巡检间隔是否调整到 PRD 约定的 30 分钟
  - [x] 补充回归测试：禁用账号不参与巡检，`unavailable` 账号巡检成功后恢复为 `active`
  - [x] 补充回归测试：环境变量覆盖可驱动巡检开关、间隔与抽样数（对齐 G4）
  - [x] 修复回归测试并发污染：环境变量覆盖测试在同进程并发执行时改为串行化
  - [x] 对齐 PRD / 验收术语：巡检恢复态以现有账号状态枚举 `active` 为准

- [x] **后端：RPC 接口**
  - [x] `healthcheck/config/get`、`healthcheck/config/set`
  - [x] `healthcheck/run`（手动触发）

- [x] **前端**
  - [x] 设置页复用现有巡检配置区，支持开关 / 间隔 / 抽样数配置
  - [x] 设置页增加「立即巡检」按钮与最近巡检结果摘要
  - [x] 仪表盘展示最近巡检时间、抽检通过率与采样结果概览

- [x] **集成**
  - [x] 巡检异常结果接入告警通知系统（F02）
  - [x] 新增回归测试：`healthcheck/run` 失败后触发 `token_refresh_fail` 告警 webhook 与历史记录
  - [x] 新增回归测试：`dashboard/health` 返回最近巡检摘要，保证首页巡检卡片与 `healthcheck/run` 数据一致
  - [x] 补强测试隔离：巡检告警 mock webhook / probe override 在断言失败时也会自动清理，避免污染后续用例

---

## Phase 4 — 安全与审计

### F12 API Key 模型访问控制

- [x] **数据库**
  - [x] `api_keys` 表新增 `allowed_models_json` 列（nullable TEXT）

- [x] **后端**
  - [x] gateway model_picker 阶段检查白名单
  - [x] 白名单外模型返回 403
  - [x] `apikey/allowedModels/get`、`apikey/allowedModels/set`
  - [x] 验收补齐：显式请求白名单外模型时，即使 API Key 配置了允许的默认模型覆盖，也会先返回 403；白名单为空时仍全放行

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
  - [x] 补充回归测试：`audit/list` 只读查询不会反向写入新的审计日志（对齐验收 13.5）

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

- [x] **联调与验证**
  - [x] 服务端 / Web 登录页 / 桌面端设置入口编译通过
  - [x] 回归测试覆盖 setup / verify / recovery code / disable / clear password 清空 2FA
  - [x] Web handler 测试覆盖密码后进入 2FA 页面与 pending cookie -> 正式登录 cookie 交换
  - [x] Web handler 测试覆盖 recovery code 登录成功并消耗剩余次数
  - [x] Web handler 测试覆盖错误 2FA 验证码返回失败且保留 pending cookie
  - [x] `docs/API.md` 已补齐 Web auth/2FA、重试策略、巡检、导出与审计接口说明，并挂到 README / docs 索引
  - [x] 本轮复验：`cargo test -p codexmanager-service web_auth_two_factor_rpc_supports_setup_verify_recovery_and_disable -- --nocapture` 与 `cargo test -p codexmanager-web login_submit_ -- --nocapture` 均通过，确认 2FA 服务端与 Web 登录链路已闭环
  - [!] 非阻塞环境备注：当前 automation 沙箱内无法访问 `unix:///Users/jfwang/.orbstack/run/docker.sock`，因此未重复执行 Docker 本地源码镜像复验；不影响本轮 2FA 功能闭环判断

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

- [x] **后端**
  - [x] 评估 MCP 协议实现方案（首轮采用手写 stdio `Content-Length` JSON-RPC 骨架，先避免在未接入 HTTP SSE / tools/call 前引入 `rmcp` 依赖）
  - [x] 实现 MCP Tools：chat_completion / list_models / list_accounts / get_usage
    - [x] `tools/list` 已暴露 4 个规划工具定义
    - [x] `tools/call` 已接通 `list_models` / `list_accounts` / `get_usage` 真实只读执行，并返回 `structuredContent`
    - [x] `chat_completion` 已通过进程内一次性 backend server 复用真实 `/v1/chat/completions` 网关链路，支持 `arguments.apiKey` / `CODEXMANAGER_MCP_API_KEY` 取 key，并返回 `response + gateway` 元数据
  - [x] 支持 stdio + HTTP SSE 传输
    - [x] 已新增 feature-gated `codexmanager-mcp` 实验二进制，支持 stdio `initialize` / `ping` / `tools/list` / `tools/call`
    - [x] `mcpEnabled=false` 时，`initialize` / `tools/list` / `tools/call` 均会直接返回禁用错误
    - [x] 已抽离 `mcp::session` 统一 `initialize` / `tools/*` 的 JSON-RPC 会话与工具执行，`stdio` 仅保留 `Content-Length` framing，后续 HTTP SSE 可直接复用同一处理入口
    - [x] HTTP SSE 传输（`GET /sse` 建立会话，`POST /message?sessionId=...` 发送 JSON-RPC，请求结果通过 SSE `message` 事件返回）
    - [x] `codexmanager-mcp` 新增 `http-sse` / `--http-sse` / `--transport=http-sse` 启动入口
  - [x] 以 feature flag 编译

- [x] **前端**
  - [x] 设置页增加 MCP Server 开关与端口配置
  - [x] `appSettings/get|set` 暴露 `mcpEnabled` / `mcpPort`，并支持 `CODEXMANAGER_MCP_ENABLED` / `CODEXMANAGER_MCP_PORT` 环境变量覆盖

- [x] **文档**
  - [x] 编写 MCP 接入指南（Claude Code / Cursor 配置示例）
  - [x] 补充 HTTP SSE 启动方式、`/sse` + `/message` 调试示例与仓库内自检命令

---

### F17 插件 / Hook 系统

- [x] **后端**
  - [x] 新增 migration：`plugins` 表（id, name, description, runtime, hook_points_json, script_content, enabled, timeout_ms, created_at, updated_at）
  - [x] 新增 storage CRUD：插件元数据、脚本内容、启用状态与 hook 点声明可持久化
  - [x] 补充 `codexmanager-core` 存储层回归：插件新增 / 更新 / 删除与 migration 追踪
  - [x] 引入 `mlua` crate
  - [x] 定义钩子点：pre_route / post_route / post_response
  - [x] 实现 Lua 脚本加载、沙箱执行、超时保护
  - [x] 插件管理 CRUD（`plugin/list`、`plugin/upsert`、`plugin/delete`，含 runtime / hook point 校验与审计快照）

- [x] **前端**
  - [x] 设置页增加插件管理区域
  - [x] 插件上传、启用/禁用、编辑、删除
  - [x] 内置插件模板

- [x] **文档**
  - [x] 编写插件开发指南与 API 参考
  - [x] `docs/report/20260323193000000_插件管理与Lua开发指南.md` 已补齐当前已落地能力、Lua 模板、建议 Hook 契约与后续收口顺序
  - [x] `docs/API.md`、`docs/README.md` 已同步插件实验能力与文档入口

---

## 借鉴增强（CLIProxyAPI）

- [x] **远程管理 API**
  - [x] 本轮先落地基础闭环：设置页新增“远程管理 API”开关与访问密钥，`appSettings/get|set` 暴露 `remoteManagementEnabled` / `remoteManagementSecretConfigured` / `remoteManagementSecret`
  - [x] Web 新增 `POST /api/management/rpc`，支持 `x-codexmanager-management-secret` 或 `Authorization: Bearer <secret>` 鉴权后代理到 service RPC
  - [x] 支持 `CODEXMANAGER_REMOTE_MANAGEMENT_ENABLED` / `CODEXMANAGER_REMOTE_MANAGEMENT_SECRET` 环境变量覆盖
  - [x] 补充 `GET /api/management/status` 只读状态端点，便于远程脚本先探测服务与 Web 安全状态
  - [x] `docs/API.md` 已补充远程管理 curl 示例与状态字段说明
  - [x] 本轮验证：`cargo test -p codexmanager-service --test app_settings app_settings_set_persists_snapshot_and_password_hash -- --nocapture`、`cargo test -p codexmanager-service --test app_settings app_settings_get_loads_env_backed_dedicated_settings_when_storage_missing -- --nocapture`、`cargo test -p codexmanager-service --test app_settings app_settings_set_rejects_enabling_remote_management_without_secret -- --nocapture`、`cargo test -p codexmanager-web resolve_management_secret_ -- --nocapture`、`pnpm exec tsc --noEmit`
  - [x] 本轮补充收尾验证：`cargo test -p codexmanager-web management_ -- --nocapture`

- [x] **声明式 Payload Rewrite**
  - [x] 已落地后端第一版：`appSettings/get|set` / `CODEXMANAGER_PAYLOAD_REWRITE_RULES` 支持声明式 JSON 规则，网关在 request rewrite 链中按 `set | set_if_missing` 改写顶层字段
  - [x] 第一版安全边界已收紧：仅支持 JSON body、精确路径或 `*` 匹配、顶层字段，且显式禁止改写 `model`
  - [x] 设置页传输配置区已补最小 JSON 编辑入口，支持直接保存 / 回滚当前规则

- [x] **模型别名 / 模型池**
  - [x] 已落地后端第一版：`appSettings/get|set` / `CODEXMANAGER_MODEL_ALIAS_POOLS` 支持全局 JSON 别名池配置，请求进入 gateway 后会先按 `ordered` / `weighted` 选出真实模型，再复用现有 API Key model fallback
  - [x] 已补 allowlist 兼容：白名单允许别名时不要求重复配置池内真实模型；若别名未放行但本次选中的真实模型在白名单中，也允许通过
  - [x] 已补日志与响应头收口：`requestedModel` 保留客户端别名，真实上游模型继续写入 `model`，当两者不同会返回 `X-CodexManager-Actual-Model`
  - [x] 设置页网关传输配置区已补 `modelAliasPoolsJson` JSON 编辑入口、示例模板、保存 / 还原操作，当前无需再通过 RPC / 环境变量裸维护
  - [x] 本轮验证：`cargo test -p codexmanager-service model_alias -- --nocapture`、`cargo test -p codexmanager-service validate_api_key_allowed_models_ -- --nocapture`、`cargo test -p codexmanager-service actual_model_header_value_only_returns_when_model_changes -- --nocapture`、`cargo test -p codexmanager-service --test app_settings app_settings_get_loads_env_backed_dedicated_settings_when_storage_missing -- --nocapture`、`cargo check -p codexmanager-service`
  - [x] 前端补充验证：`pnpm exec tsc --noEmit`、`pnpm run build:desktop`
