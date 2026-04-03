import importlib.util
import sys
import types
import unittest
from dataclasses import dataclass
from pathlib import Path



def load_any_auto_module():
    module_name = "src.core.any_auto_register"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "core"
        / "any_auto_register.py"
    )

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg

    register_module = types.ModuleType("src.core.register")

    @dataclass
    class RegistrationResult:
        success: bool
        email: str = ""
        password: str = ""
        account_id: str = ""
        workspace_id: str = ""
        access_token: str = ""
        refresh_token: str = ""
        id_token: str = ""
        session_token: str = ""
        cookies: str = ""
        error_message: str = ""
        logs: list | None = None
        metadata: dict | None = None
        source: str = "register"
        account_status: str = "active"
        is_usable: bool = True

    class RegistrationEngine:
        pass

    register_module.RegistrationEngine = RegistrationEngine
    register_module.RegistrationResult = RegistrationResult
    sys.modules["src.core.register"] = register_module

    token_refresh_module = types.ModuleType("src.core.token_refresh")

    class TokenRefreshResult:
        def __init__(self, success=False, access_token="", refresh_token="", error_message=""):
            self.success = success
            self.access_token = access_token
            self.refresh_token = refresh_token
            self.error_message = error_message

    class TokenRefreshManager:
        def __init__(self, proxy_url=None):
            self.proxy_url = proxy_url

        def refresh_by_session_token(self, session_token):
            return TokenRefreshResult(success=False, error_message=f"unmocked:{session_token}")

    token_refresh_module.TokenRefreshManager = TokenRefreshManager
    token_refresh_module.TokenRefreshResult = TokenRefreshResult
    sys.modules["src.core.token_refresh"] = token_refresh_module

    sentinel_browser_module = types.ModuleType("src.core.sentinel_browser")
    sentinel_browser_module.fetch_browser_chatgpt_session_payload = lambda **_kwargs: None
    sys.modules["src.core.sentinel_browser"] = sentinel_browser_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


ANY_AUTO_MODULE = load_any_auto_module()
AnyAutoRegistrationRunner = ANY_AUTO_MODULE.AnyAutoRegistrationRunner
RegistrationResult = sys.modules["src.core.register"].RegistrationResult


class AnyAutoRegistrationRunnerTests(unittest.TestCase):
    def _build_runner(self, *, cookies: str, extracted_session_token: str | None = None):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        runner.engine = types.SimpleNamespace(
            password="pw-123",
            _serialize_session_cookies=lambda: cookies,
            _session_browser_cookies=lambda: [{"name": "cf_clearance", "value": "cookie", "domain": ".chatgpt.com", "path": "/"}],
            _is_existing_account=False,
            session_token=None,
            _clean_text=lambda value: str(value or "").strip(),
            _extract_session_token_from_cookies=lambda: extracted_session_token,
            _extract_workspace_id_from_token=lambda _token: "",
            _post_registration_health_check=lambda _token: ("active", True, ""),
            email_service=types.SimpleNamespace(service_type=types.SimpleNamespace(value="temp_mail")),
            proxy_url=None,
        )
        runner._log = lambda *_args, **_kwargs: None
        return runner

    def test_complete_result_appends_session_token_cookie_when_missing(self):
        runner = self._build_runner(cookies="cf_clearance=abc")
        result = RegistrationResult(success=False, session_token="sess-token-123")

        completed = runner._complete_result(result)

        self.assertEqual(
            completed.cookies,
            "cf_clearance=abc; __Secure-next-auth.session-token=sess-token-123",
        )

    def test_complete_result_does_not_duplicate_existing_session_cookie(self):
        runner = self._build_runner(
            cookies="cf_clearance=abc; __Secure-next-auth.session-token=existing-token",
        )
        result = RegistrationResult(success=False, session_token="sess-token-123")

        completed = runner._complete_result(result)

        self.assertEqual(
            completed.cookies,
            "cf_clearance=abc; __Secure-next-auth.session-token=existing-token",
        )

    def test_chatgpt_session_falls_back_to_session_token_refresh(self):
        runner = self._build_runner(cookies="cf_clearance=abc", extracted_session_token="sess-from-cookie")
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        original_manager = ANY_AUTO_MODULE.TokenRefreshManager

        class FakeRefreshResult:
            success = True
            access_token = "access-from-refresh"
            refresh_token = ""
            error_message = ""

        class FakeTokenRefreshManager:
            def __init__(self, proxy_url=None):
                self.proxy_url = proxy_url

            def refresh_by_session_token(self, session_token):
                self.session_token = session_token
                return FakeRefreshResult()

        try:
            ANY_AUTO_MODULE.TokenRefreshManager = FakeTokenRefreshManager

            payload = runner._recover_chatgpt_access_token(
                {"user": {"id": "user-1"}, "account": {"id": "acct-1"}}
            )
        finally:
            ANY_AUTO_MODULE.TokenRefreshManager = original_manager

        self.assertEqual(payload["accessToken"], "access-from-refresh")
        self.assertIn(
            ("warning", "ChatGPT Session 缺少 accessToken，尝试使用 session_token 刷新"),
            logs,
        )
        self.assertIn(
            ("info", "已通过 session_token 刷新补齐 accessToken"),
            logs,
        )

    def test_fetch_chatgpt_session_uses_browser_fallback_when_http_payload_incomplete(self):
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

        original_fetch = ANY_AUTO_MODULE.fetch_browser_chatgpt_session_payload

        try:
            captured = {}

            def fake_fetch_browser_chatgpt_session_payload(**kwargs):
                captured.update(kwargs)
                return {
                    "user": {"id": "user-1"},
                    "account": {"id": "acct-1"},
                    "accessToken": "browser-access",
                    "sessionToken": "browser-session",
                }

            ANY_AUTO_MODULE.fetch_browser_chatgpt_session_payload = fake_fetch_browser_chatgpt_session_payload

            payload, error_message = runner._fetch_chatgpt_session_payload()
        finally:
            ANY_AUTO_MODULE.fetch_browser_chatgpt_session_payload = original_fetch

        self.assertEqual(payload["accessToken"], "browser-access")
        self.assertEqual(error_message, "")
        self.assertEqual(
            captured["cookies"],
            [{"name": "cf_clearance", "value": "cookie", "domain": ".chatgpt.com", "path": "/"}],
        )
        self.assertIn(
            ("warning", "HTTP ChatGPT Session 不完整，尝试浏览器会话回退"),
            logs,
        )


if __name__ == "__main__":
    unittest.main()
