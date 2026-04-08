from dataclasses import dataclass
from enum import Enum
from typing import Optional


class HotmailFailureCode(str, Enum):
    PHONE_VERIFICATION_REQUIRED = "phone_verification_required"
    UNSUPPORTED_CHALLENGE = "unsupported_challenge"
    EMAIL_VERIFICATION_TIMEOUT = "email_verification_timeout"
    USERNAME_UNAVAILABLE_EXHAUSTED = "username_unavailable_exhausted"
    PROXY_ERROR = "proxy_error"
    PAGE_STRUCTURE_CHANGED = "page_structure_changed"
    BROWSER_TIMEOUT = "browser_timeout"
    UNEXPECTED_EXCEPTION = "unexpected_exception"


@dataclass
class HotmailAccountArtifact:
    email: str
    password: str
    target_domain: str
    verification_email: str = ""
    first_name: str = ""
    last_name: str = ""

    def to_txt_line(self) -> str:
        return f"{self.email}----{self.password}"


@dataclass
class HotmailRegistrationResult:
    success: bool
    reason_code: str = ""
    error_message: str = ""
    artifact: Optional[HotmailAccountArtifact] = None
