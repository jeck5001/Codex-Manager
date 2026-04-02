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
    crud_module.appended_logs = []
    def append_task_log(_db, task_uuid, log_message):
        crud_module.appended_logs.append((task_uuid, log_message))
        return True
    crud_module.append_task_log = append_task_log
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


class BrowserbaseDDGRunnerLogTests(unittest.TestCase):
    def test_log_persists_task_log_when_task_uuid_present(self):
        BROWSERBASE_DDG_MODULE.crud.appended_logs.clear()
        captured = []
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={},
            callback_logger=captured.append,
            task_uuid="task-123",
        )

        runner._log("hello")

        self.assertTrue(captured)
        self.assertEqual(len(BROWSERBASE_DDG_MODULE.crud.appended_logs), 1)
        task_uuid, log_message = BROWSERBASE_DDG_MODULE.crud.appended_logs[0]
        self.assertEqual(task_uuid, "task-123")
        self.assertIn("hello", log_message)


class DummyWebSocketConnection:
    def __init__(self, responses):
        self._responses = list(responses)
        self.sent = []
        self.closed = False

    def send(self, payload):
        self.sent.append(payload)

    def recv(self):
        if not self._responses:
            raise RuntimeError("no more websocket responses")
        return self._responses.pop(0)

    def close(self):
        self.closed = True


class BrowserbaseDDGRunnerWaitTests(unittest.TestCase):
    def test_wait_for_target_url_accepts_target_title_keyword(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={},
        )
        conn = DummyWebSocketConnection([
            '{"id": 2, "result": {"targetInfos": [{"type": "page", "url": "https://chatgpt.com/", "title": "MISSION_ACCOMPLISHED"}]}}'
        ])
        original_create_connection = BROWSERBASE_DDG_MODULE.websocket.create_connection
        original_sleep = BROWSERBASE_DDG_MODULE.time.sleep
        try:
            BROWSERBASE_DDG_MODULE.websocket.create_connection = lambda *_args, **_kwargs: conn
            BROWSERBASE_DDG_MODULE.time.sleep = lambda *_args, **_kwargs: None
            matched = runner._wait_for_target_url("wss://example", "MISSION_ACCOMPLISHED", 1)
        finally:
            BROWSERBASE_DDG_MODULE.websocket.create_connection = original_create_connection
            BROWSERBASE_DDG_MODULE.time.sleep = original_sleep

        self.assertEqual(matched, "https://chatgpt.com/")
        self.assertTrue(conn.closed)


class DummyStreamingResponse:
    def __init__(self, status_code=200, text=""):
        self.status_code = status_code
        self.text = text
        self.closed = False

    def close(self):
        self.closed = True


class BrowserbaseDDGRunnerAgentTests(unittest.TestCase):
    def test_agent_model_normalizes_legacy_computer_use_name(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"agent_model": "computer-use-preview"},
        )

        self.assertEqual(
            runner._agent_model(),
            "google/gemini-2.5-computer-use-preview-10-2025",
        )

    def test_send_agent_goal_keeps_stream_open(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"agent_model": "computer-use-preview"},
        )
        captured = {}
        response = DummyStreamingResponse(200, "")

        def fake_http(method, url, **kwargs):
            captured["method"] = method
            captured["url"] = url
            captured["kwargs"] = kwargs
            return response

        runner._http = fake_http

        returned = runner._send_agent_goal("session-1", "do something")

        self.assertIs(returned, response)
        self.assertFalse(response.closed)
        self.assertIn("google/gemini-2.5-computer-use-preview-10-2025", captured["url"])


class BrowserbaseDDGRunnerPromptTests(unittest.TestCase):
    def test_build_phase1_goal_requires_direct_chatgpt_navigation(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"mail_inbox_url": "https://mail.example.com"},
        )

        goal = runner._build_phase1_goal(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            email="user@example.com",
            password="pass123",
            full_name="Test User",
            birthdate="1990-01-01",
        )

        self.assertIn("https://auth.openai.com/oauth/authorize?client_id=test", goal)
        self.assertIn("不要使用 Google、DuckDuckGo 或任何搜索引擎", goal)

    def test_phase_timeout_seconds_upgrades_legacy_short_timeout(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"max_wait_seconds": "300"},
        )

        self.assertEqual(runner._phase_timeout_seconds(), 900)


class BrowserbaseDDGRunnerGoalTests(unittest.TestCase):
    def test_build_phase1_goal_starts_from_oauth_auth_url_and_targets_localhost_callback(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"mail_inbox_url": "https://mail.example/inbox"},
        )

        goal = runner._build_phase1_goal(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            email="user@duck.com",
            password="pass123",
            full_name="Test User",
            birthdate="1990-01-01",
        )

        self.assertIn("https://auth.openai.com/oauth/authorize?client_id=test", goal)
        self.assertIn("localhost", goal)
        self.assertNotIn("MISSION_ACCOMPLISHED", goal)

    def test_build_phase1_goal_forbids_search_engines(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={"mail_inbox_url": "https://mail.example/inbox"},
        )

        goal = runner._build_phase1_goal(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
            email="user@duck.com",
            password="pass123",
            full_name="Test User",
            birthdate="1990-01-01",
        )

        self.assertIn("不要使用 Google、DuckDuckGo 或任何搜索引擎", goal)


class BrowserbaseDDGRunnerNavigationTests(unittest.TestCase):
    def test_open_target_url_creates_page_target(self):
        runner = BrowserbaseDDGRegistrationRunner(
            profile_id=1,
            profile_name="demo",
            profile_config={},
        )
        conn = DummyWebSocketConnection([
            '{"id": 1, "result": {"targetId": "target-1"}}'
        ])
        original_create_connection = BROWSERBASE_DDG_MODULE.websocket.create_connection
        try:
            BROWSERBASE_DDG_MODULE.websocket.create_connection = lambda *_args, **_kwargs: conn
            target_id = runner._open_target_url("wss://example", "https://auth.openai.com/test")
        finally:
            BROWSERBASE_DDG_MODULE.websocket.create_connection = original_create_connection

        self.assertEqual(target_id, "target-1")
        sent_payload = ''.join(conn.sent)
        self.assertIn('Target.createTarget', sent_payload)
        self.assertIn('https://auth.openai.com/test', sent_payload)
        self.assertTrue(conn.closed)
