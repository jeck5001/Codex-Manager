# Hotmail Web Frontend Migration Design

## Goal

把 Hotmail 自动注册能力从旧的 `vendor/codex-register` Jinja 页面迁移到用户实际使用的 Next.js Web UI（`apps/src`，默认端口 `48761`），让用户可以直接在现有侧边栏中创建、查看、取消 Hotmail 批次并查看产物。

## Scope

- 新增真实前端路由 `/hotmail`
- 在侧边栏和顶部标题中接入 Hotmail 入口
- 通过现有 `codexmanager-service` RPC 转发 Hotmail 批次相关接口到 register service
- 在前端页面中支持：
  - 创建批次
  - 轮询批次状态
  - 取消批次
  - 查看批次日志
  - 查看产物列表

## Non-Goals

- 不重写 `vendor/codex-register` 中已有的 Hotmail 引擎
- 不把 Hotmail 能力并入现有注册中心弹窗
- 不新增第二套 Hotmail 后端实现到 `codexmanager-service`

## Architecture

真实链路保持和当前注册中心一致：

1. Next.js 页面通过 `apps/src/lib/api/account-client.ts` 调用统一 transport。
2. transport 走 web RPC / Tauri invoke，到 `crates/service` 的 `rpc_dispatch`.
3. `account_register.rs` 负责把 Hotmail 批次请求代理到 `vendor/codex-register` 提供的 `/api/hotmail/*` 接口。

这样复用现有 register service，避免把旧页面逻辑直接嵌回新前端，同时保持 Web 版和桌面版调用路径一致。

## UI Design

页面采用与现有 Register / Email Services 一致的 glass card 结构：

- 顶部一张配置卡：数量、并发、最小间隔、最大间隔、代理
- 中间一张状态卡：总数、完成数、成功数、失败数、完成状态、取消状态
- 下方两张卡：运行日志、产物列表
- 当存在活动批次时自动轮询；页面刷新后如果用户手动填入批次 ID 也能继续查看

页面默认只允许跟踪一个当前批次，保持实现简单，避免引入新的批次管理状态机。

## Data Model

前端新增两类类型：

- `RegisterHotmailBatchStartResult`
- `RegisterHotmailBatchSnapshot`

其中 `RegisterHotmailBatchSnapshot` 额外包含 `artifacts` 字段，元素至少包括：

- `path`
- `filename`
- `size`

批次日志统一为字符串数组。

## Error Handling

- 创建、取消、查询失败统一走 `transport`/toast 现有错误展开逻辑
- 轮询失败不清空现有页面状态，只提示一次错误并停止轮询
- 未找到批次时允许用户重新创建新批次

## Testing

- 前端最少补两个回归点：
  - 导航包含 Hotmail 菜单
  - Hotmail API normalize / state helper 能正确处理基础数据
- 服务端补 Hotmail RPC 映射测试或最小单元测试（如现有测试基础允许）
- 最终跑 `pnpm --dir apps run build:desktop`

## Acceptance Criteria

- `48761` 的侧边栏可见 `Hotmail` 菜单
- 访问 `/hotmail` 可创建批次并看到状态区
- 页面可取消批次、查看日志、查看产物
- 不再需要访问 `9000/hotmail` 才能使用该功能
