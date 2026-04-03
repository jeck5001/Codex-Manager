"""
Any-auto-register 风格的独立注册模式。

复用现有 RegistrationEngine 的前置注册步骤，在账号创建后优先尝试
chatgpt.com/api/auth/session 会话复用，失败时再回退现有 OAuth callback 收敛。
"""

from __future__ import annotations

import time
from datetime import datetime
from typing import Any, Callable, Dict, Optional, Tuple

from .register import RegistrationEngine, RegistrationResult
from .token_refresh import TokenRefreshManager


ANY_AUTO_REGISTER_MODE = "any_auto"
CHATGPT_HOME_URL = "https://chatgpt.com/"
CHATGPT_SESSION_URL = "https://chatgpt.com/api/auth/session"


class AnyAutoRegistrationRunner:
    def __init__(
        self,
        email_service,
        proxy_url: Optional[str] = None,
        callback_logger: Optional[Callable[[str], None]] = None,
        task_uuid: Optional[str] = None,
        email_code_timeout_override: Optional[int] = None,
        email_code_poll_interval_override: Optional[int] = None,
    ):
        self.engine = RegistrationEngine(
            email_service=email_service,
            proxy_url=proxy_url,
            callback_logger=callback_logger,
            task_uuid=task_uuid,
            email_code_timeout_override=email_code_timeout_override,
            email_code_poll_interval_override=email_code_poll_interval_override,
        )

    def save_to_database(self, result: RegistrationResult) -> bool:
        return self.engine.save_to_database(result)

    def _log(self, message: str, level: str = "info"):
        self.engine._log(message, level)

    def _build_final_metadata(self, result: RegistrationResult, health_error: str) -> Dict[str, Any]:
        metadata = dict(result.metadata or {})
        metadata.update({
            "email_service": self.engine.email_service.service_type.value,
            "proxy_used": self.engine.proxy_url,
            "registered_at": datetime.now().isoformat(),
            "is_existing_account": self.engine._is_existing_account,
            "register_mode": ANY_AUTO_REGISTER_MODE,
            "health_check": {
                "checked": True,
                "is_usable": result.is_usable,
                "account_status": result.account_status,
                "error": health_error if not result.is_usable else "",
            },
        })
        return metadata

    @staticmethod
    def _inject_session_token_cookie(cookies: str, session_token: str) -> str:
        cookies = (cookies or "").strip()
        session_token = (session_token or "").strip()
        if not session_token:
            return cookies

        if (
            "__Secure-next-auth.session-token=" in cookies
            or "next-auth.session-token=" in cookies
        ):
            return cookies

        session_cookie = f"__Secure-next-auth.session-token={session_token}"
        if not cookies:
            return session_cookie
        return f"{cookies}; {session_cookie}"

    def _complete_result(self, result: RegistrationResult) -> RegistrationResult:
        result.password = self.engine.password or ""
        result.cookies = self.engine._serialize_session_cookies()
        result.source = "login" if self.engine._is_existing_account else "register"

        if not result.session_token:
            session_cookie = self.engine._extract_session_token_from_cookies()
            if session_cookie:
                self.engine.session_token = session_cookie
                result.session_token = session_cookie
                self._log("获取到 Session Token")

        result.cookies = self._inject_session_token_cookie(
            result.cookies,
            result.session_token,
        )

        if not result.workspace_id:
            result.workspace_id = (
                self.engine._extract_workspace_id_from_token(result.id_token)
                or self.engine._extract_workspace_id_from_token(result.access_token)
                or result.account_id
                or ""
            )
            if result.workspace_id:
                self._log(f"从 Token 中回填 Workspace ID: {result.workspace_id}")

        self._log("17. 执行账号健康检查...")
        account_status, is_usable, health_error = self.engine._post_registration_health_check(
            result.access_token
        )
        result.account_status = account_status
        result.is_usable = is_usable
        if not is_usable:
            result.error_message = health_error
            self._log(f"账号健康检查失败: {health_error}", "warning")
        else:
            self._log("账号健康检查通过")

        self._log("=" * 60)
        if not result.is_usable:
            self._log("注册链路完成，但账号当前不可用")
        elif self.engine._is_existing_account:
            self._log("登录成功! (Any-Auto 模式)")
        else:
            self._log("注册成功! (Any-Auto 模式)")
        self._log(f"邮箱: {result.email}")
        self._log(f"Account ID: {result.account_id}")
        self._log(f"Workspace ID: {result.workspace_id}")
        self._log("=" * 60)

        result.success = True
        result.metadata = self._build_final_metadata(result, health_error)
        return result

    def _populate_from_oauth_tokens(
        self,
        result: RegistrationResult,
        token_info: Dict[str, Any],
        extra_metadata: Optional[Dict[str, Any]] = None,
    ) -> RegistrationResult:
        result.account_id = self.engine._clean_text(token_info.get("account_id"))
        result.access_token = self.engine._clean_text(token_info.get("access_token"))
        result.refresh_token = self.engine._clean_text(token_info.get("refresh_token"))
        result.id_token = self.engine._clean_text(token_info.get("id_token"))
        if extra_metadata:
            result.metadata = dict(extra_metadata)
        return self._complete_result(result)

    def _recover_chatgpt_access_token(
        self,
        session_payload: Optional[Dict[str, Any]],
    ) -> Optional[Dict[str, Any]]:
        payload = dict(session_payload or {})
        access_token = self.engine._clean_text(payload.get("accessToken"))
        if access_token:
            return payload

        self._log("ChatGPT Session 缺少 accessToken，尝试使用 session_token 刷新", "warning")
        session_token = (
            self.engine._clean_text(payload.get("sessionToken"))
            or self.engine._extract_session_token_from_cookies()
            or ""
        )
        if not session_token:
            self._log("未找到可用的 session_token，无法补齐 accessToken", "warning")
            return None

        refresh_manager = TokenRefreshManager(proxy_url=self.engine.proxy_url)
        refresh_result = refresh_manager.refresh_by_session_token(session_token)
        if not getattr(refresh_result, "success", False):
            self._log(
                f"session_token 刷新失败: {getattr(refresh_result, 'error_message', '') or 'unknown error'}",
                "warning",
            )
            return None

        refreshed_access_token = self.engine._clean_text(getattr(refresh_result, "access_token", ""))
        if not refreshed_access_token:
            self._log("session_token 刷新成功但缺少 access_token", "warning")
            return None

        payload["accessToken"] = refreshed_access_token
        payload["sessionToken"] = session_token
        self._log("已通过 session_token 刷新补齐 accessToken")
        return payload

    def _fetch_chatgpt_session_payload(self) -> Tuple[Optional[Dict[str, Any]], str]:
        session = getattr(self.engine, "session", None)
        if session is None:
            return None, "未初始化 HTTP 会话"

        last_error = ""
        for target_url in (
            CHATGPT_HOME_URL,
            f"{CHATGPT_HOME_URL}?model=auto",
        ):
            try:
                self._log(f"尝试落地 ChatGPT 会话: {target_url}")
                response = session.get(
                    target_url,
                    headers={
                        "accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                        "referer": "https://auth.openai.com/",
                    },
                    allow_redirects=True,
                    timeout=30,
                )
                final_url = self.engine._clean_text(getattr(response, "url", ""))
                if final_url:
                    self._log(f"ChatGPT 会话落地 URL: {final_url[:120]}...")
            except Exception as exc:
                last_error = f"访问 ChatGPT 首页失败: {exc}"
                self._log(last_error, "warning")
                continue

            try:
                response = session.get(
                    CHATGPT_SESSION_URL,
                    headers={
                        "accept": "application/json",
                        "referer": CHATGPT_HOME_URL,
                    },
                    timeout=30,
                )
                self._log(f"ChatGPT Session 接口状态: {response.status_code}")
                if response.status_code != 200:
                    last_error = f"ChatGPT Session 接口返回 HTTP {response.status_code}"
                    continue

                payload = response.json()
                recovered_payload = self._recover_chatgpt_access_token(payload)
                if recovered_payload:
                    return recovered_payload, ""
                last_error = "ChatGPT Session 接口未返回 accessToken"
                continue
            except Exception as exc:
                last_error = f"读取 ChatGPT Session 失败: {exc}"
                self._log(last_error, "warning")

        return None, last_error or "未能复用 ChatGPT Session"

    def _populate_from_chatgpt_session(
        self,
        result: RegistrationResult,
        session_payload: Dict[str, Any],
    ) -> RegistrationResult:
        user = session_payload.get("user") if isinstance(session_payload.get("user"), dict) else {}
        account = session_payload.get("account") if isinstance(session_payload.get("account"), dict) else {}
        access_token = self.engine._clean_text(session_payload.get("accessToken"))
        session_token = (
            self.engine._clean_text(session_payload.get("sessionToken"))
            or self.engine._extract_session_token_from_cookies()
            or ""
        )
        account_id = (
            self.engine._clean_text(account.get("id"))
            or self.engine._clean_text(user.get("id"))
            or self.engine._extract_workspace_id_from_token(access_token)
            or ""
        )
        workspace_id = (
            self.engine._clean_text(account.get("id"))
            or self.engine._extract_workspace_id_from_token(access_token)
            or account_id
        )

        result.account_id = account_id
        result.workspace_id = workspace_id
        result.access_token = access_token
        result.session_token = session_token
        result.metadata = {
            "token_source": "chatgpt_session",
            "chatgpt_session": {
                "expires": session_payload.get("expires"),
                "auth_provider": session_payload.get("authProvider"),
            },
        }
        return self._complete_result(result)

    def run(self) -> RegistrationResult:
        result = RegistrationResult(success=False, logs=self.engine.logs)

        try:
            callback_url: Optional[str] = None

            self._log("=" * 60)
            self._log("开始注册流程 (Any-Auto 模式)")
            self._log("=" * 60)

            self._log("1. 检查 IP 地理位置...")
            ip_ok, location = self.engine._check_ip_location()
            if not ip_ok:
                result.error_message = f"IP 地理位置不支持: {location}"
                self._log(f"IP 检查失败: {location}", "error")
                return result
            self._log(f"IP 位置: {location}")

            self._log("2. 创建邮箱...")
            if not self.engine._create_email():
                result.error_message = "创建邮箱失败"
                return result
            result.email = self.engine.email or ""

            self._log("3. 初始化会话...")
            if not self.engine._init_session():
                result.error_message = "初始化会话失败"
                return result

            self._log("4. 开始 OAuth 授权流程...")
            if not self.engine._start_oauth():
                result.error_message = "开始 OAuth 流程失败"
                return result

            self._log("5. 获取 Device ID...")
            did = self.engine._get_device_id()
            if not did:
                result.error_message = "获取 Device ID 失败"
                return result

            self._log("6. 检查 Sentinel 拦截...")
            sen_token = self.engine._check_sentinel(did)
            if sen_token:
                self._log("Sentinel 检查通过")
            else:
                self._log("Sentinel 检查失败或未启用", "warning")

            self._log("7. 提交注册表单...")
            signup_result = self.engine._submit_signup_form(did, sen_token)
            if not signup_result.success:
                result.error_message = f"提交注册表单失败: {signup_result.error_message}"
                return result

            if self.engine._is_existing_account:
                self._log("8. [已注册账号] 跳过密码设置，OTP 已自动发送")
            else:
                self._log("8. 注册密码...")
                password_ok, _password = self.engine._register_password()
                if not password_ok:
                    result.error_message = "注册密码失败"
                    return result

            if self.engine._is_existing_account:
                self._log("9. [已注册账号] 跳过发送验证码，使用自动发送的 OTP")
                self.engine._otp_sent_at = time.time()
            else:
                self._log("9. 发送验证码...")
                if not self.engine._send_verification_code():
                    result.error_message = "发送验证码失败"
                    return result

            self._log("10. 等待验证码...")
            code = self.engine._wait_for_signup_verification_code()
            if not code:
                result.error_message = "获取验证码失败"
                return result

            self._log("11. 验证验证码...")
            if not self.engine._validate_signup_verification_code_with_retry(code):
                result.error_message = "验证验证码失败"
                return result

            if self.engine._is_existing_account:
                self._log("12. [已注册账号] 跳过创建用户账户")
            else:
                self._log("12. 创建用户账户...")
                if not self.engine._create_user_account():
                    result.error_message = "创建用户账户失败"
                    return result

            if (
                not self.engine._is_existing_account
                and (
                    getattr(self.engine, "_post_create_page_type", "") == "add_phone"
                    or "add-phone" in getattr(self.engine, "_post_create_continue_url", "")
                )
            ):
                self._log("13. 检测到 add_phone，先尝试登录回退以便复用会话", "warning")
                callback_url = self.engine._attempt_add_phone_login_bypass(did, sen_token)
            else:
                self._log("13. 尝试直接复用当前会话")

            self._log("14. 尝试复用已登录会话直取 ChatGPT Session...")
            session_payload, session_error = self._fetch_chatgpt_session_payload()
            if session_payload:
                self._log("15. 已通过 ChatGPT Session 提取 Access Token")
                return self._populate_from_chatgpt_session(result, session_payload)

            self._log(f"会话复用未成功，回退 OAuth 收敛: {session_error}", "warning")

            if not callback_url:
                redirect_result = self.engine._get_flow_runner().resolve_post_registration_callback(did, sen_token)
                callback_url = redirect_result.callback_url
                if redirect_result.workspace_id:
                    result.workspace_id = redirect_result.workspace_id
                if redirect_result.metadata:
                    result.metadata = dict(redirect_result.metadata)
                if redirect_result.error_message:
                    result.error_message = redirect_result.error_message
                    return result

            self._log("16. 处理 OAuth 回调...")
            token_info = self.engine._handle_oauth_callback(callback_url)
            if not token_info:
                result.error_message = "处理 OAuth 回调失败"
                return result

            return self._populate_from_oauth_tokens(
                result,
                token_info,
                extra_metadata=result.metadata if isinstance(result.metadata, dict) else None,
            )

        except Exception as exc:
            self._log(f"Any-Auto 注册过程中发生未预期错误: {exc}", "error")
            result.error_message = str(exc)
            return result
