import importlib.util
import sys
import types
import unittest
from pathlib import Path

from fastapi import FastAPI
from fastapi.testclient import TestClient
from pydantic import SecretStr

from src.config import settings as settings_module


def _load_settings_router():
    routes_dir = Path(__file__).resolve().parents[1] / "src" / "web" / "routes"
    pkg_name = "src.web.routes"
    if pkg_name not in sys.modules:
        routes_pkg = types.ModuleType(pkg_name)
        routes_pkg.__path__ = [str(routes_dir)]
        sys.modules[pkg_name] = routes_pkg

    spec = importlib.util.spec_from_file_location(
        "src.web.routes.settings",
        routes_dir / "settings.py",
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules["src.web.routes.settings"] = module
    spec.loader.exec_module(module)
    return module.router


settings_router = _load_settings_router()


class TempMailCloudflareRoutesTests(unittest.TestCase):
    def setUp(self):
        self.original_settings = settings_module._settings
        self.original_save = settings_module._save_settings_to_db
        self.updated_values = {}
        settings_module._settings = settings_module.Settings()
        settings_module._save_settings_to_db = lambda **kwargs: self.updated_values.update(kwargs)

        app = FastAPI()
        app.include_router(settings_router, prefix="/api/settings")
        self.client = TestClient(app)

    def tearDown(self):
        settings_module._save_settings_to_db = self.original_save
        settings_module._settings = self.original_settings

    def test_get_cloudflare_settings_hides_raw_token(self):
        settings_module._settings.cloudflare_api_token = SecretStr("secret-value")
        settings_module._settings.cloudflare_global_api_key = SecretStr("global-key")
        settings_module._settings.cloudflare_api_email = "admin@example.com"
        response = self.client.get("/api/settings/temp-mail/cloudflare")
        self.assertEqual(response.status_code, 200)
        payload = response.json()
        self.assertTrue(payload["has_api_token"])
        self.assertTrue(payload["has_global_api_key"])
        self.assertEqual(payload["cloudflare_api_email"], "admin@example.com")
        self.assertNotIn("cloudflare_api_token", payload)
        self.assertNotIn("cloudflare_global_api_key", payload)

    def test_post_cloudflare_settings_updates_values(self):
        payload = {
            "cloudflare_api_token": "token-1",
            "cloudflare_api_email": "admin@example.com",
            "cloudflare_global_api_key": "global-key-1",
            "cloudflare_account_id": "acc-1",
            "cloudflare_zone_id": "zone-1",
            "cloudflare_worker_name": "temp-email",
            "temp_mail_domain_base": "mail.example.com",
            "temp_mail_subdomain_mode": "random",
            "temp_mail_subdomain_length": 6,
            "temp_mail_subdomain_prefix": "tm",
            "temp_mail_sync_cloudflare_enabled": True,
            "temp_mail_require_cloudflare_sync": True,
        }
        response = self.client.post("/api/settings/temp-mail/cloudflare", json=payload)
        self.assertEqual(response.status_code, 200)
        self.assertEqual(settings_module._settings.temp_mail_domain_base, payload["temp_mail_domain_base"])
        self.assertTrue(settings_module._settings.temp_mail_sync_cloudflare_enabled)
        self.assertEqual(self.updated_values["cloudflare_zone_id"], payload["cloudflare_zone_id"])
        self.assertEqual(self.updated_values["cloudflare_api_email"], payload["cloudflare_api_email"])
        self.assertEqual(self.updated_values["temp_mail_require_cloudflare_sync"], True)
        get_response = self.client.get("/api/settings/temp-mail/cloudflare")
        self.assertTrue(get_response.json()["has_api_token"])
        self.assertTrue(get_response.json()["has_global_api_key"])
        self.assertNotIn("cloudflare_api_token", get_response.json())
        self.assertNotIn("cloudflare_global_api_key", get_response.json())

    def test_post_invalid_subdomain_mode_is_rejected(self):
        response = self.client.post(
            "/api/settings/temp-mail/cloudflare",
            json={"temp_mail_subdomain_mode": "invalid"}
        )
        self.assertEqual(response.status_code, 422)

    def test_post_invalid_subdomain_length_is_rejected(self):
        response = self.client.post(
            "/api/settings/temp-mail/cloudflare",
            json={"temp_mail_subdomain_length": 20}
        )
        self.assertEqual(response.status_code, 422)

    def test_post_empty_body_returns_no_changes(self):
        response = self.client.post("/api/settings/temp-mail/cloudflare", json={})
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.json()["message"], "Cloudflare 临时邮箱设置未修改")
        self.assertEqual(self.updated_values, {})
