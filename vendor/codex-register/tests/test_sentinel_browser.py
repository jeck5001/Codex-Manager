import importlib.util
import sys
import unittest
from pathlib import Path


MODULE_PATH = (
    Path(__file__).resolve().parents[1]
    / "src"
    / "core"
    / "sentinel_browser.py"
)


def load_module():
    module_name = "src.core.sentinel_browser"
    spec = importlib.util.spec_from_file_location(module_name, MODULE_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


SENTINEL_BROWSER = load_module()


class SentinelBrowserTests(unittest.TestCase):
    def test_parse_cookie_str_uses_url_for_host_prefixed_cookie(self):
        cookies = SENTINEL_BROWSER._parse_cookie_str(
            "__Host-next-auth.csrf-token=abc123; cf_clearance=def456",
            ".openai.com",
        )

        host_cookie = next(item for item in cookies if item["name"] == "__Host-next-auth.csrf-token")
        self.assertEqual(host_cookie["url"], "https://auth.openai.com/")
        self.assertNotIn("domain", host_cookie)

        regular_cookie = next(item for item in cookies if item["name"] == "cf_clearance")
        self.assertEqual(regular_cookie["domain"], ".openai.com")
        self.assertEqual(regular_cookie["path"], "/")


if __name__ == "__main__":
    unittest.main()
