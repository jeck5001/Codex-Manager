# CPA Scheduled Sync Design

## Goal

在当前 `Codex-Manager` 项目中，为现有 `CLIProxyAPI / CPA` 单向账号同步能力补充一套“服务端常驻定时同步”机制。用户可以在设置页开启或关闭该能力，并配置固定的同步间隔分钟数。系统在 Docker / NAS 这种长期运行的服务场景下，应该在不依赖前端页面常驻的前提下，自动按间隔执行 CPA 账号同步。

## Current Context

- 现有项目已经具备手动 CPA 同步能力：
  - 设置页 `CLIProxyAPI / CPA` 卡片支持 `启用同步源`、`CPA API URL`、`Management Key`、`测试连接`、`立即同步`
  - 后端已存在 `service_account_cpa_sync` RPC 与 `account_cpa_sync.rs` 负责拉取 CPA auth files 并复用现有账号导入流程
- 现有设计仅覆盖“手动立即同步”，之前的设计文档明确写了“不自动定时同步”
- 当前部署场景是 NAS 上的 Docker 服务，因此定时行为必须驻留在服务端进程内，而不是靠浏览器页面轮询

## Scope

### In Scope

- 在设置页 `CLIProxyAPI / CPA` 卡片增加：
  - `启用定时同步`
  - `同步间隔（分钟）`
- 将以上配置持久化到现有应用设置
- 在服务端增加 CPA 定时同步调度器
- 复用现有 CPA 手动同步逻辑执行定时任务
- 暴露最近一次运行状态给前端展示
- 设置变更后无需重启服务即可生效
- 防止同一时间存在多个 CPA 同步任务并发执行

### Out of Scope

- 不引入通用任务平台或完整 cron 系统
- 不做随机区间调度
- 不做多条 CPA 同步计划
- 不做双向同步
- 不改现有 CPA auth file 解析和导入规则

## User Experience

设置页中的 `CLIProxyAPI / CPA` 卡片新增两项：

- `启用定时同步`
- `同步间隔（分钟）`

交互规则：

- `启用定时同步` 关闭时：
  - 间隔输入框禁用
  - 已保存的间隔值保留
  - 后端调度器停止后续自动触发
- `启用定时同步` 开启时：
  - 间隔必须是正整数
  - 若缺少 `CPA API URL` 或 `Management Key`，保存仍允许，但运行状态明确显示配置不完整
- `立即同步` 按钮继续保留，并与定时同步并存
- `测试连接`、`保存 CPA 设置`、`立即同步` 保持现有独立按钮模式

卡片底部增加运行态摘要，至少展示：

- 定时同步状态：已启用 / 已关闭 / 配置不完整
- 同步间隔：例如“每 30 分钟”
- 最近开始时间
- 最近结束时间
- 最近结果摘要
- 最近错误
- 下次计划时间
- 当前是否有同步正在执行

## Architecture

采用“现有 CPA 同步器 + 轻量服务端调度器”的最小扩展方案。

### 1. Existing Sync Logic Stays the Source of Truth

现有 `crates/service/src/account/account_cpa_sync.rs` 继续负责：

- 读取和校验 CPA 配置
- 调用 CPA Management API
- 下载 auth files
- 过滤可导入账号
- 复用现有导入器完成入库
- 返回同步摘要

新增定时同步不会复制这套逻辑，只会通过内部服务层调用它。

### 2. New Scheduler State Layer

新增一个 CPA 定时同步运行时状态模块，负责两类职责：

- 保存调度配置快照：
  - 是否启用
  - 同步间隔分钟数
- 保存运行态：
  - 是否正在同步
  - 最近开始时间
  - 最近结束时间
  - 最近成功/失败摘要
  - 最近错误
  - 下次计划时间

这层状态驻留在服务进程内，不要求写入独立任务表。配置本身仍存数据库；运行态以内存为主，并通过 RPC 暴露给前端。

### 3. Background Loop

服务启动时初始化 CPA 调度器。调度器内部是一个常驻后台循环：

1. 读取当前 CPA 定时同步设置
2. 若未启用，则进入轻量等待并继续观察配置变化
3. 若已启用，则按照固定分钟间隔计算下一次运行时间
4. 到点后触发一次同步
5. 更新运行态
6. 再次进入下一轮等待

调度器不依赖设置页打开，不依赖浏览器存活。

### 4. Hot Reload on Settings Change

当用户通过现有设置保存 RPC 修改以下字段时：

- `cpaSyncEnabled`
- `cpaSyncApiUrl`
- `cpaSyncManagementKey`
- `cpaSyncScheduleEnabled`
- `cpaSyncScheduleIntervalMinutes`

调度器应在服务内收到一次“配置已变更”通知，并重新装载最新配置，而不是要求容器重启。

实现上可以通过现有 app settings 更新路径尾部追加一次 scheduler refresh 调用，也可以让调度器按短周期轮询配置快照；推荐显式 refresh，避免无意义轮询。

## Scheduling Semantics

### Interval Semantics

- 间隔单位固定为分钟
- 输入值为正整数
- 系统内部统一换算为秒
- 为避免过于激进的误配置，最小值应夹紧到 `1` 分钟

### First Run Semantics

推荐行为：

- 服务启动后若已启用定时同步，不立即抢跑一次同步
- 而是按“当前时间 + 间隔”计算第一次计划时间

这样更符合“固定间隔任务”的直觉，也避免容器刚启动就立刻打 CPA。

### Overlap Protection

同一时刻只允许一个 CPA 同步执行：

- 若手动同步正在运行，到达定时点时本轮自动同步直接跳过，并记录“因已有同步任务运行而跳过”
- 若定时同步正在运行，用户点击“立即同步”时：
  - 推荐直接拒绝，并提示“CPA 同步正在执行中”

不做排队，不做并发执行。

## Runtime Status Model

前端所需运行态建议包含：

- `scheduleEnabled`
- `intervalMinutes`
- `isRunning`
- `status`
  - `disabled`
  - `idle`
  - `running`
  - `misconfigured`
  - `error`
- `lastStartedAt`
- `lastFinishedAt`
- `lastSuccessAt`
- `lastSummary`
- `lastError`
- `nextRunAt`
- `lastTrigger`
  - `manual`
  - `scheduled`
  - `startup_reload` 不需要，本轮不引入

该结构只服务 CPA 卡片，不扩展成通用任务状态协议。

## Settings Model

在现有 app settings 中新增两项：

- `cpa.sync_schedule_enabled`
- `cpa.sync_schedule_interval_minutes`

并同步到前后端设置快照：

- 后端 `app_settings`
- 前端 `settings snapshot`
- 设置页表单默认值与保存逻辑

保留现有 `cpaSyncEnabled` 语义：

- `cpaSyncEnabled` 表示这组 CPA 同步源配置是否启用
- `cpaSyncScheduleEnabled` 表示是否允许后台自动按间隔执行

只有当两者都为 `true`，且 URL / Key 配置完整时，后台定时同步才真正处于启用状态。

## RPC Surface

除现有：

- `service_account_cpa_test`
- `service_account_cpa_sync`

之外，新增一个只读运行态接口，供设置页展示当前调度状态，例如：

- `service_account_cpa_sync_status`

该接口返回上文的运行态模型。

手动同步 RPC 保持不变，但内部需共享同一个“互斥执行保护”。

## Error Handling

### Configuration Errors

如果用户打开了定时同步，但缺少：

- `CPA API URL`
- `Management Key`

则：

- 不启动实际同步请求
- 运行态标为 `misconfigured`
- `lastError` 显示缺少配置原因
- 调度器继续存活，等待后续配置修复

### Sync Errors

若某次定时同步执行失败：

- 不退出调度器
- 更新 `lastFinishedAt`
- 更新 `lastError`
- 保留 `nextRunAt`
- 下一个周期继续正常尝试

### Manual vs Scheduled

手动同步和定时同步共用一套同步核心，并统一更新运行态，但要区分触发来源，方便排查：

- 手动触发时 `lastTrigger = manual`
- 定时触发时 `lastTrigger = scheduled`

## File-Level Design

预计主要变更文件：

- `crates/service/src/account/account_cpa_sync.rs`
  - 抽出“执行一次 CPA 同步”的可复用入口
  - 新增运行态结构和状态读接口
- `crates/service/src/app_settings/...`
  - 新增 CPA 定时同步配置字段
  - 保存设置后触发调度器刷新
- `crates/service/src/lib.rs`
  - 服务启动时初始化调度器
- `crates/service/tests/app_settings.rs`
  - 覆盖设置读写和持久化
- `crates/service/src/account/tests/account_cpa_sync_tests.rs`
  - 覆盖运行态、互斥、配置缺失、调度执行
- `apps/src/app/settings/page.tsx`
  - 新增开关、间隔输入框和运行态展示
- `apps/src/lib/api/account-client.ts`
  - 新增读取 CPA 同步状态接口

## Testing Strategy

### Rust

- 设置持久化测试
- 调度启停测试
- 间隔计算测试
- 配置缺失时状态为 `misconfigured`
- 同步失败后下一轮仍继续
- 手动同步与定时同步互斥
- 运行态接口返回正确字段

### Frontend

- CPA 卡片展示新增开关与固定间隔输入框
- 保存后能刷新展示值
- 运行态摘要正确渲染
- 输入非法间隔时阻止提交

### End-to-End Verification

- `cargo test` 覆盖 CPA sync 与 app settings
- `pnpm run build:desktop`

## Recommendation

采用这套“服务内轻量调度器”方案。

原因：

- 完全契合 NAS + Docker 常驻服务场景
- 复用现有 CPA 手动同步逻辑，风险最小
- 配置与使用入口都留在当前设置页，用户心智简单
- 后续若要扩展成更通用的定时任务能力，也可以在这层之上再抽象，而不需要推翻本轮实现
