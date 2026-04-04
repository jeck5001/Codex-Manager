# Any-Auto Session Simplification Implementation Plan

I'm using the writing-plans skill to create the implementation plan.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Simplify the Any-Auto runner so ChatGPT session probing makes exactly one HTTP attempt and no longer falls back to the browser session refresher while updating regressions accordingly.

**Architecture:** Keep the runner’s session-probing helper limited to a single `GET /api/auth/session` attempt right after landing the ChatGPT home page, remove the `?model=auto` trial and browser fallback, and bake a descriptive regression test that enforces the simpler flow.

**Tech Stack:** Python 3.11 (unittest), pnpm (Next.js/Tauri desktop build), pnpm-managed apps, Tailwind-based UI unaffected.

---
### Task 1: Prove the single-probe expectation

**Files:**
- Modify: `vendor/codex-register/tests/test_any_auto_register_runner.py` (near the `_fetch_chatgpt_session_payload` tests)
- Test command: `python3 -m unittest vendor/codex-register/tests/test_any_auto_register_runner.py -k single_http_probe`

- [ ] Step 1: Add the regression test definition that tracks only `https://chatgpt.com/` and `https://chatgpt.com/api/auth/session` without logging the browser fallback warning. Insert the following function into `AnyAutoRegistrationRunnerTests` next to the other session tests:

```python
    def test_fetch_chatgpt_session_uses_single_http_probe_without_browser_fallback(self):
        class FakeResponse:
            def __init__(self, status_code, payload=None, url="https://chatgpt.com/"):
                self.status_code = status_code
                self._payload = payload or {}
                self.url = url

            def json(self):
                return self._payload

        class FakeSession:
            def __init__(self):
                self.calls = []

            def get(self, url, **kwargs):
                self.calls.append(url)
                if "api/auth/session" in url:
                    return FakeResponse(200, {"user": {"id": "user-1"}, "account": {"id": "acct-1"}})
                return FakeResponse(200, url=url)

        runner = self._build_runner(cookies="cf_clearance=abc", extracted_session_token=None)
        runner.engine.session = FakeSession()
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        payload, error_message = runner._fetch_chatgpt_session_payload()

        self.assertIsNone(payload)
        self.assertEqual(error_message, "ChatGPT Session 接口未返回 accessToken")
        self.assertEqual(
            runner.engine.session.calls,
            ["https://chatgpt.com/", "https://chatgpt.com/api/auth/session"],
        )
        self.assertNotIn(
            ("warning", "HTTP ChatGPT Session 不完整，尝试浏览器会话回退"),
            logs,
        )
```
- [ ] Step 2: Run the focused test with `python3 -m unittest vendor/codex-register/tests/test_any_auto_register_runner.py -k single_http_probe`. Expected: FAIL because the current flow still hits `https://chatgpt.com/?model=auto` and invokes the browser fallback path before returning the error.

### Task 2: Simplify `_fetch_chatgpt_session_payload`

**Files:**
- Modify: `vendor/codex-register/src/core/any_auto_register.py` (the `_fetch_chatgpt_session_payload` implementation around line 269)
- Update: `vendor/codex-register/tests/test_any_auto_register_runner.py` to stop relying on browser fallback logging from the old test
- Tests: `python3 -m unittest vendor/codex-register/tests/test_any_auto_register_runner.py`

- [ ] Step 1: Replace the existing `_fetch_chatgpt_session_payload` loop with the single-probe implementation below. Keep the logging helpers (`_log_chatgpt_session_payload_summary`, `_log_chatgpt_session_cookie_summary`) intact while removing any `fetch_browser_chatgpt_session_payload` calls.

```python
    def _fetch_chatgpt_session_payload(self) -> Tuple[Optional[Dict[str, Any]], str]:
        session = getattr(self.engine, "session", None)
        if session is None:
            return None, "未初始化 HTTP 会话"

        try:
            self._log(f"尝试落地 ChatGPT 会话: {CHATGPT_HOME_URL}")
            response = session.get(
                CHATGPT_HOME_URL,
                headers={
                    "accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                    "referer": "https://auth.openai.com/",
                },
                allow_redirects=True,
                timeout=30,
            )
            final_url = self.engine._clean_text(getattr(response, "url", ""))
            if final_url:
                self._log(f"ChatGPT 会话落地 URL: {final_url[:120]}...")

            response = session.get(
                CHATGPT_SESSION_URL,
                headers={
                    "accept": "application/json",
                    "referer": CHATGPT_HOME_URL,
                },
                timeout=30,
            )
            self._log(f"ChatGPT Session 接口状态: {response.status_code}")
            if response.status_code != 200:
                return None, f"ChatGPT Session 接口返回 HTTP {response.status_code}"

            payload = response.json()
            recovered_payload = self._recover_chatgpt_access_token(payload)
            if recovered_payload:
                return recovered_payload, ""

            self._log_chatgpt_session_payload_summary(payload)
            self._log_chatgpt_session_cookie_summary()
            return None, "ChatGPT Session 接口未返回 accessToken"
        except Exception as exc:
            return None, f"读取 ChatGPT Session 失败: {exc}"
```
- [ ] Step 2: Update the pre-existing `test_fetch_chatgpt_session_uses_browser_fallback_when_http_payload_incomplete` so it no longer hijacks `fetch_browser_chatgpt_session_payload`. Rename or reframe the test to assert that incomplete HTTP payloads now just log that the session was incomplete and return the error without invoking any browser helpers. For example, remove the fake browser fetch assignment and assert that `_log` never sees the warning about browser fallback. Keep the payload summary assertions if they still make sense.
- [ ] Step 3: Run `python3 -m unittest vendor/codex-register/tests/test_any_auto_register_runner.py` to ensure both the new single-probe test and the updated legacy test pass now that the fallback is retired.

### Task 3: Run broader regression suites and desktop build

**Files:** n/a (test/build commands only)

- [ ] Step 1: Run `python3 -m unittest vendor/codex-register/tests/test_any_auto_register_runner.py` to double-check the targeted coverage after the code changes. Expect the suite to pass with zero failures.
- [ ] Step 2: Run `python3 -m unittest vendor/codex-register/tests/test_register_add_phone.py vendor/codex-register/tests/test_register_flow_runner.py` (this matches the user’s requested regression targets). Expect PASS.
- [ ] Step 3: Run `pnpm --dir apps run build:desktop` to verify the desktop build still succeeds after the backend tweak. Expect exit code 0.

### Task 4: Stage and commit the simplification

**Files:**
- Stage: `vendor/codex-register/src/core/any_auto_register.py`, `vendor/codex-register/tests/test_any_auto_register_runner.py`, `docs/superpowers/plans/2026-04-04-any-auto-session-simplification.md`
- Commit: `git commit -m "Simplify Any-Auto session probing"`

- [ ] Step 1: `git add vendor/codex-register/src/core/any_auto_register.py vendor/codex-register/tests/test_any_auto_register_runner.py docs/superpowers/plans/2026-04-04-any-auto-session-simplification.md`
- [ ] Step 2: `git commit -m "Simplify Any-Auto session probing"`

Plan complete and saved to `docs/superpowers/plans/2026-04-04-any-auto-session-simplification.md`. Execution choice: Inline Execution (superpowers:executing-plans) because I am implementing the steps directly in this session.
