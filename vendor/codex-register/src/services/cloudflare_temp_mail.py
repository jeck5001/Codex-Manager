"""Cloudflare Temp Mail provisioning helpers."""

import json
from typing import Any, Dict, List, Optional

from ..core.http_client import HTTPClient, RequestConfig
from ..config.settings import Settings
from pydantic.types import SecretStr


class CloudflareProvisioningError(Exception):
    """Raised when required Cloudflare settings are missing or invalid."""


def parse_domains_binding(raw: Optional[str]) -> List[str]:
    if not raw:
        return []

    trimmed = str(raw).strip()
    if not trimmed:
        return []

    try:
        parsed = json.loads(trimmed)
    except json.JSONDecodeError:
        return [segment.strip() for segment in trimmed.split(",") if segment.strip()]

    if isinstance(parsed, list):
        return [str(item).strip() for item in parsed if str(item).strip()]

    return []


class CloudflareTempMailProvisioner:
    """Encapsulates Cloudflare Temp Mail provisioning operations."""

    REQUIRED_FIELDS = [
        "cloudflare_api_token",
        "cloudflare_account_id",
        "cloudflare_zone_id",
        "cloudflare_worker_name",
        "temp_mail_domain_base",
    ]

    def __init__(self, settings: Settings, http_client: Optional[HTTPClient] = None):
        self.settings = settings
        self._api_token = self._extract_token(settings)
        self.validate_settings(settings)
        self.http_client = http_client or HTTPClient(config=RequestConfig())

    @staticmethod
    def _extract_token(settings: Settings) -> str:
        raw_token = settings.cloudflare_api_token
        if isinstance(raw_token, SecretStr):
            return raw_token.get_secret_value()
        return str(raw_token or "").strip()

    @classmethod
    def validate_settings(cls, settings: Settings) -> None:
        missing: List[str] = []

        if not cls._extract_token(settings):
            missing.append("cloudflare_api_token")

        if not settings.cloudflare_account_id:
            missing.append("cloudflare_account_id")
        if not settings.cloudflare_zone_id:
            missing.append("cloudflare_zone_id")
        if not settings.cloudflare_worker_name:
            missing.append("cloudflare_worker_name")
        if not settings.temp_mail_domain_base:
            missing.append("temp_mail_domain_base")

        if missing:
            raise CloudflareProvisioningError(
                f"Missing required Cloudflare settings: {', '.join(missing)}"
            )

    def _compose_domain(self, label: str) -> str:
        prefix = str(self.settings.temp_mail_subdomain_prefix or "").strip().rstrip("-")
        base = str(self.settings.temp_mail_domain_base or "").strip()
        if not base:
            raise CloudflareProvisioningError("temp_mail_domain_base is required to build domains")
        label_part = f"{prefix}-{label}" if prefix else label
        return f"{label_part}.{base}"

    def _upsert_domains_binding(
        self, bindings: List[Dict[str, Any]], domain: str
    ) -> List[Dict[str, Any]]:
        updated: List[Dict[str, Any]] = []
        replaced = False
        for binding in bindings:
            if binding.get("name") == "DOMAINS":
                domains = parse_domains_binding(binding.get("text"))
                if domain not in domains:
                    domains.append(domain)
                updated.append({**binding, "type": "plain_text", "text": json.dumps(domains)})
                replaced = True
                continue
            updated.append(binding)

        if not replaced:
            updated.append({
                "type": "plain_text",
                "name": "DOMAINS",
                "text": json.dumps([domain]),
            })
        return updated

    def _worker_settings_url(self) -> str:
        account_id = self.settings.cloudflare_account_id
        worker_name = self.settings.cloudflare_worker_name
        return (
            f"https://api.cloudflare.com/client/v4/accounts/{account_id}"
            f"/workers/scripts/{worker_name}/settings"
        )

    def _zones_subdomain_url(self) -> str:
        zone_id = self.settings.cloudflare_zone_id
        return (
            f"https://api.cloudflare.com/client/v4/zones/{zone_id}"
            "/email/sending/subdomains"
        )

    def _auth_headers(self) -> Dict[str, str]:
        return {
            "Authorization": f"Bearer {self._api_token}",
            "Content-Type": "application/json",
        }

    def create_subdomain(self, domain: str) -> Any:
        return self.http_client.request(
            "POST",
            self._zones_subdomain_url(),
            json={"name": domain},
            headers=self._auth_headers(),
        )

    def get_worker_settings(self) -> Any:
        return self.http_client.request(
            "GET",
            self._worker_settings_url(),
            headers=self._auth_headers(),
        )

    def patch_worker_settings(self, bindings: List[Dict[str, Any]]) -> Any:
        return self.http_client.request(
            "PATCH",
            self._worker_settings_url(),
            json={"settings": {"bindings": bindings}},
            headers=self._auth_headers(),
        )
