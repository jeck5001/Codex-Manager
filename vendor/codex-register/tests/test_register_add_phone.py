import importlib.util
import json
import sys
import types
import unittest
from dataclasses import dataclass
from pathlib import Path


def load_register_module():
    base_dir = Path(__file__).resolve().parents[1] / "src" / "core"

    def load_core_module(module_basename: str):
        module_name = f"src.core.{module_basename}"
        module_path = base_dir / f"{module_basename}.py"
        spec = importlib.util.spec_from_file_location(module_name, module_path)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        sys.modules[module_name] = module
        spec.loader.exec_module(module)
        return module

    module_name = "src.core.register"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "core"
        / "register.py"
    )

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    services_pkg = types.ModuleType("src.services")
    services_pkg.__path__ = []
    database_pkg = types.ModuleType("src.database")
    database_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []

    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg
    sys.modules["src.services"] = services_pkg
    sys.modules["src.database"] = database_pkg
    sys.modules["src.config"] = config_pkg

    oauth_module = types.ModuleType("src.core.oauth")

    @dataclass
    class OAuthStart:
        auth_url: str
        state: str
        code_verifier: str
        redirect_uri: str

    class OAuthManager:
        def __init__(self, *args, **kwargs):
            pass

        def start_oauth(self):
            return OAuthStart(
                auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
                state="state",
                code_verifier="verifier",
                redirect_uri="http://localhost/callback",
            )

    oauth_module.OAuthManager = OAuthManager
    oauth_module.OAuthStart = OAuthStart
    sys.modules["src.core.oauth"] = oauth_module

    http_client_module = types.ModuleType("src.core.http_client")

    class OpenAIHTTPClient:
        def __init__(self, *args, **kwargs):
            self.session = types.SimpleNamespace(cookies={})

        def close(self):
            return None

    class HTTPClientError(Exception):
        pass

    http_client_module.OpenAIHTTPClient = OpenAIHTTPClient
    http_client_module.HTTPClientError = HTTPClientError
    http_client_module.OPENAI_BROWSER_USER_AGENT = (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/120.0.0.0 Safari/537.36"
    )
    sys.modules["src.core.http_client"] = http_client_module

    services_module = types.ModuleType("src.services")

    class BaseEmailService:
        def __init__(self, *args, **kwargs):
            self.service_type = types.SimpleNamespace(value="temp_mail")

    class EmailServiceFactory:
        pass

    class EmailServiceType:
        TEMP_MAIL = "temp_mail"

    services_module.EmailServiceFactory = EmailServiceFactory
    services_module.BaseEmailService = BaseEmailService
    services_module.EmailServiceType = EmailServiceType
    sys.modules["src.services"] = services_module

    crud_module = types.ModuleType("src.database.crud")
    crud_module.append_task_log = lambda *args, **kwargs: None
    crud_module.get_account_by_email = lambda *args, **kwargs: None
    crud_module.create_account = lambda *args, **kwargs: None
    sys.modules["src.database.crud"] = crud_module

    session_module = types.ModuleType("src.database.session")

    class DummyDbContext:
        def __enter__(self):
            return object()

        def __exit__(self, exc_type, exc, tb):
            return False

    session_module.get_db = lambda: DummyDbContext()
    sys.modules["src.database.session"] = session_module

    constants_module = types.ModuleType("src.config.constants")
    constants_module.OPENAI_API_ENDPOINTS = {
        "login": "",
        "verify_password": "",
        "validate_otp": "",
        "resend_otp": "",
        "signup": "",
        "register": "",
        "send_otp": "",
        "create_account": "",
        "client_auth_session_dump": "",
        "select_workspace": "",
        "sentinel": "",
    }
    constants_module.OPENAI_PAGE_TYPES = {"EMAIL_OTP_VERIFICATION": "email_otp_verification"}
    constants_module.generate_random_user_info = lambda: {"name": "A", "birthdate": "2000-01-01"}
    constants_module.OTP_CODE_PATTERN = r"\b(\d{6})\b"
    constants_module.DEFAULT_PASSWORD_LENGTH = 12
    constants_module.PASSWORD_CHARSET = "abc123XYZ"
    constants_module.AccountStatus = type("AccountStatus", (), {})
    constants_module.TaskStatus = type("TaskStatus", (), {})
    sys.modules["src.config.constants"] = constants_module

    settings_module = types.ModuleType("src.config.settings")

    class Settings:
        openai_client_id = "client"
        openai_auth_url = "https://auth.openai.com/oauth/authorize"
        openai_token_url = "https://auth.openai.com/oauth/token"
        openai_redirect_uri = "http://localhost/callback"
        openai_scope = "openid"
        email_code_timeout = 240
        email_code_poll_interval = 3

    settings_module.get_settings = lambda: Settings()
    sys.modules["src.config.settings"] = settings_module

    curl_module = types.ModuleType("curl_cffi")
    curl_requests_module = types.ModuleType("curl_cffi.requests")
    curl_module.requests = curl_requests_module
    sys.modules["curl_cffi"] = curl_module
    sys.modules["curl_cffi.requests"] = curl_requests_module

    load_core_module("register_flow_state")
    load_core_module("register_retry_policy")
    load_core_module("register_token_resolver")
    load_core_module("register_flow_runner")
    load_core_module("sentinel_browser")

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


REGISTER_MODULE = load_register_module()
RegistrationEngine = REGISTER_MODULE.RegistrationEngine
AuthResolutionResult = REGISTER_MODULE.AuthResolutionResult
OAuthStart = sys.modules["src.core.oauth"].OAuthStart


class RegisterAddPhoneTests(unittest.TestCase):
    def test_session_cookie_debug_summary_reports_key_cookie_flags(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine._session_cookie_items = lambda: [
            ("cf_clearance", "cf-token"),
            ("oai-did", "did-123"),
            ("__Host-next-auth.csrf-token", "csrf-token"),
            ("__Secure-next-auth.session-token", "session-token"),
            ("misc", "value"),
        ]

        summary = engine._session_cookie_debug_summary()

        self.assertIn("count=5", summary)
        self.assertIn("cf_clearance=yes", summary)
        self.assertIn("oai-did=yes", summary)
        self.assertIn("csrf=yes", summary)
        self.assertIn("session=yes", summary)
        self.assertIn("names=cf_clearance,oai-did,__Host-next-auth.csrf-token,__Secure-next-auth.session-token,misc", summary)

    def test_auth_response_debug_summary_includes_location_and_set_cookie(self):
        class FakeHeaders(dict):
            def get(self, key, default=None):
                return super().get(key, default)

        class FakeResponse:
            status_code = 302
            url = "https://auth.openai.com/create-account/password"
            headers = FakeHeaders({
                "content-type": "text/html; charset=utf-8",
                "location": "https://auth.openai.com/u/next",
                "set-cookie": "cf_clearance=abc; Path=/; Secure",
            })

        engine = RegistrationEngine.__new__(RegistrationEngine)

        summary = engine._auth_response_debug_summary(FakeResponse())

        self.assertIn("status=302", summary)
        self.assertIn("url=https://auth.openai.com/create-account/password", summary)
        self.assertIn("content-type=text/html; charset=utf-8", summary)
        self.assertIn("location=https://auth.openai.com/u/next", summary)
        self.assertIn("set-cookie=yes", summary)

    def test_session_browser_cookies_preserve_duplicate_names_across_domains(self):
        class FakeCookie:
            def __init__(self, name, value, domain, path="/", secure=True, http_only=False):
                self.name = name
                self.value = value
                self.domain = domain
                self.path = path
                self.secure = secure
                self._http_only = http_only

            def has_nonstandard_attr(self, name):
                return name.lower() == "httponly" and self._http_only

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine._log = lambda *_args, **_kwargs: None
        engine.session = types.SimpleNamespace(
            cookies=types.SimpleNamespace(
                jar=[
                    FakeCookie("cf_clearance", "openai-cookie", ".openai.com"),
                    FakeCookie("cf_clearance", "chatgpt-cookie", ".chatgpt.com"),
                    FakeCookie("__Host-next-auth.csrf-token", "csrf-cookie", "chatgpt.com", http_only=True),
                ]
            )
        )

        cookies = engine._session_browser_cookies()

        self.assertEqual(
            [cookie["value"] for cookie in cookies if cookie["name"] == "cf_clearance"],
            ["openai-cookie", "chatgpt-cookie"],
        )
        host_cookie = next(
            cookie for cookie in cookies if cookie["name"] == "__Host-next-auth.csrf-token"
        )
        self.assertEqual(host_cookie["url"], "https://chatgpt.com/")
        self.assertNotIn("domain", host_cookie)

    def test_advance_workspace_authorization_uses_consent_response(self):
        class FakeResponse:
            url = "https://auth.openai.com/sign-in-with-chatgpt/codex/consent"
            text = '<script>window.__NEXT_DATA__={"activeWorkspaceId":"ws-consent"}</script>'
            history = []

            def json(self):
                raise ValueError("not json")

        class FakeSession:
            def get(self, url, **_kwargs):
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._clean_text = lambda value: str(value or "").strip()
        engine._cached_workspace_id = ""
        engine._select_workspace = lambda workspace_id: "https://auth.openai.com/continue"
        engine._follow_redirects = lambda url: "http://localhost/callback?code=ok&state=state"

        callback = engine._advance_workspace_authorization("https://auth.openai.com/add-phone")

        self.assertEqual(callback, "http://localhost/callback?code=ok&state=state")
        self.assertEqual(engine._cached_workspace_id, "ws-consent")

    def test_advance_workspace_authorization_follows_discovered_consent_link(self):
        class AddPhoneResponse:
            url = "https://auth.openai.com/add-phone"
            text = '<a href="/sign-in-with-chatgpt/codex/consent?step=1">continue</a>'
            history = []

            def json(self):
                raise ValueError("not json")

        class ConsentResponse:
            url = "https://auth.openai.com/sign-in-with-chatgpt/codex/consent?step=1"
            text = '<script>window.__NEXT_DATA__={"activeWorkspaceId":"ws-linked"}</script>'
            history = []

            def json(self):
                raise ValueError("not json")

        class FakeSession:
            def __init__(self):
                self.calls = []

            def get(self, url, **_kwargs):
                self.calls.append(url)
                if "sign-in-with-chatgpt/codex/consent" in url:
                    return ConsentResponse()
                return AddPhoneResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._clean_text = lambda value: str(value or "").strip()
        engine._cached_workspace_id = ""
        engine._select_workspace = lambda workspace_id: "https://auth.openai.com/continue"
        engine._follow_redirects = lambda url: "http://localhost/callback?code=ok&state=state"

        callback = engine._advance_workspace_authorization("https://auth.openai.com/add-phone")

        self.assertEqual(callback, "http://localhost/callback?code=ok&state=state")
        self.assertEqual(engine._cached_workspace_id, "ws-linked")
        self.assertEqual(
            engine.session.calls,
            [
                "https://auth.openai.com/add-phone",
                "https://auth.openai.com/sign-in-with-chatgpt/codex/consent?step=1",
            ],
        )

    def test_follow_auth_continue_url_caches_workspace_id_from_response(self):
        class FakeResponse:
            url = "https://auth.openai.com/sign-in-with-chatgpt/codex/consent"
            text = '<script>window.__NEXT_DATA__={"activeWorkspaceId":"ws-script"}</script>'

            def json(self):
                raise ValueError("not json")

        class FakeSession:
            def get(self, url, **_kwargs):
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._clean_text = lambda value: str(value or "").strip()
        engine._cached_workspace_id = ""

        engine._follow_auth_continue_url(
            {"continue_url": "https://auth.openai.com/add-phone"},
            "登录邮箱验证码",
        )

        self.assertEqual(engine._cached_workspace_id, "ws-script")

    def test_submit_signup_form_follows_continue_url_for_password_page(self):
        class FakeResponse:
            status_code = 200

            def json(self):
                return {
                    "page": {"type": "create_account_password"},
                    "continue_url": "https://auth.openai.com/create-account/password",
                }

        class FakeSession:
            def post(self, *_args, **_kwargs):
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        followed = []
        engine._follow_auth_continue_url = lambda payload, stage: followed.append((payload, stage))

        result = engine._submit_signup_form("did", "sentinel")

        self.assertTrue(result.success)
        self.assertEqual(result.page_type, "create_account_password")
        self.assertEqual(len(followed), 1)
        self.assertEqual(followed[0][0]["continue_url"], "https://auth.openai.com/create-account/password")
        self.assertEqual(followed[0][1], "注册邮箱")

    def test_run_falls_back_to_workspace_flow_when_add_phone_bypass_has_no_callback(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.logs = []
        engine._log = lambda *_args, **_kwargs: None
        engine._is_existing_account = False
        engine._post_create_page_type = ""
        engine._post_create_continue_url = ""
        engine.password = "secret"
        engine.proxy_url = None
        engine.session = None
        engine.email_service = types.SimpleNamespace(
            service_type=types.SimpleNamespace(value="temp_mail")
        )
        engine.oauth_start = OAuthStart(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            state="state",
            code_verifier="verifier",
            redirect_uri="http://localhost/callback",
        )

        engine._check_ip_location = lambda: (True, "US")

        def fake_create_email():
            engine.email = "user@example.com"
            return True

        engine._create_email = fake_create_email
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did"
        engine._check_sentinel = lambda _did: "sentinel"

        class SignupResult:
            success = True
            error_message = ""

        engine._submit_signup_form = lambda _did, _sen: SignupResult()
        engine._register_password = lambda: (True, "secret")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda code: True

        def fake_create_user_account():
            engine._post_create_page_type = "add_phone"
            engine._post_create_continue_url = "https://auth.openai.com/add-phone"
            return True

        engine._create_user_account = fake_create_user_account
        engine._attempt_add_phone_login_bypass = lambda _did, _sen: None
        engine._get_workspace_id = lambda: "ws-1"
        engine._select_workspace = lambda workspace_id: "https://auth.openai.com/continue"
        engine._follow_redirects = lambda url: "http://localhost/callback?code=ok&state=state"
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acc-1",
            "access_token": "access",
            "refresh_token": "refresh",
            "id_token": "id",
        }
        engine._post_registration_health_check = lambda access_token: ("active", True, "")

        result = engine.run()

        self.assertTrue(result.success)
        self.assertTrue(result.is_usable)
        self.assertEqual(result.account_status, "active")
        self.assertEqual(result.workspace_id, "ws-1")
        self.assertEqual(result.account_id, "acc-1")

    def test_run_uses_authenticated_oauth_url_after_add_phone_fallback(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.logs = []
        engine._log = lambda *_args, **_kwargs: None
        engine._is_existing_account = False
        engine._post_create_page_type = ""
        engine._post_create_continue_url = ""
        engine.password = "secret"
        engine.proxy_url = None
        engine.session = None
        engine.email_service = types.SimpleNamespace(
            service_type=types.SimpleNamespace(value="temp_mail")
        )
        engine.oauth_start = OAuthStart(
            auth_url=(
                "https://auth.openai.com/oauth/authorize"
                "?client_id=test&response_type=code&prompt=login&state=state"
            ),
            state="state",
            code_verifier="verifier",
            redirect_uri="http://localhost/callback",
        )

        engine._check_ip_location = lambda: (True, "US")

        def fake_create_email():
            engine.email = "user@example.com"
            return True

        engine._create_email = fake_create_email
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did"
        engine._check_sentinel = lambda _did: "sentinel"

        class SignupResult:
            success = True
            error_message = ""

        engine._submit_signup_form = lambda _did, _sen: SignupResult()
        engine._register_password = lambda: (True, "secret")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda code: True

        def fake_create_user_account():
            engine._post_create_page_type = "add_phone"
            engine._post_create_continue_url = "https://auth.openai.com/add-phone"
            return True

        engine._create_user_account = fake_create_user_account
        engine._attempt_add_phone_login_bypass = lambda _did, _sen: None
        engine._get_workspace_id = lambda: None
        followed_urls = []
        engine._follow_redirects = lambda url: followed_urls.append(url) or "http://localhost/callback?code=ok&state=state"
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acc-1",
            "access_token": "access",
            "refresh_token": "refresh",
            "id_token": "id",
        }
        engine._post_registration_health_check = lambda access_token: ("active", True, "")

        result = engine.run()

        self.assertTrue(result.success)
        self.assertEqual(
            followed_urls,
            [
                "https://auth.openai.com/oauth/authorize"
                "?client_id=test&response_type=code&state=state"
            ],
        )

    def test_run_marks_account_unusable_when_health_check_fails(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.logs = []
        engine._log = lambda *_args, **_kwargs: None
        engine._is_existing_account = False
        engine._post_create_page_type = ""
        engine._post_create_continue_url = ""
        engine.password = "secret"
        engine.proxy_url = None
        engine.session = None
        engine.email_service = types.SimpleNamespace(
            service_type=types.SimpleNamespace(value="temp_mail")
        )
        engine.oauth_start = OAuthStart(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            state="state",
            code_verifier="verifier",
            redirect_uri="http://localhost/callback",
        )

        engine._check_ip_location = lambda: (True, "US")

        def fake_create_email():
            engine.email = "user@example.com"
            return True

        engine._create_email = fake_create_email
        engine._init_session = lambda: True
        engine._start_oauth = lambda: True
        engine._get_device_id = lambda: "did"
        engine._check_sentinel = lambda _did: "sentinel"

        class SignupResult:
            success = True
            error_message = ""

        engine._submit_signup_form = lambda _did, _sen: SignupResult()
        engine._register_password = lambda: (True, "secret")
        engine._send_verification_code = lambda: True
        engine._wait_for_signup_verification_code = lambda: "123456"
        engine._validate_signup_verification_code_with_retry = lambda code: True
        engine._create_user_account = lambda: True
        engine._get_workspace_id = lambda: "ws-1"
        engine._select_workspace = lambda workspace_id: "https://auth.openai.com/continue"
        engine._follow_redirects = lambda url: "http://localhost/callback?code=ok&state=state"
        engine._handle_oauth_callback = lambda callback_url: {
            "account_id": "acc-1",
            "access_token": "access",
            "refresh_token": "refresh",
            "id_token": "id",
        }
        engine._post_registration_health_check = lambda access_token: (
            "banned",
            False,
            "账号健康检查返回 403，疑似已受限或被封禁",
        )

        result = engine.run()

        self.assertTrue(result.success)
        self.assertFalse(result.is_usable)
        self.assertEqual(result.account_status, "banned")
        self.assertIn("403", result.error_message)

    def test_submit_login_identifier_follows_continue_url(self):
        class FakeResponse:
            status_code = 200

            def json(self):
                return {
                    "continue_url": "https://auth.openai.com/log-in/password",
                    "page": {"type": "login_password"},
                }

        class FakeSession:
            def __init__(self):
                self.get_calls = []

            def post(self, *_args, **_kwargs):
                return FakeResponse()

            def get(self, url, **_kwargs):
                self.get_calls.append(url)
                return None

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._log_auth_response_preview = lambda *_args, **_kwargs: None
        engine._clean_text = lambda value: str(value or "").strip()

        page = engine._submit_login_identifier(None, None)

        self.assertEqual(page, {"type": "login_password"})
        self.assertEqual(
            engine.session.get_calls,
            ["https://auth.openai.com/log-in/password"],
        )

    def test_verify_login_password_follows_continue_url(self):
        class FakeResponse:
            status_code = 200

            def json(self):
                return {
                    "continue_url": "https://auth.openai.com/email-verification",
                    "page": {"type": "email_otp_verification"},
                }

        class FakeSession:
            def __init__(self):
                self.get_calls = []

            def post(self, *_args, **_kwargs):
                return FakeResponse()

            def get(self, url, **_kwargs):
                self.get_calls.append(url)
                return None

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._log_auth_response_preview = lambda *_args, **_kwargs: None
        engine._clean_text = lambda value: str(value or "").strip()

        page = engine._verify_login_password("secret")

        self.assertEqual(page, {"type": "email_otp_verification"})
        self.assertEqual(
            engine.session.get_calls,
            ["https://auth.openai.com/email-verification"],
        )

    def test_validate_login_otp_follows_continue_url(self):
        class FakeResponse:
            status_code = 200

            def json(self):
                return {
                    "continue_url": "https://auth.openai.com/add-phone",
                    "page": {"type": "add_phone"},
                }

        class FakeSession:
            def __init__(self):
                self.get_calls = []

            def post(self, *_args, **_kwargs):
                return FakeResponse()

            def get(self, url, **_kwargs):
                self.get_calls.append(url)
                return None

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._log_auth_response_preview = lambda *_args, **_kwargs: None
        engine._clear_otp_error_state = lambda: None
        engine._update_otp_error_state = lambda *_args, **_kwargs: None
        engine._clean_text = lambda value: str(value or "").strip()

        payload = engine._validate_verification_code_with_payload("123456")

        self.assertEqual(payload["page"]["type"], "add_phone")
        self.assertEqual(
            engine.session.get_calls,
            ["https://auth.openai.com/add-phone"],
        )

    def test_reuses_current_session_oauth_after_login_otp_returns_add_phone(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.password = "secret"
        engine.oauth_start = OAuthStart(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            state="state",
            code_verifier="verifier",
            redirect_uri="http://localhost/callback",
        )
        engine._last_login_recovery_page_type = ""
        engine.logs = []
        engine._log = lambda *_args, **_kwargs: None

        followed_urls = []
        restart_calls = []

        engine._submit_login_identifier = lambda did, sen: {"type": "login_password"}
        engine._verify_login_password = lambda password: {"type": "email_otp_verification"}
        engine._complete_login_email_otp_verification = lambda: AuthResolutionResult(
            callback_url=None,
            page_type="add_phone",
            continue_url="https://auth.openai.com/add-phone",
        )

        def fake_follow_redirects(url):
            followed_urls.append(url)
            return "http://localhost/callback?code=ok&state=state"

        engine._follow_redirects = fake_follow_redirects

        def fake_restart():
            restart_calls.append(True)
            return ("new-did", "new-sentinel")

        engine._restart_oauth_session_for_login = fake_restart
        engine._resolve_callback_from_auth_page = lambda page, stage: None
        engine._clean_text = lambda value: str(value or "").strip()

        callback = engine._attempt_add_phone_login_bypass("did", "sentinel")

        self.assertEqual(
            callback,
            "http://localhost/callback?code=ok&state=state",
        )
        self.assertEqual(
            followed_urls,
            ["https://auth.openai.com/oauth/authorize?client_id=test"],
        )
        self.assertEqual(restart_calls, [])

    def test_reuses_authenticated_oauth_url_without_prompt_login(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.password = "secret"
        engine.oauth_start = OAuthStart(
            auth_url=(
                "https://auth.openai.com/oauth/authorize"
                "?client_id=test&response_type=code&prompt=login&state=state"
            ),
            state="state",
            code_verifier="verifier",
            redirect_uri="http://localhost/callback",
        )
        engine._last_login_recovery_page_type = ""
        engine.logs = []
        engine._log = lambda *_args, **_kwargs: None

        followed_urls = []

        engine._submit_login_identifier = lambda did, sen: {"type": "login_password"}
        engine._verify_login_password = lambda password: {"type": "email_otp_verification"}
        engine._complete_login_email_otp_verification = lambda: AuthResolutionResult(
            callback_url=None,
            page_type="add_phone",
            continue_url="https://auth.openai.com/add-phone",
        )
        engine._advance_workspace_authorization = lambda auth_target: None
        engine._restart_oauth_session_for_login = lambda: ("new-did", "new-sentinel")
        engine._resolve_callback_from_auth_page = lambda page, stage: None
        engine._clean_text = lambda value: str(value or "").strip()
        engine._follow_redirects = (
            lambda url: followed_urls.append(url) or "http://localhost/callback?code=ok&state=state"
        )

        callback = engine._attempt_add_phone_login_bypass("did", "sentinel")

        self.assertEqual(
            callback,
            "http://localhost/callback?code=ok&state=state",
        )
        self.assertEqual(
            followed_urls,
            [
                "https://auth.openai.com/oauth/authorize"
                "?client_id=test&response_type=code&state=state"
            ],
        )

    def test_attempt_add_phone_login_bypass_tries_workspace_authorization_before_oauth_retry(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.password = "secret"
        engine.oauth_start = OAuthStart(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            state="state",
            code_verifier="verifier",
            redirect_uri="http://localhost/callback",
        )
        engine._last_login_recovery_page_type = ""
        engine.logs = []
        engine._log = lambda *_args, **_kwargs: None

        followed_urls = []
        advanced_targets = []

        engine._submit_login_identifier = lambda did, sen: {"type": "login_password"}
        engine._verify_login_password = lambda password: {"type": "email_otp_verification"}
        engine._complete_login_email_otp_verification = lambda: AuthResolutionResult(
            callback_url=None,
            page_type="add_phone",
            continue_url="https://auth.openai.com/add-phone",
        )
        engine._advance_workspace_authorization = lambda auth_target: (
            advanced_targets.append(auth_target)
            or "http://localhost/callback?code=ok&state=state"
        )
        engine._follow_redirects = lambda url: followed_urls.append(url) or None
        engine._restart_oauth_session_for_login = lambda: ("new-did", "new-sentinel")
        engine._resolve_callback_from_auth_page = lambda page, stage: None
        engine._clean_text = lambda value: str(value or "").strip()

        callback = engine._attempt_add_phone_login_bypass("did", "sentinel")

        self.assertEqual(
            callback,
            "http://localhost/callback?code=ok&state=state",
        )
        self.assertEqual(
            advanced_targets,
            ["https://auth.openai.com/add-phone"],
        )
        self.assertEqual(followed_urls, [])

    def test_add_phone_bypass_does_not_restart_new_oauth_session_after_current_session_hits_add_phone(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.password = "secret"
        engine.oauth_start = OAuthStart(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            state="state",
            code_verifier="verifier",
            redirect_uri="http://localhost/callback",
        )
        engine._last_login_recovery_page_type = ""
        engine.logs = []
        engine._log = lambda *_args, **_kwargs: None

        engine._submit_login_identifier = lambda did, sen: {"type": "login_password"}
        engine._verify_login_password = lambda password: {"type": "email_otp_verification"}
        engine._complete_login_email_otp_verification = lambda: AuthResolutionResult(
            callback_url=None,
            page_type="add_phone",
            continue_url="https://auth.openai.com/add-phone",
        )
        engine._advance_workspace_authorization = lambda auth_target: None
        engine._follow_redirects = lambda url: None
        engine._resolve_callback_from_auth_page = lambda page, stage: None
        engine._clean_text = lambda value: str(value or "").strip()
        engine._restart_oauth_session_for_login = (
            lambda: (_ for _ in ()).throw(AssertionError("should not restart new oauth session"))
        )

        callback = engine._attempt_add_phone_login_bypass("did", "sentinel")

        self.assertIsNone(callback)

    def test_register_password_sends_updated_sentinel_flow_and_passkey_capabilities(self):
        captured = {}

        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                captured["url"] = url
                captured["kwargs"] = kwargs
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._generate_password = lambda: "StrongPassw0rd!"
        engine._current_device_id = "did-123"
        engine._current_sentinel_token = "sentinel-c"

        ok, password = engine._register_password()

        self.assertTrue(ok)
        self.assertEqual(password, "StrongPassw0rd!")
        headers = captured["kwargs"]["headers"]
        sentinel = json.loads(headers["openai-sentinel-token"])
        self.assertEqual(sentinel["id"], "did-123")
        self.assertEqual(sentinel["c"], "sentinel-c")
        self.assertEqual(sentinel["flow"], "username_password_create")
        self.assertIn("ext-passkey-client-capabilities", headers)
        self.assertTrue(headers["ext-passkey-client-capabilities"])

    def test_register_password_requests_flow_specific_sentinel_token(self):
        captured = {}

        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                captured["url"] = url
                captured["kwargs"] = kwargs
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._generate_password = lambda: "StrongPassw0rd!"
        engine._current_device_id = "did-999"
        engine._current_sentinel_token = "old-authorize-token"

        sentinel_calls = []

        def fake_check_sentinel(did, flow="authorize_continue"):
            sentinel_calls.append((did, flow))
            return "fresh-flow-token"

        engine._check_sentinel = fake_check_sentinel

        ok, _password = engine._register_password()

        self.assertTrue(ok)
        self.assertEqual(
            sentinel_calls,
            [("did-999", "username_password_create")],
        )
        sentinel = json.loads(captured["kwargs"]["headers"]["openai-sentinel-token"])
        self.assertEqual(sentinel["c"], "fresh-flow-token")
        self.assertEqual(sentinel["flow"], "username_password_create")

    def test_register_password_prefers_browser_sentinel_payload(self):
        captured = {}

        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                captured["url"] = url
                captured["kwargs"] = kwargs
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._generate_password = lambda: "StrongPassw0rd!"
        engine._current_device_id = "did-browser"
        engine._current_sentinel_token = "old-authorize-token"

        browser_calls = []
        sentinel_calls = []

        def fake_browser_payload(flow, referer):
            browser_calls.append((flow, referer))
            return {
                "p": "browser-p",
                "t": "browser-t",
                "c": "browser-c",
                "id": "did-browser",
                "flow": flow,
            }

        def fake_check_sentinel(did, flow="authorize_continue"):
            sentinel_calls.append((did, flow))
            return "http-fallback-token"

        engine._get_browser_sentinel_payload = fake_browser_payload
        engine._check_sentinel = fake_check_sentinel

        ok, _password = engine._register_password()

        self.assertTrue(ok)
        self.assertEqual(
            browser_calls,
            [("username_password_create", "https://auth.openai.com/create-account/password")],
        )
        self.assertEqual(sentinel_calls, [])
        sentinel = json.loads(captured["kwargs"]["headers"]["openai-sentinel-token"])
        self.assertEqual(sentinel["p"], "browser-p")
        self.assertEqual(sentinel["t"], "browser-t")
        self.assertEqual(sentinel["c"], "browser-c")
        self.assertEqual(sentinel["flow"], "username_password_create")

    def test_register_password_falls_back_to_http_when_browser_sentinel_missing(self):
        captured = {}

        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                captured["url"] = url
                captured["kwargs"] = kwargs
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._generate_password = lambda: "StrongPassw0rd!"
        engine._current_device_id = "did-http"
        engine._current_sentinel_token = "old-authorize-token"

        browser_calls = []
        sentinel_calls = []

        def fake_browser_payload(flow, referer):
            browser_calls.append((flow, referer))
            return None

        def fake_check_sentinel(did, flow="authorize_continue"):
            sentinel_calls.append((did, flow))
            return "http-fallback-token"

        engine._get_browser_sentinel_payload = fake_browser_payload
        engine._check_sentinel = fake_check_sentinel

        ok, _password = engine._register_password()

        self.assertTrue(ok)
        self.assertEqual(
            browser_calls,
            [("username_password_create", "https://auth.openai.com/create-account/password")],
        )
        self.assertEqual(
            sentinel_calls,
            [("did-http", "username_password_create")],
        )
        sentinel = json.loads(captured["kwargs"]["headers"]["openai-sentinel-token"])
        self.assertEqual(sentinel["c"], "http-fallback-token")
        self.assertEqual(sentinel["flow"], "username_password_create")

    def test_register_password_logs_browser_sentinel_source_and_fields(self):
        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                return FakeResponse()

        logs = []
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda message, level="info": logs.append((level, message))
        engine._generate_password = lambda: "StrongPassw0rd!"
        engine._current_device_id = "did-browser"
        engine._current_sentinel_token = "old-authorize-token"
        engine._get_browser_sentinel_payload = lambda flow, referer: {
            "p": "browser-p",
            "t": "browser-t",
            "c": "browser-c",
            "id": "did-browser",
            "flow": flow,
        }
        engine._check_sentinel = lambda did, flow="authorize_continue": "http-fallback-token"

        ok, _password = engine._register_password()

        self.assertTrue(ok)
        self.assertIn(
            (
                "info",
                "密码注册 Sentinel 来源=browser flow=username_password_create p=yes t=yes c=yes",
            ),
            logs,
        )

    def test_register_password_logs_browser_and_http_fallback_failures(self):
        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                return FakeResponse()

        logs = []
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine._log = lambda message, level="info": logs.append((level, message))
        engine._generate_password = lambda: "StrongPassw0rd!"
        engine._current_device_id = "did-missing"
        engine._current_sentinel_token = ""
        engine._get_browser_sentinel_payload = lambda flow, referer: None
        engine._check_sentinel = lambda did, flow="authorize_continue": None

        ok, _password = engine._register_password()

        self.assertTrue(ok)
        self.assertIn(
            ("warning", "密码注册浏览器 Sentinel 获取失败，准备回退 HTTP Sentinel"),
            logs,
        )
        self.assertIn(
            ("warning", "密码注册 HTTP Sentinel 获取失败"),
            logs,
        )

    def test_register_password_recovers_device_id_from_session_cookies(self):
        captured = {}

        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def __init__(self):
                self.cookies = {"oai-did": "did-from-cookie"}

            def post(self, url, **kwargs):
                captured["url"] = url
                captured["kwargs"] = kwargs
                return FakeResponse()

        logs = []
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.email = "user@example.com"
        engine.session = FakeSession()
        engine.http_client = types.SimpleNamespace(session=engine.session)
        engine._log = lambda message, level="info": logs.append((level, message))
        engine._generate_password = lambda: "StrongPassw0rd!"
        engine._current_device_id = ""
        engine._current_sentinel_token = ""
        engine._get_browser_sentinel_payload = lambda flow, referer: (
            {
                "p": "browser-p",
                "t": "browser-t",
                "c": "browser-c",
                "id": "did-from-cookie",
                "flow": flow,
            }
            if engine._current_device_id == "did-from-cookie"
            else None
        )
        engine._check_sentinel = lambda did, flow="authorize_continue": None

        ok, _password = engine._register_password()

        self.assertTrue(ok)
        sentinel = json.loads(captured["kwargs"]["headers"]["openai-sentinel-token"])
        self.assertEqual(sentinel["id"], "did-from-cookie")
        self.assertIn(
            ("warning", "当前 Device ID 丢失，已从会话 Cookie 恢复"),
            logs,
        )

    def test_create_user_account_prefers_browser_sentinel_token_payload(self):
        captured = {}

        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                captured["url"] = url
                captured["kwargs"] = kwargs
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._current_device_id = "did-123"
        engine._get_create_account_sentinel_payload = lambda: {
            "p": "browser-payload",
            "t": "browser-turnstile",
            "c": "browser-c",
            "id": "did-123",
            "flow": "username_password_create",
        }

        ok = engine._create_user_account()

        self.assertTrue(ok)
        headers = captured["kwargs"]["headers"]
        sentinel = json.loads(headers["openai-sentinel-token"])
        self.assertEqual(sentinel["p"], "browser-payload")
        self.assertEqual(sentinel["t"], "browser-turnstile")
        self.assertEqual(sentinel["c"], "browser-c")
        self.assertEqual(sentinel["flow"], "username_password_create")

    def test_create_user_account_falls_back_to_http_sentinel_token_payload(self):
        captured = {}

        class FakeResponse:
            status_code = 200
            text = "{}"

            def json(self):
                return {}

        class FakeSession:
            def post(self, url, **kwargs):
                captured["url"] = url
                captured["kwargs"] = kwargs
                return FakeResponse()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine.session = FakeSession()
        engine._log = lambda *_args, **_kwargs: None
        engine._current_device_id = "did-456"
        engine._get_create_account_sentinel_payload = lambda: None
        engine._check_sentinel = lambda did: "http-fallback-token"

        ok = engine._create_user_account()

        self.assertTrue(ok)
        headers = captured["kwargs"]["headers"]
        sentinel = json.loads(headers["openai-sentinel-token"])
        self.assertEqual(sentinel["id"], "did-456")
        self.assertEqual(sentinel["c"], "http-fallback-token")
        self.assertEqual(sentinel["p"], "")
        self.assertEqual(sentinel["t"], "")
        self.assertEqual(sentinel["flow"], "username_password_create")

    def test_browser_create_account_sentinel_payload_tries_oauth_flow_before_password_flow(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine._log = lambda *_args, **_kwargs: None
        engine._current_device_id = "did-789"
        engine.proxy_url = None
        engine._serialize_session_cookies = lambda: "foo=bar"

        original_fetch = REGISTER_MODULE.fetch_browser_sentinel_token
        calls = []

        try:
            def fake_fetch_browser_sentinel_token(**kwargs):
                calls.append(kwargs["flow"])
                if kwargs["flow"] == "oauth_create_account":
                    return None
                return {
                    "p": "browser-p",
                    "t": "browser-t",
                    "c": "browser-c",
                    "id": "did-789",
                    "flow": kwargs["flow"],
                }

            REGISTER_MODULE.fetch_browser_sentinel_token = fake_fetch_browser_sentinel_token

            payload = engine._get_browser_create_account_sentinel_payload()
        finally:
            REGISTER_MODULE.fetch_browser_sentinel_token = original_fetch

        self.assertEqual(
            calls,
            ["oauth_create_account", "username_password_create"],
        )
        self.assertEqual(payload["flow"], "username_password_create")
        self.assertEqual(payload["t"], "browser-t")

    def test_browser_sentinel_payload_syncs_browser_cookies_into_http_session(self):
        class FakeCookieStore:
            def __init__(self):
                self.calls = []

            def set(self, name, value, **kwargs):
                self.calls.append((name, value, kwargs))

        cookie_store = FakeCookieStore()

        engine = RegistrationEngine.__new__(RegistrationEngine)
        engine._log = lambda *_args, **_kwargs: None
        engine._current_device_id = "did-789"
        engine.proxy_url = None
        engine.session = types.SimpleNamespace(cookies=cookie_store)
        engine._serialize_session_cookies = lambda: "oai-did=did-789"

        original_fetch = REGISTER_MODULE.fetch_browser_sentinel_token

        try:
            def fake_fetch_browser_sentinel_token(**kwargs):
                return {
                    "p": "browser-p",
                    "t": "browser-t",
                    "c": "browser-c",
                    "id": "did-789",
                    "flow": kwargs["flow"],
                    "cookies": [
                        {
                            "name": "cf_clearance",
                            "value": "cf-cookie",
                            "domain": ".openai.com",
                            "path": "/",
                            "secure": True,
                        },
                        {
                            "name": "__Host-next-auth.csrf-token",
                            "value": "csrf-cookie",
                            "url": "https://auth.openai.com/",
                            "secure": True,
                            "httpOnly": True,
                        },
                    ],
                }

            REGISTER_MODULE.fetch_browser_sentinel_token = fake_fetch_browser_sentinel_token

            payload = engine._get_browser_sentinel_payload(
                "username_password_create",
                "https://auth.openai.com/create-account/password",
            )
        finally:
            REGISTER_MODULE.fetch_browser_sentinel_token = original_fetch

        self.assertEqual(payload["flow"], "username_password_create")
        self.assertEqual(payload["t"], "browser-t")
        self.assertEqual(
            cookie_store.calls,
            [
                ("cf_clearance", "cf-cookie", {"domain": ".openai.com", "path": "/"}),
                ("__Host-next-auth.csrf-token", "csrf-cookie", {"domain": "auth.openai.com", "path": "/"}),
            ],
        )

    def test_generate_password_meets_current_policy(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)

        password = engine._generate_password()

        self.assertGreaterEqual(len(password), 16)
        self.assertRegex(password, r"[a-z]")
        self.assertRegex(password, r"[A-Z]")
        self.assertRegex(password, r"\d")
        self.assertRegex(password, r"[^A-Za-z0-9]")

    def test_generate_password_uses_safe_special_characters_only(self):
        engine = RegistrationEngine.__new__(RegistrationEngine)
        original_choice = REGISTER_MODULE.secrets.choice
        original_randbelow = REGISTER_MODULE.secrets.randbelow

        try:
            REGISTER_MODULE.secrets.choice = lambda seq: seq[-1]
            REGISTER_MODULE.secrets.randbelow = lambda upper: 0

            password = engine._generate_password()
        finally:
            REGISTER_MODULE.secrets.choice = original_choice
            REGISTER_MODULE.secrets.randbelow = original_randbelow

        special_chars = {char for char in password if not char.isalnum()}
        self.assertTrue(special_chars, password)
        self.assertTrue(special_chars.issubset({"!"}), password)


if __name__ == "__main__":
    unittest.main()
