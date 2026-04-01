# Codex Register 注册核心重构设计

## 背景

当前注册核心主要集中在：

- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/core/register.py`

该文件同时承担：

- 注册流程编排
- 认证页面状态解析
- OAuth 跳转推进
- 验证码处理
- add_phone 回退处理
- callback / token / workspace 提取
- 重试与错误分支控制

随着 `standard`、`browserbase_ddg`、自动恢复登录、注册导入等能力不断叠加，当前实现已经出现以下问题：

1. 主流程职责过多，定位故障成本高。
2. 页面状态提取、流程推进、token 提取逻辑交叉，难以局部验证。
3. 后续如果继续吸收 `any-auto-register` 的状态机拆分思路，现有结构扩展成本较高。
4. 现有逻辑已被 Rust service、前端和任务系统依赖，不能做破坏性接口调整。

本次改造目标是在**不改变现有外部行为**的前提下，完成注册核心的内部模块化拆分。

---

## 目标

### 主要目标

将 `register.py` 拆成更清晰的内部状态机结构，但保持以下兼容性：

- `RegistrationEngine` 对外类名与调用方式不变
- `RegistrationResult` 返回结构不变
- 现有任务接口、Rust service 调用链、前端调用链不变
- `standard` 模式行为不主动调整
- `add_phone` 相关 fallback 行为先保持现状，仅重组实现位置

### 非目标

本次不包含以下内容：

- 不新增新的注册模式
- 不替换现有 `codex-register` 为其他注册项目
- 不主动重写手机号验证策略
- 不调整 Rust service RPC 结构
- 不修改前端注册表单与调用协议
- 不对现有流程进行“策略型增强”或“成功率优化”

---

## 参考来源

本次重构借鉴 `any-auto-register` 的内部组织方式，重点吸收其“状态机拆层、职责分离”的思路，而不是直接迁移其整套实现。

参考点：

- `core/base_platform.py`
- `core/base_executor.py`
- `api/tasks.py`
- `platforms/chatgpt/register_v2.py`
- `platforms/chatgpt/chatgpt_client.py`
- `platforms/chatgpt/token_refresh.py`

借鉴原则：

1. 借鉴结构，不直接照搬流程实现。
2. 保持现有 `codex-register` 对外 API 稳定。
3. 优先提炼“状态解析 / 流程推进 / token 提取 / 重试判定”四类职责。

---

## 目标结构

本次建议新增以下内部模块：

### 1. `register_flow_state.py`

建议路径：

- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/core/register_flow_state.py`

职责：

- 抽取认证响应中的 `page.type`
- 抽取 `continue_url`
- 抽取 callback URL
- 抽取 workspace 相关信息
- 归一化各类认证页面状态
- 形成统一的“流程状态描述对象”或状态辅助函数

目标：

让“当前到底在什么页面 / 下一步该去哪”不再散落在 `register.py` 的多个分支中。

### 2. `register_flow_runner.py`

建议路径：

- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/core/register_flow_runner.py`

职责：

- 推进注册流程
- 推进登录恢复流程
- 跟随 `continue_url`
- 处理认证流程中的跳转与阶段推进
- 将主流程拆成更小的步骤函数

目标：

让主流程从“巨型顺序脚本”变成“由多个可命名步骤组成的流程编排器”。

### 3. `register_retry_policy.py`

建议路径：

- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/core/register_retry_policy.py`

职责：

- 定义哪些错误属于可重试
- 定义哪些响应属于流程终止
- 统一描述 retry / non-retry 判定逻辑

目标：

避免将重试判定散落在多个 `if/else` 里，便于后续单测覆盖。

### 4. `register_token_resolver.py`

建议路径：

- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/core/register_token_resolver.py`

职责：

- 从 callback URL 中提取 code/state
- 推进 token exchange
- 从 cookies / session 中提取会话信息
- 统一解析 workspace / account / token 相关数据

目标：

把 token/callback/workspace 解析从主流程中分离出来，降低主逻辑复杂度。

### 5. 保留 `register.py` 作为兼容入口

保留路径：

- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/core/register.py`

职责调整为：

- 暴露现有 `RegistrationEngine`
- 保留现有数据结构和外部方法名
- 组装 state / flow runner / retry policy / token resolver
- 作为兼容层，避免外部 import 路径变化

---

## 拆分原则

### 原则 1：外部兼容优先

不修改以下外部稳定边界：

- `RegistrationEngine` 初始化参数
- `RegistrationResult` 字段
- 现有 routes / task manager 对注册引擎的调用方式
- Rust service 依赖的 register service 行为

### 原则 2：仅做职责迁移，不做流程策略改变

例如：

- `add_phone` fallback 逻辑迁移到 flow runner，但不改变分支顺序
- callback URL 提取逻辑迁移到 token resolver，但不改变成功/失败条件
- OAuth 参数构造仍沿用当前已修复版本

### 原则 3：主流程变短，辅助模块变清晰

重构完成后，`register.py` 应主要表现为：

1. 初始化上下文
2. 执行步骤
3. 调用辅助模块
4. 汇总结果

而不是继续承载大量页面判定和 token 细节。

---

## 拟议数据边界

### RegistrationEngine

保留现有实例字段，继续作为长期上下文容器，例如：

- `email`
- `password`
- `email_info`
- `oauth_start`
- `session`
- `session_token`
- `_is_existing_account`
- `_post_create_page_type`
- `_post_create_continue_url`
- `_cached_workspace_id`
- 日志与验证码等待配置

### Flow State

新增统一状态表示，至少覆盖：

- `page_type`
- `continue_url`
- `callback_url`
- `workspace_id`
- `payload`
- `current_url`（如需要）

如果不引入 dataclass，也应通过统一 helper 保证同一字段的提取规则一致。

### Token Resolution Result

统一 token 提取输出，至少覆盖：

- `access_token`
- `refresh_token`
- `id_token`
- `session_token`
- `workspace_id`
- `account_id`
- `cookies`

---

## 错误处理设计

本次不改业务语义，但会统一错误流向：

1. **页面解析失败**：由 `register_flow_state` 返回空值或标准错误。
2. **流程推进失败**：由 `register_flow_runner` 负责记录阶段信息。
3. **token 提取失败**：由 `register_token_resolver` 返回标准错误消息。
4. **重试判定**：由 `register_retry_policy` 统一决定是否可重试。
5. **最终对外错误**：仍由 `RegistrationEngine` 汇总为现有 `error_message`。

这样可以让日志里更容易区分：

- 失败在页面状态识别
- 失败在 continue_url 推进
- 失败在 callback/token 解析
- 失败在 workspace 提取

---

## 测试策略

本次重构至少补充以下验证：

### 1. 保留现有测试通过

重点确保当前相关测试继续通过，例如：

- `vendor/codex-register/tests/test_register_add_phone.py`
- `vendor/codex-register/tests/test_oauth_config.py`
- 现有与注册流程相关的回归测试

### 2. 新增模块级测试

建议新增：

- `vendor/codex-register/tests/test_register_flow_state.py`
  - 验证 `page.type` / `continue_url` / callback URL 提取
- `vendor/codex-register/tests/test_register_retry_policy.py`
  - 验证 retry / no-retry 判定

### 3. 语法与导入检查

至少执行：

- 新增模块 `py_compile`
- 目标测试文件运行

---

## 实施顺序

### 阶段 1：提取状态解析

先从 `register.py` 中抽出纯函数或轻量 dataclass：

- 页面类型提取
- continue_url 提取
- callback/workspace 提取辅助

### 阶段 2：提取 token 解析

把 callback / token / workspace 提取相关逻辑迁出到 token resolver。

### 阶段 3：提取重试策略

把文本匹配、错误类型判定、重试条件统一收口。

### 阶段 4：提取流程推进器

将主流程拆为可读步骤函数，`register.py` 只保留入口与装配逻辑。

### 阶段 5：回归测试与最小清理

确认行为未变，再做最小命名整理与死代码清理。

---

## 风险与控制

### 风险 1：拆分后循环依赖

控制方式：

- `register.py` 保持为组装入口
- 新模块只依赖最小公共类型
- 尽量以纯函数方式提取，不把复杂状态互相引用

### 风险 2：行为悄悄变化

控制方式：

- 先迁逻辑，再整理命名
- 不在同一轮顺手改分支策略
- 通过现有回归测试锁定行为

### 风险 3：日志内容变化影响排查

控制方式：

- 保持现有关键日志文本尽量不变
- 新模块只补充阶段标签，不删除原有关键信息

---

## 预期结果

重构完成后应达到：

1. `register.py` 显著变短，主流程更清晰。
2. 状态提取、token 提取、重试逻辑可单测。
3. 后续若继续迁入 `any-auto-register` 的状态机思路，可在现有边界上迭代。
4. 外部 API、前端、Rust service 不需要同步改造。

---

## 后续阶段建议（不在本次范围内）

本次完成后，后续可以再单独开两轮：

1. **策略增强轮**
   - 基于拆好的 flow runner，引入更清晰的 retry / fallback 策略

2. **任务调度增强轮**
   - 吸收 `any-auto-register` 的任务调度和并发节流模型

3. **统一模式配置轮**
   - 将 `standard`、`browserbase_ddg` 等模式收口到统一 register profile

