import json
import unittest
from typing import Any, List

from pydantic.types import SecretStr

from src.config.settings import Settings
from src.services import cloudflare_temp_mail


class CloudflareTempMailProvisionerTests(unittest.TestCase):
    def make_settings(self, **overrides) -> Settings:
        base_kwargs = {
            "cloudflare_api_token": SecretStr("token"),
            "cloudflare_api_email": "",
            "cloudflare_global_api_key": SecretStr(""),
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

    def test_cloudflare_wrapper_returns_parsed_json(self):
        settings = self.make_settings()
        response = self._make_response(200, {"id": "subdomain"})
        http_client = self._DummyHttpClient([response])
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings, http_client=http_client)

        result = provisioner.create_subdomain("new.example.com")
        self.assertEqual(result, {"id": "subdomain"})
        self.assertEqual(
            http_client.requests[0]["kwargs"]["headers"]["Authorization"],
            "Bearer token",
        )

    def test_cloudflare_wrapper_raises_on_http_error(self):
        settings = self.make_settings()
        response = self._make_response(500, {"errors": ["oops"]})
        http_client = self._DummyHttpClient([response])
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings, http_client=http_client)

        with self.assertRaises(cloudflare_temp_mail.CloudflareProvisioningError) as ctx:
            provisioner.patch_worker_settings([])
        self.assertIn("failed with status 500", str(ctx.exception))

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
        expected_text = json.dumps(
            ["exist.example.com", "new.example.com"]
        )
        self.assertEqual(binding["text"], expected_text)

        second = provisioner._upsert_domains_binding(updated, "new.example.com")
        binding_again = next(b for b in second if b.get("name") == "DOMAINS")
        self.assertEqual(binding_again["text"], expected_text)

    def test_upsert_domains_binding_creates_plain_text_binding(self):
        settings = self.make_settings()
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings)
        bindings: list[dict[str, str]] = []
        updated = provisioner._upsert_domains_binding(bindings, "new.example.com")
        binding = next(b for b in updated if b.get("name") == "DOMAINS")
        self.assertEqual(binding.get("type"), "plain_text")
        expected_text = json.dumps(["new.example.com"])
        self.assertEqual(binding["text"], expected_text)

    def test_upsert_domains_binding_preserves_json_binding(self):
        settings = self.make_settings()
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings)
        binding = {
            "name": "DOMAINS",
            "json": ["alpha.example.com", "beta.example.com"],
        }
        updated = provisioner._upsert_domains_binding([binding], "beta.example.com")
        binding_result = next(b for b in updated if b.get("name") == "DOMAINS")
        self.assertEqual(binding_result.get("type"), "plain_text")
        self.assertEqual(binding_result["text"], json.dumps(["alpha.example.com", "beta.example.com"]))

    def test_validate_settings_raises_when_required_fields_missing(self):
        settings = Settings(
            cloudflare_api_token=SecretStr(""),
            cloudflare_api_email="",
            cloudflare_global_api_key=SecretStr(""),
            cloudflare_account_id="",
            cloudflare_zone_id="",
            cloudflare_worker_name="",
            temp_mail_domain_base="",
        )
        with self.assertRaises(cloudflare_temp_mail.CloudflareProvisioningError) as ctx:
            cloudflare_temp_mail.CloudflareTempMailProvisioner.validate_settings(settings)
        self.assertIn("Missing required Cloudflare settings", str(ctx.exception))

    def test_validate_settings_rejects_whitespace_values(self):
        settings = self.make_settings(cloudflare_account_id="   ")
        with self.assertRaises(cloudflare_temp_mail.CloudflareProvisioningError) as ctx:
            cloudflare_temp_mail.CloudflareTempMailProvisioner.validate_settings(settings)
        self.assertIn("cloudflare_account_id", str(ctx.exception))

    def test_validate_settings_accepts_global_api_key_without_token(self):
        settings = self.make_settings(
            cloudflare_api_token=SecretStr(""),
            cloudflare_api_email="admin@example.com",
            cloudflare_global_api_key=SecretStr("global-key"),
        )
        normalized = cloudflare_temp_mail.CloudflareTempMailProvisioner.validate_settings(settings)
        self.assertEqual(normalized["api_email"], "admin@example.com")
        self.assertEqual(normalized["global_api_key"], "global-key")

    def test_create_subdomain_prefers_global_api_key_for_email_api(self):
        settings = self.make_settings(
            cloudflare_api_email="admin@example.com",
            cloudflare_global_api_key=SecretStr("global-key"),
        )
        response = self._make_response(200, {"id": "subdomain"})
        http_client = self._DummyHttpClient([response])
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings, http_client=http_client)

        provisioner.create_subdomain("new.example.com")

        self.assertEqual(
            http_client.requests[0]["url"],
            "https://api.cloudflare.com/client/v4/zones/zone/email/routing/enable",
        )
        self.assertEqual(
            http_client.requests[0]["kwargs"]["json"],
            {"name": "new.example.com"},
        )
        headers = http_client.requests[0]["kwargs"]["headers"]
        self.assertEqual(headers["X-Auth-Email"], "admin@example.com")
        self.assertEqual(headers["X-Auth-Key"], "global-key")
        self.assertNotIn("Authorization", headers)

    def test_worker_settings_prefers_api_token_when_available(self):
        settings = self.make_settings(
            cloudflare_api_email="admin@example.com",
            cloudflare_global_api_key=SecretStr("global-key"),
        )
        response = self._make_response(200, {"result": {"bindings": []}})
        http_client = self._DummyHttpClient([response])
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings, http_client=http_client)

        provisioner.get_worker_settings()

        headers = http_client.requests[0]["kwargs"]["headers"]
        self.assertEqual(headers["Authorization"], "Bearer token")
        self.assertNotIn("X-Auth-Key", headers)

    def test_patch_worker_settings_uses_multipart_metadata_payload(self):
        settings = self.make_settings()
        response = self._make_response(200, {"result": {"bindings": []}})
        http_client = self._DummyHttpClient([response])
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings, http_client=http_client)

        provisioner.patch_worker_settings([{"type": "plain_text", "name": "DOMAINS", "text": "[]"}])

        request = http_client.requests[0]
        self.assertEqual(
            request["url"],
            "https://api.cloudflare.com/client/v4/accounts/acct/workers/scripts/worker/settings",
        )
        self.assertNotIn("json", request["kwargs"])
        self.assertIn("files", request["kwargs"])
        metadata = request["kwargs"]["files"]["metadata"]
        self.assertEqual(metadata[2], "application/json")
        self.assertEqual(
            json.loads(metadata[1]),
            {"bindings": [{"type": "plain_text", "name": "DOMAINS", "text": "[]"}]},
        )
        self.assertNotIn("Content-Type", request["kwargs"]["headers"])
        self.assertEqual(request["kwargs"]["headers"]["Authorization"], "Bearer token")

    def test_delete_subdomain_uses_email_routing_disable_with_domain_name(self):
        settings = self.make_settings(
            cloudflare_api_email="admin@example.com",
            cloudflare_global_api_key=SecretStr("global-key"),
        )
        response = self._make_response(200, {"success": True})
        http_client = self._DummyHttpClient([response])
        provisioner = cloudflare_temp_mail.CloudflareTempMailProvisioner(settings, http_client=http_client)

        provisioner.delete_subdomain("tm-abc.mail.example.com")

        self.assertEqual(http_client.requests[0]["method"], "POST")
        self.assertEqual(
            http_client.requests[0]["url"],
            "https://api.cloudflare.com/client/v4/zones/zone/email/routing/disable",
        )
        self.assertEqual(
            http_client.requests[0]["kwargs"]["json"],
            {"name": "tm-abc.mail.example.com"},
        )

    class _DummyHttpClient:
        def __init__(self, responses: List[Any]):
            self._responses = responses
            self.requests: List[dict[str, Any]] = []

        def request(self, method: str, url: str, **kwargs) -> Any:
            self.requests.append({"method": method, "url": url, "kwargs": kwargs})
            return self._responses.pop(0)

    @staticmethod
    def _make_response(status: int, payload: Any) -> Any:
        class Response:
            def __init__(self, status_code: int, payload: Any):
                self.status_code = status_code
                self._payload = payload

            def json(self):
                return self._payload

        return Response(status, payload)
