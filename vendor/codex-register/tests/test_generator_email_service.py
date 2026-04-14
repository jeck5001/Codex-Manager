import importlib.util
import sys
import types
import unittest
from pathlib import Path


class _FakeRequestsModule(types.ModuleType):
    def __init__(self):
        super().__init__("curl_cffi.requests")
        self._responses = []

    def queue(self, response):
        self._responses.append(response)

    def get(self, *_args, **_kwargs):
        if not self._responses:
            raise AssertionError("no queued response for requests.get")
        return self._responses.pop(0)


def load_generator_email_module():
    module_name = "src.services.generator_email"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "services"
        / "generator_email.py"
    )

    for name in list(sys.modules):
        if name == "src" or name.startswith("src.") or name.startswith("curl_cffi"):
            sys.modules.pop(name, None)

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
        GENERATOR_EMAIL = "generator_email"

    base_module.BaseEmailService = BaseEmailService
    base_module.EmailServiceError = EmailServiceError
    base_module.EmailServiceType = EmailServiceType
    sys.modules["src.services.base"] = base_module

    constants_module = types.ModuleType("src.config.constants")
    constants_module.EMAIL_SERVICE_DEFAULTS = {
        "generator_email": {
            "base_url": "https://generator.email",
            "timeout": 30,
            "poll_interval": 3,
        }
    }
    constants_module.OTP_CODE_PATTERN = r"(?<!\d)(\d{6})(?!\d)"
    sys.modules["src.config.constants"] = constants_module

    curl_cffi_module = types.ModuleType("curl_cffi")
    requests_module = _FakeRequestsModule()
    curl_cffi_module.requests = requests_module
    sys.modules["curl_cffi"] = curl_cffi_module
    sys.modules["curl_cffi.requests"] = requests_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module, requests_module


class GeneratorEmailServiceTests(unittest.TestCase):
    def test_create_email_parses_generator_homepage(self):
        module, requests_module = load_generator_email_module()
        requests_module.queue(
            types.SimpleNamespace(
                status_code=200,
                text='<span id="email_ch_text">tester.box@generator.local</span>',
            )
        )

        service = module.GeneratorEmailService(name="generator-email")
        created = service.create_email()

        self.assertEqual(created["email"], "tester.box@generator.local")
        self.assertEqual(created["email_id"], "generator.local/tester.box")
        self.assertEqual(created["credentials"]["surl"], "generator.local/tester.box")

    def test_create_email_falls_back_to_hidden_username_and_domain_inputs(self):
        module, requests_module = load_generator_email_module()
        requests_module.queue(
            types.SimpleNamespace(
                status_code=200,
                text="""
                <input id="userName" value="Mixed.User+Tag" />
                <input id="domainName2" value="generator.local" />
                """,
            )
        )

        service = module.GeneratorEmailService(name="generator-email")
        created = service.create_email()

        self.assertEqual(created["email"], "Mixed.User+Tag@generator.local")
        self.assertEqual(created["email_id"], "generator.local/mixed.usertag")

    def test_get_verification_code_reads_latest_openai_otp(self):
        module, requests_module = load_generator_email_module()
        requests_module.queue(
            types.SimpleNamespace(
                status_code=200,
                text="""
                <html>
                  <div>OpenAI verification message</div>
                  <div>Your ChatGPT code is 654321</div>
                </html>
                """,
            )
        )

        service = module.GeneratorEmailService(name="generator-email")
        code = service.get_verification_code(
            email="tester.box@generator.local",
            email_id="generator.local/tester.box",
        )

        self.assertEqual(code, "654321")

    def test_get_verification_code_prefers_contextual_latest_openai_code(self):
        module, requests_module = load_generator_email_module()
        requests_module.queue(
            types.SimpleNamespace(
                status_code=200,
                text="""
                <html>
                  <div>Old unrelated code 111111</div>
                  <div>OpenAI verification notice</div>
                  <div>Use 222222 to continue</div>
                </html>
                """,
            )
        )

        service = module.GeneratorEmailService(name="generator-email")
        code = service.get_verification_code(
            email="tester.box@generator.local",
            email_id="generator.local/tester.box",
        )

        self.assertEqual(code, "222222")


if __name__ == "__main__":
    unittest.main()
