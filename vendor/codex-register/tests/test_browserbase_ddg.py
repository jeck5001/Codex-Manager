import importlib.util
import sys
import types
import unittest
from pathlib import Path


def load_browserbase_ddg_module():
    module_name = "src.core.browserbase_ddg"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "core"
        / "browserbase_ddg.py"
    )

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []
    database_pkg = types.ModuleType("src.database")
    database_pkg.__path__ = []

    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg
    sys.modules["src.config"] = config_pkg
    sys.modules["src.database"] = database_pkg

    websocket_module = types.ModuleType("websocket")
    websocket_module.create_connection = lambda *_args, **_kwargs: None
    sys.modules["websocket"] = websocket_module

    curl_cffi_module = types.ModuleType("curl_cffi")
    curl_cffi_requests_module = types.ModuleType("curl_cffi.requests")
    curl_cffi_requests_module.get = lambda *_args, **_kwargs: None
    curl_cffi_requests_module.post = lambda *_args, **_kwargs: None
    curl_cffi_module.requests = curl_cffi_requests_module
    sys.modules["curl_cffi"] = curl_cffi_module
    sys.modules["curl_cffi.requests"] = curl_cffi_requests_module

    constants_module = types.ModuleType("src.config.constants")
    constants_module.PASSWORD_CHARSET = "abc123"
    constants_module.generate_random_user_info = lambda: {
        "first_name": "Test",
        "last_name": "User",
        "full_name": "Test User",
        "birthdate": "1990-01-01",
    }
    sys.modules["src.config.constants"] = constants_module

    settings_module = types.ModuleType("src.config.settings")
    settings_module.get_settings = lambda: types.SimpleNamespace(proxy_url=None)
    sys.modules["src.config.settings"] = settings_module

    crud_module = types.ModuleType("src.database.crud")
    sys.modules["src.database.crud"] = crud_module

    session_module = types.ModuleType("src.database.session")

    class DummyDbContext:
        def __enter__(self):
            return object()

        def __exit__(self, exc_type, exc, tb):
            return False

    session_module.get_db = lambda: DummyDbContext()
    sys.modules["src.database.session"] = session_module

    oauth_module = types.ModuleType("src.core.oauth")
    oauth_module.generate_oauth_url = lambda *_args, **_kwargs: "https://example.com/oauth"
    oauth_module.submit_callback_url = lambda *_args, **_kwargs: None
    sys.modules["src.core.oauth"] = oauth_module

    register_module = types.ModuleType("src.core.register")
    register_module.RegistrationResult = type("RegistrationResult", (), {})
    sys.modules["src.core.register"] = register_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


BROWSERBASE_DDG_MODULE = load_browserbase_ddg_module()
BrowserbaseDDGRegistrationRunner = BROWSERBASE_DDG_MODULE.BrowserbaseDDGRegistrationRunner


class DummyResponse:
    def __init__(self, status_code, payload):
        self.status_code = status_code
        self._payload = payload

    def json(self):
        return self._payload


class BrowserbaseDDGRunnerTests(unittest.TestCase):
    def test_generate_alias_accepts_201_created_response(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"ddg_token": "token"},
        )
        runner._http = lambda *_args, **_kwargs: DummyResponse(201, {"address": "alias123"})

        email = runner._generate_alias()

        self.assertEqual(email, "alias123@duck.com")


if __name__ == "__main__":
    unittest.main()


class DummyInvalidJsonResponse:
    def __init__(self, status_code=200, text=""):
        self.status_code = status_code
        self.text = text

    def json(self):
        import json
        raise json.JSONDecodeError("Expecting value", self.text, 0)


class BrowserbaseDDGRunnerJsonErrorTests(unittest.TestCase):
    def test_generate_alias_reports_invalid_json_response(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"ddg_token": "token"},
        )
        runner._http = lambda *_args, **_kwargs: DummyInvalidJsonResponse(200, "")

        with self.assertRaisesRegex(RuntimeError, "DDG 邮箱别名生成失败: 返回了空响应或非 JSON 响应"):
            runner._generate_alias()

    def test_create_browserbase_session_reports_invalid_json_response(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={},
        )
        runner._http = lambda *_args, **_kwargs: DummyInvalidJsonResponse(200, "")

        with self.assertRaisesRegex(RuntimeError, "Browserbase 会话创建失败: 返回了空响应或非 JSON 响应"):
            runner._create_browserbase_session()


class BrowserbaseDDGRunnerApiBaseTests(unittest.TestCase):
    def test_browserbase_api_base_normalizes_marketing_site_v1(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"browserbase_api_base": "https://www.browserbase.com/v1"},
        )

        self.assertEqual(
            runner._browserbase_api_base(),
            "https://gemini.browserbase.com",
        )

    def test_browserbase_api_base_defaults_to_gemini_endpoint(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={},
        )

        self.assertEqual(
            runner._browserbase_api_base(),
            "https://gemini.browserbase.com",
        )
