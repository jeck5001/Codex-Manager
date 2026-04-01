# Codex Register Core Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不改变现有外部行为的前提下，将 `vendor/codex-register/src/core/register.py` 拆分为状态解析、流程推进、重试策略、token 解析四个内部模块，并保留 `RegistrationEngine` 兼容入口。

**Architecture:** 保留 `register.py` 作为对外入口与上下文容器，先抽出纯函数级能力，再引入轻量流程 runner，最后把原入口改为组装层。测试优先锁定页面状态提取、重试判定与 token 解析，回归测试用于确保 add_phone / OAuth 相关行为不变。

**Tech Stack:** Python 3、curl_cffi、unittest、现有 `vendor/codex-register` 测试体系。

---

## File Map

### Create
- `vendor/codex-register/src/core/register_flow_state.py` — 统一提取 `page_type`、`continue_url`、`callback_url`、`workspace_id` 等认证状态。
- `vendor/codex-register/src/core/register_retry_policy.py` — 统一放置注册流程中的 retry / non-retry 判定。
- `vendor/codex-register/src/core/register_token_resolver.py` — 统一放置 callback、token、cookies、workspace 解析。
- `vendor/codex-register/src/core/register_flow_runner.py` — 承载分步骤流程推进逻辑，供 `RegistrationEngine` 调用。
- `vendor/codex-register/tests/test_register_flow_state.py` — 状态提取模块测试。
- `vendor/codex-register/tests/test_register_retry_policy.py` — 重试策略模块测试。
- `vendor/codex-register/tests/test_register_token_resolver.py` — token 解析模块测试。

### Modify
- `vendor/codex-register/src/core/register.py` — 保留入口，改为依赖新模块。
- `vendor/codex-register/tests/test_register_add_phone.py` — 如需补最小回归用例，确认重构后 add_phone 分支仍走旧行为。

---

### Task 1: 提取认证状态解析模块

**Files:**
- Create: `vendor/codex-register/src/core/register_flow_state.py`
- Create: `vendor/codex-register/tests/test_register_flow_state.py`
- Modify: `vendor/codex-register/src/core/register.py`

- [ ] **Step 1: 写失败测试，锁定状态提取行为**

```python
import unittest

from src.core.register_flow_state import (
    extract_auth_page_type,
    extract_auth_continue_url,
    extract_callback_url,
    extract_workspace_id,
)


class RegisterFlowStateTests(unittest.TestCase):
    def test_extract_auth_page_type_prefers_nested_page_type(self):
        payload = {
            "page": {"type": "create_account_password"},
            "type": "ignored",
        }
        self.assertEqual(extract_auth_page_type(payload), "create_account_password")

    def test_extract_auth_continue_url_supports_multiple_aliases(self):
        payload = {"redirect_url": "https://auth.openai.com/email-verification"}
        self.assertEqual(
            extract_auth_continue_url(payload),
            "https://auth.openai.com/email-verification",
        )

    def test_extract_callback_url_reads_code_from_absolute_url(self):
        url = "http://localhost:1455/auth/callback?code=abc123&state=state123"
        self.assertEqual(extract_callback_url(url), url)

    def test_extract_workspace_id_prefers_payload_value(self):
        payload = {"workspace_id": "ws_123"}
        self.assertEqual(extract_workspace_id(payload), "ws_123")
```

- [ ] **Step 2: 运行测试，确认当前失败**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register && python3 -m unittest tests/test_register_flow_state.py
```

Expected: FAIL，提示 `register_flow_state` 模块或目标函数不存在。

- [ ] **Step 3: 写最小实现，承接 `register.py` 里已有提取逻辑**

```python
from typing import Any


def _clean_text(value: Any) -> str:
    if value is None:
        return ""
    return str(value).strip()


def extract_auth_page_type(payload: Any) -> str:
    if not isinstance(payload, dict):
        return ""
    page = payload.get("page")
    if isinstance(page, dict):
        page_type = _clean_text(page.get("type"))
        if page_type:
            return page_type
    return _clean_text(payload.get("type"))


def extract_auth_continue_url(payload: Any) -> str:
    if not isinstance(payload, dict):
        return ""
    return _clean_text(
        payload.get("continue_url")
        or payload.get("redirect_url")
        or payload.get("callback_url")
        or payload.get("next_url")
    )
```

- [ ] **Step 4: 让 `register.py` 复用新模块，不保留重复实现**

```python
from .register_flow_state import (
    extract_auth_continue_url,
    extract_auth_page_type,
    extract_callback_url,
    extract_workspace_id,
)

# 将 RegistrationEngine._extract_auth_page_type 改成直接委托

def _extract_auth_page_type(self, payload: Any) -> str:
    return extract_auth_page_type(payload)
```

- [ ] **Step 5: 重新运行测试，确认通过**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register && python3 -m unittest tests/test_register_flow_state.py
```

Expected: PASS。

- [ ] **Step 6: 提交阶段性 commit**

```bash
git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager add \
  vendor/codex-register/src/core/register_flow_state.py \
  vendor/codex-register/src/core/register.py \
  vendor/codex-register/tests/test_register_flow_state.py

git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager commit -m "refactor: extract register flow state helpers"
```

### Task 2: 提取重试策略模块

**Files:**
- Create: `vendor/codex-register/src/core/register_retry_policy.py`
- Create: `vendor/codex-register/tests/test_register_retry_policy.py`
- Modify: `vendor/codex-register/src/core/register.py`

- [ ] **Step 1: 写失败测试，锁定 retry 判定**

```python
import unittest

from src.core.register_retry_policy import should_retry_register_error


class RegisterRetryPolicyTests(unittest.TestCase):
    def test_retries_transient_authorize_failure(self):
        self.assertTrue(should_retry_register_error("authorize continue failed"))

    def test_retries_email_otp_failure(self):
        self.assertTrue(should_retry_register_error("邮箱验证码超时"))

    def test_does_not_retry_missing_email_service(self):
        self.assertFalse(should_retry_register_error("no available email service"))
```

- [ ] **Step 2: 运行测试，确认失败**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register && python3 -m unittest tests/test_register_retry_policy.py
```

Expected: FAIL，提示模块不存在。

- [ ] **Step 3: 写最小实现，先把文本判定独立出来**

```python
RETRIABLE_MARKERS = (
    "authorize",
    "otp",
    "验证码",
    "workspace",
    "session",
    "access token",
)

NON_RETRIABLE_MARKERS = (
    "no available email service",
    "missing recoverable email",
)


def should_retry_register_error(message: str) -> bool:
    text = str(message or "").strip().lower()
    if not text:
        return False
    if any(marker in text for marker in NON_RETRIABLE_MARKERS):
        return False
    return any(marker in text for marker in RETRIABLE_MARKERS)
```

- [ ] **Step 4: 将 `register.py` 里已有 retry 判定逻辑迁移到策略模块**

```python
from .register_retry_policy import should_retry_register_error

# 原 register.py 内部相关逻辑改成：
if should_retry_register_error(last_error):
    ...
```

- [ ] **Step 5: 重新运行测试，确认通过**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register && python3 -m unittest tests/test_register_retry_policy.py
```

Expected: PASS。

- [ ] **Step 6: 提交阶段性 commit**

```bash
git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager add \
  vendor/codex-register/src/core/register_retry_policy.py \
  vendor/codex-register/src/core/register.py \
  vendor/codex-register/tests/test_register_retry_policy.py

git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager commit -m "refactor: extract register retry policy"
```

### Task 3: 提取 token / callback 解析模块

**Files:**
- Create: `vendor/codex-register/src/core/register_token_resolver.py`
- Create: `vendor/codex-register/tests/test_register_token_resolver.py`
- Modify: `vendor/codex-register/src/core/register.py`

- [ ] **Step 1: 写失败测试，锁定 callback 与 workspace 提取结果**

```python
import unittest

from src.core.register_token_resolver import (
    resolve_callback_url,
    resolve_workspace_id_from_tokens,
)


class RegisterTokenResolverTests(unittest.TestCase):
    def test_resolve_callback_url_accepts_absolute_callback(self):
        callback = "http://localhost:1455/auth/callback?code=code123&state=state123"
        self.assertEqual(resolve_callback_url(callback), callback)

    def test_resolve_workspace_id_from_tokens_prefers_explicit_workspace(self):
        payload = {"workspace_id": "ws_explicit", "account_id": "acct_1"}
        resolved = resolve_workspace_id_from_tokens(payload)
        self.assertEqual(resolved, "ws_explicit")
```

- [ ] **Step 2: 运行测试，确认失败**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register && python3 -m unittest tests/test_register_token_resolver.py
```

Expected: FAIL，提示模块不存在。

- [ ] **Step 3: 写最小实现，集中 callback / workspace 解析**

```python
from typing import Any


def resolve_callback_url(candidate: str) -> str:
    text = str(candidate or "").strip()
    if "code=" in text and "state=" in text:
        return text
    return ""


def resolve_workspace_id_from_tokens(payload: Any) -> str:
    if not isinstance(payload, dict):
        return ""
    for key in ("workspace_id", "workspaceId", "org_id", "organization_id"):
        value = str(payload.get(key, "") or "").strip()
        if value:
            return value
    return ""
```

- [ ] **Step 4: 将 `register.py` 中 callback / workspace 提取逻辑委托到新模块**

```python
from .register_token_resolver import (
    resolve_callback_url,
    resolve_workspace_id_from_tokens,
)

callback_url = resolve_callback_url(candidate_url)
workspace_id = resolve_workspace_id_from_tokens(token_payload)
```

- [ ] **Step 5: 重新运行测试，确认通过**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register && python3 -m unittest tests/test_register_token_resolver.py
```

Expected: PASS。

- [ ] **Step 6: 提交阶段性 commit**

```bash
git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager add \
  vendor/codex-register/src/core/register_token_resolver.py \
  vendor/codex-register/src/core/register.py \
  vendor/codex-register/tests/test_register_token_resolver.py

git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager commit -m "refactor: extract register token resolver"
```

### Task 4: 提取流程推进器并收缩 `register.py`

**Files:**
- Create: `vendor/codex-register/src/core/register_flow_runner.py`
- Modify: `vendor/codex-register/src/core/register.py`
- Test: `vendor/codex-register/tests/test_register_add_phone.py`

- [ ] **Step 1: 写失败测试，锁定 add_phone 回退主路径仍可调用**

```python
import unittest
from unittest.mock import Mock

from src.core.register_flow_runner import RegisterFlowRunner


class RegisterFlowRunnerTests(unittest.TestCase):
    def test_runner_returns_auth_resolution_result_shape(self):
        engine = Mock()
        runner = RegisterFlowRunner(engine)
        result = runner.resolve_auth_result({"page": {"type": "add_phone"}})
        self.assertEqual(result.page_type, "add_phone")
```

- [ ] **Step 2: 运行测试，确认失败**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register && python3 -m unittest tests/test_register_add_phone.py tests/test_register_flow_state.py tests/test_register_retry_policy.py tests/test_register_token_resolver.py
```

Expected: FAIL，提示 `RegisterFlowRunner` 不存在或 `AuthResolutionResult` 接口未接好。

- [ ] **Step 3: 写最小流程推进器骨架，先只托管认证结果归一化**

```python
from .register import AuthResolutionResult
from .register_flow_state import extract_auth_continue_url, extract_auth_page_type
from .register_token_resolver import resolve_callback_url


class RegisterFlowRunner:
    def __init__(self, engine):
        self.engine = engine

    def resolve_auth_result(self, payload):
        page_type = extract_auth_page_type(payload)
        continue_url = extract_auth_continue_url(payload)
        callback_url = resolve_callback_url(continue_url)
        return AuthResolutionResult(
            callback_url=callback_url or None,
            page_type=page_type,
            continue_url=continue_url,
        )
```

- [ ] **Step 4: 将 `register.py` 中认证结果推进相关逻辑迁移到 runner，入口只保留委托**

```python
from .register_flow_runner import RegisterFlowRunner

self.flow_runner = RegisterFlowRunner(self)

# 原有逻辑收缩为：
resolution = self.flow_runner.resolve_auth_result(payload)
```

- [ ] **Step 5: 运行回归测试，确认 add_phone 与 OAuth 配置未回退**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager && python3 -m unittest \
  vendor/codex-register/tests/test_register_add_phone.py \
  vendor/codex-register/tests/test_oauth_config.py \
  vendor/codex-register/tests/test_register_flow_state.py \
  vendor/codex-register/tests/test_register_retry_policy.py \
  vendor/codex-register/tests/test_register_token_resolver.py
```

Expected: PASS。

- [ ] **Step 6: 提交阶段性 commit**

```bash
git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager add \
  vendor/codex-register/src/core/register_flow_runner.py \
  vendor/codex-register/src/core/register.py \
  vendor/codex-register/tests/test_register_add_phone.py \
  vendor/codex-register/tests/test_register_flow_state.py \
  vendor/codex-register/tests/test_register_retry_policy.py \
  vendor/codex-register/tests/test_register_token_resolver.py

git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager commit -m "refactor: extract register flow runner"
```

### Task 5: 最终回归与清理

**Files:**
- Modify: `vendor/codex-register/src/core/register.py`
- Modify: `vendor/codex-register/src/core/register_flow_state.py`
- Modify: `vendor/codex-register/src/core/register_retry_policy.py`
- Modify: `vendor/codex-register/src/core/register_token_resolver.py`
- Modify: `vendor/codex-register/src/core/register_flow_runner.py`

- [ ] **Step 1: 运行语法检查，确认新模块可导入**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager && python3 -m py_compile \
  vendor/codex-register/src/core/register.py \
  vendor/codex-register/src/core/register_flow_state.py \
  vendor/codex-register/src/core/register_retry_policy.py \
  vendor/codex-register/src/core/register_token_resolver.py \
  vendor/codex-register/src/core/register_flow_runner.py
```

Expected: 无输出，退出码 0。

- [ ] **Step 2: 运行目标测试集，确认重构未破坏现有行为**

Run:
```bash
cd /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager && python3 -m unittest \
  vendor/codex-register/tests/test_register_add_phone.py \
  vendor/codex-register/tests/test_oauth_config.py \
  vendor/codex-register/tests/test_register_flow_state.py \
  vendor/codex-register/tests/test_register_retry_policy.py \
  vendor/codex-register/tests/test_register_token_resolver.py
```

Expected: 全部 PASS。

- [ ] **Step 3: 检查 `register.py` 体积是否明显缩小，确认入口职责已经收缩**

Run:
```bash
wc -l /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/core/register.py
```

Expected: 行数相较当前 2226 行明显下降，且核心提取函数已迁出。

- [ ] **Step 4: 提交最终 commit**

```bash
git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager add \
  vendor/codex-register/src/core/register.py \
  vendor/codex-register/src/core/register_flow_state.py \
  vendor/codex-register/src/core/register_retry_policy.py \
  vendor/codex-register/src/core/register_token_resolver.py \
  vendor/codex-register/src/core/register_flow_runner.py \
  vendor/codex-register/tests/test_register_flow_state.py \
  vendor/codex-register/tests/test_register_retry_policy.py \
  vendor/codex-register/tests/test_register_token_resolver.py \
  vendor/codex-register/tests/test_register_add_phone.py

git -C /Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager commit -m "refactor: split codex register core flow modules"
```

---

## Self-Review

### Spec coverage
- 状态解析：Task 1 覆盖。
- 重试判定：Task 2 覆盖。
- token / callback / workspace 解析：Task 3 覆盖。
- flow runner 与入口组装：Task 4 覆盖。
- 兼容回归与语法检查：Task 5 覆盖。

### Placeholder scan
- 未使用 TBD / TODO / “后续补充”。
- 每个代码步骤都给了目标代码骨架。
- 每个验证步骤都给了明确命令和预期结果。

### Type consistency
- 新模块命名统一为：`register_flow_state` / `register_retry_policy` / `register_token_resolver` / `register_flow_runner`。
- 入口仍为 `RegistrationEngine` 与 `AuthResolutionResult`。
- 测试用到的方法名与模块名已在各任务中保持一致。

