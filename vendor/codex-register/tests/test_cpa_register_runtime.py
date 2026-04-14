import importlib.util
import sys
import unittest
from pathlib import Path


def load_cpa_page_driver_module():
    module_name = "src.core.cpa_page_driver"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "core"
        / "cpa_page_driver.py"
    )

    for name in list(sys.modules):
        if name == "src" or name.startswith("src."):
            sys.modules.pop(name, None)

    src_dir = Path(__file__).resolve().parents[1] / "src"
    core_dir = src_dir / "core"
    src_pkg = type(sys)("src")
    src_pkg.__path__ = [str(src_dir)]
    core_pkg = type(sys)("src.core")
    core_pkg.__path__ = [str(core_dir)]
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


class CPAPageDriverTests(unittest.TestCase):
    def test_classify_signup_state_prefers_existing_account_signal(self):
        module = load_cpa_page_driver_module()

        state = module.classify_signup_state(
            {
                "is_signup_password_page": True,
                "page_text": "Account associated with this email address already exists",
                "has_retry_button": False,
            }
        )

        self.assertEqual(state["kind"], "email_exists")
        self.assertFalse(state["retryable"])

    def test_classify_signup_state_marks_password_retry_as_retryable(self):
        module = load_cpa_page_driver_module()

        state = module.classify_signup_state(
            {
                "is_signup_password_page": True,
                "page_text": "Something went wrong. Operation timed out.",
                "has_retry_button": True,
            }
        )

        self.assertEqual(state["kind"], "password_retry")
        self.assertTrue(state["retryable"])


if __name__ == "__main__":
    unittest.main()
