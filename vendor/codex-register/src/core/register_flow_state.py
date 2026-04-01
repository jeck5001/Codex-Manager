"""
注册流程状态提取辅助。
"""

from __future__ import annotations

import re
import urllib.parse
from typing import Any, Optional


def clean_text(value: Any) -> str:
    if value is None:
        return ""
    return str(value).strip()


def extract_auth_page_type(payload: Any) -> str:
    """从认证响应或 page 对象中提取页面类型。"""
    if not isinstance(payload, dict):
        return ""

    page = payload.get("page")
    if isinstance(page, dict):
        page_type = clean_text(page.get("type"))
        if page_type:
            return page_type

    return clean_text(payload.get("type"))


def extract_auth_continue_url(payload: Any) -> str:
    """从认证响应中提取 continue_url。"""
    if not isinstance(payload, dict):
        return ""

    return clean_text(
        payload.get("continue_url")
        or payload.get("redirect_url")
        or payload.get("callback_url")
        or payload.get("next_url")
    )


def extract_callback_url(candidate: Any) -> str:
    """从候选 URL 中识别 OAuth callback URL。"""
    text = clean_text(candidate)
    if not text:
        return ""
    return text if "code=" in text and "state=" in text else ""


def workspace_id_from_mapping(data: Any) -> str:
    """从 auth/session 映射里提取 workspace / organization ID。"""
    if not isinstance(data, dict):
        return ""

    direct_keys = (
        "workspace_id",
        "organization_id",
        "org_id",
        "default_workspace_id",
        "default_organization_id",
        "chatgpt_account_id",
    )
    item_keys = ("id", "workspace_id", "organization_id", "org_id")
    nested_keys = (
        "default_workspace",
        "default_organization",
        "workspace",
        "organization",
    )

    for key in direct_keys:
        candidate = clean_text(data.get(key))
        if candidate:
            return candidate

    for nested_key in nested_keys:
        nested = data.get(nested_key)
        if isinstance(nested, dict):
            for key in item_keys:
                candidate = clean_text(nested.get(key))
                if candidate:
                    return candidate

    for list_key in ("workspaces", "organizations"):
        items = data.get(list_key)
        if not isinstance(items, list):
            continue

        default_item = next(
            (
                item
                for item in items
                if isinstance(item, dict) and item.get("is_default") is True
            ),
            None,
        )
        ordered_items = ([default_item] if default_item is not None else []) + [
            item for item in items if isinstance(item, dict) and item is not default_item
        ]

        for item in ordered_items:
            for key in item_keys:
                candidate = clean_text(item.get(key))
                if candidate:
                    return candidate

    for nested_key in ("auth", "https://api.openai.com/auth"):
        nested = data.get(nested_key)
        candidate = workspace_id_from_mapping(nested)
        if candidate:
            return candidate

    return ""


def extract_workspace_id_from_text(text: str) -> Optional[str]:
    """从 HTML/脚本文本中提取 Workspace ID。"""
    if not text:
        return None

    patterns = [
        r'"workspace_id"\s*:\s*"([^"]+)"',
        r'"workspaceId"\s*:\s*"([^"]+)"',
        r'"default_workspace_id"\s*:\s*"([^"]+)"',
        r'"defaultWorkspaceId"\s*:\s*"([^"]+)"',
        r'"active_workspace_id"\s*:\s*"([^"]+)"',
        r'"activeWorkspaceId"\s*:\s*"([^"]+)"',
        r'"workspace"\s*:\s*\{[^{}]*"id"\s*:\s*"([^"]+)"',
        r'"default_workspace"\s*:\s*\{[^{}]*"id"\s*:\s*"([^"]+)"',
        r'"active_workspace"\s*:\s*\{[^{}]*"id"\s*:\s*"([^"]+)"',
    ]
    for pattern in patterns:
        match = re.search(pattern, text)
        if match:
            workspace_id = clean_text(match.group(1))
            if workspace_id:
                return workspace_id
    return None


def extract_workspace_id_from_url(url: str) -> Optional[str]:
    """从 URL 查询参数或片段中提取 Workspace ID。"""
    if not url:
        return None

    parsed = urllib.parse.urlparse(url)
    for raw_query in (parsed.query, parsed.fragment):
        query = urllib.parse.parse_qs(raw_query)
        for key in (
            "workspace_id",
            "workspaceId",
            "default_workspace_id",
            "active_workspace_id",
        ):
            values = query.get(key) or []
            if values:
                workspace_id = clean_text(values[0])
                if workspace_id:
                    return workspace_id
    return None


def extract_workspace_id_from_response_payload(
    payload: Any,
    depth: int = 0,
) -> Optional[str]:
    """递归扫描响应载荷中的 Workspace ID。"""
    if payload is None or depth > 5:
        return None

    if isinstance(payload, dict):
        workspace_id = workspace_id_from_mapping(payload)
        if workspace_id:
            return workspace_id
        for value in payload.values():
            workspace_id = extract_workspace_id_from_response_payload(value, depth + 1)
            if workspace_id:
                return workspace_id
        return None

    if isinstance(payload, list):
        for item in payload:
            workspace_id = extract_workspace_id_from_response_payload(item, depth + 1)
            if workspace_id:
                return workspace_id

    return None


def extract_workspace_id_from_response(
    response: Optional[Any] = None,
    html: Optional[str] = None,
    url: Optional[str] = None,
) -> Optional[str]:
    """统一从响应 JSON、HTML、脚本内容和 URL 中提取 Workspace ID。"""
    response_url = clean_text(getattr(response, "url", "") if response is not None else "")
    response_text = html if html is not None else str(getattr(response, "text", "") or "")
    candidate_url = url or response_url

    if response is not None:
        try:
            payload = response.json()
        except Exception:
            payload = None
        workspace_id = extract_workspace_id_from_response_payload(payload)
        if workspace_id:
            return workspace_id

    for extractor in (
        lambda: extract_workspace_id_from_text(response_text),
        lambda: extract_workspace_id_from_url(candidate_url),
    ):
        workspace_id = extractor()
        if workspace_id:
            return workspace_id

    return None


def extract_workspace_id(payload: Any) -> str:
    """给测试和轻量调用使用的统一入口。"""
    return (
        workspace_id_from_mapping(payload)
        or extract_workspace_id_from_response_payload(payload)
        or ""
    )
