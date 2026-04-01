import importlib.util
import sys
import types
import unittest
from pathlib import Path


MODULE_NAME = "src.core.register_flow_state"
MODULE_PATH = (
    Path(__file__).resolve().parents[1]
    / "src"
    / "core"
    / "register_flow_state.py"
)


def load_module():
    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg

    spec = importlib.util.spec_from_file_location(MODULE_NAME, MODULE_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[MODULE_NAME] = module
    spec.loader.exec_module(module)
    return module


FLOW_STATE_MODULE = load_module()


class RegisterFlowStateTests(unittest.TestCase):
    def test_extract_auth_page_type_prefers_nested_page_type(self):
        payload = {"page": {"type": "create_account_password"}, "type": "ignored"}
        self.assertEqual(
            FLOW_STATE_MODULE.extract_auth_page_type(payload),
            "create_account_password",
        )

    def test_extract_auth_continue_url_supports_multiple_aliases(self):
        payload = {"redirect_url": "https://auth.openai.com/email-verification"}
        self.assertEqual(
            FLOW_STATE_MODULE.extract_auth_continue_url(payload),
            "https://auth.openai.com/email-verification",
        )

    def test_extract_callback_url_reads_code_from_absolute_url(self):
        url = "http://localhost:1455/auth/callback?code=abc123&state=state123"
        self.assertEqual(FLOW_STATE_MODULE.extract_callback_url(url), url)

    def test_extract_workspace_id_prefers_payload_value(self):
        payload = {"workspace_id": "ws_123"}
        self.assertEqual(FLOW_STATE_MODULE.extract_workspace_id(payload), "ws_123")


if __name__ == "__main__":
    unittest.main()
