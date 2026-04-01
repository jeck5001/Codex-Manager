"""
注册流程中 callback / token / workspace 解析辅助。
"""

from __future__ import annotations

import base64
import json
import urllib.parse
from typing import Any, Optional

from .register_flow_state import clean_text, extract_callback_url, workspace_id_from_mapping


def resolve_callback_url(candidate: Any) -> str:
    return extract_callback_url(candidate)


def resolve_workspace_id_from_tokens(payload: Any) -> str:
    if not isinstance(payload, dict):
        return ""
    for key in (
        "workspace_id",
        "workspaceId",
        "organization_id",
        "org_id",
        "chatgpt_account_id",
    ):
        value = clean_text(payload.get(key))
        if value:
            return value
    return ""


def extract_workspace_id_from_token(token: str) -> Optional[str]:
    """从 JWT token 中提取 workspace / organization ID。"""
    try:
        raw = clean_text(token)
        if not raw:
            return None
        parts = raw.split(".")
        if len(parts) < 2:
            return None

        payload = parts[1]
        pad = "=" * ((4 - (len(payload) % 4)) % 4)
        decoded = base64.urlsafe_b64decode((payload + pad).encode("ascii"))
        data = json.loads(decoded.decode("utf-8"))
        workspace_id = workspace_id_from_mapping(data)
        return workspace_id or None
    except Exception:
        return None


def build_callback_url_from_page(page: Any) -> Optional[str]:
    """从 token_exchange 页面构造 OAuth 回调 URL。"""
    try:
        if not isinstance(page, dict) or clean_text(page.get("type")) != "token_exchange":
            return None

        continue_url = clean_text(page.get("continue_url"))
        payload = page.get("payload") if isinstance(page.get("payload"), dict) else {}
        if not continue_url:
            return None

        direct_callback = resolve_callback_url(continue_url)
        if direct_callback:
            return direct_callback

        parsed = urllib.parse.urlsplit(continue_url)
        query = dict(urllib.parse.parse_qsl(parsed.query, keep_blank_values=True))
        for key in ("code", "state", "error", "error_description"):
            value = clean_text(payload.get(key))
            if value:
                query[key] = value

        callback_url = urllib.parse.urlunsplit(
            (
                parsed.scheme,
                parsed.netloc,
                parsed.path,
                urllib.parse.urlencode(query),
                parsed.fragment,
            )
        )
        return resolve_callback_url(callback_url) or None
    except Exception:
        return None
