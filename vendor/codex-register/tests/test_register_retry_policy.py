import importlib.util
import sys
import types
import unittest
from pathlib import Path


MODULE_NAME = "src.core.register_retry_policy"
MODULE_PATH = (
    Path(__file__).resolve().parents[1]
    / "src"
    / "core"
    / "register_retry_policy.py"
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


RETRY_POLICY_MODULE = load_module()


class RegisterRetryPolicyTests(unittest.TestCase):
    def test_retries_transient_authorize_failure(self):
        self.assertTrue(
            RETRY_POLICY_MODULE.should_retry_register_error("authorize continue failed")
        )

    def test_retries_email_otp_failure(self):
        self.assertTrue(RETRY_POLICY_MODULE.should_retry_register_error("邮箱验证码超时"))

    def test_does_not_retry_missing_email_service(self):
        self.assertFalse(
            RETRY_POLICY_MODULE.should_retry_register_error("no available email service")
        )


if __name__ == "__main__":
    unittest.main()
