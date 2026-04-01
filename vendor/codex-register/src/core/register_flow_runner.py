"""
注册流程推进器。
"""

from __future__ import annotations

import urllib.parse
from dataclasses import dataclass
from typing import Any, Optional

from .register_flow_state import extract_auth_continue_url, extract_auth_page_type


@dataclass
class FlowResolutionResult:
    callback_url: Optional[str] = None
    page_type: str = ""
    continue_url: str = ""


class RegisterFlowRunner:
    def __init__(self, engine: Any):
        self.engine = engine

    def resolve_callback_from_auth_page(self, page: Any, stage: str) -> Optional[str]:
        page_type = getattr(self.engine, "_clean_text", lambda value: str(value or "").strip())(
            (page or {}).get("type")
        )
        if not page_type:
            self.engine._log(f"{stage} 缺少 page.type", "warning")
            return None

        if page_type == "token_exchange":
            return self.engine._build_callback_url_from_page(page)

        if page_type in (
            "workspace",
            "sign_in_with_chatgpt_codex_consent",
            "sign_in_with_chatgpt_codex_org",
        ):
            workspace_id = self.engine._get_workspace_id()
            if not workspace_id:
                self.engine._log(f"{stage} 需要选择 Workspace，但暂未拿到 workspace_id", "warning")
                return None

            continue_url = self.engine._select_workspace(workspace_id)
            if not continue_url:
                return None
            return self.engine._follow_redirects(continue_url)

        if page_type == "external_url":
            payload = page.get("payload") if isinstance(page.get("payload"), dict) else {}
            external_url = getattr(self.engine, "_clean_text", lambda value: str(value or "").strip())(
                payload.get("url")
            )
            if external_url:
                self.engine._log(f"{stage} 返回 external_url，继续跟随跳转...", "warning")
                return self.engine._follow_redirects(external_url)
            return None

        if page_type == "add_phone":
            return None

        return None

    def resolve_callback_from_continue_url(self, continue_url: str, stage: str) -> Optional[str]:
        target_url = getattr(self.engine, "_clean_text", lambda value: str(value or "").strip())(
            continue_url
        )
        if not target_url:
            return None

        direct_callback = (
            self.engine._extract_callback_url(target_url)
            if hasattr(self.engine, "_extract_callback_url")
            else (target_url if "code=" in target_url and "state=" in target_url else "")
        )
        if direct_callback:
            self.engine._log(f"{stage} 直接拿到回调 URL: {target_url[:100]}...")
            return direct_callback

        parsed = urllib.parse.urlsplit(target_url)
        path = getattr(self.engine, "_clean_text", lambda value: str(value or "").strip())(parsed.path)

        if path in (
            "/workspace",
            "/sign-in-with-chatgpt/codex/consent",
            "/sign-in-with-chatgpt/codex/organization",
        ):
            workspace_id = self.engine._get_workspace_id()
            if not workspace_id:
                self.engine._log(f"{stage} 需要 workspace，但当前还没拿到 workspace_id", "warning")
                return None

            continue_after_select = self.engine._select_workspace(workspace_id)
            if not continue_after_select:
                return None
            return self.engine._follow_redirects(continue_after_select)

        if path == "/add-phone":
            callback_url = self.engine._advance_workspace_authorization(target_url)
            if callback_url:
                return callback_url
            self.engine._log(f"{stage} 仍然指向 add_phone", "warning")
            return None

        return self.engine._follow_redirects(target_url)

    def resolve_auth_result(self, payload: Any, stage: str = "") -> FlowResolutionResult:
        result = FlowResolutionResult()
        if not isinstance(payload, dict):
            return result

        result.page_type = extract_auth_page_type(payload)
        result.continue_url = extract_auth_continue_url(payload)

        page = payload.get("page")
        if isinstance(page, dict):
            callback_url = self.resolve_callback_from_auth_page(
                page,
                stage or "认证响应",
            )
            if callback_url:
                result.callback_url = callback_url
                return result
            if result.page_type == "add_phone":
                return result

        if result.continue_url:
            result.callback_url = self.resolve_callback_from_continue_url(
                result.continue_url,
                stage or "认证响应",
            )

        return result
