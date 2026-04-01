import importlib.util
import sys
import types
import unittest
from pathlib import Path


MODULE_NAME = "src.core.register_token_resolver"
MODULE_PATH = (
    Path(__file__).resolve().parents[1]
    / "src"
    / "core"
    / "register_token_resolver.py"
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


TOKEN_RESOLVER_MODULE = load_module()


class RegisterTokenResolverTests(unittest.TestCase):
    def test_resolve_callback_url_accepts_absolute_callback(self):
        callback = "http://localhost:1455/auth/callback?code=code123&state=state123"
        self.assertEqual(TOKEN_RESOLVER_MODULE.resolve_callback_url(callback), callback)

    def test_resolve_workspace_id_from_tokens_prefers_explicit_workspace(self):
        payload = {"workspace_id": "ws_explicit", "account_id": "acct_1"}
        resolved = TOKEN_RESOLVER_MODULE.resolve_workspace_id_from_tokens(payload)
        self.assertEqual(resolved, "ws_explicit")


if __name__ == "__main__":
    unittest.main()
