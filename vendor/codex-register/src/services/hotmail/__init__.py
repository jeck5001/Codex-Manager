from .engine import HotmailRegistrationEngine, classify_hotmail_page_state
from .profile import HOTMAIL_DOMAIN_POLICY, build_username_candidates, choose_target_domains
from .types import (
    HotmailAccountArtifact,
    HotmailChallengeHandoff,
    HotmailFailureCode,
    HotmailRegistrationProfile,
    HotmailRegistrationResult,
)
from .verification import (
    HotmailVerificationMailbox,
    HotmailVerificationMailboxProvider,
    build_default_hotmail_verification_provider,
)

__all__ = [
    "HotmailRegistrationEngine",
    "HotmailVerificationMailbox",
    "HotmailVerificationMailboxProvider",
    "build_default_hotmail_verification_provider",
    "HOTMAIL_DOMAIN_POLICY",
    "HotmailAccountArtifact",
    "HotmailChallengeHandoff",
    "HotmailFailureCode",
    "HotmailRegistrationProfile",
    "HotmailRegistrationResult",
    "build_username_candidates",
    "classify_hotmail_page_state",
    "choose_target_domains",
]
