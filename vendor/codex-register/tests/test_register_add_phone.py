import importlib.util
import json
import sys
import types
import unittest
from dataclasses import dataclass
from pathlib import Path


def load_register_module():
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
        email_code_timeout = 120
        email_code_poll_interval = 3

    settings_module.get_settings = lambda: Settings()
    sys.modules["src.config.settings"] = settings_module

    curl_module = types.ModuleType("curl_cffi")
    curl_requests_module = types.ModuleType("curl_cffi.requests")
    curl_module.requests = curl_requests_module
    sys.modules["curl_cffi"] = curl_module
    sys.modules["curl_cffi.requests"] = curl_requests_module

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


if __name__ == "__main__":
    unittest.main()
