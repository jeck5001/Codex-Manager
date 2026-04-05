"""Cloudflare Temp Mail provisioning helpers."""

import json
import secrets
import string
import time
from urllib.parse import quote
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
        normalized = self.validate_settings(settings)
        self.settings = settings
        self._api_token = normalized["token"]
        self._account_id = normalized["account_id"]
        self._zone_id = normalized["zone_id"]
        self._worker_name = normalized["worker_name"]
        self._domain_base = normalized["domain_base"]
        self._prefix = self._normalize_prefix(settings.temp_mail_subdomain_prefix)
        self.http_client = http_client or HTTPClient(config=RequestConfig())

    @staticmethod
    def _extract_token(settings: Settings) -> str:
        raw_token = settings.cloudflare_api_token
        if isinstance(raw_token, SecretStr):
            return raw_token.get_secret_value()
        return str(raw_token or "").strip()

    @staticmethod
    def _normalize_prefix(value: Optional[str]) -> str:
        prefix = str(value or "").strip()
        return prefix.rstrip("-")

    @staticmethod
    def _normalize_str(value: Optional[str]) -> str:
        return str(value or "").strip()

    @classmethod
    def validate_settings(cls, settings: Settings) -> Dict[str, str]:
        normalized_token = cls._extract_token(settings)
        missing_fields: List[str] = []

        if not normalized_token:
            missing_fields.append("cloudflare_api_token")

        normalized_account = cls._normalize_str(settings.cloudflare_account_id)
        normalized_zone = cls._normalize_str(settings.cloudflare_zone_id)
        normalized_worker = cls._normalize_str(settings.cloudflare_worker_name)
        normalized_domain = cls._normalize_str(settings.temp_mail_domain_base)

        mapping = [
            ("cloudflare_account_id", normalized_account),
            ("cloudflare_zone_id", normalized_zone),
            ("cloudflare_worker_name", normalized_worker),
            ("temp_mail_domain_base", normalized_domain),
        ]

        for key, value in mapping:
            if not value:
                missing_fields.append(key)

        if missing_fields:
            raise CloudflareProvisioningError(
                f"Missing required Cloudflare settings: {', '.join(missing_fields)}"
            )

        return {
            "token": normalized_token,
            "account_id": normalized_account,
            "zone_id": normalized_zone,
            "worker_name": normalized_worker,
            "domain_base": normalized_domain,
        }

    def _compose_domain(self, label: str) -> str:
        label_part = str(label or "").strip()
        if not label_part:
            raise CloudflareProvisioningError("Label is required to compose domain")
        prefix = self._prefix
        base = self._domain_base
        if not base:
            raise CloudflareProvisioningError("temp_mail_domain_base is required to build domains")
        domain = f"{prefix}-{label_part}" if prefix else label_part
        return f"{domain}.{base}"

    def _generate_label(self) -> str:
        mode = self._normalize_str(getattr(self.settings, "temp_mail_subdomain_mode", "random")).lower()
        length = int(getattr(self.settings, "temp_mail_subdomain_length", 6) or 6)
        length = max(3, min(16, length))

        if mode == "sequence":
            sequence = self._to_base36(int(time.time() * 1000))
            return sequence.rjust(length, "0")[-length:]

        alphabet = string.ascii_lowercase + string.digits
        return "".join(secrets.choice(alphabet) for _ in range(length))

    @staticmethod
    def _to_base36(value: int) -> str:
        digits = "0123456789abcdefghijklmnopqrstuvwxyz"
        number = int(value)
        if number <= 0:
            return "0"

        result: List[str] = []
        while number:
            number, rem = divmod(number, 36)
            result.append(digits[rem])
        return "".join(reversed(result))

    def _extract_worker_bindings(self, payload: Dict[str, Any]) -> List[Dict[str, Any]]:
        if not isinstance(payload, dict):
            return []

        result = payload.get("result")
        candidates: List[Any] = [payload]
        if isinstance(result, dict):
            candidates.insert(0, result)
            settings_payload = result.get("settings")
            if isinstance(settings_payload, dict):
                candidates.insert(0, settings_payload)

        for candidate in candidates:
            bindings = candidate.get("bindings")
            if isinstance(bindings, list):
                return bindings
        return []

    def _upsert_domains_binding(
        self, bindings: List[Dict[str, Any]], domain: str
    ) -> List[Dict[str, Any]]:
        updated: List[Dict[str, Any]] = []
        replaced = False
        for binding in bindings:
            if binding.get("name") != "DOMAINS":
                updated.append(binding)
                continue

            domains = self._extract_binding_domains(binding)
            domains.append(domain)
            normalized = self._dedupe_domains(domains)
            updated.append({
                "type": "plain_text",
                "name": "DOMAINS",
                "text": json.dumps(normalized),
            })
            replaced = True

        if not replaced:
            updated.append({
                "type": "plain_text",
                "name": "DOMAINS",
                "text": json.dumps(self._dedupe_domains([domain])),
            })
        return updated

    @staticmethod
    def _extract_binding_domains(binding: Dict[str, Any]) -> List[str]:
        domains: List[str] = []
        domains.extend(parse_domains_binding(binding.get("text")))
        raw_json = binding.get("json")
        if raw_json is not None:
            if isinstance(raw_json, str):
                domains.extend(parse_domains_binding(raw_json))
            elif isinstance(raw_json, list):
                domains.extend(
                    str(item).strip() for item in raw_json if str(item).strip()
                )
            elif isinstance(raw_json, dict):
                for value in raw_json.values():
                    if isinstance(value, (list, tuple)):
                        domains.extend(
                            str(item).strip() for item in value if str(item).strip()
                        )
                    else:
                        text_value = str(value).strip()
                        if text_value:
                            domains.append(text_value)
        return [d for d in domains if d]

    @staticmethod
    def _dedupe_domains(domains: List[str]) -> List[str]:
        seen = set()
        result: List[str] = []
        for item in domains:
            normalized = str(item or "").strip()
            if not normalized or normalized in seen:
                continue
            seen.add(normalized)
            result.append(normalized)
        return result

    def _worker_settings_url(self) -> str:
        account_id = self.settings.cloudflare_account_id
        worker_name = self.settings.cloudflare_worker_name
        return (
            f"https://api.cloudflare.com/client/v4/accounts/{self._account_id}"
            f"/workers/scripts/{self._worker_name}/settings"
        )

    def _zones_subdomain_url(self) -> str:
        zone_id = self.settings.cloudflare_zone_id
        return (
            f"https://api.cloudflare.com/client/v4/zones/{self._zone_id}"
            "/email/sending/subdomains"
        )

    def _zones_subdomain_item_url(self, subdomain_identifier: str) -> str:
        identifier = quote(str(subdomain_identifier or "").strip(), safe="")
        return f"{self._zones_subdomain_url()}/{identifier}"

    def _auth_headers(self) -> Dict[str, str]:
        return {
            "Authorization": f"Bearer {self._api_token}",
            "Content-Type": "application/json",
        }

    def _process_response(self, response: Any, action: str) -> Dict[str, Any]:
        payload = None
        try:
            payload = response.json()
        except Exception as exc:
            if response.status_code >= 400:
                raise CloudflareProvisioningError(
                    f"{action} failed with status {response.status_code} and invalid JSON"
                ) from exc
            raise CloudflareProvisioningError(
                f"Failed to parse Cloudflare {action} response: {exc}"
            ) from exc

        if response.status_code >= 400:
            raise CloudflareProvisioningError(
                f"{action} failed with status {response.status_code}: {payload}"
            )

        if not isinstance(payload, dict):
            raise CloudflareProvisioningError(
                f"{action} returned unexpected payload: {payload}"
            )

        return payload

    def create_subdomain(self, domain: str) -> Dict[str, Any]:
        response = self.http_client.request(
            "POST",
            self._zones_subdomain_url(),
            json={"name": domain},
            headers=self._auth_headers(),
        )
        return self._process_response(response, "create subdomain")

    def get_worker_settings(self) -> Dict[str, Any]:
        response = self.http_client.request(
            "GET",
            self._worker_settings_url(),
            headers=self._auth_headers(),
        )
        return self._process_response(response, "get worker settings")

    def patch_worker_settings(self, bindings: List[Dict[str, Any]]) -> Dict[str, Any]:
        response = self.http_client.request(
            "PATCH",
            self._worker_settings_url(),
            json={"settings": {"bindings": bindings}},
            headers=self._auth_headers(),
        )
        return self._process_response(response, "patch worker settings")

    def delete_subdomain(self, subdomain_identifier: str) -> Dict[str, Any]:
        response = self.http_client.request(
            "DELETE",
            self._zones_subdomain_item_url(subdomain_identifier),
            headers=self._auth_headers(),
        )
        return self._process_response(response, "delete subdomain")

    def _extract_subdomain_identifier(
        self, subdomain_payload: Optional[Dict[str, Any]], domain: Optional[str]
    ) -> str:
        payload = subdomain_payload if isinstance(subdomain_payload, dict) else {}
        candidates: List[Dict[str, Any]] = [payload]
        result = payload.get("result")
        if isinstance(result, dict):
            candidates.insert(0, result)
        elif isinstance(result, list):
            first = result[0] if result else None
            if isinstance(first, dict):
                candidates.insert(0, first)

        for candidate in candidates:
            for key in ("id", "subdomain_id", "name", "domain"):
                value = self._normalize_str(candidate.get(key))
                if value:
                    return value

        return self._normalize_str(domain)

    def cleanup_provisioned_domain(
        self, provisioned: Optional[Dict[str, Any]], domain: Optional[str] = None
    ) -> Optional[Dict[str, Any]]:
        cleanup_result: Dict[str, Any] = {}
        cleanup_errors: List[str] = []

        previous_bindings = None
        if isinstance(provisioned, dict):
            bindings = provisioned.get("cloudflare_worker_previous_bindings")
            if isinstance(bindings, list):
                previous_bindings = bindings

        if previous_bindings is not None:
            try:
                cleanup_result["cloudflare_worker_restore"] = self.patch_worker_settings(previous_bindings)
            except Exception as exc:
                cleanup_errors.append(f"restore worker settings failed: {exc}")

        subdomain_payload = None
        if isinstance(provisioned, dict):
            payload = provisioned.get("cloudflare_subdomain")
            if isinstance(payload, dict):
                subdomain_payload = payload
            elif "result" in provisioned:
                subdomain_payload = provisioned

        identifier = self._extract_subdomain_identifier(subdomain_payload, domain)
        if identifier:
            try:
                cleanup_result["cloudflare_subdomain_delete"] = self.delete_subdomain(identifier)
            except Exception as exc:
                cleanup_errors.append(f"delete subdomain failed: {exc}")

        if cleanup_errors:
            raise CloudflareProvisioningError("; ".join(cleanup_errors))
        if cleanup_result:
            return cleanup_result
        return None

    def provision_domain(self) -> Dict[str, Any]:
        label = self._generate_label()
        domain = self._compose_domain(label)
        subdomain_payload = self.create_subdomain(domain)
        provisioned = {
            "domain": domain,
            "cloudflare_subdomain": subdomain_payload,
        }

        try:
            worker_settings_payload = self.get_worker_settings()
            existing_bindings = self._extract_worker_bindings(worker_settings_payload)
            provisioned["cloudflare_worker_previous_bindings"] = existing_bindings
            updated_bindings = self._upsert_domains_binding(existing_bindings, domain)
            patched_payload = self.patch_worker_settings(updated_bindings)
        except Exception:
            try:
                self.cleanup_provisioned_domain(provisioned, domain=domain)
            except Exception:
                pass
            raise

        provisioned["cloudflare_worker_settings"] = patched_payload
        return provisioned
