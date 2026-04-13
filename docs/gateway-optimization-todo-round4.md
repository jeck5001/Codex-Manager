# Gateway 优化 TODO Round 4

更新时间：2026-04-13

本轮目标：收敛 `candidate_executor` 内部重复的 failover 记账与 terminal 收尾模板，不改变既有判定逻辑。

- [x] 抽取 `candidate_executor` 内部 failover bookkeeping helper
- [x] 抽取 `candidate_executor` 内部 terminal response helper
- [x] 运行关键测试并记录结果

本轮验证：

- `cargo test -p codexmanager-service candidate_executor -- --nocapture`
- `cargo test -p codexmanager-service outcome -- --nocapture`
- `cargo test -p codexmanager-service fallback_branch -- --nocapture`
