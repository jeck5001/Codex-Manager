import importlib.util
import sys
import types
import unittest
from pathlib import Path
from urllib.parse import parse_qs, urlparse


def load_oauth_module():
    curl_module = types.ModuleType("curl_cffi")
    curl_requests_module = types.ModuleType("curl_cffi.requests")
    curl_module.requests = curl_requests_module
    sys.modules["curl_cffi"] = curl_module
    sys.modules["curl_cffi.requests"] = curl_requests_module

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg
    sys.modules["src.config"] = config_pkg

    constants_name = "src.config.constants"
    constants_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "config"
        / "constants.py"
    )
    constants_spec = importlib.util.spec_from_file_location(constants_name, constants_path)
    assert constants_spec and constants_spec.loader
    constants_module = importlib.util.module_from_spec(constants_spec)
    sys.modules[constants_name] = constants_module
    constants_spec.loader.exec_module(constants_module)

    module_name = "src.core.oauth"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "core"
        / "oauth.py"
    )
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


OAUTH_MODULE = load_oauth_module()


class OAuthConfigTests(unittest.TestCase):
    def test_generate_oauth_url_includes_codex_originator_and_connector_scopes(self):
        start = OAUTH_MODULE.generate_oauth_url()
        query = parse_qs(urlparse(start.auth_url).query)

        self.assertEqual(query.get("originator"), ["codex_cli_rs"])
        self.assertEqual(
            query.get("scope"),
            ["openid profile email offline_access api.connectors.read api.connectors.invoke"],
        )
        self.assertIsNone(query.get("prompt"))


if __name__ == "__main__":
    unittest.main()
