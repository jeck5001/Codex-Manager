"""
注册流程推进器。
"""

from __future__ import annotations

import re
import time
import urllib.parse
from dataclasses import dataclass
from typing import Any, Optional

from .register_flow_state import extract_auth_continue_url, extract_auth_page_type


@dataclass
class FlowResolutionResult:
    callback_url: Optional[str] = None
    page_type: str = ""
    continue_url: str = ""


@dataclass
class PostRegistrationRedirectResult:
    callback_url: Optional[str] = None
    workspace_id: str = ""
    error_message: str = ""
    metadata: dict[str, Any] | None = None


class RegisterFlowRunner:
    def __init__(self, engine: Any):
        self.engine = engine

    def _clean_text(self, value: Any) -> str:
        return getattr(self.engine, "_clean_text", lambda raw: str(raw or "").strip())(value)

    @staticmethod
    def _blocked_by_add_phone_error() -> str:
        return "注册流程已进入手机号验证（add_phone），当前无法自动完成，请先完成官方手机号验证后再继续授权"

    def discover_auth_navigation_urls(
        self,
        response: Optional[Any],
        fallback_url: str,
    ) -> list[str]:
        """从响应历史、Location 头和 HTML 中提取可能的授权下一跳 URL。"""
        candidates: list[str] = []
        seen: set[str] = set()

        def add_candidate(raw_url: Any, base_url: str):
            value = self._clean_text(raw_url)
            if not value:
                return

            if (
                "code=" not in value
                and "consent" not in value
                and "workspace" not in value
                and "organization" not in value
                and "/api/accounts/" not in value
            ):
                return

            resolved = urllib.parse.urljoin(base_url or fallback_url, value)
            normalized = self._clean_text(resolved)
            if not normalized or normalized in seen:
                return
            seen.add(normalized)
            candidates.append(normalized)

        responses = []
        history = getattr(response, "history", None)
        if isinstance(history, list):
            responses.extend(history)
        if response is not None:
            responses.append(response)

        for item in responses:
            current_url = self._clean_text(getattr(item, "url", "")) or fallback_url
            add_candidate(current_url, current_url)

            headers = getattr(item, "headers", None)
            if headers and hasattr(headers, "get"):
                add_candidate(headers.get("Location"), current_url)

            body = str(getattr(item, "text", "") or "")
            for pattern in (
                r'action="([^"]+)"',
                r"action='([^']+)'",
                r'href="([^"]+)"',
                r"href='([^']+)'",
            ):
                for match in re.finditer(pattern, body):
                    add_candidate(match.group(1), current_url)

        return candidates

    def advance_workspace_authorization(
        self,
        auth_target: str,
        _visited: Optional[set[str]] = None,
    ) -> Optional[str]:
        """主动请求授权页，若命中 consent/workspace 则完成 workspace 选择并继续拿 callback。"""
        try:
            target_url = self._clean_text(auth_target)
            session = getattr(self.engine, "session", None)
            if not target_url or not session:
                return None

            visited = _visited or set()
            if target_url in visited:
                return None
            visited.add(target_url)

            self.engine._log(f"尝试推进 Workspace 授权: {target_url[:120]}...", "warning")
            response = session.get(
                target_url,
                timeout=20,
            )

            current_url = self._clean_text(getattr(response, "url", ""))
            html = str(getattr(response, "text", "") or "")

            if "code=" in current_url and "state=" in current_url:
                self.engine._log(f"Workspace 授权页直接命中回调 URL: {current_url[:100]}...", "warning")
                return current_url

            workspace_id = self.engine._extract_workspace_id_from_response(
                response=response,
                html=html,
                url=current_url,
            )
            if workspace_id:
                self.engine._cached_workspace_id = workspace_id
                self.engine._log(f"Workspace 授权页提取到 Workspace ID: {workspace_id}", "warning")

            consent_markers = (
                "sign-in-with-chatgpt/codex/consent" in current_url
                or 'action="/sign-in-with-chatgpt/codex/consent"' in html
                or "/workspace" in current_url
                or "/organization" in current_url
            )

            if consent_markers and workspace_id:
                continue_url = self.engine._select_workspace(workspace_id)
                if not continue_url:
                    return None
                return self.engine._follow_redirects(continue_url)

            for next_url in self.discover_auth_navigation_urls(response, target_url):
                if next_url == target_url:
                    continue
                callback_url = self.advance_workspace_authorization(
                    next_url,
                    _visited=visited,
                )
                if callback_url:
                    return callback_url

            return None
        except Exception as e:
            self.engine._log(f"推进 Workspace 授权失败: {e}", "warning")
            return None

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
        target_url = self._clean_text(continue_url)
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

    def complete_login_email_otp_verification(self) -> FlowResolutionResult:
        """完成登录后的邮箱验证码验证，并继续推进 OAuth。"""
        self.engine._last_login_recovery_page_type = ""
        self.engine._log("登录后触发邮箱二次验证，开始获取验证码...", "warning")
        self.engine._otp_sent_at = time.time()

        max_attempts = 2
        code: Optional[str] = None
        validate_payload: Optional[dict[str, Any]] = None

        for attempt in range(max_attempts):
            if attempt == 0:
                code = self.engine._get_verification_code()
                if code:
                    validate_payload = self.engine._validate_verification_code_with_payload(code)
            else:
                reason = "首次等待登录验证码超时" if not code else "登录验证码不是最新一封或已失效"
                self.engine._log(f"{reason}，尝试重发后重新获取...", "warning")
                if not self.engine._resend_email_verification_code():
                    return FlowResolutionResult()
                code = self.engine._get_verification_code()
                if not code:
                    continue
                validate_payload = self.engine._validate_verification_code_with_payload(code)

            if validate_payload:
                break

            if not self.engine._is_wrong_email_otp_code_error() and code:
                return FlowResolutionResult()
        else:
            return FlowResolutionResult()

        resolution = self.resolve_auth_result(
            validate_payload,
            "登录邮箱验证码校验",
        )
        self.engine._last_login_recovery_page_type = resolution.page_type
        if resolution.callback_url:
            return resolution

        if resolution.page_type == "add_phone":
            return resolution

        if getattr(self.engine, "oauth_start", None):
            self.engine._log("登录邮箱验证码校验后未拿到 continue_url，回退到 OAuth URL 重试跳转...", "warning")
            retry_url = self.engine._build_authenticated_oauth_url()
            if retry_url:
                resolution.callback_url = self.engine._follow_redirects(retry_url)

        return resolution

    def resolve_post_registration_callback(
        self,
        did: Optional[str],
        sen_token: Optional[str],
    ) -> PostRegistrationRedirectResult:
        result = PostRegistrationRedirectResult(metadata={})
        callback_url: Optional[str] = None

        if not getattr(self.engine, "_is_existing_account", False):
            if (
                getattr(self.engine, "_post_create_page_type", "") == "add_phone"
                or "add-phone" in getattr(self.engine, "_post_create_continue_url", "")
            ):
                if getattr(self.engine, "_add_phone_login_bypass_attempted", False):
                    self.engine._log("add_phone 登录回退已执行过，跳过重复登录链路", "warning")
                else:
                    callback_url = self.engine._attempt_add_phone_login_bypass(did, sen_token)
                if not callback_url:
                    result.metadata = {
                        "blocked_step": "add_phone",
                        "continue_url": getattr(self.engine, "_post_create_continue_url", ""),
                    }
                    self.engine._log(
                        "OpenAI 注册后进入手机号验证，登录回退未直接拿到 OAuth 回调；继续尝试从当前会话提取 Workspace",
                        "warning",
                    )

        if callback_url:
            self.engine._log("13. 已通过登录回退链路拿到 OAuth 回调，跳过 Workspace 预选择")
            result.callback_url = callback_url
            return result

        self.engine._log("13. 获取 Workspace ID...")
        workspace_id = self.engine._get_workspace_id()
        continue_url = ""
        if workspace_id:
            result.workspace_id = workspace_id
            self.engine._log("14. 选择 Workspace...")
            continue_url = self.engine._select_workspace(workspace_id)
            if not continue_url:
                self.engine._log("workspace/select 失败，回退到原始 OAuth URL 继续流程", "warning")
                continue_url = self.engine._build_authenticated_oauth_url()
        else:
            self.engine._log("未提前拿到 Workspace ID，回退到原始 OAuth URL 继续流程", "warning")
            continue_url = self.engine._build_authenticated_oauth_url()

        self.engine._log("15. 跟随重定向链...")
        result.callback_url = self.engine._follow_redirects(continue_url)
        if not result.callback_url:
            if isinstance(result.metadata, dict) and result.metadata.get("blocked_step") == "add_phone":
                result.error_message = self._blocked_by_add_phone_error()
            else:
                result.error_message = "跟随重定向链失败"

        return result

    def attempt_add_phone_login_bypass(self, did: Optional[str], sen_token: Optional[str]) -> Optional[str]:
        """注册后若进入 add_phone，尝试改走登录流继续完成 OAuth。"""
        if not getattr(self.engine, "email", None) or not getattr(self.engine, "password", None):
            self.engine._log("缺少邮箱或密码，无法执行 add_phone 登录回退", "error")
            return None
        self.engine._add_phone_login_bypass_attempted = True

        attempts = [
            ("当前会话", did, sen_token, False),
            ("新 OAuth 会话", None, None, True),
        ]

        for attempt_name, current_did, current_sentinel, recreate_session in attempts:
            self.engine._last_login_recovery_page_type = ""
            if recreate_session:
                self.engine._log(f"add_phone 回退：切换到{attempt_name}重试登录链路...", "warning")
                current_did, current_sentinel = self.engine._restart_oauth_session_for_login()
                if not current_did:
                    continue
            else:
                self.engine._log("检测到 add_phone，尝试改走登录链路继续 OAuth...", "warning")

            login_page = self.engine._submit_login_identifier(current_did, current_sentinel)
            if not login_page:
                continue

            callback_url = self.resolve_callback_from_auth_page(login_page, f"{attempt_name}登录邮箱")
            if callback_url:
                return callback_url

            page_type = self._clean_text(login_page.get("type"))
            otp_resolution = None
            if page_type == "login_password":
                password_page = self.engine._verify_login_password(self.engine.password)
                if not password_page:
                    continue

                callback_url = self.resolve_callback_from_auth_page(password_page, f"{attempt_name}登录密码")
                if callback_url:
                    return callback_url

                page_type = self._clean_text(password_page.get("type"))
                if page_type == "email_otp_verification":
                    otp_resolution = self.engine._complete_login_email_otp_verification()
                    if otp_resolution.callback_url:
                        return otp_resolution.callback_url
                    page_type = (
                        otp_resolution.page_type
                        or self.engine._last_login_recovery_page_type
                        or page_type
                    )

            if page_type == "add_phone":
                auth_target = (
                    getattr(otp_resolution, "continue_url", "")
                    or getattr(self.engine, "_post_create_continue_url", "")
                    or self._clean_text(getattr(getattr(self.engine, "oauth_start", None), "auth_url", ""))
                )
                callback_url = self.engine._advance_workspace_authorization(auth_target)
                if callback_url:
                    return callback_url

                if getattr(self.engine, "oauth_start", None):
                    self.engine._log(f"{attempt_name} 命中 add_phone，先复用当前已登录会话重新跟随 OAuth URL...", "warning")
                    retry_url = self.engine._build_authenticated_oauth_url()
                    if retry_url:
                        callback_url = self.engine._follow_redirects(retry_url)
                    if callback_url:
                        return callback_url

                if recreate_session:
                    self.engine._log("新 OAuth 会话登录后仍停留在 add_phone", "warning")
                else:
                    self.engine._log("当前会话登录后仍停留在 add_phone，停止重复登录链路，交由后续浏览器/会话恢复处理", "warning")
                    break
                continue

            if getattr(self.engine, "oauth_start", None):
                self.engine._log(f"{attempt_name} 未直接拿到回调，尝试重新跟随 OAuth URL...", "warning")
                retry_url = self.engine._build_authenticated_oauth_url()
                if not retry_url:
                    continue
                callback_url = self.engine._follow_redirects(retry_url)
                if callback_url:
                    return callback_url

        return None

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
