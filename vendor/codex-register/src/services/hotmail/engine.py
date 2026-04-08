from typing import Optional

from .profile import choose_target_domains
from .types import HotmailFailureCode


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
        raise NotImplementedError
