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
            _build_authenticated_oauth_url=lambda: "https://auth.openai.com/oauth/authorize?client_id=test&state=state",
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
            captured["auth_url"],
            "https://auth.openai.com/oauth/authorize?client_id=test&state=state",
        )
        self.assertEqual(
            captured["cookies"],
            [{"name": "cf_clearance", "value": "cookie", "domain": ".chatgpt.com", "path": "/"}],
        )
        self.assertIn(
            ("warning", "HTTP ChatGPT Session 不完整，尝试浏览器会话回退"),
            logs,
        )
        self.assertIn(
            (
                "warning",
                'ChatGPT Session 响应摘要: {"has_accessToken": false, "has_sessionToken": false, "authProvider": "", "expires": "", "user_keys": ["id"], "account_keys": ["id"]}',
            ),
            logs,
        )
        self.assertIn(
            (
                "warning",
                "ChatGPT Session 相关 Cookies: cf_clearance",
            ),
            logs,
        )

    def test_run_prefers_oauth_resolution_when_post_create_page_is_consent(self):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        engine = types.SimpleNamespace()
        engine.logs = []
        engine.email = "demo@example.com"
        engine.password = "pw-123"
        engine.proxy_url = None
        engine.email_service = types.SimpleNamespace(service_type=types.SimpleNamespace(value="temp_mail"))
        engine._is_existing_account = False
        engine._post_create_page_type = "sign_in_with_chatgpt_codex_consent"
        engine._post_create_continue_url = "https://auth.openai.com/sign-in-with-chatgpt/codex/consent"
        engine._check_ip_location = lambda: (True, "US")
        engine._create_email = lambda: True
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did-1"
        engine._check_sentinel = lambda _did: "sentinel"
        engine._submit_signup_form = lambda *_args: types.SimpleNamespace(success=True)
        engine._register_password = lambda: (True, "pw-123")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda _code: True
        engine._create_user_account = lambda: True
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acct-1",
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "id_token": "id-1",
        }
        engine._clean_text = lambda value: str(value or "").strip()
        engine._serialize_session_cookies = lambda: ""
        engine._extract_session_token_from_cookies = lambda: ""
        engine._extract_workspace_id_from_token = lambda _token: ""
        engine._post_registration_health_check = lambda _token: ("active", True, "")

        flow_runner = types.SimpleNamespace(
            resolve_post_registration_callback=lambda _did, _sen: types.SimpleNamespace(
                callback_url="http://localhost:1455/auth/callback?code=ok&state=state",
                workspace_id="acct-1",
                metadata=None,
                error_message="",
            )
        )
        engine._get_flow_runner = lambda: flow_runner
        runner.engine = engine

        original_fetch = runner._fetch_chatgpt_session_payload
        runner._fetch_chatgpt_session_payload = lambda: (_ for _ in ()).throw(AssertionError("should not probe chatgpt session"))

        try:
            result = runner.run()
        finally:
            runner._fetch_chatgpt_session_payload = original_fetch

        self.assertTrue(result.success)
        self.assertEqual(result.access_token, "access-1")
        self.assertIn(("info", "13. 已命中 OAuth 授权页，直接走 OAuth 收敛"), logs)

    def test_run_prefers_oauth_when_continue_url_looks_like_oauth(self):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        engine = types.SimpleNamespace()
        engine.logs = []
        engine.email = "demo@example.com"
        engine.password = "pw-123"
        engine.proxy_url = None
        engine.email_service = types.SimpleNamespace(service_type=types.SimpleNamespace(value="temp_mail"))
        engine._is_existing_account = False
        engine._post_create_page_type = ""
        engine._post_create_continue_url = "https://auth.openai.com/sign-in-with-chatgpt/codex/workspace?state=abc"
        engine._check_ip_location = lambda: (True, "US")
        engine._create_email = lambda: True
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did-1"
        engine._check_sentinel = lambda _did: "sentinel"
        engine._submit_signup_form = lambda *_args: types.SimpleNamespace(success=True)
        engine._register_password = lambda: (True, "pw-123")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda _code: True
        engine._create_user_account = lambda: True
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acct-1",
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "id_token": "id-1",
        }
        engine._clean_text = lambda value: str(value or "").strip()
        engine._serialize_session_cookies = lambda: ""
        engine._extract_session_token_from_cookies = lambda: ""
        engine._extract_workspace_id_from_token = lambda _token: ""
        engine._post_registration_health_check = lambda _token: ("active", True, "")

        flow_runner = types.SimpleNamespace(
            resolve_post_registration_callback=lambda _did, _sen: types.SimpleNamespace(
                callback_url="http://localhost:1455/auth/callback?code=ok&state=state",
                workspace_id="acct-1",
                metadata=None,
                error_message="",
            )
        )
        engine._get_flow_runner = lambda: flow_runner
        runner.engine = engine

        original_fetch = runner._fetch_chatgpt_session_payload
        runner._fetch_chatgpt_session_payload = lambda: (_ for _ in ()).throw(AssertionError("should not probe chatgpt session"))

        try:
            result = runner.run()
        finally:
            runner._fetch_chatgpt_session_payload = original_fetch

        self.assertTrue(result.success)
        self.assertEqual(result.access_token, "access-1")
        self.assertIn(("info", "13. 已命中 OAuth 授权页，直接走 OAuth 收敛"), logs)

    def test_run_prefers_oauth_when_continue_url_is_callback(self):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        engine = types.SimpleNamespace()
        engine.logs = []
        engine.email = "demo@example.com"
        engine.password = "pw-123"
        engine.proxy_url = None
        engine.email_service = types.SimpleNamespace(service_type=types.SimpleNamespace(value="temp_mail"))
        engine._is_existing_account = False
        engine._post_create_page_type = ""
        engine._post_create_continue_url = "http://localhost:1455/auth/callback?code=cb123&state=state"
        engine._check_ip_location = lambda: (True, "US")
        engine._create_email = lambda: True
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did-1"
        engine._check_sentinel = lambda _did: "sentinel"
        engine._submit_signup_form = lambda *_args: types.SimpleNamespace(success=True)
        engine._register_password = lambda: (True, "pw-123")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda _code: True
        engine._create_user_account = lambda: True
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acct-1",
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "id_token": "id-1",
        }
        engine._clean_text = lambda value: str(value or "").strip()
        engine._serialize_session_cookies = lambda: ""
        engine._extract_session_token_from_cookies = lambda: ""
        engine._extract_workspace_id_from_token = lambda _token: ""
        engine._post_registration_health_check = lambda _token: ("active", True, "")

        flow_runner = types.SimpleNamespace(
            resolve_post_registration_callback=lambda _did, _sen: types.SimpleNamespace(
                callback_url="http://localhost:1455/auth/callback?code=ok&state=state",
                workspace_id="acct-1",
                metadata=None,
                error_message="",
            )
        )
        engine._get_flow_runner = lambda: flow_runner
        runner.engine = engine

        original_fetch = runner._fetch_chatgpt_session_payload
        runner._fetch_chatgpt_session_payload = lambda: (_ for _ in ()).throw(AssertionError("should not probe chatgpt session"))

        try:
            result = runner.run()
        finally:
            runner._fetch_chatgpt_session_payload = original_fetch

        self.assertTrue(result.success)
        self.assertEqual(result.access_token, "access-1")
        self.assertIn(("info", "13. 已命中 OAuth 授权页，直接走 OAuth 收敛"), logs)

    def test_run_probes_chatgpt_session_when_continue_url_has_wrong_host(self):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        engine = types.SimpleNamespace()
        engine.logs = []
        engine.email = "demo@example.com"
        engine.password = "pw-123"
        engine.proxy_url = None
        engine.email_service = types.SimpleNamespace(service_type=types.SimpleNamespace(value="temp_mail"))
        engine._is_existing_account = False
        engine._post_create_page_type = ""
        engine._post_create_continue_url = "https://auth.openai.com.evil.test/sign-in-with-chatgpt/codex/consent?state=abc"
        engine._check_ip_location = lambda: (True, "US")
        engine._create_email = lambda: True
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did-1"
        engine._check_sentinel = lambda _did: "sentinel"
        engine._submit_signup_form = lambda *_args: types.SimpleNamespace(success=True)
        engine._register_password = lambda: (True, "pw-123")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda _code: True
        engine._create_user_account = lambda: True
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acct-1",
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "id_token": "id-1",
        }
        engine._clean_text = lambda value: str(value or "").strip()
        engine._serialize_session_cookies = lambda: ""
        engine._extract_session_token_from_cookies = lambda: ""
        engine._extract_workspace_id_from_token = lambda _token: ""
        engine._post_registration_health_check = lambda _token: ("active", True, "")

        flow_runner = types.SimpleNamespace(
            resolve_post_registration_callback=lambda _did, _sen: types.SimpleNamespace(
                callback_url="http://localhost:1455/auth/callback?code=ok&state=state",
                workspace_id="acct-1",
                metadata=None,
                error_message="",
            )
        )
        engine._get_flow_runner = lambda: flow_runner
        runner.engine = engine

        fetch_called = {"count": 0}

        def fake_fetch_chatgpt_session_payload():
            fetch_called["count"] += 1
            return None, "chatgpt-error"

        original_fetch = runner._fetch_chatgpt_session_payload
        runner._fetch_chatgpt_session_payload = fake_fetch_chatgpt_session_payload

        try:
            result = runner.run()
        finally:
            runner._fetch_chatgpt_session_payload = original_fetch

        self.assertTrue(result.success)
        self.assertEqual(fetch_called["count"], 1)
        self.assertIn(("info", "13. 尝试直接复用当前会话"), logs)
        self.assertNotIn(("info", "13. 已命中 OAuth 授权页，直接走 OAuth 收敛"), logs)

    def test_add_phone_login_bypass_runs_before_session_probe(self):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        engine = types.SimpleNamespace()
        engine.logs = []
        engine.email = "demo@example.com"
        engine.password = "pw-123"
        engine.proxy_url = None
        engine.email_service = types.SimpleNamespace(service_type=types.SimpleNamespace(value="temp_mail"))
        engine._is_existing_account = False
        engine._post_create_page_type = "add_phone"
        engine._post_create_continue_url = ""
        engine._check_ip_location = lambda: (True, "US")
        engine._create_email = lambda: True
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did-1"
        engine._check_sentinel = lambda _did: "sentinel"
        engine._submit_signup_form = lambda *_args: types.SimpleNamespace(success=True)
        engine._register_password = lambda: (True, "pw-123")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda _code: True
        engine._create_user_account = lambda: True
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acct-1",
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "id_token": "id-1",
        }
        engine._clean_text = lambda value: str(value or "").strip()
        engine._serialize_session_cookies = lambda: ""
        engine._extract_session_token_from_cookies = lambda: ""
        engine._extract_workspace_id_from_token = lambda _token: ""
        engine._post_registration_health_check = lambda _token: ("active", True, "")

        flow_runner = types.SimpleNamespace(
            resolve_post_registration_callback=lambda _did, _sen: types.SimpleNamespace(
                callback_url="http://localhost:1455/auth/callback?code=ok&state=state",
                workspace_id="acct-1",
                metadata=None,
                error_message="",
            )
        )
        engine._get_flow_runner = lambda: flow_runner
        call_sequence = []
        engine._attempt_add_phone_login_bypass = lambda _did, _sen: call_sequence.append("bypass") or "http://localhost:1455/auth/callback?code=ok&state=state"
        runner.engine = engine

        def fake_fetch_chatgpt_session_payload():
            call_sequence.append("session")
            return None, "session-error"

        original_fetch = runner._fetch_chatgpt_session_payload
        runner._fetch_chatgpt_session_payload = fake_fetch_chatgpt_session_payload

        try:
            result = runner.run()
        finally:
            runner._fetch_chatgpt_session_payload = original_fetch

        self.assertTrue(result.success)
        self.assertEqual(call_sequence, ["bypass"])
        self.assertIn(("warning", "13. 检测到 add_phone，先尝试登录回退以便复用会话"), logs)
        self.assertNotIn(("info", "14. 尝试复用已登录会话直取 ChatGPT Session..."), logs)

    def test_existing_account_reuses_session_before_oauth(self):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        logs = []
        runner._log = lambda message, level="info": logs.append((level, message))

        engine = types.SimpleNamespace()
        engine.logs = []
        engine.email = "demo@example.com"
        engine.password = "pw-123"
        engine.proxy_url = None
        engine.email_service = types.SimpleNamespace(service_type=types.SimpleNamespace(value="temp_mail"))
        engine._is_existing_account = True
        engine._post_create_page_type = ""
        engine._post_create_continue_url = ""
        engine._check_ip_location = lambda: (True, "US")
        engine._create_email = lambda: True
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did-1"
        engine._check_sentinel = lambda _did: "sentinel"
        engine._submit_signup_form = lambda *_args: types.SimpleNamespace(success=True)
        engine._register_password = lambda: (True, "pw-123")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda _code: True
        engine._create_user_account = lambda: True
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acct-1",
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "id_token": "id-1",
        }
        engine._clean_text = lambda value: str(value or "").strip()
        engine._serialize_session_cookies = lambda: ""
        engine._extract_session_token_from_cookies = lambda: ""
        engine._extract_workspace_id_from_token = lambda _token: ""
        engine._post_registration_health_check = lambda _token: ("active", True, "")

        flow_runner = types.SimpleNamespace(
            resolve_post_registration_callback=lambda _did, _sen: types.SimpleNamespace(
                callback_url="http://localhost:1455/auth/callback?code=ok&state=state",
                workspace_id="acct-1",
                metadata=None,
                error_message="",
            )
        )
        engine._get_flow_runner = lambda: flow_runner
        runner.engine = engine
        fetch_called = {"count": 0}

        def fake_fetch_chatgpt_session_payload():
            fetch_called["count"] += 1
            return None, "session-error"

        original_fetch = runner._fetch_chatgpt_session_payload
        runner._fetch_chatgpt_session_payload = fake_fetch_chatgpt_session_payload

        try:
            result = runner.run()
        finally:
            runner._fetch_chatgpt_session_payload = original_fetch

        self.assertTrue(result.success)
        self.assertEqual(fetch_called["count"], 1)
        self.assertIn(("info", "13. 尝试直接复用当前会话"), logs)
        self.assertNotIn(("info", "13. 已命中 OAuth 授权页，直接走 OAuth 收敛"), logs)

    def _build_helper_runner(self):
        runner = AnyAutoRegistrationRunner.__new__(AnyAutoRegistrationRunner)
        runner.engine = types.SimpleNamespace(
            _clean_text=lambda value: str(value or "").strip(),
            _post_create_page_type="",
            _post_create_continue_url="",
        )
        return runner

    def test_should_try_chatgpt_session_first_skips_allowlisted_page_types(self):
        for page_type in (
            "workspace",
            "token_exchange",
            "sign_in_with_chatgpt_codex_org",
        ):
            runner = self._build_helper_runner()
            runner.engine._post_create_page_type = page_type
            runner.engine._post_create_continue_url = ""
            self.assertFalse(
                runner._should_try_chatgpt_session_first(),
                f"expected skip for page_type {page_type}",
            )

    def test_should_try_chatgpt_session_first_skips_allowlisted_continue_urls(self):
        continue_urls = (
            "https://auth.openai.com/workspace",
            "https://auth.openai.com/sign-in-with-chatgpt/codex/organization?foo=bar",
            "https://auth.openai.com/sign-in-with-chatgpt/codex/token_exchange?foo=bar",
        )
        for continue_url in continue_urls:
            runner = self._build_helper_runner()
            runner.engine._post_create_page_type = ""
            runner.engine._post_create_continue_url = continue_url
            self.assertFalse(
                runner._should_try_chatgpt_session_first(),
                f"expected skip for continue_url {continue_url}",
            )


if __name__ == "__main__":
    unittest.main()
