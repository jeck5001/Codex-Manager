"""
CPA page-state helpers extracted from the openai-cpa browser flow.

This module keeps Task 2 small: it does not execute the full browser workflow yet.
It only classifies the signup/password verification state so the backend can start
consuming CPA-style page semantics before the full register runtime is rewired.
"""

from __future__ import annotations

import re
from typing import Any, Dict


SIGNUP_PASSWORD_ERROR_TITLE_PATTERN = re.compile(
    r"糟糕，出错了|something\s+went\s+wrong|oops",
    re.IGNORECASE,
)
SIGNUP_PASSWORD_ERROR_DETAIL_PATTERN = re.compile(
    r"operation\s+timed\s+out|timed\s+out|请求超时|操作超时",
    re.IGNORECASE,
)
SIGNUP_EMAIL_EXISTS_PATTERN = re.compile(
    r"与此电子邮件地址相关联的帐户已存在|account\s+associated\s+with\s+this\s+email\s+address\s+already\s+exists|email\s+address.*already\s+exists",
    re.IGNORECASE,
)


def _normalize_text(value: Any) -> str:
    return str(value or "").strip()


def classify_signup_state(snapshot: Dict[str, Any]) -> Dict[str, Any]:
    text = _normalize_text(snapshot.get("page_text"))
    is_signup_password_page = bool(snapshot.get("is_signup_password_page"))
    has_retry_button = bool(snapshot.get("has_retry_button"))
    has_password_input = bool(snapshot.get("has_password_input", is_signup_password_page))

    if is_signup_password_page and SIGNUP_EMAIL_EXISTS_PATTERN.search(text):
        return {
            "kind": "email_exists",
            "retryable": False,
            "reason": "signup_email_already_exists",
        }

    if (
        is_signup_password_page
        and has_retry_button
        and (
            SIGNUP_PASSWORD_ERROR_TITLE_PATTERN.search(text)
            or SIGNUP_PASSWORD_ERROR_DETAIL_PATTERN.search(text)
        )
    ):
        return {
            "kind": "password_retry",
            "retryable": True,
            "reason": "signup_password_retryable_error",
        }

    if has_password_input:
        return {
            "kind": "awaiting_password_submit",
            "retryable": False,
            "reason": "signup_password_page",
        }

    return {
        "kind": "unknown",
        "retryable": False,
        "reason": "unclassified",
    }
