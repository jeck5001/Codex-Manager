import importlib.util
import sys
import types
import unittest
from pathlib import Path


def load_mail_33_module():
    module_name = "src.services.mail_33_imap"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "services"
        / "mail_33_imap.py"
    )

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    services_pkg = types.ModuleType("src.services")
    services_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []

    sys.modules["src"] = src_pkg
    sys.modules["src.services"] = services_pkg
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
        MAIL_33_IMAP = "mail_33_imap"

    base_module.BaseEmailService = BaseEmailService
    base_module.EmailServiceError = EmailServiceError
    base_module.EmailServiceType = EmailServiceType
    sys.modules["src.services.base"] = base_module

    constants_module = types.ModuleType("src.config.constants")
    constants_module.OTP_CODE_PATTERN = r"(?<!\d)(\d{6})(?!\d)"
    constants_module.OPENAI_VERIFICATION_KEYWORDS = [
        "verify your email",
        "verification code",
        "验证码",
        "your openai code",
        "code is",
        "one-time code",
    ]
    sys.modules["src.config.constants"] = constants_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


MAIL33_MODULE = load_mail_33_module()
Mail33ImapService = MAIL33_MODULE.Mail33ImapService


class Mail33ImapServiceTests(unittest.TestCase):
    def test_create_email_builds_alias_under_33mail_domain(self):
        service = Mail33ImapService(
            config={
                "alias_domain": "alias.33mail.com",
                "real_inbox_email": "real@example.com",
                "imap_host": "imap.example.com",
                "imap_port": 993,
                "imap_username": "real@example.com",
                "imap_password": "secret",
            }
        )

        created = service.create_email({"alias_length": 8})

        self.assertEqual(created["forward_to"], "real@example.com")
        self.assertTrue(created["email"].endswith("@alias.33mail.com"))
        self.assertEqual(created["service_id"], created["email"])
        self.assertEqual(len(created["email"].split("@", 1)[0]), 8)

    def test_extracts_latest_matching_openai_code_from_messages(self):
        service = Mail33ImapService(
            config={
                "alias_domain": "alias.33mail.com",
                "real_inbox_email": "real@example.com",
                "imap_host": "imap.example.com",
                "imap_port": 993,
                "imap_username": "real@example.com",
                "imap_password": "secret",
                "from_filter": "openai.com",
                "subject_keyword": "OpenAI",
                "otp_pattern": r"(?<!\d)(\d{6})(?!\d)",
            }
        )

        messages = [
            {
                "to": "abc@alias.33mail.com",
                "from": "noreply@openai.com",
                "subject": "OpenAI verification code",
                "body": "Old code 111111",
                "timestamp": 100,
            },
            {
                "to": "abc@alias.33mail.com",
                "from": "noreply@openai.com",
                "subject": "OpenAI verification code",
                "body": "New code 222222",
                "timestamp": 200,
            },
        ]

        code = service._extract_verification_code_from_messages(
            email="abc@alias.33mail.com",
            messages=messages,
            otp_sent_at=150,
        )

        self.assertEqual(code, "222222")

    def test_matches_forwarded_33mail_message_when_alias_is_only_in_subject(self):
        service = Mail33ImapService(
            config={
                "alias_domain": "295542345.33mail.com",
                "real_inbox_email": "295542345@qq.com",
                "imap_host": "imap.qq.com",
                "imap_port": 993,
                "imap_username": "295542345@qq.com",
                "imap_password": "secret",
                "from_filter": "openai.com",
                "subject_keyword": "Your ChatGPT code is",
                "otp_pattern": r"(?<!\d)(\d{6})(?!\d)",
            }
        )

        messages = [
            {
                "to": "295542345@qq.com",
                "from": "'noreply@tm.openai.com' via 33Mail <sender@mailer1.33mail.com>",
                "subject": "Your ChatGPT code is 696479 - This email was sent to the alias '4434p1bixfkc@295542345.33mail.com'",
                "body": "Hi there,\nUse this code: 696479",
                "timestamp": 200,
            },
        ]

        code = service._extract_verification_code_from_messages(
            email="4434p1bixfkc@295542345.33mail.com",
            messages=messages,
            otp_sent_at=150,
        )

        self.assertEqual(code, "696479")

    def test_matches_forwarded_33mail_message_with_legacy_openai_subject_keyword(self):
        service = Mail33ImapService(
            config={
                "alias_domain": "295542345.33mail.com",
                "real_inbox_email": "295542345@qq.com",
                "imap_host": "imap.qq.com",
                "imap_port": 993,
                "imap_username": "295542345@qq.com",
                "imap_password": "secret",
                "from_filter": "openai.com",
                "subject_keyword": "OpenAI",
                "otp_pattern": r"(?<!\d)(\d{6})(?!\d)",
            }
        )

        messages = [
            {
                "to": "295542345@qq.com",
                "from": "'noreply@tm.openai.com' via 33Mail <sender@mailer1.33mail.com>",
                "subject": "Your ChatGPT code is 196580",
                "body": "OpenAI\nChatGPT Log-in Code\nUse this code: 196580\n"
                "This email was sent to the alias 'jvdf5mmde0h9@295542345.33mail.com'",
                "timestamp": 200,
            },
        ]

        code = service._extract_verification_code_from_messages(
            email="jvdf5mmde0h9@295542345.33mail.com",
            messages=messages,
            otp_sent_at=150,
        )

        self.assertEqual(code, "196580")

    def test_matches_multiple_sender_filters_separated_by_chinese_delimiter(self):
        service = Mail33ImapService(
            config={
                "alias_domain": "295542345.33mail.com",
                "real_inbox_email": "295542345@qq.com",
                "imap_host": "imap.qq.com",
                "imap_port": 993,
                "imap_username": "295542345@qq.com",
                "imap_password": "secret",
                "from_filter": "sender@mailer1.33mail.com、openai.com",
                "subject_keyword": "Your ChatGPT code is",
                "otp_pattern": r"(?<!\d)(\d{6})(?!\d)",
            }
        )

        messages = [
            {
                "to": "295542345@qq.com",
                "from": "'noreply@tm.openai.com' via 33Mail <sender@mailer1.33mail.com>",
                "subject": "Your ChatGPT code is 215600",
                "body": "Hi there,\nUse this code: 215600\n"
                "This email was sent to the alias '4434p1bixfkc@295542345.33mail.com'",
                "timestamp": 200,
            },
        ]

        code = service._extract_verification_code_from_messages(
            email="4434p1bixfkc@295542345.33mail.com",
            messages=messages,
            otp_sent_at=150,
        )

        self.assertEqual(code, "215600")


if __name__ == "__main__":
    unittest.main()
