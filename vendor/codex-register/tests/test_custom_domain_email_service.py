import importlib.util
import sys
import types
import unittest
from pathlib import Path


def load_custom_domain_module():
    module_name = "src.services.custom_domain"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "services"
        / "custom_domain.py"
    )

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    services_pkg = types.ModuleType("src.services")
    services_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []

    sys.modules["src"] = src_pkg
    sys.modules["src.services"] = services_pkg
    sys.modules["src.core"] = core_pkg
    sys.modules["src.config"] = config_pkg

    base_module = types.ModuleType("src.services.base")

    class EmailServiceError(Exception):
        pass

    class BaseEmailService:
        def __init__(self, service_type, name=None):
            self.service_type = service_type
            self.name = name

        def update_status(self, *_args, **_kwargs):
            return None

    class EmailServiceType:
        CUSTOM_DOMAIN = "custom_domain"
        TEMP_MAIL = "temp_mail"

    base_module.BaseEmailService = BaseEmailService
    base_module.EmailServiceError = EmailServiceError
    base_module.EmailServiceType = EmailServiceType
    sys.modules["src.services.base"] = base_module

    http_client_module = types.ModuleType("src.core.http_client")

    class RequestConfig:
        def __init__(self, timeout=30, max_retries=3):
            self.timeout = timeout
            self.max_retries = max_retries

    class HTTPClient:
        def __init__(self, proxy_url=None, config=None):
            self.proxy_url = proxy_url
            self.config = config

    http_client_module.HTTPClient = HTTPClient
    http_client_module.RequestConfig = RequestConfig
    sys.modules["src.core.http_client"] = http_client_module

    constants_module = types.ModuleType("src.config.constants")
    constants_module.OTP_CODE_PATTERN = r"\b(\d{6})\b"
    sys.modules["src.config.constants"] = constants_module

    settings_module = types.ModuleType("src.config.settings")

    class Settings:
        email_code_timeout = 120
        email_code_poll_interval = 3

    settings_module.get_settings = lambda: Settings()
    sys.modules["src.config.settings"] = settings_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


CUSTOM_DOMAIN_MODULE = load_custom_domain_module()
CustomDomainEmailService = CUSTOM_DOMAIN_MODULE.CustomDomainEmailService


class CustomDomainEmailServiceTests(unittest.TestCase):
    def test_prefers_latest_openai_otp_after_resend(self):
        service = CustomDomainEmailService(
            config={"base_url": "https://mail.example.com", "api_key": "test-key"}
        )
        service._emails_cache["email-1"] = {"email": "user@example.com"}

        messages = [
            {"id": "msg-old", "created_at": 210},
            {"id": "msg-new", "created_at": 220},
        ]
        contents = {
            "msg-old": "OpenAI verification code 111111",
            "msg-new": "OpenAI verification code 222222",
        }

        def fake_make_request(method, endpoint, **_kwargs):
            self.assertEqual(method, "GET")
            if endpoint == "/api/emails/email-1":
                return {"messages": messages}
            self.fail(f"unexpected endpoint: {endpoint}")

        service._make_request = fake_make_request
        service._get_message_content = lambda _email_id, message_id: contents[message_id]

        code = service.get_verification_code(
            email="user@example.com",
            email_id="email-1",
            timeout=1,
            poll_interval=1,
            otp_sent_at=200,
        )

        self.assertEqual(code, "222222")


def load_temp_mail_module():
    module_name = "src.services.temp_mail"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "services"
        / "temp_mail.py"
    )

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


TEMP_MAIL_MODULE = load_temp_mail_module()
TempMailService = TEMP_MAIL_MODULE.TempMailService


class TempMailServiceTests(unittest.TestCase):
    def test_prefers_latest_openai_otp_after_resend(self):
        service = TempMailService(
            config={
                "base_url": "https://mail.example.com",
                "admin_password": "test-password",
                "domain": "example.com",
            }
        )

        mails = [
            {
                "id": "mail-old",
                "created_at": 210,
                "from": "OpenAI <noreply@openai.com>",
                "subject": "Your verification code",
                "text": "111111",
            },
            {
                "id": "mail-new",
                "created_at": 220,
                "from": "OpenAI <noreply@openai.com>",
                "subject": "Your verification code",
                "text": "222222",
            },
        ]

        def fake_make_request(method, path, **_kwargs):
            self.assertEqual(method, "GET")
            self.assertEqual(path, "/admin/mails")
            return {"results": mails}

        service._make_request = fake_make_request

        code = service.get_verification_code(
            email="user@example.com",
            timeout=1,
            poll_interval=1,
            otp_sent_at=200,
        )

        self.assertEqual(code, "222222")

    def test_ignores_six_digits_inside_recipient_email_domain(self):
        service = TempMailService(
            config={
                "base_url": "https://mail.example.com",
                "admin_password": "test-password",
                "domain": "example.com",
            }
        )

        mails = [
            {
                "id": "mail-domain-digits",
                "created_at": 220,
                "from": "OpenAI <noreply@openai.com>",
                "subject": "OpenAI verification message",
                "text": "",
                "raw": (
                    "To: guakcuwf6sn@a.mail.920508.xyz\n"
                    "From: OpenAI <noreply@openai.com>\n"
                    "Subject: OpenAI verification message\n"
                ),
            },
        ]

        def fake_make_request(method, path, **_kwargs):
            self.assertEqual(method, "GET")
            self.assertEqual(path, "/admin/mails")
            return {"results": mails}

        service._make_request = fake_make_request

        code = service.get_verification_code(
            email="guakcuwf6sn@a.mail.920508.xyz",
            timeout=1,
            poll_interval=1,
            otp_sent_at=200,
        )

        self.assertIsNone(code)


if __name__ == "__main__":
    unittest.main()
