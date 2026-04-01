# Any-Auto Register Secondary Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在现有 codex-register 中并存一条 any-auto-register 风格的新注册模式，允许用户在前端单独选择，并优先尝试“会话复用直取 ChatGPT Session / AccessToken”的注册收敛方式。

**Architecture:** 新增 `any_auto` register mode，并在后端路由中接入独立 runner。该 runner 复用现有 `RegistrationEngine` 的前置注册步骤，在账号创建后优先尝试 any-auto 风格的 ChatGPT session 复用；失败时再回退现有 OAuth callback 收敛链路，确保新模式独立存在且不破坏标准模式与 Browserbase-DDG 模式。

**Tech Stack:** Python/FastAPI 路由、现有 curl_cffi 注册引擎、TypeScript/Next.js 前端弹窗与本地 node:test。

---

### Task 1: 定义模式常量与后端接入口

**Files:**
- Create: `vendor/codex-register/src/core/any_auto_register.py`
- Modify: `vendor/codex-register/src/web/routes/registration.py`
- Modify: `vendor/codex-register/src/database/models.py`

- [ ] 定义 `ANY_AUTO_REGISTER_MODE = "any_auto"` 与独立 runner。
- [ ] 在 `registration.py` 的 mode 归一化、任务执行分支中接入新模式。
- [ ] 保持数据库 `register_mode` 字段兼容第三种模式。

### Task 2: 实现 any-auto 风格会话复用收敛

**Files:**
- Create: `vendor/codex-register/src/core/any_auto_register.py`
- Modify: `vendor/codex-register/src/core/register.py`（仅在必要时复用现有 helper，不改标准流程行为）

- [ ] 复用现有注册前置步骤（IP 检查、邮箱、Sentinel、注册、验证码、创建账号）。
- [ ] 在创建账号后优先尝试访问 `https://chatgpt.com/` 并调用 `/api/auth/session` 提取 `access_token + session_token + account/workspace`。
- [ ] 若会话复用失败，则回退现有 OAuth callback 收敛逻辑。
- [ ] 结果对象沿用 `RegistrationResult`，允许新模式成功时仅提供 `access_token/session_token`。

### Task 3: 前端增加第三种注册通道

**Files:**
- Modify: `apps/src/components/modals/register-mode-options.ts`
- Modify: `apps/src/components/modals/add-account-modal.tsx`

- [ ] 新增 `any_auto` 注册通道 label 与类型。
- [ ] 在注册弹窗中增加第三个通道按钮。
- [ ] 提交参数时将新通道映射到后端 `registerMode: "any_auto"`。
- [ ] 保持 Outlook 批量等现有限制逻辑不被 Browserbase 模式污染。

### Task 4: 补最小回归测试与验证

**Files:**
- Create or Modify: `vendor/codex-register/tests/test_register_any_auto_mode.py`
- Modify: `apps/src/components/modals/register-mode-options.test.ts`

- [ ] 后端测试覆盖：`any_auto` mode 可归一化、可写入任务。
- [ ] 前端测试覆盖：新通道 label 与模式净化逻辑。
- [ ] 运行针对性 Python 单测。
- [ ] 运行前端本地测试与 `pnpm run build:desktop` 验证。
