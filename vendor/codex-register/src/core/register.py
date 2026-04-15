"""
注册流程引擎
从 main.py 中提取并重构的注册流程
"""

import json
import re
import time
import logging
import secrets
import string
import urllib.parse
from typing import Optional, Dict, Any, Tuple, Callable
from dataclasses import dataclass
from datetime import datetime

from curl_cffi import requests as cffi_requests

from .register_flow_runner import RegisterFlowRunner
from .cpa_register_runtime import CPARegisterRuntime
from .cpa_page_driver import classify_signup_state
from .register_flow_state import (
    clean_text as flow_clean_text,
    extract_auth_continue_url,
    extract_auth_page_type,
    extract_workspace_id_from_response,
    extract_workspace_id_from_response_payload,
    extract_workspace_id_from_text,
    extract_workspace_id_from_url,
    workspace_id_from_mapping,
)
from .register_retry_policy import should_retry_signup_otp_validation
from .register_token_resolver import (
    build_callback_url_from_page,
    extract_workspace_id_from_token,
)
from .sentinel_browser import (
    fetch_browser_auth_state,
    fetch_browser_device_id,
    fetch_browser_sentinel_token,
)
from .oauth import OAuthManager, OAuthStart
from .http_client import OpenAIHTTPClient, HTTPClientError
from ..services import EmailServiceFactory, BaseEmailService, EmailServiceType
from ..database import crud
from ..database.session import get_db
from ..config.constants import (
    OPENAI_API_ENDPOINTS,
    OPENAI_PAGE_TYPES,
    generate_random_user_info,
    OTP_CODE_PATTERN,
    DEFAULT_PASSWORD_LENGTH,
    PASSWORD_CHARSET,
    AccountStatus,
    TaskStatus,
)
from ..config.settings import get_settings


logger = logging.getLogger(__name__)
CREATE_ACCOUNT_SENTINEL_FLOWS = (
    "oauth_create_account",
    "create_account",
    "username_password_create",
)


@dataclass
class RegistrationResult:
    """注册结果"""
    success: bool
    email: str = ""
    password: str = ""  # 注册密码
    account_id: str = ""
    workspace_id: str = ""
    access_token: str = ""
    refresh_token: str = ""
    id_token: str = ""
    session_token: str = ""  # 会话令牌
    cookies: str = ""  # 完整 cookie 串，用于支付/会话复用
    error_message: str = ""
    logs: list = None
    metadata: dict = None
    source: str = "register"  # 'register' 或 'login'，区分账号来源
    account_status: str = "active"  # active / expired / banned / failed
    is_usable: bool = True

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            "success": self.success,
            "email": self.email,
            "password": self.password,
            "account_id": self.account_id,
            "workspace_id": self.workspace_id,
            "access_token": self.access_token[:20] + "..." if self.access_token else "",
            "refresh_token": self.refresh_token[:20] + "..." if self.refresh_token else "",
            "id_token": self.id_token[:20] + "..." if self.id_token else "",
            "session_token": self.session_token[:20] + "..." if self.session_token else "",
            "cookies": self.cookies[:40] + "..." if self.cookies else "",
            "error_message": self.error_message,
            "logs": self.logs or [],
            "metadata": self.metadata or {},
            "source": self.source,
            "account_status": self.account_status,
            "is_usable": self.is_usable,
        }


@dataclass
class SignupFormResult:
    """提交注册表单的结果"""
    success: bool
    page_type: str = ""  # 响应中的 page.type 字段
    is_existing_account: bool = False  # 是否为已注册账号
    retryable: bool = False  # 是否可重试（用于 CPA 密码页恢复）
    response_data: Dict[str, Any] = None  # 完整的响应数据
    error_message: str = ""


@dataclass
class AuthResolutionResult:
    """认证响应推进结果"""
    callback_url: Optional[str] = None
    page_type: str = ""
    continue_url: str = ""


class RegistrationEngine:
    """
    注册引擎
    负责协调邮箱服务、OAuth 流程和 OpenAI API 调用
    """

    def __init__(
        self,
        email_service: BaseEmailService,
        proxy_url: Optional[str] = None,
        callback_logger: Optional[Callable[[str], None]] = None,
        task_uuid: Optional[str] = None,
        email_code_timeout_override: Optional[int] = None,
        email_code_poll_interval_override: Optional[int] = None,
    ):
        """
        初始化注册引擎

        Args:
            email_service: 邮箱服务实例
            proxy_url: 代理 URL
            callback_logger: 日志回调函数
            task_uuid: 任务 UUID（用于数据库记录）
            email_code_timeout_override: 本次任务专用验证码等待超时覆盖值
            email_code_poll_interval_override: 本次任务专用验证码轮询间隔覆盖值
        """
        self.email_service = email_service
        self.proxy_url = proxy_url
        self.callback_logger = callback_logger or (lambda msg: logger.info(msg))
        self.task_uuid = task_uuid

        # 创建 HTTP 客户端
        self.http_client = OpenAIHTTPClient(proxy_url=proxy_url)

        # 创建 OAuth 管理器
        settings = get_settings()
        self.oauth_manager = OAuthManager(
            client_id=settings.openai_client_id,
            auth_url=settings.openai_auth_url,
            token_url=settings.openai_token_url,
            redirect_uri=settings.openai_redirect_uri,
            scope=settings.openai_scope,
            proxy_url=proxy_url  # 传递代理配置
        )

        # 状态变量
        self.email: Optional[str] = None
        self.password: Optional[str] = None  # 注册密码
        self.email_info: Optional[Dict[str, Any]] = None
        self.oauth_start: Optional[OAuthStart] = None
        self.session: Optional[cffi_requests.Session] = None
        self.session_token: Optional[str] = None  # 会话令牌
        self.logs: list = []
        self._otp_sent_at: Optional[float] = None  # OTP 发送时间戳
        self._email_code_timeout_override = email_code_timeout_override
        self._email_code_poll_interval_override = email_code_poll_interval_override
        self._is_existing_account: bool = False  # 是否为已注册账号（用于自动登录）
        self._post_create_page_type: str = ""
        self._post_create_continue_url: str = ""
        self._last_login_recovery_page_type: str = ""
        self._last_otp_error_code: str = ""
        self._last_otp_error_message: str = ""
        self._cached_workspace_id: str = ""
        self._current_device_id: str = ""
        self._current_sentinel_token: str = ""
        self.flow_runner = RegisterFlowRunner(self)
        self.cpa_runtime = CPARegisterRuntime(self)

    def _get_flow_runner(self) -> RegisterFlowRunner:
        runner = getattr(self, "flow_runner", None)
        if runner is None:
            runner = RegisterFlowRunner(self)
            self.flow_runner = runner
        return runner

    def _get_email_code_wait_settings(self) -> Tuple[int, int]:
        """获取验证码等待配置。"""
        settings = get_settings()
        timeout = max(
            30,
            int(
                self._email_code_timeout_override
                or getattr(settings, "email_code_timeout", 120)
                or 120
            ),
        )
        poll_interval = max(
            1,
            min(
                30,
                int(
                    self._email_code_poll_interval_override
                    or getattr(settings, "email_code_poll_interval", 3)
                    or 3
                ),
            ),
        )
        return timeout, poll_interval

    def _log(self, message: str, level: str = "info"):
        """记录日志"""
        timestamp = datetime.now().strftime("%H:%M:%S")
        log_message = f"[{timestamp}] {message}"

        # 添加到日志列表
        self.logs.append(log_message)

        # 调用回调函数
        if self.callback_logger:
            self.callback_logger(log_message)

        # 记录到数据库（如果有关联任务）
        if self.task_uuid:
            try:
                with get_db() as db:
                    crud.append_task_log(db, self.task_uuid, log_message)
            except Exception as e:
                logger.warning(f"记录任务日志失败: {e}")

        # 根据级别记录到日志系统
        if level == "error":
            logger.error(message)
        elif level == "warning":
            logger.warning(message)
        else:
            logger.info(message)

    @staticmethod
    def _clean_text(value: Any) -> str:
        """清洗文本值"""
        return flow_clean_text(value)

    def _extract_auth_page_type(self, payload: Any) -> str:
        """从认证响应或 page 对象中提取页面类型"""
        return extract_auth_page_type(payload)

    def _extract_auth_continue_url(self, payload: Any) -> str:
        """从认证响应中提取 continue_url"""
        return extract_auth_continue_url(payload)

    def _build_authenticated_oauth_url(self) -> str:
        """构造适用于已登录会话的 OAuth URL，去掉 prompt=login 避免再次落到登录页。"""
        raw_url = self._clean_text(self.oauth_start.auth_url if self.oauth_start else "")
        if not raw_url:
            return ""

        parsed = urllib.parse.urlsplit(raw_url)
        query_items = urllib.parse.parse_qsl(parsed.query, keep_blank_values=True)
        filtered_items = []
        for key, value in query_items:
            normalized_key = self._clean_text(key)
            normalized_value = self._clean_text(value)
            if normalized_key == "prompt" and normalized_value.lower() == "login":
                continue
            filtered_items.append((key, value))

        rebuilt_query = urllib.parse.urlencode(filtered_items, doseq=True)
        return urllib.parse.urlunsplit((
            parsed.scheme,
            parsed.netloc,
            parsed.path,
            rebuilt_query,
            parsed.fragment,
        ))

    def _clear_otp_error_state(self):
        """清空最近一次 OTP 校验错误状态"""
        self._last_otp_error_code = ""
        self._last_otp_error_message = ""

    def _update_otp_error_state(self, response_json: Any):
        """记录最近一次 OTP 校验错误状态"""
        self._clear_otp_error_state()
        if not isinstance(response_json, dict):
            return

        error = response_json.get("error")
        if not isinstance(error, dict):
            return

        self._last_otp_error_code = self._clean_text(error.get("code"))
        self._last_otp_error_message = self._clean_text(error.get("message"))

    def _is_wrong_email_otp_code_error(self) -> bool:
        """是否为错误验证码"""
        return self._last_otp_error_code == "wrong_email_otp_code"

    def _workspace_id_from_mapping(self, data: Any) -> str:
        """从 auth/session 映射里提取 workspace / organization ID"""
        return workspace_id_from_mapping(data)

    def _extract_workspace_id_from_text(self, text: str) -> Optional[str]:
        """从 HTML/脚本文本中提取 Workspace ID。"""
        return extract_workspace_id_from_text(text)

    def _extract_workspace_id_from_url(self, url: str) -> Optional[str]:
        """从 URL 查询参数或片段中提取 Workspace ID。"""
        return extract_workspace_id_from_url(url)

    def _extract_workspace_id_from_response_payload(self, payload: Any, depth: int = 0) -> Optional[str]:
        """递归扫描响应载荷中的 Workspace ID。"""
        return extract_workspace_id_from_response_payload(payload, depth)

    def _extract_workspace_id_from_response(
        self,
        response: Optional[Any] = None,
        html: Optional[str] = None,
        url: Optional[str] = None,
    ) -> Optional[str]:
        """统一从响应 JSON、HTML、脚本内容和 URL 中提取 Workspace ID。"""
        return extract_workspace_id_from_response(response=response, html=html, url=url)

    def _log_auth_response_preview(self, prefix: str, payload: Any):
        """记录认证接口响应摘要"""
        try:
            if not isinstance(payload, dict):
                return

            preview = {}
            for key in (
                "continue_url",
                "redirect_url",
                "callback_url",
                "next_url",
                "page",
                "type",
                "payload",
                "account",
            ):
                if key in payload:
                    preview[key] = payload.get(key)

            if preview:
                self._log(f"{prefix}: {json.dumps(preview, ensure_ascii=False)[:500]}")
        except Exception:
            pass

    def _generate_password(self, length: int = DEFAULT_PASSWORD_LENGTH) -> str:
        """生成符合当前 OpenAI 注册要求的随机密码。"""
        normalized_length = max(int(length or DEFAULT_PASSWORD_LENGTH), 16)
        lowercase_chars = string.ascii_lowercase
        uppercase_chars = string.ascii_uppercase
        digit_chars = string.digits
        base_pool = PASSWORD_CHARSET

        password_chars = [
            secrets.choice(lowercase_chars),
            secrets.choice(uppercase_chars),
            secrets.choice(digit_chars),
            "!",
        ]
        while len(password_chars) < normalized_length:
            password_chars.append(secrets.choice(base_pool))

        for index in range(len(password_chars) - 1, 0, -1):
            swap_index = secrets.randbelow(index + 1)
            password_chars[index], password_chars[swap_index] = (
                password_chars[swap_index],
                password_chars[index],
            )

        return ''.join(password_chars)

    def _check_ip_location(self) -> Tuple[bool, Optional[str]]:
        """检查 IP 地理位置"""
        try:
            return self.http_client.check_ip_location()
        except Exception as e:
            self._log(f"检查 IP 地理位置失败: {e}", "error")
            return False, None

    def _create_email(self) -> bool:
        """创建邮箱"""
        try:
            self._log(f"正在创建 {self.email_service.service_type.value} 邮箱...")
            self.email_info = self.email_service.create_email()

            if not self.email_info or "email" not in self.email_info:
                self._log("创建邮箱失败: 返回信息不完整", "error")
                return False

            self.email = self.email_info["email"]
            self._log(f"成功创建邮箱: {self.email}")
            return True

        except Exception as e:
            self._log(f"创建邮箱失败: {e}", "error")
            return False

    def _start_oauth(self) -> bool:
        """开始 OAuth 流程"""
        try:
            self.oauth_start = self.oauth_manager.start_oauth()
            self._log(f"OAuth URL 已生成: {self.oauth_start.auth_url[:80]}...")
            return True
        except Exception as e:
            self._log(f"生成 OAuth URL 失败: {e}", "error")
            return False

    def _init_session(self) -> bool:
        """初始化会话"""
        try:
            self.session = self.http_client.session
            return True
        except Exception as e:
            self._log(f"初始化会话失败: {e}", "error")
            return False

    def _get_device_id(self) -> Optional[str]:
        """获取 Device ID"""
        if not self.oauth_start:
            return None

        max_attempts = 3
        for attempt in range(1, max_attempts + 1):
            try:
                if not self.session:
                    self.session = self.http_client.session

                response = self.session.get(
                    self.oauth_start.auth_url,
                    timeout=20
                )
                did = self._extract_device_id_from_cookie_source(getattr(self.session, "cookies", None))
                if not did:
                    did = self._extract_device_id_from_response(response)

                if did:
                    self._store_device_id_in_session(did)
                    self._log(f"Device ID: {did}")
                    return did

                if getattr(response, "status_code", 0) == 403:
                    self._log("获取 Device ID 遇到 HTTP 403，尝试浏览器兜底...", "warning")
                    browser_auth_state = self._fetch_browser_auth_state(
                        self.oauth_start.auth_url,
                    )
                    browser_did = self._clean_text((browser_auth_state or {}).get("did"))
                    if browser_did:
                        self._log("浏览器认证态已同步到当前会话，准备重试 Device ID", "warning")
                        self._store_device_id_in_session(browser_did)
                        self._log(f"Device ID: {browser_did}")
                        return browser_did

                    browser_did = fetch_browser_device_id(
                        auth_url=self.oauth_start.auth_url,
                        proxy_url=getattr(self, "proxy_url", None),
                        callback_logger=self._log,
                    )
                    if browser_did:
                        self._store_device_id_in_session(browser_did)
                        self._log(f"Device ID: {browser_did}")
                        return browser_did

                self._log(
                    f"获取 Device ID 失败: 未返回 oai-did Cookie (HTTP {response.status_code}, 第 {attempt}/{max_attempts} 次)",
                    "warning" if attempt < max_attempts else "error"
                )
            except Exception as e:
                self._log(
                    f"获取 Device ID 失败: {e} (第 {attempt}/{max_attempts} 次)",
                    "warning" if attempt < max_attempts else "error"
                )

            if attempt < max_attempts:
                time.sleep(attempt)
                self.http_client.close()
                self.session = self.http_client.session

        return None

    def _extract_device_id_from_cookie_source(self, cookies: Any) -> str:
        if cookies is None:
            return ""

        try:
            direct = self._clean_text(cookies.get("oai-did"))
        except Exception:
            direct = ""
        if direct:
            return direct

        jar = getattr(cookies, "jar", None)
        if jar:
            for cookie in jar:
                name = self._clean_text(getattr(cookie, "name", ""))
                if name != "oai-did":
                    continue
                value = self._clean_text(getattr(cookie, "value", ""))
                if value:
                    return value

        return ""

    def _extract_device_id_from_response(self, response: Any) -> str:
        if response is None:
            return ""

        did = self._extract_device_id_from_cookie_source(getattr(response, "cookies", None))
        if did:
            return did

        headers = getattr(response, "headers", None)
        set_cookie = ""
        if headers is not None:
            try:
                set_cookie = self._clean_text(headers.get("set-cookie"))
            except Exception:
                set_cookie = ""
        if set_cookie:
            match = re.search(r"(?:^|[;,]\s*)oai-did=([^;,\s]+)", set_cookie, re.IGNORECASE)
            if match:
                return self._clean_text(urllib.parse.unquote(match.group(1)))

        text = self._clean_text(getattr(response, "text", ""))
        if text:
            match = re.search(r'"oai-did"\s*[:=]\s*"([^"]+)"', text, re.IGNORECASE)
            if match:
                return self._clean_text(urllib.parse.unquote(match.group(1)))

        return ""

    def _store_device_id_in_session(self, did: str):
        normalized = self._clean_text(did)
        if not normalized:
            return
        self._current_device_id = normalized
        session = getattr(self, "session", None)
        cookies = getattr(session, "cookies", None) if session is not None else None
        if cookies is None:
            return
        try:
            cookies["oai-did"] = normalized
            return
        except Exception:
            pass
        try:
            cookies.set("oai-did", normalized)
        except Exception:
            pass

    def _session_cookie_items(self) -> list[tuple[str, str]]:
        """提取当前会话中的 cookie 列表"""
        if not self.session or not getattr(self.session, "cookies", None):
            return []

        items: list[tuple[str, str]] = []
        seen: set[str] = set()
        jar = getattr(self.session.cookies, "jar", None)

        if jar is not None:
            for cookie in jar:
                name = str(getattr(cookie, "name", "") or "").strip()
                value = str(getattr(cookie, "value", "") or "").strip()
                if not name or not value or name in seen:
                    continue
                seen.add(name)
                items.append((name, value))

        if items:
            return items

        try:
            for name, value in self.session.cookies.items():
                normalized_name = str(name or "").strip()
                normalized_value = str(value or "").strip()
                if not normalized_name or not normalized_value or normalized_name in seen:
                    continue
                seen.add(normalized_name)
                items.append((normalized_name, normalized_value))
        except Exception:
            return []

        return items

    def _session_cookie_debug_summary(self) -> str:
        """输出当前会话关键 cookie 摘要，便于定位认证状态丢失。"""
        items = self._session_cookie_items()
        names = [name for name, _value in items]
        name_set = set(names)
        has_csrf = any("csrf" in name.lower() for name in names)
        has_session = any("session-token" in name.lower() for name in names)
        return (
            f"count={len(names)} "
            f"cf_clearance={'yes' if 'cf_clearance' in name_set else 'no'} "
            f"oai-did={'yes' if 'oai-did' in name_set else 'no'} "
            f"csrf={'yes' if has_csrf else 'no'} "
            f"session={'yes' if has_session else 'no'} "
            f"names={','.join(names)}"
        )

    def _auth_response_debug_summary(self, response: Any) -> str:
        """输出认证跳转响应摘要，观察会话是否在 continue_url 阶段发生变化。"""
        headers = getattr(response, "headers", None) or {}
        status_code = getattr(response, "status_code", "")
        url = self._clean_text(getattr(response, "url", ""))
        content_type = self._clean_text(headers.get("content-type"))
        location = self._clean_text(headers.get("location"))
        has_set_cookie = bool(self._clean_text(headers.get("set-cookie")))
        return (
            f"status={status_code} "
            f"url={url or '-'} "
            f"content-type={content_type or '-'} "
            f"location={location or '-'} "
            f"set-cookie={'yes' if has_set_cookie else 'no'}"
        )

    def _resolve_password_auth_refresh_url(self, response: Any) -> str:
        """避免把浏览器兜底错误地落到 API 端点上。"""
        candidate = self._clean_text(getattr(response, "url", ""))
        if candidate:
            try:
                parsed = urllib.parse.urlsplit(candidate)
            except Exception:
                parsed = None
            if (
                parsed
                and parsed.scheme in {"http", "https"}
                and parsed.hostname == "auth.openai.com"
                and not parsed.path.startswith("/api/")
            ):
                return candidate
        return "https://auth.openai.com/create-account/password"

    def _refresh_password_auth_state_in_browser(self, response: Any) -> bool:
        """在密码注册失败后尽量按真实浏览器路径重新落地认证态。"""
        refreshed = False
        refresh_url = self._resolve_password_auth_refresh_url(response)
        auth_state = self._fetch_browser_auth_state(refresh_url)
        if auth_state:
            refreshed = True

        if not self._extract_session_token_from_cookies():
            self._log("密码注册浏览器认证态仍缺少 session token，尝试落地 ChatGPT 会话", "warning")
            chatgpt_state = self._fetch_browser_auth_state("https://chatgpt.com/")
            if chatgpt_state:
                refreshed = True

        return refreshed

    def _sync_browser_cookies_to_session(self, cookies: Any) -> int:
        """将浏览器上下文中的关键 cookie 回灌到当前 HTTP 会话。"""
        session = getattr(self, "session", None)
        cookie_store = getattr(session, "cookies", None) if session else None
        if cookie_store is None or not isinstance(cookies, list):
            return 0

        synced: list[str] = []
        for item in cookies:
            if not isinstance(item, dict):
                continue

            name = self._clean_text(item.get("name"))
            value = self._clean_text(item.get("value"))
            if not name or not value:
                continue

            domain = self._clean_text(item.get("domain"))
            if not domain:
                raw_url = self._clean_text(item.get("url"))
                if raw_url:
                    try:
                        domain = self._clean_text(urllib.parse.urlsplit(raw_url).hostname)
                    except Exception:
                        domain = ""
            path = self._clean_text(item.get("path")) or "/"

            try:
                if domain:
                    cookie_store.set(name, value, domain=domain, path=path)
                    synced.append(f"{name}@{domain}")
                else:
                    cookie_store.set(name, value, path=path)
                    synced.append(name)
            except TypeError:
                try:
                    cookie_store.set(name, value)
                    synced.append(f"{name}@fallback")
                except Exception:
                    continue
            except Exception:
                continue

        if synced:
            self._log(
                "浏览器 Sentinel 已同步 Cookie: " + ", ".join(synced[:8]),
                "warning",
            )
        return len(synced)

    def _fetch_browser_auth_state(self, auth_url: str) -> Dict[str, Any]:
        """通过浏览器落地认证页，并将回收的认证态同步回当前会话。"""
        try:
            auth_state = fetch_browser_auth_state(
                auth_url=auth_url,
                cookies_str=self._serialize_session_cookies(),
                cookies=self._session_browser_cookies(),
                proxy_url=getattr(self, "proxy_url", None),
                callback_logger=self._log,
            )
        except Exception as e:
            self._log(f"浏览器认证态获取异常: {e}", "warning")
            return {}

        if not isinstance(auth_state, dict):
            return {}

        self._sync_browser_cookies_to_session(auth_state.get("cookies"))
        did = self._clean_text(auth_state.get("did"))
        if did:
            self._store_device_id_in_session(did)
        return auth_state

    def _session_browser_cookies(self) -> list[dict[str, Any]]:
        """导出适合 Playwright 注入的结构化 cookie，保留域名维度。"""
        if not self.session or not getattr(self.session, "cookies", None):
            return []

        jar = getattr(self.session.cookies, "jar", None)
        if jar is None:
            return []

        cookies: list[dict[str, Any]] = []
        for cookie in jar:
            name = str(getattr(cookie, "name", "") or "").strip()
            value = str(getattr(cookie, "value", "") or "").strip()
            domain = str(getattr(cookie, "domain", "") or "").strip()
            path = str(getattr(cookie, "path", "") or "").strip() or "/"
            if not name or not value:
                continue

            secure = bool(getattr(cookie, "secure", True))
            http_only = False
            try:
                http_only = bool(cookie.has_nonstandard_attr("HttpOnly"))
            except Exception:
                http_only = False

            browser_cookie: dict[str, Any] = {
                "name": name,
                "value": value,
                "secure": secure,
                "httpOnly": http_only,
            }
            if name.startswith("__Host-"):
                host = domain.lstrip(".")
                if host:
                    browser_cookie["url"] = f"https://{host}/"
                else:
                    browser_cookie["url"] = "https://auth.openai.com/"
            else:
                browser_cookie["domain"] = domain or ".openai.com"
                browser_cookie["path"] = path

            cookies.append(browser_cookie)

        return cookies

    def _serialize_session_cookies(self) -> str:
        """序列化当前会话 cookie，供支付接口复用"""
        return "; ".join(f"{name}={value}" for name, value in self._session_cookie_items())

    def _resolve_current_device_id(self) -> str:
        """获取当前 Device ID，优先实例字段，缺失时回退到 cookie。"""
        current = self._clean_text(getattr(self, "_current_device_id", ""))
        if current:
            return current

        cookie_sources = []
        session = getattr(self, "session", None)
        if session is not None:
            cookie_sources.append(getattr(session, "cookies", None))
        http_client = getattr(self, "http_client", None)
        if http_client is not None:
            http_session = getattr(http_client, "session", None)
            if http_session is not None:
                cookie_sources.append(getattr(http_session, "cookies", None))

        for cookies in cookie_sources:
            if cookies is None:
                continue
            try:
                did = self._clean_text(cookies.get("oai-did"))
            except Exception:
                did = ""
            if did:
                self._current_device_id = did
                self._log("当前 Device ID 丢失，已从会话 Cookie 恢复", "warning")
                return did

        return ""

    def _extract_session_token_from_cookies(self) -> Optional[str]:
        """兼容 next-auth 分片 cookie，尽量提取完整 session token"""
        items = self._session_cookie_items()
        if not items:
            return None

        cookie_map = {name: value for name, value in items}
        for cookie_name in (
            "__Secure-next-auth.session-token",
            "next-auth.session-token",
        ):
            direct = cookie_map.get(cookie_name)
            if direct:
                return direct

            prefix = f"{cookie_name}."
            chunks: list[tuple[int, str]] = []
            for name, value in items:
                if not name.startswith(prefix):
                    continue
                suffix = name[len(prefix):]
                if suffix.isdigit():
                    chunks.append((int(suffix), value))
            if chunks:
                chunks.sort(key=lambda item: item[0])
                return "".join(value for _, value in chunks)

        return None

    def _check_sentinel(self, did: str, flow: str = "authorize_continue") -> Optional[str]:
        """检查 Sentinel 拦截"""
        try:
            normalized_flow = self._clean_text(flow) or "authorize_continue"
            sen_req_body = json.dumps({
                "p": "",
                "id": did,
                "flow": normalized_flow,
            }, separators=(",", ":"))

            response = self.http_client.post(
                OPENAI_API_ENDPOINTS["sentinel"],
                headers={
                    "origin": "https://sentinel.openai.com",
                    "referer": "https://sentinel.openai.com/backend-api/sentinel/frame.html?sv=20260219f9f6",
                    "content-type": "text/plain;charset=UTF-8",
                },
                data=sen_req_body,
            )

            if response.status_code == 200:
                sen_token = response.json().get("token")
                self._log(f"Sentinel token 获取成功 (flow={normalized_flow})")
                return sen_token
            else:
                self._log(
                    f"Sentinel 检查失败: {response.status_code} (flow={normalized_flow})",
                    "warning",
                )
                return None

        except Exception as e:
            self._log(f"Sentinel 检查异常: {e}", "warning")
            return None

    def _normalize_sentinel_payload(
        self,
        payload: Any,
        *,
        did: Optional[str],
        flow: str,
    ) -> Optional[Dict[str, str]]:
        """归一化 openai-sentinel-token 载荷。"""
        normalized_did = self._clean_text(did)
        normalized_flow = self._clean_text(flow)

        if isinstance(payload, str):
            raw = self._clean_text(payload)
            if not raw:
                return None
            if raw.startswith("{") and raw.endswith("}"):
                try:
                    payload = json.loads(raw)
                except Exception:
                    payload = {"c": raw}
            else:
                payload = {"c": raw}

        if not isinstance(payload, dict):
            return None

        normalized = {
            "p": self._clean_text(payload.get("p")),
            "t": self._clean_text(payload.get("t")),
            "c": self._clean_text(payload.get("c") or payload.get("token")),
            "id": self._clean_text(payload.get("id")) or normalized_did,
            "flow": self._clean_text(payload.get("flow")) or normalized_flow,
        }

        if not normalized["c"]:
            return None
        if not normalized["id"]:
            return None
        if not normalized["flow"]:
            return None
        return normalized

    def _build_sentinel_header(
        self,
        payload: Any,
        *,
        did: Optional[str],
        flow: str,
    ) -> Optional[str]:
        normalized = self._normalize_sentinel_payload(payload, did=did, flow=flow)
        if not normalized:
            return None
        return json.dumps(normalized, ensure_ascii=False)

    def _log_sentinel_payload_summary(
        self,
        stage: str,
        source: str,
        payload: Any,
        *,
        did: Optional[str],
        flow: str,
    ):
        """输出简明的 Sentinel 载荷诊断信息。"""
        normalized = self._normalize_sentinel_payload(payload, did=did, flow=flow)
        if not normalized:
            self._log(f"{stage} Sentinel 来源={source} flow={flow} payload=missing", "warning")
            return

        self._log(
            f"{stage} Sentinel 来源={source} flow={normalized['flow']}"
            f" p={'yes' if normalized.get('p') else 'no'}"
            f" t={'yes' if normalized.get('t') else 'no'}"
            f" c={'yes' if normalized.get('c') else 'no'}"
        )

    @staticmethod
    def _build_passkey_client_capabilities() -> str:
        """为新注册接口补齐 passkey capabilities 头。"""
        return json.dumps({
            "conditionalMediation": False,
            "userVerifyingPlatformAuthenticator": False,
            "userVerifyingCrossPlatformAuthenticator": False,
        }, separators=(",", ":"))

    def _build_browser_like_auth_headers(
        self,
        *,
        referer: str,
        did: Optional[str] = None,
        accept: str = "application/json",
        content_type: Optional[str] = None,
    ) -> Dict[str, str]:
        """对齐 openai-cpa 的认证请求头，尽量贴近浏览器路径。"""
        normalized_referer = self._clean_text(referer)
        resolved_did = self._clean_text(did) or self._resolve_current_device_id()
        headers: Dict[str, str] = {
            "accept": self._clean_text(accept) or "application/json",
        }
        if normalized_referer:
            headers["referer"] = normalized_referer
            try:
                parsed = urllib.parse.urlsplit(normalized_referer)
            except Exception:
                parsed = None
            if parsed and parsed.scheme and parsed.hostname:
                headers["origin"] = f"{parsed.scheme}://{parsed.hostname}"
        if content_type:
            headers["content-type"] = self._clean_text(content_type)
        if resolved_did:
            headers["oai-device-id"] = resolved_did

        default_headers = getattr(getattr(self, "http_client", None), "default_headers", None) or {}
        user_agent = self._clean_text(default_headers.get("User-Agent"))
        accept_language = self._clean_text(default_headers.get("Accept-Language"))
        if user_agent:
            headers["user-agent"] = user_agent
        if accept_language:
            headers["accept-language"] = accept_language

        headers.setdefault("sec-ch-ua", '"Google Chrome";v="120", "Chromium";v="120", "Not_A Brand";v="24"')
        headers.setdefault("sec-ch-ua-mobile", "?0")
        headers.setdefault("sec-ch-ua-platform", '"Windows"')
        return headers

    def _build_auth_step_sentinel_header(
        self,
        *,
        flow: str,
        referer: str,
        prefer_browser: bool = False,
    ) -> Optional[str]:
        """为注册中的认证步骤生成 Sentinel 请求头。"""
        did = self._resolve_current_device_id()
        if not did:
            return None

        payload: Any = None
        if prefer_browser:
            payload = self._get_browser_sentinel_payload(flow, referer)

        if not payload:
            payload = self._clean_text(self._check_sentinel(did, flow=flow))

        return self._build_sentinel_header(payload, did=did, flow=flow)

    def _get_browser_sentinel_payload(
        self,
        flow: str,
        referer: str,
    ) -> Optional[Dict[str, str]]:
        """按 flow 通过浏览器获取完整 sentinel token。"""
        did = self._resolve_current_device_id()
        normalized_flow = self._clean_text(flow)
        normalized_referer = self._clean_text(referer)
        if not did or not normalized_flow:
            return None

        try:
            payload = fetch_browser_sentinel_token(
                did=did,
                flow=normalized_flow,
                referer=normalized_referer,
                cookies_str=self._serialize_session_cookies(),
                proxy_url=self.proxy_url,
                callback_logger=lambda message: self._log(message),
            )
        except Exception as e:
            self._log(f"浏览器 Sentinel token 获取异常: {e}", "warning")
            return None

        normalized = self._normalize_sentinel_payload(
            payload,
            did=did,
            flow=normalized_flow,
        )
        if isinstance(payload, dict):
            synced_count = self._sync_browser_cookies_to_session(payload.get("cookies"))
            if synced_count:
                self._log(
                    f"浏览器 Sentinel Cookie 同步后会话摘要: {self._session_cookie_debug_summary()}",
                    "warning",
                )
        if normalized and normalized.get("t"):
            self._log(f"浏览器 Sentinel token 获取成功，flow={normalized_flow}")
            return normalized

        if payload:
            self._log(
                f"浏览器 Sentinel token 缺少 Turnstile 字段，flow={normalized_flow}",
                "warning",
            )
        return None

    def _get_browser_create_account_sentinel_payload(self) -> Optional[Dict[str, str]]:
        """在 create_account 前优先通过浏览器拿完整 sentinel token（含 p/t/c）。"""
        for flow in CREATE_ACCOUNT_SENTINEL_FLOWS:
            normalized = self._get_browser_sentinel_payload(
                flow,
                "https://auth.openai.com/about-you",
            )
            if normalized:
                return normalized

        return None

    def _get_create_account_sentinel_payload(self) -> Optional[Dict[str, str]]:
        """create_account 优先用浏览器获取完整 token，失败再回退纯 HTTP。"""
        did = self._resolve_current_device_id()
        if not did:
            return None

        browser_payload = self._get_browser_create_account_sentinel_payload()
        if browser_payload:
            return browser_payload

        fallback_token = self._check_sentinel(did, flow=CREATE_ACCOUNT_SENTINEL_FLOWS[-1])
        if not fallback_token:
            return None

        self._log("create_account 浏览器 Sentinel 失败，已回退纯 HTTP token", "warning")
        return self._normalize_sentinel_payload(
            fallback_token,
            did=did,
            flow=CREATE_ACCOUNT_SENTINEL_FLOWS[-1],
        )

    def _submit_signup_form(self, did: str, sen_token: Optional[str]) -> SignupFormResult:
        """
        提交注册表单

        Returns:
            SignupFormResult: 提交结果，包含账号状态判断
        """
        try:
            signup_body = f'{{"username":{{"value":"{self.email}","kind":"email"}},"screen_hint":"signup"}}'

            headers = self._build_browser_like_auth_headers(
                referer="https://auth.openai.com/create-account",
                did=did,
                content_type="application/json",
            )

            if sen_token:
                sentinel = self._build_sentinel_header(
                    sen_token,
                    did=did,
                    flow="authorize_continue",
                )
                if sentinel:
                    headers["openai-sentinel-token"] = sentinel

            response = self.session.post(
                OPENAI_API_ENDPOINTS["signup"],
                headers=headers,
                data=signup_body,
            )

            self._log(f"提交注册表单状态: {response.status_code}")

            if response.status_code != 200:
                return SignupFormResult(
                    success=False,
                    error_message=f"HTTP {response.status_code}: {response.text[:200]}"
                )

            # 解析响应判断账号状态
            try:
                response_data = response.json()
                page_type = response_data.get("page", {}).get("type", "")
                self._log(f"响应页面类型: {page_type}")
                self._log_auth_response_preview("注册邮箱响应摘要", response_data)
                page_text = ""
                try:
                    page_text = json.dumps(response_data, ensure_ascii=False)
                except Exception:
                    page_text = str(response_data or "")

                signup_state = classify_signup_state(
                    {
                        "page_text": page_text,
                        "is_signup_password_page": page_type == "create_account_password",
                        "has_retry_button": bool(
                            re.search(r"重试|try\s+again", page_text, re.IGNORECASE)
                        ),
                        "has_password_input": page_type == "create_account_password",
                    }
                )
                signup_kind = self._clean_text(signup_state.get("kind"))
                if signup_kind and signup_kind != "unknown":
                    self._log(f"CPA 注册页面状态: {signup_kind}")

                if signup_kind == "password_retry":
                    return SignupFormResult(
                        success=False,
                        page_type=page_type,
                        retryable=True,
                        response_data=response_data,
                        error_message="CPA signup password page hit retryable error, 请重试当前流程",
                    )

                if signup_kind != "password_retry":
                    self._follow_auth_continue_url(response_data, "注册邮箱")

                # 判断是否为已注册账号
                is_existing = (
                    page_type == OPENAI_PAGE_TYPES["EMAIL_OTP_VERIFICATION"]
                    or signup_kind == "email_exists"
                )

                if is_existing:
                    self._log(f"检测到已注册账号，将自动切换到登录流程")
                    self._is_existing_account = True

                return SignupFormResult(
                    success=True,
                    page_type=page_type,
                    is_existing_account=is_existing,
                    response_data=response_data
                )

            except Exception as parse_error:
                self._log(f"解析响应失败: {parse_error}", "warning")
                # 无法解析，默认成功
                return SignupFormResult(success=True)

        except Exception as e:
            self._log(f"提交注册表单失败: {e}", "error")
            return SignupFormResult(success=False, error_message=str(e))

    def _register_password(self) -> Tuple[bool, Optional[str]]:
        """注册密码"""
        try:
            # 生成密码
            password = self._generate_password()
            self.password = password  # 保存密码到实例变量
            self._log(f"生成密码: {password}")

            # 提交密码注册
            register_body = json.dumps({
                "password": password,
                "username": self.email
            })
            for attempt in range(1, 3):
                did = self._resolve_current_device_id()
                sentinel_payload = self._get_browser_sentinel_payload(
                    "username_password_create",
                    "https://auth.openai.com/create-account/password",
                )
                sen_token = ""
                if sentinel_payload:
                    self._log("密码注册浏览器 Sentinel 获取成功，准备提交注册请求")
                elif did:
                    self._log("密码注册浏览器 Sentinel 获取失败，准备回退 HTTP Sentinel", "warning")
                    sen_token = self._clean_text(
                        self._check_sentinel(did, flow="username_password_create")
                    )
                    if sen_token:
                        self._log("密码注册 HTTP Sentinel 获取成功")
                    else:
                        self._log("密码注册 HTTP Sentinel 获取失败", "warning")
                else:
                    self._log("密码注册缺少 Device ID，无法获取 Sentinel", "warning")
                if not sen_token:
                    sen_token = self._clean_text(getattr(self, "_current_sentinel_token", ""))

                headers = self._build_browser_like_auth_headers(
                    referer="https://auth.openai.com/create-account/password",
                    did=did,
                    content_type="application/json",
                )
                headers["ext-passkey-client-capabilities"] = self._build_passkey_client_capabilities()
                sentinel_source = "browser" if sentinel_payload else "http" if sen_token else "missing"
                self._log_sentinel_payload_summary(
                    "密码注册",
                    sentinel_source,
                    sentinel_payload or sen_token,
                    did=did,
                    flow="username_password_create",
                )
                sentinel = self._build_sentinel_header(
                    sentinel_payload or sen_token,
                    did=did,
                    flow="username_password_create",
                )
                if sentinel:
                    headers["openai-sentinel-token"] = sentinel

                self._log(f"密码注册会话摘要: {self._session_cookie_debug_summary()}", "warning")
                response = self.session.post(
                    OPENAI_API_ENDPOINTS["register"],
                    headers=headers,
                    data=register_body,
                )

                self._log(f"提交密码状态: {response.status_code}")

                if response.status_code == 200:
                    return True, password

                error_text = response.text[:500]
                self._log(f"密码注册失败: {error_text}", "warning")
                self._log(f"密码注册响应摘要: {self._auth_response_debug_summary(response)}", "warning")

                error_msg = ""
                error_code = ""
                try:
                    error_json = response.json()
                    error_msg = self._clean_text(error_json.get("error", {}).get("message", ""))
                    error_code = self._clean_text(error_json.get("error", {}).get("code", ""))
                except Exception:
                    error_msg = ""
                    error_code = ""

                if (
                    attempt == 1
                    and error_msg == "Failed to create account. Please try again."
                ):
                    self._log("密码注册遇到通用 400，尝试浏览器落地认证状态后重试", "warning")
                    if self._refresh_password_auth_state_in_browser(response):
                        self._current_sentinel_token = ""
                        continue

                if "already" in error_msg.lower() or "exists" in error_msg.lower() or error_code == "user_exists":
                    self._log(f"邮箱 {self.email} 可能已在 OpenAI 注册过", "error")
                    self._mark_email_as_registered()

                return False, None

            return False, None

        except Exception as e:
            self._log(f"密码注册失败: {e}", "error")
            return False, None

    def _mark_email_as_registered(self):
        """标记邮箱为已注册状态（用于防止重复尝试）"""
        try:
            with get_db() as db:
                # 检查是否已存在该邮箱的记录
                existing = crud.get_account_by_email(db, self.email)
                if not existing:
                    # 创建一个失败记录，标记该邮箱已注册过
                    crud.create_account(
                        db,
                        email=self.email,
                        password="",  # 空密码表示未成功注册
                        email_service=self.email_service.service_type.value,
                        email_service_id=self.email_info.get("service_id") if self.email_info else None,
                        status="failed",
                        extra_data={"register_failed_reason": "email_already_registered_on_openai"}
                    )
                    self._log(f"已在数据库中标记邮箱 {self.email} 为已注册状态")
        except Exception as e:
            logger.warning(f"标记邮箱状态失败: {e}")

    def _send_verification_code(self) -> bool:
        """发送验证码"""
        try:
            # 记录发送时间戳
            self._otp_sent_at = time.time()
            did = self._resolve_current_device_id()
            headers = self._build_browser_like_auth_headers(
                referer="https://auth.openai.com/create-account/password",
                did=did,
                content_type="application/json",
            )
            sentinel = self._build_auth_step_sentinel_header(
                flow="authorize_continue",
                referer="https://auth.openai.com/create-account/password",
            )
            if sentinel:
                headers["openai-sentinel-token"] = sentinel

            response = self.session.post(
                OPENAI_API_ENDPOINTS["send_otp"],
                headers=headers,
                data="{}",
            )

            self._log(f"验证码发送状态: {response.status_code}")
            if response.status_code != 200:
                return False

            try:
                response_json = response.json()
            except Exception:
                response_json = None
            if isinstance(response_json, dict):
                self._log_auth_response_preview("验证码发送响应摘要", response_json)
                self._follow_auth_continue_url(response_json, "注册验证码")
            return True

        except Exception as e:
            self._log(f"发送验证码失败: {e}", "error")
            return False

    def _get_verification_code(self) -> Optional[str]:
        """获取验证码"""
        try:
            timeout, poll_interval = self._get_email_code_wait_settings()
            self._log(
                f"正在等待邮箱 {self.email} 的验证码..."
                f"（超时 {timeout}s，轮询间隔 {poll_interval}s）"
            )

            email_id = self.email_info.get("service_id") if self.email_info else None
            code = self.email_service.get_verification_code(
                email=self.email,
                email_id=email_id,
                timeout=timeout,
                poll_interval=poll_interval,
                pattern=OTP_CODE_PATTERN,
                otp_sent_at=self._otp_sent_at,
            )

            if code:
                self._log(f"成功获取验证码: {code}")
                return code
            else:
                self._log("等待验证码超时", "error")
                return None

        except Exception as e:
            self._log(f"获取验证码失败: {e}", "error")
            return None

    def _validate_verification_code(self, code: str) -> bool:
        """验证验证码"""
        try:
            self._clear_otp_error_state()
            code_body = f'{{"code":"{code}"}}'
            did = self._resolve_current_device_id()
            headers = self._build_browser_like_auth_headers(
                referer="https://auth.openai.com/email-verification",
                did=did,
                content_type="application/json",
            )
            sentinel = self._build_auth_step_sentinel_header(
                flow="authorize_continue",
                referer="https://auth.openai.com/email-verification",
            )
            if sentinel:
                headers["openai-sentinel-token"] = sentinel

            response = self.session.post(
                OPENAI_API_ENDPOINTS["validate_otp"],
                headers=headers,
                data=code_body,
            )

            self._log(f"验证码校验状态: {response.status_code}")
            if response.status_code == 200:
                return True

            try:
                self._update_otp_error_state(response.json())
            except Exception:
                self._clear_otp_error_state()
            return False

        except Exception as e:
            self._log(f"验证验证码失败: {e}", "error")
            return False

    def _resend_email_verification_code(self) -> bool:
        """重新发送邮箱验证码（用于登录后二次验证）"""
        try:
            self._otp_sent_at = time.time()

            response = self.session.post(
                OPENAI_API_ENDPOINTS["resend_otp"],
                headers={
                    "referer": "https://auth.openai.com/email-verification",
                    "accept": "application/json",
                },
            )

            self._log(f"邮箱验证码重发状态: {response.status_code}")
            return response.status_code == 200
        except Exception as e:
            self._log(f"重发邮箱验证码失败: {e}", "error")
            return False

    def _validate_verification_code_with_payload(self, code: str) -> Optional[Dict[str, Any]]:
        """验证验证码并返回响应载荷"""
        try:
            self._clear_otp_error_state()
            did = self._resolve_current_device_id()
            headers = self._build_browser_like_auth_headers(
                referer="https://auth.openai.com/email-verification",
                did=did,
                content_type="application/json",
            )
            sentinel = self._build_auth_step_sentinel_header(
                flow="authorize_continue",
                referer="https://auth.openai.com/email-verification",
            )
            if sentinel:
                headers["openai-sentinel-token"] = sentinel
            response = self.session.post(
                OPENAI_API_ENDPOINTS["validate_otp"],
                headers=headers,
                data=json.dumps({"code": code}),
            )

            self._log(f"验证码校验状态: {response.status_code}")
            if response.status_code != 200:
                try:
                    response_json = response.json()
                except Exception:
                    response_json = None
                self._update_otp_error_state(response_json)
                self._log(f"验证码校验失败: {response.text[:300]}", "warning")
                return None

            response_json = response.json()
            self._clear_otp_error_state()
            self._log_auth_response_preview("验证码校验响应摘要", response_json)
            self._follow_auth_continue_url(response_json, "登录邮箱验证码")
            return response_json if isinstance(response_json, dict) else {}
        except Exception as e:
            self._log(f"验证验证码失败: {e}", "error")
            return None

    def _wait_for_signup_verification_code(self) -> Optional[str]:
        """等待注册阶段验证码，超时后自动重发一次"""
        code = self._get_verification_code()
        if code:
            return code

        self._log("首次等待验证码超时，尝试重发一次注册验证码...", "warning")
        if not self._send_verification_code():
            return None

        return self._get_verification_code()

    def _validate_signup_verification_code_with_retry(self, code: str) -> bool:
        """验证注册阶段验证码，遇到旧验证码时自动重取一次"""
        current_code = code
        for attempt in range(2):
            if self._validate_verification_code(current_code):
                return True

            if not should_retry_signup_otp_validation(
                is_wrong_email_otp_code_error=self._is_wrong_email_otp_code_error(),
                attempt=attempt,
                max_attempts=2,
            ):
                return False

            self._log("注册阶段拿到的验证码不是最新一封，重发后再获取一次...", "warning")
            if not self._send_verification_code():
                return False

            current_code = self._get_verification_code()
            if not current_code:
                return False

        return False

    def _create_user_account(self) -> bool:
        """创建用户账户"""
        try:
            user_info = generate_random_user_info()
            self._log(f"生成用户信息: {user_info['name']}, 生日: {user_info['birthdate']}")
            create_account_body = json.dumps(user_info)
            current_did = self._clean_text(getattr(self, "_current_device_id", ""))
            headers = self._build_browser_like_auth_headers(
                referer="https://auth.openai.com/about-you",
                did=current_did,
                content_type="application/json",
            )
            sentinel_payload = self._get_create_account_sentinel_payload()
            if not sentinel_payload and current_did:
                fallback_token = self._check_sentinel(current_did, flow="create_account")
                sentinel_payload = self._normalize_sentinel_payload(
                    fallback_token,
                    did=current_did,
                    flow="create_account",
                )
            sentinel = self._build_sentinel_header(
                sentinel_payload,
                did=current_did,
                flow="create_account",
            )
            if sentinel:
                headers["openai-sentinel-token"] = sentinel

            response = self.session.post(
                OPENAI_API_ENDPOINTS["create_account"],
                headers=headers,
                data=create_account_body,
            )

            self._log(f"账户创建状态: {response.status_code}")

            if response.status_code != 200:
                self._log(f"账户创建失败: {response.text[:200]}", "warning")
                return False

            try:
                response_json = response.json()
                if isinstance(response_json, dict):
                    self._post_create_page_type = str(
                        ((response_json.get("page") or {}).get("type") or "")
                    ).strip()
                    self._post_create_continue_url = str(
                        response_json.get("continue_url") or response_json.get("redirect_url") or ""
                    ).strip()
                    preview = {}
                    for key in (
                        "continue_url",
                        "redirect_url",
                        "callback_url",
                        "next_url",
                        "page",
                        "account",
                    ):
                        if key in response_json:
                            preview[key] = response_json.get(key)
                    if preview:
                        self._log(f"账户创建响应摘要: {json.dumps(preview, ensure_ascii=False)[:500]}")
            except Exception:
                pass

            return True

        except Exception as e:
            self._log(f"创建账户失败: {e}", "error")
            return False

    def _fetch_client_auth_session_dump(self) -> Optional[Dict[str, Any]]:
        """获取 client auth session dump，作为 cookie 缺失时的兜底"""
        try:
            response = self.session.get(
                OPENAI_API_ENDPOINTS["client_auth_session_dump"],
                headers={
                    "accept": "application/json",
                    "referer": "https://auth.openai.com/",
                },
                timeout=15,
            )

            if response.status_code != 200:
                self._log(
                    f"client_auth_session_dump 获取失败: HTTP {response.status_code}",
                    "warning",
                )
                return None

            response_json = response.json()
            auth_session = (response_json or {}).get("client_auth_session")
            if isinstance(auth_session, dict):
                return auth_session

            self._log("client_auth_session_dump 响应缺少 client_auth_session", "warning")
            return None
        except Exception as e:
            self._log(f"获取 client_auth_session_dump 失败: {e}", "warning")
            return None

    def _get_workspace_id(self) -> Optional[str]:
        """获取 Workspace ID"""
        try:
            cached_workspace_id = self._clean_text(getattr(self, "_cached_workspace_id", ""))
            if cached_workspace_id:
                self._log(f"Workspace ID (cached): {cached_workspace_id}")
                return cached_workspace_id

            auth_cookie = self.session.cookies.get("oai-client-auth-session")

            import base64
            import json as json_module

            try:
                inspected_keys = []
                if auth_cookie:
                    segments = auth_cookie.split(".")
                    if segments:
                        for idx, payload in enumerate(segments):
                            raw = self._clean_text(payload)
                            if not raw:
                                continue
                            pad = "=" * ((4 - (len(raw) % 4)) % 4)
                            try:
                                decoded = base64.urlsafe_b64decode((raw + pad).encode("ascii"))
                                auth_json = json_module.loads(decoded.decode("utf-8"))
                            except Exception:
                                continue

                            if isinstance(auth_json, dict):
                                inspected_keys.append(
                                    f"segment[{idx}]={','.join(sorted(auth_json.keys())[:8])}"
                                )
                                workspace_id = self._workspace_id_from_mapping(auth_json)
                                if workspace_id:
                                    self._log(f"Workspace ID: {workspace_id}")
                                    return workspace_id
                    else:
                        self._log("授权 Cookie 格式错误，尝试 client_auth_session_dump", "warning")
                else:
                    self._log("未能获取到授权 Cookie，尝试 client_auth_session_dump", "warning")

                if inspected_keys:
                    self._log(
                        "授权 Cookie 里没有可识别的 workspace 信息，已检查: "
                        + " | ".join(inspected_keys),
                        "warning",
                    )
                elif auth_cookie:
                    self._log("授权 Cookie 里没有可解析的 JSON segment", "warning")

                auth_session = self._fetch_client_auth_session_dump()
                workspace_id = self._workspace_id_from_mapping(auth_session)
                if workspace_id:
                    self._log(f"Workspace ID (client_auth_session_dump): {workspace_id}")
                    return workspace_id

                auth_target = self._clean_text(self._post_create_continue_url) or (
                    self.oauth_start.auth_url if self.oauth_start else ""
                )
                if auth_target and self.session:
                    try:
                        response = self.session.get(
                            auth_target,
                            timeout=20,
                        )
                        workspace_id = self._extract_workspace_id_from_response(
                            response=response,
                            html=str(getattr(response, "text", "") or ""),
                            url=str(getattr(response, "url", "") or "").strip(),
                        )
                        if workspace_id:
                            self._cached_workspace_id = workspace_id
                            self._log(f"Workspace ID (response): {workspace_id}")
                            return workspace_id
                    except Exception as e:
                        self._log(f"从授权页面提取 Workspace ID 失败: {e}", "warning")

                return None

            except Exception as e:
                self._log(f"解析授权 Cookie 失败: {e}", "error")
                return None

        except Exception as e:
            self._log(f"获取 Workspace ID 失败: {e}", "error")
            return None

    def _select_workspace(self, workspace_id: str) -> Optional[str]:
        """选择 Workspace"""
        try:
            select_body = f'{{"workspace_id":"{workspace_id}"}}'

            response = self.session.post(
                OPENAI_API_ENDPOINTS["select_workspace"],
                headers={
                    "referer": "https://auth.openai.com/sign-in-with-chatgpt/codex/consent",
                    "content-type": "application/json",
                },
                data=select_body,
            )

            if response.status_code != 200:
                self._log(f"选择 workspace 失败: {response.status_code}", "error")
                self._log(f"响应: {response.text[:200]}", "warning")
                return None

            continue_url = str((response.json() or {}).get("continue_url") or "").strip()
            if not continue_url:
                self._log("workspace/select 响应里缺少 continue_url", "error")
                return None

            self._log(f"Continue URL: {continue_url[:100]}...")
            return continue_url

        except Exception as e:
            self._log(f"选择 Workspace 失败: {e}", "error")
            return None

    def _follow_redirects(self, start_url: str) -> Optional[str]:
        """跟随重定向链，寻找回调 URL"""
        try:
            import re
            current_url = start_url
            max_redirects = 6

            for i in range(max_redirects):
                self._log(f"重定向 {i+1}/{max_redirects}: {current_url[:100]}...")

                response = self.session.get(
                    current_url,
                    allow_redirects=False,
                    timeout=15
                )

                location = response.headers.get("Location") or ""

                # 如果不是重定向状态码，停止
                if response.status_code not in [301, 302, 303, 307, 308]:
                    self._log(f"非重定向状态码: {response.status_code}")
                    body = response.text or ""
                    page_title = ""
                    title_match = re.search(r"<title>(.*?)</title>", body, re.IGNORECASE | re.DOTALL)
                    if title_match:
                        page_title = " ".join(title_match.group(1).split())[:120]
                    if page_title:
                        self._log(f"当前页面标题: {page_title}")

                    discovered = []
                    for pattern in (
                        r'login_challenge[^& <>"\']*',
                        r'consent_challenge[^& <>"\']*',
                        r'/api/accounts/[^ <>"\']+',
                        r'action="([^"]+)"',
                        r'href="([^"]+)"',
                    ):
                        for match in re.finditer(pattern, body):
                            value = match.group(1) if match.groups() else match.group(0)
                            value = str(value or "").strip()
                            if not value:
                                continue
                            if "code=" in value and "state=" in value:
                                self._log(f"在页面内容里找到回调 URL: {value[:160]}...")
                                return value
                            if (
                                "login" in value
                                or "consent" in value
                                or "authorize" in value
                                or "/api/accounts/" in value
                            ) and value not in discovered:
                                discovered.append(value)
                            if len(discovered) >= 8:
                                break
                        if len(discovered) >= 8:
                            break
                    if discovered:
                        self._log("页面线索: " + " | ".join(item[:160] for item in discovered), "warning")
                    snippet = " ".join(body[:500].split())
                    if snippet:
                        self._log(f"页面摘要: {snippet}", "warning")
                    break

                if not location:
                    self._log("重定向响应缺少 Location 头")
                    break

                # 构建下一个 URL
                import urllib.parse
                next_url = urllib.parse.urljoin(current_url, location)

                # 检查是否包含回调参数
                if "code=" in next_url and "state=" in next_url:
                    self._log(f"找到回调 URL: {next_url[:100]}...")
                    return next_url

                current_url = next_url

            self._log("未能在重定向链中找到回调 URL", "error")
            return None

        except Exception as e:
            self._log(f"跟随重定向失败: {e}", "error")
            return None

    def _extract_workspace_id_from_token(self, token: str) -> Optional[str]:
        """从 token 中提取 workspace / organization ID"""
        return extract_workspace_id_from_token(token)

    def _submit_login_identifier(self, did: Optional[str], sen_token: Optional[str]) -> Optional[Dict[str, Any]]:
        """提交登录邮箱，进入登录密码页或后续 OAuth 页面"""
        try:
            login_body = json.dumps({
                "username": {
                    "value": self.email,
                    "kind": "email",
                }
            })

            headers = {
                "referer": "https://auth.openai.com/log-in",
                "accept": "application/json",
                "content-type": "application/json",
            }

            if did and sen_token:
                sentinel = self._build_sentinel_header(
                    sen_token,
                    did=did,
                    flow="authorize_continue",
                )
                if sentinel:
                    headers["openai-sentinel-token"] = sentinel

            response = self.session.post(
                OPENAI_API_ENDPOINTS["login"],
                headers=headers,
                data=login_body,
            )

            self._log(f"登录邮箱提交状态: {response.status_code}")
            if response.status_code != 200:
                self._log(f"登录邮箱提交失败: {response.text[:300]}", "warning")
                return None

            response_json = response.json()
            self._log_auth_response_preview("登录邮箱响应摘要", response_json)
            self._follow_auth_continue_url(response_json, "登录邮箱")

            page = (response_json or {}).get("page")
            if not isinstance(page, dict):
                self._log("登录邮箱响应缺少 page", "warning")
                return None

            page_type = self._clean_text(page.get("type"))
            if page_type:
                self._log(f"登录邮箱响应页面类型: {page_type}")
            return page
        except Exception as e:
            self._log(f"提交登录邮箱失败: {e}", "error")
            return None

    def _verify_login_password(self, password: str) -> Optional[Dict[str, Any]]:
        """提交登录密码"""
        try:
            response = self.session.post(
                OPENAI_API_ENDPOINTS["verify_password"],
                headers={
                    "referer": "https://auth.openai.com/log-in/password",
                    "accept": "application/json",
                    "content-type": "application/json",
                },
                data=json.dumps({"password": password}),
            )

            self._log(f"登录密码提交状态: {response.status_code}")
            if response.status_code != 200:
                self._log(f"登录密码提交失败: {response.text[:300]}", "warning")
                return None

            response_json = response.json()
            self._log_auth_response_preview("登录密码响应摘要", response_json)
            self._follow_auth_continue_url(response_json, "登录密码")

            page = (response_json or {}).get("page")
            if not isinstance(page, dict):
                self._log("登录密码响应缺少 page", "warning")
                return None

            page_type = self._clean_text(page.get("type"))
            if page_type:
                self._log(f"登录密码响应页面类型: {page_type}")
            return page
        except Exception as e:
            self._log(f"提交登录密码失败: {e}", "error")
            return None

    def _follow_auth_continue_url(self, payload: Any, stage: str):
        """最佳努力跟进认证接口返回的 continue_url，推进当前会话状态。"""
        try:
            continue_url = self._extract_auth_continue_url(payload)
            if not continue_url or not self.session:
                return

            self._log(f"{stage} 跟进 continue_url: {continue_url[:120]}...", "warning")
            response = self.session.get(
                continue_url,
                timeout=15,
            )
            self._log(f"{stage} continue_url 响应: {self._auth_response_debug_summary(response)}", "warning")
            workspace_id = self._extract_workspace_id_from_response(
                response=response,
                html=str(getattr(response, "text", "") or ""),
                url=str(getattr(response, "url", "") or "").strip(),
            )
            if workspace_id:
                self._cached_workspace_id = workspace_id
                self._log(f"{stage} 提取到 Workspace ID: {workspace_id}", "warning")
        except Exception as e:
            self._log(f"{stage} 跟进 continue_url 失败: {e}", "warning")

    def _advance_workspace_authorization(
        self,
        auth_target: str,
        _visited: Optional[set[str]] = None,
    ) -> Optional[str]:
        """主动请求授权页，若命中 consent/workspace 则完成 workspace 选择并继续拿 callback。"""
        try:
            return self._get_flow_runner().advance_workspace_authorization(
                auth_target,
                _visited=_visited,
            )
        except Exception as e:
            self._log(f"推进 Workspace 授权失败: {e}", "warning")
            return None

    def _build_callback_url_from_page(self, page: Dict[str, Any]) -> Optional[str]:
        """从 token_exchange 页面构造 OAuth 回调 URL"""
        callback_url = build_callback_url_from_page(page)
        if callback_url:
            self._log(f"从 token_exchange 页面构造回调 URL: {callback_url[:100]}...")
            return callback_url

        if self._clean_text((page or {}).get("type")) == "token_exchange":
            self._log("token_exchange 页面缺少 code/state", "error")
        return None

    def _resolve_callback_from_continue_url(self, continue_url: str, stage: str) -> Optional[str]:
        """根据 continue_url 推进到 OAuth 回调"""
        try:
            return self._get_flow_runner().resolve_callback_from_continue_url(continue_url, stage)
        except Exception as e:
            self._log(f"{stage} 解析 continue_url 失败: {e}", "error")
            return None

    def _resolve_callback_from_auth_response(
        self,
        payload: Dict[str, Any],
        stage: str,
    ) -> AuthResolutionResult:
        """根据认证接口响应推进到 OAuth 回调"""
        flow_result = self._get_flow_runner().resolve_auth_result(payload, stage)
        return AuthResolutionResult(
            callback_url=flow_result.callback_url,
            page_type=flow_result.page_type,
            continue_url=flow_result.continue_url,
        )

    def _complete_login_email_otp_verification(self) -> AuthResolutionResult:
        """完成登录后的邮箱验证码验证，并继续推进 OAuth"""
        flow_result = self._get_flow_runner().complete_login_email_otp_verification()
        return AuthResolutionResult(
            callback_url=flow_result.callback_url,
            page_type=flow_result.page_type,
            continue_url=flow_result.continue_url,
        )

    def _resolve_callback_from_auth_page(self, page: Dict[str, Any], stage: str) -> Optional[str]:
        """根据页面类型推进到 OAuth 回调"""
        return self._get_flow_runner().resolve_callback_from_auth_page(page, stage)

    def _restart_oauth_session_for_login(self) -> Tuple[Optional[str], Optional[str]]:
        """新开 OAuth 会话，按已存在账号重新登录"""
        try:
            self.http_client.close()
            self.http_client = OpenAIHTTPClient(proxy_url=self.proxy_url)
            self.session = self.http_client.session

            if not self._start_oauth():
                return None, None

            did = self._get_device_id()
            if not did:
                return None, None

            sen_token = self._check_sentinel(did)
            self._current_device_id = self._clean_text(did)
            self._current_sentinel_token = self._clean_text(sen_token)
            if sen_token:
                self._log("新会话 Sentinel 检查通过")
            else:
                self._log("新会话 Sentinel 检查失败或未启用", "warning")

            return did, sen_token
        except Exception as e:
            self._log(f"重建 OAuth 会话失败: {e}", "error")
            return None, None

    def _attempt_add_phone_login_bypass(self, did: Optional[str], sen_token: Optional[str]) -> Optional[str]:
        """注册后若进入 add_phone，尝试改走登录流继续完成 OAuth"""
        return self._get_flow_runner().attempt_add_phone_login_bypass(did, sen_token)

    def _handle_oauth_callback(self, callback_url: str) -> Optional[Dict[str, Any]]:
        """处理 OAuth 回调"""
        try:
            if not self.oauth_start:
                self._log("OAuth 流程未初始化", "error")
                return None

            token_info = self.cpa_runtime.handle_oauth_callback(callback_url)

            self._log("OAuth 授权成功")
            return token_info

        except Exception as e:
            self._log(f"处理 OAuth 回调失败: {e}", "error")
            return None

    def _post_registration_health_check(self, access_token: str) -> Tuple[str, bool, str]:
        """注册完成后的账号健康检查，仅用于识别是否可用。"""
        try:
            token = self._clean_text(access_token)
            if not token:
                return "failed", False, "缺少 access_token"

            session = cffi_requests.Session(
                impersonate="chrome120",
                proxy=self.proxy_url,
            )
            response = session.get(
                "https://chatgpt.com/backend-api/me",
                headers={
                    "authorization": f"Bearer {token}",
                    "accept": "application/json",
                },
                timeout=30,
            )

            if response.status_code == 200:
                return "active", True, ""
            if response.status_code == 403:
                return "banned", False, "账号健康检查返回 403，疑似已受限或被封禁"
            if response.status_code == 401:
                return "failed", False, "账号健康检查返回 401，Token 无效或已失效"
            return "failed", False, f"账号健康检查失败: HTTP {response.status_code}"
        except Exception as e:
            return "failed", False, f"账号健康检查异常: {e}"

    def run(self) -> RegistrationResult:
        """
        执行完整的注册流程

        支持已注册账号自动登录：
        - 如果检测到邮箱已注册，自动切换到登录流程
        - 已注册账号跳过：设置密码、发送验证码、创建用户账户
        - 共用步骤：获取验证码、验证验证码、Workspace 和 OAuth 回调

        Returns:
            RegistrationResult: 注册结果
        """
        result = RegistrationResult(success=False, logs=self.logs)

        try:
            callback_url: Optional[str] = None

            self._log("=" * 60)
            self._log("开始注册流程")
            self._log("=" * 60)

            # 1. 检查 IP 地理位置
            self._log("1. 检查 IP 地理位置...")
            ip_ok, location = self._check_ip_location()
            if not ip_ok:
                result.error_message = f"IP 地理位置不支持: {location}"
                self._log(f"IP 检查失败: {location}", "error")
                return result

            self._log(f"IP 位置: {location}")

            # 2. 创建邮箱
            self._log("2. 创建邮箱...")
            if not self._create_email():
                result.error_message = "创建邮箱失败"
                return result

            result.email = self.email

            # 3. 初始化会话
            self._log("3. 初始化会话...")
            if not self._init_session():
                result.error_message = "初始化会话失败"
                return result

            # 4. 开始 OAuth 流程
            self._log("4. 开始 OAuth 授权流程...")
            if not self._start_oauth():
                result.error_message = "开始 OAuth 流程失败"
                return result

            # 5. 获取 Device ID
            self._log("5. 获取 Device ID...")
            did = self._get_device_id()
            if not did:
                result.error_message = "获取 Device ID 失败"
                return result
            self._current_device_id = self._clean_text(did)

            # 6. 检查 Sentinel 拦截
            self._log("6. 检查 Sentinel 拦截...")
            sen_token = self._check_sentinel(did)
            self._current_sentinel_token = self._clean_text(sen_token)
            if sen_token:
                self._log("Sentinel 检查通过")
            else:
                self._log("Sentinel 检查失败或未启用", "warning")

            signup_sequence = self.cpa_runtime.execute_signup_sequence(did, sen_token)
            if not signup_sequence.success:
                result.error_message = signup_sequence.error_message
                return result
            redirect_result = self.cpa_runtime.resolve_post_registration_callback(did, sen_token)
            callback_url = redirect_result.callback_url
            if redirect_result.workspace_id:
                result.workspace_id = redirect_result.workspace_id
            if redirect_result.metadata:
                result.metadata = redirect_result.metadata
            if redirect_result.error_message:
                result.error_message = redirect_result.error_message
                return result

            # 16. 处理 OAuth 回调
            self._log("16. 处理 OAuth 回调...")
            token_info = self._handle_oauth_callback(callback_url)
            if not token_info:
                result.error_message = "处理 OAuth 回调失败"
                return result

            # 提取账户信息
            result.account_id = token_info.get("account_id", "")
            result.access_token = token_info.get("access_token", "")
            result.refresh_token = token_info.get("refresh_token", "")
            result.id_token = token_info.get("id_token", "")
            result.password = self.password or ""  # 保存密码（已注册账号为空）
            result.cookies = self._serialize_session_cookies()

            if not result.workspace_id:
                result.workspace_id = (
                    self._extract_workspace_id_from_token(result.id_token)
                    or self._extract_workspace_id_from_token(result.access_token)
                    or ""
                )
                if result.workspace_id:
                    self._log(f"从 Token 中回填 Workspace ID: {result.workspace_id}")

            # 设置来源标记
            result.source = "login" if self._is_existing_account else "register"

            # 尝试获取 session_token 从 cookie
            session_cookie = self._extract_session_token_from_cookies()
            if session_cookie:
                self.session_token = session_cookie
                result.session_token = session_cookie
                self._log(f"获取到 Session Token")
            if result.cookies:
                self._log(f"获取到 Cookies，长度: {len(result.cookies)}")

            # 17. 健康检查：区分“注册完成”和“账号可用”
            self._log("17. 执行账号健康检查...")
            account_status, is_usable, health_error = self._post_registration_health_check(
                result.access_token
            )
            result.account_status = account_status
            result.is_usable = is_usable
            if not is_usable:
                result.error_message = health_error
                self._log(f"账号健康检查失败: {health_error}", "warning")
            else:
                self._log("账号健康检查通过")

            # 18. 完成
            self._log("=" * 60)
            if not result.is_usable:
                self._log("注册链路完成，但账号当前不可用")
            elif self._is_existing_account:
                self._log("登录成功! (已注册账号)")
            else:
                self._log("注册成功!")
            self._log(f"邮箱: {result.email}")
            self._log(f"Account ID: {result.account_id}")
            self._log(f"Workspace ID: {result.workspace_id}")
            self._log("=" * 60)

            result.success = True
            result.metadata = {
                "email_service": self.email_service.service_type.value,
                "proxy_used": self.proxy_url,
                "registered_at": datetime.now().isoformat(),
                "is_existing_account": self._is_existing_account,
                "health_check": {
                    "checked": True,
                    "is_usable": result.is_usable,
                    "account_status": result.account_status,
                    "error": health_error if not result.is_usable else "",
                },
            }

            return result

        except Exception as e:
            self._log(f"注册过程中发生未预期错误: {e}", "error")
            result.error_message = str(e)
            return result

    def save_to_database(self, result: RegistrationResult) -> bool:
        """
        保存注册结果到数据库

        Args:
            result: 注册结果

        Returns:
            是否保存成功
        """
        if not result.success:
            return False

        try:
            # 获取默认 client_id
            settings = get_settings()

            with get_db() as db:
                # 保存账户信息
                account = crud.create_account(
                    db,
                    email=result.email,
                    password=result.password,
                    client_id=settings.openai_client_id,
                    session_token=result.session_token,
                    cookies=result.cookies,
                    email_service=self.email_service.service_type.value,
                    email_service_id=self.email_info.get("service_id") if self.email_info else None,
                    account_id=result.account_id,
                    workspace_id=result.workspace_id,
                    access_token=result.access_token,
                    refresh_token=result.refresh_token,
                    id_token=result.id_token,
                    proxy_used=self.proxy_url,
                    extra_data=result.metadata,
                    status=result.account_status,
                    source=result.source
                )

                self._log(f"账户已保存到数据库，ID: {account.id}")
                return True

        except Exception as e:
            self._log(f"保存到数据库失败: {e}", "error")
            return False
