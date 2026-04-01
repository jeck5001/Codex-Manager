# Codex Register Core Refactor Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 继续把 `register.py` 中真正的认证结果推进逻辑下沉到 `register_flow_runner.py`，进一步收缩入口文件但不改变外部行为。

**Architecture:** 在现有 `register_flow_runner.py` 的基础上，新增 `resolve_callback_from_auth_page` 与 `resolve_callback_from_continue_url` 两类能力；`register.py` 改成委托调用。用新单测锁定 token_exchange、workspace、external_url、add_phone 等关键分支行为。

**Tech Stack:** Python 3、unittest、现有 `vendor/codex-register` 测试体系。

---

## File Map

### Modify
- `vendor/codex-register/src/core/register_flow_runner.py` — 增加 auth page / continue_url 推进逻辑。
- `vendor/codex-register/src/core/register.py` — 改为委托 flow runner。
- `vendor/codex-register/tests/test_register_flow_runner.py` — 补覆盖关键 page type / continue_url 分支。

---

### Task 1: 提取 auth page 推进逻辑

**Files:**
- Modify: `vendor/codex-register/src/core/register_flow_runner.py`
- Modify: `vendor/codex-register/tests/test_register_flow_runner.py`
- Modify: `vendor/codex-register/src/core/register.py`

- [ ] **Step 1: 先补失败测试，覆盖关键分支**
- [ ] **Step 2: 跑测试确认红**
- [ ] **Step 3: 在 `register_flow_runner.py` 实现 `resolve_callback_from_auth_page` 与 `resolve_callback_from_continue_url`**
- [ ] **Step 4: 让 `register.py` 改为委托 flow runner**
- [ ] **Step 5: 跑回归测试确认绿**

### Task 2: 最终回归

**Files:**
- Modify: `vendor/codex-register/src/core/register.py`
- Modify: `vendor/codex-register/src/core/register_flow_runner.py`
- Modify: `vendor/codex-register/tests/test_register_flow_runner.py`

- [ ] **Step 1: 跑 register 相关单测**
- [ ] **Step 2: 跑 `py_compile`**
- [ ] **Step 3: 检查 `register.py` 行数继续下降**
- [ ] **Step 4: 提交 commit**
