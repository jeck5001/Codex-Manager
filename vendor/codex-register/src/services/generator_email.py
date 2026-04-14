"""
Generator.email 临时邮箱服务
"""

import logging
import re
from typing import Any, Dict, List, Optional

from curl_cffi import requests as curl_requests

from .base import BaseEmailService, EmailServiceError, EmailServiceType
from ..config.constants import EMAIL_SERVICE_DEFAULTS

logger = logging.getLogger(__name__)
requests = curl_requests


class GeneratorEmailService(BaseEmailService):
    """
    基于 Generator.email 的邮箱服务实现
    """

    _EMAIL_SPAN_PATTERN = re.compile(
        r'<span[^>]+id=["\']email_ch_text["\'][^>]*>([^<]+)</span>', re.IGNORECASE
    )
    _USERNAME_INPUT_PATTERN = re.compile(
        r'id=["\']userName["\'][^>]*value=["\']([^"\']+)["\']', re.IGNORECASE
    )
    _DOMAIN_INPUT_PATTERN = re.compile(
        r'id=["\']domainName2["\'][^>]*value=["\']([^"\']+)["\']', re.IGNORECASE
    )

    def __init__(self, config: Dict[str, Any] = None, name: str = None):
        defaults = EMAIL_SERVICE_DEFAULTS.get("generator_email", {})
        merged = {**defaults, **(config or {})}
        super().__init__(EmailServiceType.GENERATOR_EMAIL, name)
        self.base_url = str(merged.get("base_url") or "https://generator.email").strip().rstrip("/")
        self.timeout = int(merged.get("timeout") or 30)
        self.poll_interval = int(merged.get("poll_interval") or 3)

    def create_email(self, config: Dict[str, Any] = None) -> Dict[str, Any]:
        try:
            resp = requests.get(
                self.base_url,
                headers=self._headers(),
                timeout=self.timeout,
                impersonate="chrome110"
            )
        except Exception as exc:
            self.update_status(False, exc)
            raise EmailServiceError(f"Generator.email inbox request failed: {exc}") from exc

        if resp.status_code != 200:
            error = EmailServiceError(f"Generator.email inbox request failed: HTTP {resp.status_code}")
            self.update_status(False, error)
            raise error

        email = self._parse_email(resp.text or "")
        surl = self._build_surl(email)
        if not email or not surl:
            error = EmailServiceError("无法解析 Generator.email 邮箱地址")
            self.update_status(False, error)
            raise error

        self.update_status(True)
        return {
            "email": email,
            "email_id": surl,
            "service_id": surl,
            "credentials": {"surl": surl},
        }

    def get_verification_code(
        self,
        email: str,
        email_id: str = None,
        timeout: int = 120,
        poll_interval: Optional[int] = None,
        pattern: str = r"(?<!\d)(\d{6})(?!\d)",
        otp_sent_at: Optional[float] = None,
    ) -> Optional[str]:
        mailbox_id = str(email_id or "").strip()
        if not mailbox_id:
            raise EmailServiceError("Generator.email 需要 email_id/surl")

        try:
            resp = requests.get(
                f"{self.base_url}/{mailbox_id}",
                headers=self._headers(),
                cookies={"surl": mailbox_id},
                timeout=self.timeout,
                impersonate="chrome110"
            )
        except Exception as exc:
            self.update_status(False, exc)
            raise EmailServiceError(f"Generator.email mailbox request failed: {exc}") from exc

        if resp.status_code != 200:
            error = EmailServiceError(f"Generator.email mailbox request failed: HTTP {resp.status_code}")
            self.update_status(False, error)
            raise error

        self.update_status(True)
        return self._extract_code(resp.text or "", pattern)

    def list_emails(self, **kwargs) -> List[Dict[str, Any]]:
        return []

    def delete_email(self, email_id: str) -> bool:
        return False

    def check_health(self) -> bool:
        try:
            resp = requests.head(self.base_url, headers=self._headers(), timeout=5, impersonate="chrome110")
            healthy = resp.status_code == 200
            self.update_status(healthy)
            return healthy
        except Exception as exc:
            self.update_status(False, exc)
            return False

    def _headers(self) -> Dict[str, str]:
        return {
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
            "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36",
        }

    @classmethod
    def _parse_email(cls, html: str) -> str:
        match = cls._EMAIL_SPAN_PATTERN.search(html)
        if match:
            return match.group(1).strip()

        username_match = cls._USERNAME_INPUT_PATTERN.search(html)
        domain_match = cls._DOMAIN_INPUT_PATTERN.search(html)
        if username_match and domain_match:
            return f"{username_match.group(1).strip()}@{domain_match.group(1).strip()}"

        return ""

    @staticmethod
    def _build_surl(email: str) -> Optional[str]:
        if not email or "@" not in email:
            return None
        local, domain = email.split("@", 1)
        safe_local = re.sub(r"[^a-zA-Z0-9._-]", "", local).lower()
        return f"{domain.lower()}/{safe_local}" if safe_local else None

    @staticmethod
    def _extract_code(text: str, pattern: str) -> Optional[str]:
        try:
            compiled = re.compile(pattern)
        except re.error:
            compiled = re.compile(r"(?<!\d)(\d{6})(?!\d)")
        content = text or ""

        direct_matches = re.findall(r"Your ChatGPT code is (\d{6})", content, re.IGNORECASE)
        if direct_matches:
            return direct_matches[-1]

        contextual_matches = re.findall(
            r"(?:openai|chatgpt)[\s\S]{0,200}?(\d{6})",
            content,
            re.IGNORECASE,
        )
        if contextual_matches:
            return contextual_matches[-1]

        if "openai" in content.lower() or "chatgpt" in content.lower():
            generic_matches = compiled.findall(content)
            if generic_matches:
                if isinstance(generic_matches[0], tuple):
                    normalized = [match[0] for match in generic_matches if match and match[0]]
                    return normalized[-1] if normalized else None
                return generic_matches[-1]

        generic_match = compiled.search(content)
        if not generic_match:
            return None
        if generic_match.lastindex:
            return generic_match.group(1)
        return generic_match.group(0)
