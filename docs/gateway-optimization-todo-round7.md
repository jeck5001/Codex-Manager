# Gateway 优化 TODO Round 7

更新时间：2026-04-13

本轮目标：收敛 `candidate_executor` 中最后一段 gateway terminal failover bookkeeping 样板，统一 terminal 转 failover 的入口。

- [x] 抽取 gateway terminal failover helper
- [x] 用 helper 替换 `candidate_executor` 中剩余的手写 bookkeeping
- [x] 运行关键测试并记录结果

本轮验证：

- `cargo test -p codexmanager-service candidate_executor -- --nocapture`
- `cargo test -p codexmanager-service gateway_request_log_keeps_only_final_result_for_multi_attempt_flow -- --nocapture`
