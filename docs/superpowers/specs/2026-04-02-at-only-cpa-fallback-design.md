# AT-Only Account CPA Fallback Design

## Goal

让只有 `accessToken`、没有 `refreshToken` 的账号在现有直连链路失败时，自动退化到 CPA 风格的兼容请求模式，而不是直接返回 `401`。

## Current Findings

- 当前网关已经允许 `AT-only` 账号直接参与候选选择，并直接使用 `accessToken` 发请求。
- 当 `accessToken` 仍然有效且上游接受该 token 时，`AT-only` 账号已经可以正常工作。
- 现有项目中的 “CPA” 兼容能力并不是独立协议，而是现有 Codex 上游请求的一组兼容头部/会话亲和策略，核心行为包括：
  - 不依赖上游 cookie
  - 更激进地移除 `turn_state` / `conversation_id` / 旧 `session_id` 粘性
  - 使用现有 `codex` 请求改写与 header profile
- 目前缺口不在“账号无法入选”，而在“直连 401 后没有再尝试 CPA 风格兼容模式”。

## Recommended Approach

采用最小变更方案：保持当前直连为主路径，仅对 `AT-only` 账号新增一次 CPA 风格兼容重试。

### Why this approach

- 不破坏已有正常账号的行为。
- 不改变导入格式、存储模型、账号类型。
- 只在已经确认直连失败时才触发额外兼容逻辑，风险最小。

## Request Flow

对 `AT-only` 账号的 `/v1/responses` 请求执行以下顺序：

1. 先走当前默认直连路径。
2. 如果上游成功，直接返回。
3. 如果上游返回 `401`，并且该账号：
   - 没有 `refreshToken`
   - 没有可用 `session/cookies`
   - 仍然属于可直发的 `accessToken` 账号
   
   则对同一账号执行一次 CPA 风格兼容重试。
4. 如果兼容重试成功，返回结果。
5. 如果兼容重试仍失败，再进入现有 failover，下一个候选账号接管。

## CPA-Compatible Retry Shape

这次兼容重试不引入新的 RPC 或存储字段，只复用现有 gateway 内部能力：

- 使用现有 Codex 上游 URL，不改路由目标。
- 使用现有 `accessToken` 作为认证 token。
- 切换到 CPA/no-cookie 风格的 header 组合：
  - 不透传 cookie
  - 不透传旧的 turn-state / conversation affinity
  - 尽量减少会话粘性
- 保持现有 request body 改写逻辑不变，避免扩大影响面。

## Scope Boundaries

本轮不做以下事情：

- 不新增新的账号类型。
- 不修改导入协议，仍允许 `AT-only` 账号按现有格式导入。
- 不承诺“裸 `accessToken` 永远可用”。
- 不处理 `accessToken` 本身已失效且上游无论哪种 header profile 都拒绝的场景。

## Expected User-Visible Behavior

- `AT-only` 账号如果本来就能直连，上游行为不变。
- `AT-only` 账号如果直连 `401`，系统会自动再试一次 CPA 风格兼容模式。
- 如果 CPA 风格重试也不行，才切下一个账号或最终返回 `401`。

## Test Plan

至少覆盖以下回归：

1. `AT-only` 账号直连成功，仍直接走当前路径。
2. `AT-only` 账号直连 `401`，CPA 风格兼容重试成功。
3. `AT-only` 账号直连 `401`，CPA 风格兼容重试失败，继续 failover 到下一个账号。
4. 普通带 `refreshToken` 账号的 refresh/failover 行为保持不变。

## Risks

- 如果某些账号的 `401` 根因并非 header profile，而是 token 本身无效，这次兼容重试不会救活它。
- 若 CPA 风格兼容头部过度放宽，可能导致少量本来依赖线程锚点的请求在重试时失去上下文，因此重试必须严格限制为单次 fallback。

