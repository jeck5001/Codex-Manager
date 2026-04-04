import json
import unittest

from pydantic.types import SecretStr

from src.config.settings import Settings
from src.services import cloudflare_temp_mail


class CloudflareTempMailProvisionerTests(unittest.TestCase):
    def make_settings(self, **overrides) -> Settings:
        base_kwargs = {
            "cloudflare_api_token": SecretStr("token"),
            "cloudflare_account_id": "acct",
            "cloudflare_zone_id": "zone",
            "cloudflare_worker_name": "worker",
            "temp_mail_domain_base": "mail.example.com",
            "temp_mail_subdomain_prefix": "tm",
        }
        base_kwargs.update(overrides)
        return Settings(**base_kwargs)

    def test_parse_domains_binding_accepts_json_array_text(self):
        raw = json.dumps(["alpha.example.com", "beta.example.com"])
        parsed = cloudflare_temp_mail.parse_domains_binding(raw)
        self.assertEqual(parsed, ["alpha.example.com", "beta.example.com"])

    def test_parse_domains_binding_accepts_comma_separated_text(self):
        raw = "alpha.example.com, beta.example.com , gamma.example.com"
        parsed = cloudflare_temp_mail.parse_domains_binding(raw)
        self.assertEqual(parsed, ["alpha.example.com", "beta.example.com", "gamma.example.com"])

    def test_compose_domain_uses_prefix_label_and_base(self):
        settings = self.make_settings(
            temp_mail_subdomain_prefix="tm",
            temp_mail_domain_base="mail.example.com",
        )
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings)
        built = provisioner._compose_domain("abc123")
        self.assertEqual(built, "tm-abc123.mail.example.com")

    def test_upsert_domains_binding_appends_idempotently(self):
        settings = self.make_settings()
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings)
        existing = {"type": "secret_text", "name": "DOMAINS", "text": json.dumps(["exist.example.com"])}
        bindings = [existing]
        updated = provisioner._upsert_domains_binding(bindings, "new.example.com")
        binding = next(b for b in updated if b.get("name") == "DOMAINS")
        self.assertEqual(binding.get("type"), "plain_text")
        parsed = cloudflare_temp_mail.parse_domains_binding(binding["text"])
        self.assertEqual(parsed, ["exist.example.com", "new.example.com"])

        second = provisioner._upsert_domains_binding(updated, "new.example.com")
        binding_again = next(b for b in second if b.get("name") == "DOMAINS")
        parsed_again = cloudflare_temp_mail.parse_domains_binding(binding_again["text"])
        self.assertEqual(parsed_again, ["exist.example.com", "new.example.com"])

    def test_upsert_domains_binding_creates_plain_text_binding(self):
        settings = self.make_settings()
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings)
        bindings: list[dict[str, str]] = []
        updated = provisioner._upsert_domains_binding(bindings, "new.example.com")
        binding = next(b for b in updated if b.get("name") == "DOMAINS")
        self.assertEqual(binding.get("type"), "plain_text")
        parsed = cloudflare_temp_mail.parse_domains_binding(binding["text"])
        self.assertEqual(parsed, ["new.example.com"])

    def test_validate_settings_raises_when_required_fields_missing(self):
        settings = Settings(
            cloudflare_api_token=SecretStr(""),
            cloudflare_account_id="",
            cloudflare_zone_id="",
            cloudflare_worker_name="",
            temp_mail_domain_base="",
        )
        with self.assertRaises(cloudflare_temp_mail.CloudflareProvisioningError) as ctx:
            cloudflare_temp_mail.CloudflareTempMailProvisioner.validate_settings(settings)
        self.assertIn("Missing required Cloudflare settings", str(ctx.exception))
