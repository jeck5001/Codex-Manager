# Gateway 优化 TODO Round 5

更新时间：2026-04-13

本轮目标：收敛 gateway error follow-up 的副作用执行入口，统一 `candidate_executor` 与 `response_finalize` 的 cooldown / unavailable 处理。

- [x] 在 `execution_context` 中抽取 gateway error follow-up 副作用 helper
- [x] 让 `candidate_executor` 与 `response_finalize` 复用同一副作用入口
- [x] 运行关键测试并记录结果

本轮验证：

- `cargo test -p codexmanager-service classify_account_availability_signal_separates_usage_refresh_and_deactivation -- --nocapture`
- `cargo test -p codexmanager-service outcome -- --nocapture`
