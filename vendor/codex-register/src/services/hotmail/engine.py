from typing import Optional

from .profile import choose_target_domains
from .types import HotmailFailureCode, HotmailRegistrationResult


def classify_hotmail_page_state(text: str) -> Optional[HotmailFailureCode]:
    normalized = str(text or "").lower()
    if "phone number" in normalized:
        return HotmailFailureCode.PHONE_VERIFICATION_REQUIRED
    if "puzzle" in normalized or "captcha" in normalized:
        return HotmailFailureCode.UNSUPPORTED_CHALLENGE
    return None


class HotmailRegistrationEngine:
    def __init__(self, browser_factory=None, verification_provider=None, callback_logger=None, proxy_url=None):
        self.browser_factory = browser_factory
        self.verification_provider = verification_provider
        self.callback_logger = callback_logger
        self.proxy_url = proxy_url
        self._attempt_domain = lambda domain: False

    def _choose_domain_by_attempt(self) -> Optional[str]:
        for domain in choose_target_domains():
            if self._attempt_domain(domain):
                return domain
        return None

    def run(self):
        selected_domain = self._choose_domain_by_attempt()
        if not selected_domain:
            return HotmailRegistrationResult(
                success=False,
                reason_code=HotmailFailureCode.USERNAME_UNAVAILABLE_EXHAUSTED.value,
                error_message="No domain attempt succeeded",
            )
        return HotmailRegistrationResult(
            success=False,
            reason_code=HotmailFailureCode.UNEXPECTED_EXCEPTION.value,
            error_message=f"Hotmail flow not implemented for {selected_domain}",
        )
