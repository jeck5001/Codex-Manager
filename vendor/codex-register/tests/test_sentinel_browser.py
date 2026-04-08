import importlib.util
import sys
import types
import unittest
from pathlib import Path


MODULE_PATH = (
    Path(__file__).resolve().parents[1]
    / "src"
    / "core"
    / "sentinel_browser.py"
)
HTTP_CLIENT_MODULE_PATH = (
    Path(__file__).resolve().parents[1]
    / "src"
    / "core"
    / "http_client.py"
)


def load_module():
    module_name = "src.core.sentinel_browser"
    spec = importlib.util.spec_from_file_location(module_name, MODULE_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def load_http_client_module():
    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []

    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg
    sys.modules["src.config"] = config_pkg

    constants_module = types.ModuleType("src.config.constants")
    constants_module.ERROR_MESSAGES = {}
    sys.modules["src.config.constants"] = constants_module

    settings_module = types.ModuleType("src.config.settings")
    settings_module.get_settings = lambda: types.SimpleNamespace()
    sys.modules["src.config.settings"] = settings_module

    module_name = "src.core.http_client"
    spec = importlib.util.spec_from_file_location(module_name, HTTP_CLIENT_MODULE_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


HTTP_CLIENT = load_http_client_module()
SENTINEL_BROWSER = load_module()


class SentinelBrowserTests(unittest.TestCase):
    def test_build_sentinel_target_urls_prefers_frame_then_referer_then_legacy_target(self):
        targets = SENTINEL_BROWSER._build_sentinel_target_urls(
            "https://auth.openai.com/create-account/password"
        )

        self.assertEqual(
            targets,
            [
                "https://sentinel.openai.com/backend-api/sentinel/frame.html?sv=20260219f9f6",
                "https://auth.openai.com/create-account/password",
                "https://auth.openai.com/about-you",
            ],
        )

    def test_build_sentinel_target_urls_dedupes_frame_and_legacy_target(self):
        frame_url = "https://sentinel.openai.com/backend-api/sentinel/frame.html?sv=20260219f9f6"

        self.assertEqual(
            SENTINEL_BROWSER._build_sentinel_target_urls(frame_url),
            [
                frame_url,
                "https://auth.openai.com/about-you",
            ],
        )
        self.assertEqual(
            SENTINEL_BROWSER._build_sentinel_target_urls("https://auth.openai.com/about-you"),
            [
                frame_url,
                "https://auth.openai.com/about-you",
            ],
        )

    def test_browser_sentinel_user_agent_matches_openai_http_client(self):
        client = HTTP_CLIENT.OpenAIHTTPClient()
        self.assertEqual(
            SENTINEL_BROWSER.DEFAULT_SENTINEL_USER_AGENT,
            client.default_headers["User-Agent"],
        )

    def test_openai_http_client_uses_fixed_chrome120_impersonation(self):
        client = HTTP_CLIENT.OpenAIHTTPClient()
        self.assertEqual(client.config.impersonate, "chrome120")

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
