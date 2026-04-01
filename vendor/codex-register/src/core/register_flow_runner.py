"""
注册流程推进器。
"""

from __future__ import annotations

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

    def resolve_auth_result(self, payload: Any, stage: str = "") -> FlowResolutionResult:
        result = FlowResolutionResult()
        if not isinstance(payload, dict):
            return result

        result.page_type = extract_auth_page_type(payload)
        result.continue_url = extract_auth_continue_url(payload)

        page = payload.get("page")
        if isinstance(page, dict):
            callback_url = self.engine._resolve_callback_from_auth_page(
                page,
                stage or "认证响应",
            )
            if callback_url:
                result.callback_url = callback_url
                return result
            if result.page_type == "add_phone":
                return result

        if result.continue_url:
            result.callback_url = self.engine._resolve_callback_from_continue_url(
                result.continue_url,
                stage or "认证响应",
            )

        return result
