"""
注册流程重试策略。
"""

from __future__ import annotations


RETRIABLE_MARKERS = (
    "authorize",
    "otp",
    "验证码",
    "workspace",
    "session",
    "access token",
)

NON_RETRIABLE_MARKERS = (
    "no available email service",
    "missing recoverable email",
)


def should_retry_register_error(message: str) -> bool:
    text = str(message or "").strip().lower()
    if not text:
        return False
    if any(marker in text for marker in NON_RETRIABLE_MARKERS):
        return False
    return any(marker in text for marker in RETRIABLE_MARKERS)


def should_retry_signup_otp_validation(
    *,
    is_wrong_email_otp_code_error: bool,
    attempt: int,
    max_attempts: int,
) -> bool:
    return is_wrong_email_otp_code_error and attempt < max_attempts - 1
