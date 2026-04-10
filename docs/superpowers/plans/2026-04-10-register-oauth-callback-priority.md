# Register OAuth Callback Priority Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make OpenAI/Codex registration prefer OAuth consent and callback convergence over add-phone-specific branching.

**Architecture:** Keep the existing HTTP registration pipeline, but simplify the post-registration authorization state machine. Treat `add_phone` as a low-priority state and continue trying `continue_url`, workspace selection, and authenticated OAuth fallback before surfacing failure.

**Tech Stack:** Python 3, pytest

---

### Task 1: Simplify RegisterFlowRunner Add-Phone Branching

**Files:**
- Modify: `vendor/codex-register/src/core/register_flow_runner.py`
- Test: `vendor/codex-register/tests/test_register_flow_runner.py`

- [ ] **Step 1: Update add-phone continue-url handling**

Change `/add-phone` handling so it first tries `advance_workspace_authorization(...)` and then falls back to `_build_authenticated_oauth_url()` plus `_follow_redirects(...)`.

- [ ] **Step 2: Remove add-phone short-circuit behavior**

Allow:

- `complete_login_email_otp_verification()` to continue OAuth fallback after `page_type == "add_phone"`
- `resolve_auth_result()` to continue resolving `continue_url` even when the page type is `add_phone`
- `resolve_post_registration_callback()` to skip preemptive add-phone bypass and go directly into workspace/OAuth convergence

- [ ] **Step 3: Update flow-runner tests**

Assert that:

- post-registration callback resolution ignores add-phone bypass priority
- add-phone still falls back to authenticated OAuth URL when needed
- login OTP recovery can still return a callback after an intermediate add-phone result

- [ ] **Step 4: Run targeted flow-runner tests**

Run: `python3 -m pytest vendor/codex-register/tests/test_register_flow_runner.py -q`

Expected: PASS

### Task 2: Remove Add-Phone Priority From Any-Auto Entry

**Files:**
- Modify: `vendor/codex-register/src/core/any_auto_register.py`
- Test: `vendor/codex-register/tests/test_any_auto_register_runner.py`

- [ ] **Step 1: Remove pre-session add-phone bypass**

Delete the Any-Auto branch that runs `_attempt_add_phone_login_bypass(...)` before ChatGPT session reuse.

- [ ] **Step 2: Update Any-Auto test expectations**

Assert that Any-Auto still probes current-session reuse first even when post-create state is `add_phone`.

- [ ] **Step 3: Run targeted Any-Auto tests**

Run: `python3 -m pytest vendor/codex-register/tests/test_any_auto_register_runner.py -q`

Expected: PASS

### Task 3: Verify Regression Coverage

**Files:**
- Test: `vendor/codex-register/tests/test_register_add_phone.py`

- [ ] **Step 1: Run add-phone regression tests**

Run: `python3 -m pytest vendor/codex-register/tests/test_register_add_phone.py -q`

Expected: PASS

- [ ] **Step 2: Run combined verification**

Run: `python3 -m pytest vendor/codex-register/tests/test_register_flow_runner.py vendor/codex-register/tests/test_any_auto_register_runner.py vendor/codex-register/tests/test_register_add_phone.py -q`

Expected: PASS
