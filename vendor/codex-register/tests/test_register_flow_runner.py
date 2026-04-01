import importlib.util
import sys
import types
import unittest
from pathlib import Path


BASE_DIR = Path(__file__).resolve().parents[1] / "src" / "core"


def load_module(module_name: str, file_name: str):
    spec = importlib.util.spec_from_file_location(module_name, BASE_DIR / file_name)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def load_flow_runner_module():
    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg

    load_module("src.core.register_flow_state", "register_flow_state.py")
    load_module("src.core.register_token_resolver", "register_token_resolver.py")
    return load_module("src.core.register_flow_runner", "register_flow_runner.py")


FLOW_RUNNER_MODULE = load_flow_runner_module()


class EngineStub:
    def __init__(self):
        self.calls = []

    def _build_callback_url_from_page(self, page):
        self.calls.append(("build_callback", page))
        return "http://localhost:1455/auth/callback?code=page&state=state"

    def _get_workspace_id(self):
        self.calls.append(("get_workspace_id",))
        return "ws_123"

    def _select_workspace(self, workspace_id):
        self.calls.append(("select_workspace", workspace_id))
        return "https://auth.openai.com/workspace/continue"

    def _follow_redirects(self, url):
        self.calls.append(("follow_redirects", url))
        return "http://localhost:1455/auth/callback?code=redir&state=state"

    def _advance_workspace_authorization(self, url):
        self.calls.append(("advance_workspace", url))
        return "http://localhost:1455/auth/callback?code=phone&state=state"

    def _log(self, *_args, **_kwargs):
        return None


class RegisterFlowRunnerTests(unittest.TestCase):
    def test_runner_returns_auth_resolution_result_shape(self):
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=EngineStub())
        result = runner.resolve_auth_result({"page": {"type": "add_phone"}})
        self.assertEqual(result.page_type, "add_phone")
        self.assertEqual(result.continue_url, "")
        self.assertIsNone(result.callback_url)

    def test_resolve_callback_from_auth_page_uses_token_exchange_builder(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_auth_page(
            {"type": "token_exchange", "continue_url": "https://auth.openai.com/token"},
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=page&state=state",
        )
        self.assertEqual(engine.calls[0][0], "build_callback")

    def test_resolve_callback_from_auth_page_selects_workspace(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_auth_page(
            {"type": "workspace"},
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(
            engine.calls,
            [
                ("get_workspace_id",),
                ("select_workspace", "ws_123"),
                ("follow_redirects", "https://auth.openai.com/workspace/continue"),
            ],
        )

    def test_resolve_callback_from_auth_page_follows_external_url(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_auth_page(
            {"type": "external_url", "payload": {"url": "https://chatgpt.com/auth/callback"}},
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(
            engine.calls,
            [("follow_redirects", "https://chatgpt.com/auth/callback")],
        )

    def test_resolve_callback_from_continue_url_advances_add_phone(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_continue_url(
            "https://auth.openai.com/add-phone",
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=phone&state=state",
        )
        self.assertEqual(
            engine.calls,
            [("advance_workspace", "https://auth.openai.com/add-phone")],
        )


if __name__ == "__main__":
    unittest.main()
