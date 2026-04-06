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

    def test_retries_same_message_id_when_content_arrives_later(self):
        service = CustomDomainEmailService(
            config={"base_url": "https://mail.example.com", "api_key": "test-key"}
        )
        service._emails_cache["email-1"] = {"email": "user@example.com"}

        messages = [
            {
                "id": "msg-1",
                "created_at": 220,
                "from_address": "OpenAI <noreply@openai.com>",
                "subject": "Your verification code",
            },
        ]
        content_by_attempt = ["", "OpenAI verification code 654321"]
        attempts = {"count": 0}

        def fake_make_request(method, endpoint, **_kwargs):
            self.assertEqual(method, "GET")
            if endpoint == "/api/emails/email-1":
                return {"messages": messages}
            self.fail(f"unexpected endpoint: {endpoint}")

        def fake_get_message_content(_email_id, _message_id):
            index = min(attempts["count"], len(content_by_attempt) - 1)
            attempts["count"] += 1
            return content_by_attempt[index]

        original_time = CUSTOM_DOMAIN_MODULE.time.time
        original_sleep = CUSTOM_DOMAIN_MODULE.time.sleep
        CUSTOM_DOMAIN_MODULE.time.time = iter([0.0, 0.0, 1.0, 2.1]).__next__
        CUSTOM_DOMAIN_MODULE.time.sleep = lambda _seconds: None
        service._make_request = fake_make_request
        service._get_message_content = fake_get_message_content

        try:
            code = service.get_verification_code(
                email="user@example.com",
                email_id="email-1",
                timeout=2,
                poll_interval=1,
                otp_sent_at=200,
            )
        finally:
            CUSTOM_DOMAIN_MODULE.time.time = original_time
            CUSTOM_DOMAIN_MODULE.time.sleep = original_sleep

        self.assertEqual(code, "654321")


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

    def test_retries_same_mail_id_when_mail_body_arrives_later(self):
        service = TempMailService(
            config={
                "base_url": "https://mail.example.com",
                "admin_password": "test-password",
                "domain": "example.com",
            }
        )

        mail_states = [
            {
                "results": [
                    {
                        "id": "mail-1",
                        "created_at": 220,
                        "from": "OpenAI <noreply@openai.com>",
                        "subject": "Your verification code",
                        "text": "",
                    }
                ]
            },
            {
                "results": [
                    {
                        "id": "mail-1",
                        "created_at": 220,
                        "from": "OpenAI <noreply@openai.com>",
                        "subject": "Your verification code",
                        "text": "654321",
                    }
                ]
            },
        ]
        attempts = {"count": 0}

        def fake_make_request(method, path, **_kwargs):
            self.assertEqual(method, "GET")
            self.assertEqual(path, "/admin/mails")
            index = min(attempts["count"], len(mail_states) - 1)
            attempts["count"] += 1
            return mail_states[index]

        original_time = TEMP_MAIL_MODULE.time.time
        original_sleep = TEMP_MAIL_MODULE.time.sleep
        TEMP_MAIL_MODULE.time.time = iter([0.0, 0.0, 1.0, 2.1]).__next__
        TEMP_MAIL_MODULE.time.sleep = lambda _seconds: None
        service._make_request = fake_make_request

        try:
            code = service.get_verification_code(
                email="user@example.com",
                timeout=2,
                poll_interval=1,
                otp_sent_at=200,
            )
        finally:
            TEMP_MAIL_MODULE.time.time = original_time
            TEMP_MAIL_MODULE.time.sleep = original_sleep

        self.assertEqual(code, "654321")

    def test_finds_target_mail_when_admin_mail_list_is_shared_and_busy(self):
        service = TempMailService(
            config={
                "base_url": "https://mail.example.com",
                "admin_password": "test-password",
                "domain": "example.com",
            }
        )

        mails = []
        for index in range(30):
            mails.append(
                {
                    "id": f"other-{index}",
                    "address": f"other-{index}@example.com",
                    "created_at": 200 + index,
                    "from": "OpenAI <noreply@openai.com>",
                    "subject": "Your verification code",
                    "text": f"{100000 + index}",
                }
            )
        mails[24] = {
            "id": "target-mail",
            "address": "user@example.com",
            "created_at": 260,
            "from": "OpenAI <noreply@openai.com>",
            "subject": "Your verification code",
            "text": "654321",
        }

        def fake_make_request(method, path, **kwargs):
            self.assertEqual(method, "GET")
            self.assertEqual(path, "/admin/mails")
            params = kwargs.get("params", {})
            limit = int(params.get("limit", 0))
            return {"results": mails[:limit]}

        service._make_request = fake_make_request

        code = service.get_verification_code(
            email="user@example.com",
            timeout=1,
            poll_interval=1,
            otp_sent_at=200,
        )

        self.assertEqual(code, "654321")

    def test_scans_multiple_admin_mail_pages_until_target_mail_is_found(self):
        service = TempMailService(
            config={
                "base_url": "https://mail.example.com",
                "admin_password": "test-password",
                "domain": "example.com",
            }
        )

        first_page = [
            {
                "id": f"other-{index}",
                "address": f"other-{index}@example.com",
                "created_at": 200 + index,
                "from": "OpenAI <noreply@openai.com>",
                "subject": "Your verification code",
                "text": f"{100000 + index}",
            }
            for index in range(100)
        ]
        second_page = [
            {
                "id": "target-mail",
                "address": "user@example.com",
                "created_at": 400,
                "from": "OpenAI <noreply@openai.com>",
                "subject": "Your verification code",
                "text": "654321",
            }
        ]
        requested_offsets = []

        def fake_make_request(method, path, **kwargs):
            self.assertEqual(method, "GET")
            self.assertEqual(path, "/admin/mails")
            params = kwargs.get("params", {})
            offset = int(params.get("offset", 0))
            requested_offsets.append(offset)
            if offset == 0:
                return {"results": first_page}
            if offset == 100:
                return {"results": second_page}
            return {"results": []}

        service._make_request = fake_make_request

        code = service.get_verification_code(
            email="user@example.com",
            timeout=1,
            poll_interval=1,
            otp_sent_at=200,
        )

        self.assertEqual(code, "654321")
        self.assertEqual(requested_offsets[:2], [0, 100])

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
