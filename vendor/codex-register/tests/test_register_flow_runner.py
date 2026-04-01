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


class RegisterFlowRunnerTests(unittest.TestCase):
    def test_runner_returns_auth_resolution_result_shape(self):
        class EngineStub:
            def _resolve_callback_from_auth_page(self, *_args, **_kwargs):
                return None

            def _resolve_callback_from_continue_url(self, *_args, **_kwargs):
                return None

        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=EngineStub())
        result = runner.resolve_auth_result({"page": {"type": "add_phone"}})
        self.assertEqual(result.page_type, "add_phone")
        self.assertEqual(result.continue_url, "")
        self.assertIsNone(result.callback_url)


if __name__ == "__main__":
    unittest.main()
