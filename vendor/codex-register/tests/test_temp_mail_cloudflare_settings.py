import unittest
from pydantic import ValidationError

from src.config import settings as settings_module


class TempMailCloudflareSettingsTests(unittest.TestCase):
    def test_settings_default_email_code_timeout_is_240_seconds(self):
        settings = settings_module.Settings()

        self.assertEqual(settings.email_code_timeout, 240)

    def test_settings_accepts_json_string_for_temp_mail_domain_configs(self):
        settings = settings_module.Settings(
            temp_mail_domain_configs='[{"id":"cfg-1","domain_base":"a.example.com"}]'
        )

        self.assertEqual(
            settings.temp_mail_domain_configs,
            [{"id": "cfg-1", "domain_base": "a.example.com"}],
        )

    def test_settings_accepts_empty_json_string_for_temp_mail_domain_configs(self):
        settings = settings_module.Settings(temp_mail_domain_configs="[]")

        self.assertEqual(settings.temp_mail_domain_configs, [])

    def test_update_settings_persists_cloudflare_temp_mail_fields(self):
        original_save = settings_module._save_settings_to_db
        original_settings = settings_module._settings
        saved = {}

        try:
            settings_module._settings = settings_module.Settings()
            settings_module._save_settings_to_db = lambda **kwargs: saved.update(kwargs)

            updated = settings_module.update_settings(
                cloudflare_api_token="token-1",
                cloudflare_api_email="admin@example.com",
                cloudflare_global_api_key="global-key-1",
                cloudflare_account_id="acc-1",
                cloudflare_zone_id="zone-1",
                cloudflare_worker_name="temp-email",
                temp_mail_domain_base="mail.example.com",
                temp_mail_subdomain_mode="random",
                temp_mail_subdomain_length=6,
                temp_mail_subdomain_prefix="tm",
                temp_mail_sync_cloudflare_enabled=True,
                temp_mail_require_cloudflare_sync=True,
            )

            self.assertEqual(
                updated.cloudflare_api_token.get_secret_value(),
                "token-1",
            )
            self.assertEqual(updated.cloudflare_api_email, "admin@example.com")
            self.assertEqual(updated.cloudflare_global_api_key.get_secret_value(), "global-key-1")
            self.assertEqual(updated.temp_mail_domain_base, "mail.example.com")
            self.assertEqual(saved["cloudflare_zone_id"], "zone-1")
            self.assertEqual(saved["cloudflare_api_email"], "admin@example.com")
            self.assertTrue(saved["temp_mail_require_cloudflare_sync"])
        finally:
            settings_module._save_settings_to_db = original_save
            settings_module._settings = original_settings

    def test_update_settings_rejects_invalid_subdomain_mode(self):
        original_save = settings_module._save_settings_to_db
        original_settings = settings_module._settings

        try:
            settings_module._settings = settings_module.Settings()
            settings_module._save_settings_to_db = lambda **kwargs: None
            with self.assertRaises(ValidationError):
                settings_module.update_settings(temp_mail_subdomain_mode="invalid")
        finally:
            settings_module._save_settings_to_db = original_save
            settings_module._settings = original_settings

    def test_update_settings_rejects_invalid_subdomain_length(self):
        original_save = settings_module._save_settings_to_db
        original_settings = settings_module._settings

        try:
            settings_module._settings = settings_module.Settings()
            settings_module._save_settings_to_db = lambda **kwargs: None
            with self.assertRaises(ValidationError):
                settings_module.update_settings(temp_mail_subdomain_length=100)
        finally:
            settings_module._save_settings_to_db = original_save
            settings_module._settings = original_settings
