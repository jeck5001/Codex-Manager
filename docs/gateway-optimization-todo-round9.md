# Gateway 优化 TODO Round 9

更新时间：2026-04-13

本轮目标：收紧前端 `service-client` 的 gateway 契约，让 getter / setter 返回形状与后端 RPC 实际 payload 保持一致。

- [x] 新建 gateway settings 读取模块
- [x] 让 `service-client` 复用共享 gateway contract reader
- [x] 为新模块补最小 Node 单测
- [x] 运行关键前端验证并记录结果

本轮验证：

- `pnpm test:runtime`
- `pnpm build:desktop`
