# 账号可用性失效切换设计（Usage 为空/用尽）

日期：2026-01-30

## 背景与问题
当前网关在转发请求时，会按照账号排序从头选择第一个可用账号。缺少“出错时切换”的逻辑，导致当首个账号出现“需升级/不可用”等问题时仍持续被选中。

## 目标
- 上游请求出错时，立即查询用量；若 5h/7d 用量为空或用尽，则切换到下一个账号。
- 后台用量轮询将“查不到额度”和“额度用尽”的账号标记为不可用，减少前台请求失败。
- 兼容现有账号排序与列表逻辑，不引入复杂轮询/轮转策略。

## 非目标
- 不实现全局轮询（round-robin）负载均衡。
- 不引入新的账号状态枚举体系（维持 active/inactive）。

## 术语与判定规则
- **Primary（5h）用量**：usage.primary_window 对应字段（used_percent/window_minutes）。
- **Secondary（7d）用量**：usage.secondary_window 对应字段（secondary_used_percent/secondary_window_minutes）。
- **用量为空**：任一窗口的 used_percent 或 window_minutes 为 null。
- **用量用尽**：任一窗口的 used_percent >= 100。

## 行为设计
### 1) 后台轮询刷新（usage_refresh）
- 每次刷新成功后，依据最新快照判断账号可用性：
  - 若 primary/secondary 任一窗口为空 → 标记为 inactive。
  - 若 primary/secondary 任一窗口用尽 → 标记为 inactive。
  - 若用量完整且未用尽 → 标记为 active。
- 若用量接口返回非 2xx（查不到额度），记录事件并标记为 inactive。
- 若是网络/解析类错误，仅记录事件，不改账号状态（避免误杀）。

### 2) 网关出错时即时切换（gateway）
- 上游返回非 2xx 时：
  1. 立即刷新当前账号用量。
  2. 读取最新快照并进行可用性判定。
  3. 若不可用 → 标记 inactive，选择下一个账号重试同一请求。
  4. 若可用 → 保留 active，直接返回原始上游错误。
- 单次请求最多重试账号数量次，避免死循环。

## 数据与接口调整
- 新增存储更新接口：`update_account_status(account_id, status)`。
- 新增事件类型（示例）：
  - `account_status_update`（message 包含原因，如 usage_missing_primary / usage_exhausted_secondary / usage_unreachable）。
- 可选：抽取“可用性判定”公共函数供轮询与网关复用。

## 测试策略
- 单元测试：覆盖“用量为空/用尽/正常”三类判定逻辑。
- 轮询逻辑测试：刷新成功后账号状态被正确更新；用量接口非 2xx 会标记为 inactive。
- 网关流程测试：模拟上游错误 → 刷新 → 切换账号 → 重试成功；以及“可用但上游仍失败”时不切换。

## 风险与回退
- 风险：短暂网络错误导致误判不可用。已通过“不改状态”的策略降低风险。
- 回退：可通过环境变量关闭轮询（`GPTTOOLS_DISABLE_POLLING`）或回退网关重试逻辑。
