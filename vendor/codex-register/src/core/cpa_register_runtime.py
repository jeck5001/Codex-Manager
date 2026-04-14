"""
CPA-oriented register runtime helpers.

This module starts the migration by normalizing callback handling around the
`openai-cpa` parsing semantics, while leaving the rest of the current register
flow intact for now.
"""

from __future__ import annotations

from dataclasses import dataclass
import urllib.parse
from typing import Any, Dict, Optional


@dataclass
class CPAPostRegistrationResult:
    callback_url: Optional[str] = None
    workspace_id: str = ""
    error_message: str = ""
    metadata: Dict[str, Any] | None = None


def resolve_callback_payload(callback_url: str) -> Dict[str, str]:
    candidate = str(callback_url or "").strip()
    if not candidate:
        return {"code": "", "state": "", "error": "", "error_description": ""}

    if "://" not in candidate:
        if candidate.startswith("?"):
            candidate = f"http://localhost{candidate}"
        elif any(ch in candidate for ch in "/?#") or ":" in candidate:
            candidate = f"http://{candidate}"
        elif "=" in candidate:
            candidate = f"http://localhost/?{candidate}"

    parsed = urllib.parse.urlparse(candidate)
    query = urllib.parse.parse_qs(parsed.query, keep_blank_values=True)
    fragment = urllib.parse.parse_qs(parsed.fragment, keep_blank_values=True)

    for key, values in fragment.items():
        if key not in query or not query[key] or not (query[key][0] or "").strip():
            query[key] = values

    def get1(key: str) -> str:
        values = query.get(key, [""])
        return str(values[0] or "").strip()

    code = get1("code")
    state = get1("state")
    error = get1("error")
    error_description = get1("error_description")

    if code and not state and "#" in code:
        code, state = code.split("#", 1)

    if not error and error_description:
        error, error_description = error_description, ""

    return {
        "code": code,
        "state": state,
        "error": error,
        "error_description": error_description,
    }


class CPARegisterRuntime:
    def __init__(self, engine: Any):
        self.engine = engine

    def normalize_callback_url(self, callback_url: str) -> str:
        raw = str(callback_url or "").strip()
        payload = resolve_callback_payload(raw)
        if not payload["code"] and not payload["error"]:
            return raw

        oauth_start = getattr(self.engine, "oauth_start", None)
        redirect_uri = str(
            getattr(oauth_start, "redirect_uri", "")
            or getattr(getattr(self.engine, "oauth_manager", None), "redirect_uri", "")
            or "http://localhost:1455/auth/callback"
        ).strip()

        params = {}
        for key in ("code", "state", "error", "error_description"):
            value = payload.get(key)
            if value:
                params[key] = value

        encoded = urllib.parse.urlencode(params)
        if not encoded:
            return raw
        return f"{redirect_uri}?{encoded}"

    def handle_oauth_callback(self, callback_url: str) -> Dict[str, Any]:
        normalized = self.normalize_callback_url(callback_url)
        return self.engine.oauth_manager.handle_callback(
            callback_url=normalized,
            expected_state=self.engine.oauth_start.state,
            code_verifier=self.engine.oauth_start.code_verifier,
        )

    def resolve_post_registration_callback(
        self,
        did: Optional[str],
        sen_token: Optional[str],
    ) -> CPAPostRegistrationResult:
        result = CPAPostRegistrationResult(metadata={})
        flow_runner = self.engine._get_flow_runner()

        workspace_id = str(self.engine._get_workspace_id() or "").strip()
        if workspace_id:
            result.workspace_id = workspace_id

        continue_url = str(getattr(self.engine, "_post_create_continue_url", "") or "").strip()
        if continue_url:
            self.engine._log("优先使用 create_account continue_url 推进 OAuth...", "warning")
            callback_url = flow_runner.resolve_callback_from_continue_url(
                continue_url,
                "注册后继续",
            )
            if callback_url:
                result.callback_url = callback_url
                return result

            self.engine._log("create_account continue_url 未收敛到回调，回退到通用 OAuth 收敛流程", "warning")

        fallback_result = flow_runner.resolve_post_registration_callback(did, sen_token)
        result.callback_url = getattr(fallback_result, "callback_url", None)
        result.workspace_id = str(getattr(fallback_result, "workspace_id", "") or "").strip()
        result.error_message = str(getattr(fallback_result, "error_message", "") or "").strip()

        fallback_metadata = getattr(fallback_result, "metadata", None)
        if isinstance(fallback_metadata, dict):
            result.metadata.update(fallback_metadata)

        can_retry_via_login = (
            not result.callback_url
            and bool(result.error_message)
            and bool(str(getattr(self.engine, "email", "") or "").strip())
            and bool(str(getattr(self.engine, "password", "") or "").strip())
        )
        if can_retry_via_login and hasattr(self.engine, "_attempt_add_phone_login_bypass"):
            self.engine._log("注册后 OAuth 收敛失败，尝试走 CPA 登录恢复链继续获取回调...", "warning")
            callback_url = self.engine._attempt_add_phone_login_bypass(did, sen_token)
            if callback_url:
                result.callback_url = callback_url
                result.error_message = ""

        return result
