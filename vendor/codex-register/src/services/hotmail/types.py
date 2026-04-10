from dataclasses import dataclass
from enum import Enum
from typing import Any, Optional


class HotmailFailureCode(str, Enum):
    PHONE_VERIFICATION_REQUIRED = "phone_verification_required"
    UNSUPPORTED_CHALLENGE = "unsupported_challenge"
    ACCOUNT_CREATION_BLOCKED = "account_creation_blocked"
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
class HotmailRegistrationProfile:
    first_name: str
    last_name: str
    birth_day: str
    birth_month: str
    birth_year: str
    password: str
    username_candidates: list[str]
    country: str = "United States"


@dataclass
class HotmailChallengeHandoff:
    handoff_id: str
    session: Any
    profile: HotmailRegistrationProfile
    email: str
    domain: str
    state: str = ""


@dataclass
class HotmailLocalHandoffOriginEntry:
    name: str
    value: str


@dataclass
class HotmailLocalHandoffOrigin:
    origin: str
    local_storage: list[HotmailLocalHandoffOriginEntry]


@dataclass
class HotmailLocalHandoffCookie:
    name: str
    value: str
    domain: str
    path: str
    expires: Optional[int] = None
    http_only: bool = False
    secure: bool = False
    same_site: str = ""


@dataclass
class HotmailLocalHandoffPayload:
    handoff_id: str
    url: str
    title: str
    user_agent: str
    proxy_url: str
    state: str
    cookies: list[dict[str, Any]]
    origins: list[dict[str, Any]]


@dataclass
class HotmailLocalTaskSnapshot:
    task_id: str
    batch_id: str
    status: str
    current_step: str
    manual_action_required: bool = False
    failure_code: str = ""
    failure_message: str = ""
    verification_email: str = ""
    target_email: str = ""
    artifact_path: str = ""


@dataclass
class HotmailLocalTaskPayload:
    batch_id: str
    task_id: str
    profile: dict[str, Any]
    target_domains: list[str]
    proxy: str = ""
    verification_mailbox: Optional[dict[str, Any]] = None
    backend_callback_base: str = ""
    backend_callback_token: str = ""


@dataclass
class HotmailRegistrationResult:
    success: bool
    reason_code: str = ""
    error_message: str = ""
    artifact: Optional[HotmailAccountArtifact] = None
    handoff_context: Optional[HotmailChallengeHandoff] = None
