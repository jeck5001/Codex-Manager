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


if __name__ == "__main__":
    unittest.main()
