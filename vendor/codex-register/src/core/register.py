"""
注册流程引擎
从 main.py 中提取并重构的注册流程
"""

import re
import json
import time
import logging
import secrets
import string
import urllib.parse
from typing import Optional, Dict, Any, Tuple, Callable
from dataclasses import dataclass
from datetime import datetime

from curl_cffi import requests as cffi_requests

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
        if value is None:
            return ""
        return str(value).strip()

    def _extract_auth_page_type(self, payload: Any) -> str:
        """从认证响应或 page 对象中提取页面类型"""
        if not isinstance(payload, dict):
            return ""

        page = payload.get("page")
        if isinstance(page, dict):
            page_type = self._clean_text(page.get("type"))
            if page_type:
                return page_type

        return self._clean_text(payload.get("type"))

    def _extract_auth_continue_url(self, payload: Any) -> str:
        """从认证响应中提取 continue_url"""
        if not isinstance(payload, dict):
            return ""

        return self._clean_text(
            payload.get("continue_url")
            or payload.get("redirect_url")
            or payload.get("callback_url")
            or payload.get("next_url")
        )

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
            candidate = self._clean_text(data.get(key))
            if candidate:
                return candidate

        for nested_key in nested_keys:
            nested = data.get(nested_key)
            if isinstance(nested, dict):
                for key in item_keys:
                    candidate = self._clean_text(nested.get(key))
                    if candidate:
                        return candidate

        for list_key in ("workspaces", "organizations"):
            items = data.get(list_key)
            if not isinstance(items, list):
                continue

            ordered_items = []
            default_item = next(
                (
                    item for item in items
                    if isinstance(item, dict) and item.get("is_default") is True
                ),
                None,
            )
            if default_item is not None:
                ordered_items.append(default_item)
            ordered_items.extend(
                item for item in items
                if isinstance(item, dict) and item is not default_item
            )

            for item in ordered_items:
                for key in item_keys:
                    candidate = self._clean_text(item.get(key))
                    if candidate:
                        return candidate

        for nested_key in ("auth", "https://api.openai.com/auth"):
            nested = data.get(nested_key)
            candidate = self._workspace_id_from_mapping(nested)
            if candidate:
                return candidate

        return ""

    def _extract_workspace_id_from_text(self, text: str) -> Optional[str]:
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
                workspace_id = str(match.group(1) or "").strip()
                if workspace_id:
                    return workspace_id
        return None

    def _extract_workspace_id_from_url(self, url: str) -> Optional[str]:
        """从 URL 查询参数或片段中提取 Workspace ID。"""
        if not url:
            return None

        import urllib.parse

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
                    workspace_id = str(values[0] or "").strip()
                    if workspace_id:
                        return workspace_id
        return None

    def _extract_workspace_id_from_response_payload(self, payload: Any, depth: int = 0) -> Optional[str]:
        """递归扫描响应载荷中的 Workspace ID。"""
        if payload is None or depth > 5:
            return None

        if isinstance(payload, dict):
            workspace_id = self._workspace_id_from_mapping(payload)
            if workspace_id:
                return workspace_id
            for value in payload.values():
                workspace_id = self._extract_workspace_id_from_response_payload(value, depth + 1)
                if workspace_id:
                    return workspace_id
            return None

        if isinstance(payload, list):
            for item in payload:
                workspace_id = self._extract_workspace_id_from_response_payload(item, depth + 1)
                if workspace_id:
                    return workspace_id

        return None

    def _extract_workspace_id_from_response(
        self,
        response: Optional[Any] = None,
        html: Optional[str] = None,
        url: Optional[str] = None,
    ) -> Optional[str]:
        """统一从响应 JSON、HTML、脚本内容和 URL 中提取 Workspace ID。"""
        response_url = str(getattr(response, "url", "") or "").strip()
        response_text = html if html is not None else str(getattr(response, "text", "") or "")
        candidate_url = url or response_url

        if response is not None:
            try:
                payload = response.json()
            except Exception:
                payload = None
            workspace_id = self._extract_workspace_id_from_response_payload(payload)
            if workspace_id:
                return workspace_id

        for extractor in (
            lambda: self._extract_workspace_id_from_text(response_text),
            lambda: self._extract_workspace_id_from_url(candidate_url),
        ):
            workspace_id = extractor()
            if workspace_id:
                return workspace_id

        return None

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
        special_chars = "!@#$%^&*()-_=+"
        base_pool = "".join(dict.fromkeys(PASSWORD_CHARSET + special_chars))

        password_chars = [
            secrets.choice(lowercase_chars),
            secrets.choice(uppercase_chars),
            secrets.choice(digit_chars),
            secrets.choice(special_chars),
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
                did = self.session.cookies.get("oai-did")

                if did:
                    self._log(f"Device ID: {did}")
                return did

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

    def _serialize_session_cookies(self) -> str:
        """序列化当前会话 cookie，供支付接口复用"""
        return "; ".join(f"{name}={value}" for name, value in self._session_cookie_items())

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

    def _check_sentinel(self, did: str) -> Optional[str]:
        """检查 Sentinel 拦截"""
        try:
            sen_req_body = f'{{"p":"","id":"{did}","flow":"authorize_continue"}}'

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
                self._log(f"Sentinel token 获取成功")
                return sen_token
            else:
                self._log(f"Sentinel 检查失败: {response.status_code}", "warning")
                return None

        except Exception as e:
            self._log(f"Sentinel 检查异常: {e}", "warning")
            return None

    def _submit_signup_form(self, did: str, sen_token: Optional[str]) -> SignupFormResult:
        """
        提交注册表单

        Returns:
            SignupFormResult: 提交结果，包含账号状态判断
        """
        try:
            signup_body = f'{{"username":{{"value":"{self.email}","kind":"email"}},"screen_hint":"signup"}}'

            headers = {
                "referer": "https://auth.openai.com/create-account",
                "accept": "application/json",
                "content-type": "application/json",
            }

            if sen_token:
                sentinel = f'{{"p": "", "t": "", "c": "{sen_token}", "id": "{did}", "flow": "authorize_continue"}}'
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

                # 判断是否为已注册账号
                is_existing = page_type == OPENAI_PAGE_TYPES["EMAIL_OTP_VERIFICATION"]

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

            response = self.session.post(
                OPENAI_API_ENDPOINTS["register"],
                headers={
                    "referer": "https://auth.openai.com/create-account/password",
                    "accept": "application/json",
                    "content-type": "application/json",
                },
                data=register_body,
            )

            self._log(f"提交密码状态: {response.status_code}")

            if response.status_code != 200:
                error_text = response.text[:500]
                self._log(f"密码注册失败: {error_text}", "warning")

                # 解析错误信息，判断是否是邮箱已注册
                try:
                    error_json = response.json()
                    error_msg = error_json.get("error", {}).get("message", "")
                    error_code = error_json.get("error", {}).get("code", "")

                    # 检测邮箱已注册的情况
                    if "already" in error_msg.lower() or "exists" in error_msg.lower() or error_code == "user_exists":
                        self._log(f"邮箱 {self.email} 可能已在 OpenAI 注册过", "error")
                        # 标记此邮箱为已注册状态
                        self._mark_email_as_registered()
                except Exception:
                    pass

                return False, None

            return True, password

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

            response = self.session.get(
                OPENAI_API_ENDPOINTS["send_otp"],
                headers={
                    "referer": "https://auth.openai.com/create-account/password",
                    "accept": "application/json",
                },
            )

            self._log(f"验证码发送状态: {response.status_code}")
            return response.status_code == 200

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

            response = self.session.post(
                OPENAI_API_ENDPOINTS["validate_otp"],
                headers={
                    "referer": "https://auth.openai.com/email-verification",
                    "accept": "application/json",
                    "content-type": "application/json",
                },
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
            response = self.session.post(
                OPENAI_API_ENDPOINTS["validate_otp"],
                headers={
                    "referer": "https://auth.openai.com/email-verification",
                    "accept": "application/json",
                    "content-type": "application/json",
                },
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

            if not self._is_wrong_email_otp_code_error() or attempt >= 1:
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

            response = self.session.post(
                OPENAI_API_ENDPOINTS["create_account"],
                headers={
                    "referer": "https://auth.openai.com/about-you",
                    "accept": "application/json",
                    "content-type": "application/json",
                },
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
        try:
            import base64
            import json as json_module

            raw = str(token or "").strip()
            if not raw:
                return None

            parts = raw.split(".")
            if len(parts) < 2:
                return None

            payload = parts[1]
            pad = "=" * ((4 - (len(payload) % 4)) % 4)
            decoded = base64.urlsafe_b64decode((payload + pad).encode("ascii"))
            data = json_module.loads(decoded.decode("utf-8"))

            def _find(mapping: Any) -> str:
                if not isinstance(mapping, dict):
                    return ""

                for key in (
                    "workspace_id",
                    "organization_id",
                    "org_id",
                    "chatgpt_account_id",
                ):
                    value = self._clean_text(mapping.get(key))
                    if value:
                        return value

                organizations = mapping.get("organizations")
                if isinstance(organizations, list):
                    default_org = next(
                        (
                            item for item in organizations
                            if isinstance(item, dict) and item.get("is_default") is True
                        ),
                        None,
                    )
                    for item in ([default_org] if default_org else []) + [
                        item for item in organizations if isinstance(item, dict) and item is not default_org
                    ]:
                        value = self._clean_text(item.get("id"))
                        if value:
                            return value

                for nested_key in ("auth", "https://api.openai.com/auth"):
                    nested = mapping.get(nested_key)
                    value = _find(nested)
                    if value:
                        return value

                return ""

            workspace_id = _find(data)
            return workspace_id or None
        except Exception:
            return None

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
                sentinel = json.dumps({
                    "p": "",
                    "t": "",
                    "c": sen_token,
                    "id": did,
                    "flow": "authorize_continue",
                })
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

    def _discover_auth_navigation_urls(
        self,
        response: Optional[Any],
        fallback_url: str,
    ) -> list[str]:
        """从响应历史、Location 头和 HTML 中提取可能的授权下一跳 URL。"""
        import urllib.parse

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

    def _advance_workspace_authorization(
        self,
        auth_target: str,
        _visited: Optional[set[str]] = None,
    ) -> Optional[str]:
        """主动请求授权页，若命中 consent/workspace 则完成 workspace 选择并继续拿 callback。"""
        try:
            target_url = self._clean_text(auth_target)
            if not target_url or not self.session:
                return None

            visited = _visited or set()
            if target_url in visited:
                return None
            visited.add(target_url)

            self._log(f"尝试推进 Workspace 授权: {target_url[:120]}...", "warning")
            response = self.session.get(
                target_url,
                timeout=20,
            )

            current_url = self._clean_text(getattr(response, "url", ""))
            html = str(getattr(response, "text", "") or "")

            if "code=" in current_url and "state=" in current_url:
                self._log(f"Workspace 授权页直接命中回调 URL: {current_url[:100]}...", "warning")
                return current_url

            workspace_id = self._extract_workspace_id_from_response(
                response=response,
                html=html,
                url=current_url,
            )
            if workspace_id:
                self._cached_workspace_id = workspace_id
                self._log(f"Workspace 授权页提取到 Workspace ID: {workspace_id}", "warning")

            consent_markers = (
                "sign-in-with-chatgpt/codex/consent" in current_url
                or 'action="/sign-in-with-chatgpt/codex/consent"' in html
                or "/workspace" in current_url
                or "/organization" in current_url
            )

            if consent_markers and workspace_id:
                continue_url = self._select_workspace(workspace_id)
                if not continue_url:
                    return None
                return self._follow_redirects(continue_url)

            for next_url in self._discover_auth_navigation_urls(response, target_url):
                if next_url == target_url:
                    continue
                callback_url = self._advance_workspace_authorization(
                    next_url,
                    _visited=visited,
                )
                if callback_url:
                    return callback_url

            return None
        except Exception as e:
            self._log(f"推进 Workspace 授权失败: {e}", "warning")
            return None

    def _build_callback_url_from_page(self, page: Dict[str, Any]) -> Optional[str]:
        """从 token_exchange 页面构造 OAuth 回调 URL"""
        try:
            if self._clean_text(page.get("type")) != "token_exchange":
                return None

            continue_url = self._clean_text(page.get("continue_url"))
            payload = page.get("payload") if isinstance(page.get("payload"), dict) else {}

            if not continue_url:
                self._log("token_exchange 页面缺少 continue_url", "error")
                return None

            if "code=" in continue_url and "state=" in continue_url:
                self._log(f"token_exchange 直接返回回调 URL: {continue_url[:100]}...")
                return continue_url

            import urllib.parse

            parsed = urllib.parse.urlsplit(continue_url)
            query = dict(urllib.parse.parse_qsl(parsed.query, keep_blank_values=True))
            for key in ("code", "state", "error", "error_description"):
                value = self._clean_text(payload.get(key))
                if value:
                    query[key] = value

            callback_url = urllib.parse.urlunsplit((
                parsed.scheme,
                parsed.netloc,
                parsed.path,
                urllib.parse.urlencode(query),
                parsed.fragment,
            ))

            if "code=" in callback_url and "state=" in callback_url:
                self._log(f"从 token_exchange 页面构造回调 URL: {callback_url[:100]}...")
                return callback_url

            self._log("token_exchange 页面缺少 code/state", "error")
            return None
        except Exception as e:
            self._log(f"构造 token_exchange 回调 URL 失败: {e}", "error")
            return None

    def _resolve_callback_from_continue_url(self, continue_url: str, stage: str) -> Optional[str]:
        """根据 continue_url 推进到 OAuth 回调"""
        try:
            target_url = self._clean_text(continue_url)
            if not target_url:
                return None

            if "code=" in target_url and "state=" in target_url:
                self._log(f"{stage} 直接拿到回调 URL: {target_url[:100]}...")
                return target_url

            import urllib.parse

            parsed = urllib.parse.urlsplit(target_url)
            path = self._clean_text(parsed.path)

            if path in (
                "/workspace",
                "/sign-in-with-chatgpt/codex/consent",
                "/sign-in-with-chatgpt/codex/organization",
            ):
                workspace_id = self._get_workspace_id()
                if not workspace_id:
                    self._log(f"{stage} 需要 workspace，但当前还没拿到 workspace_id", "warning")
                    return None

                continue_after_select = self._select_workspace(workspace_id)
                if not continue_after_select:
                    return None
                return self._follow_redirects(continue_after_select)

            if path == "/add-phone":
                callback_url = self._advance_workspace_authorization(target_url)
                if callback_url:
                    return callback_url
                self._log(f"{stage} 仍然指向 add_phone", "warning")
                return None

            return self._follow_redirects(target_url)
        except Exception as e:
            self._log(f"{stage} 解析 continue_url 失败: {e}", "error")
            return None

    def _resolve_callback_from_auth_response(
        self,
        payload: Dict[str, Any],
        stage: str,
    ) -> AuthResolutionResult:
        """根据认证接口响应推进到 OAuth 回调"""
        result = AuthResolutionResult()
        if not isinstance(payload, dict):
            return result

        result.page_type = self._extract_auth_page_type(payload)
        result.continue_url = self._extract_auth_continue_url(payload)

        page = payload.get("page")
        if isinstance(page, dict):
            callback_url = self._resolve_callback_from_auth_page(page, stage)
            if callback_url:
                result.callback_url = callback_url
                return result
            if result.page_type == "add_phone":
                return result

        if result.continue_url:
            result.callback_url = self._resolve_callback_from_continue_url(
                result.continue_url,
                stage,
            )

        return result

    def _complete_login_email_otp_verification(self) -> AuthResolutionResult:
        """完成登录后的邮箱验证码验证，并继续推进 OAuth"""
        self._last_login_recovery_page_type = ""
        self._log("登录后触发邮箱二次验证，开始获取验证码...", "warning")
        self._otp_sent_at = time.time()

        max_attempts = 3
        code: Optional[str] = None
        validate_payload: Optional[Dict[str, Any]] = None

        for attempt in range(max_attempts):
            if attempt == 0:
                code = self._get_verification_code()
                if code:
                    validate_payload = self._validate_verification_code_with_payload(code)
            else:
                reason = "首次等待登录验证码超时" if not code else "登录验证码不是最新一封或已失效"
                self._log(f"{reason}，尝试重发后重新获取...", "warning")
                if not self._resend_email_verification_code():
                    return AuthResolutionResult()
                code = self._get_verification_code()
                if not code:
                    continue
                validate_payload = self._validate_verification_code_with_payload(code)

            if validate_payload:
                break

            if not self._is_wrong_email_otp_code_error() and code:
                return AuthResolutionResult()
        else:
            return AuthResolutionResult()

        resolution = self._resolve_callback_from_auth_response(
            validate_payload,
            "登录邮箱验证码校验",
        )
        self._last_login_recovery_page_type = resolution.page_type
        if resolution.callback_url:
            return resolution

        if resolution.page_type == "add_phone":
            return resolution

        if self.oauth_start:
            self._log("登录邮箱验证码校验后未拿到 continue_url，回退到 OAuth URL 重试跳转...", "warning")
            retry_url = self._build_authenticated_oauth_url()
            if retry_url:
                resolution.callback_url = self._follow_redirects(retry_url)

        return resolution

    def _resolve_callback_from_auth_page(self, page: Dict[str, Any], stage: str) -> Optional[str]:
        """根据页面类型推进到 OAuth 回调"""
        page_type = self._clean_text((page or {}).get("type"))
        if not page_type:
            self._log(f"{stage} 缺少 page.type", "warning")
            return None

        if page_type == "token_exchange":
            return self._build_callback_url_from_page(page)

        if page_type in (
            "workspace",
            "sign_in_with_chatgpt_codex_consent",
            "sign_in_with_chatgpt_codex_org",
        ):
            workspace_id = self._get_workspace_id()
            if not workspace_id:
                self._log(f"{stage} 需要选择 Workspace，但暂未拿到 workspace_id", "warning")
                return None

            continue_url = self._select_workspace(workspace_id)
            if not continue_url:
                return None
            return self._follow_redirects(continue_url)

        if page_type == "external_url":
            payload = page.get("payload") if isinstance(page.get("payload"), dict) else {}
            external_url = self._clean_text(payload.get("url"))
            if external_url:
                self._log(f"{stage} 返回 external_url，继续跟随跳转...", "warning")
                return self._follow_redirects(external_url)
            return None

        if page_type == "add_phone":
            return None

        return None

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
        if not self.email or not self.password:
            self._log("缺少邮箱或密码，无法执行 add_phone 登录回退", "error")
            return None

        attempts = [
            ("当前会话", did, sen_token, False),
            ("新 OAuth 会话", None, None, True),
        ]

        for attempt_name, current_did, current_sentinel, recreate_session in attempts:
            self._last_login_recovery_page_type = ""
            if recreate_session:
                self._log(f"add_phone 回退：切换到{attempt_name}重试登录链路...", "warning")
                current_did, current_sentinel = self._restart_oauth_session_for_login()
                if not current_did:
                    continue
            else:
                self._log("检测到 add_phone，尝试改走登录链路继续 OAuth...", "warning")

            login_page = self._submit_login_identifier(current_did, current_sentinel)
            if not login_page:
                continue

            callback_url = self._resolve_callback_from_auth_page(login_page, f"{attempt_name}登录邮箱")
            if callback_url:
                return callback_url

            page_type = self._clean_text(login_page.get("type"))
            if page_type == "login_password":
                password_page = self._verify_login_password(self.password)
                if not password_page:
                    continue

                callback_url = self._resolve_callback_from_auth_page(password_page, f"{attempt_name}登录密码")
                if callback_url:
                    return callback_url

                page_type = self._clean_text(password_page.get("type"))
                if page_type == "email_otp_verification":
                    otp_resolution = self._complete_login_email_otp_verification()
                    if otp_resolution.callback_url:
                        return otp_resolution.callback_url
                    page_type = (
                        otp_resolution.page_type
                        or self._last_login_recovery_page_type
                        or page_type
                    )

            if page_type == "add_phone":
                auth_target = (
                    otp_resolution.continue_url
                    or self._post_create_continue_url
                    or (self.oauth_start.auth_url if self.oauth_start else "")
                )
                callback_url = self._advance_workspace_authorization(auth_target)
                if callback_url:
                    return callback_url
                if self.oauth_start:
                    self._log(f"{attempt_name} 命中 add_phone，先复用当前已登录会话重新跟随 OAuth URL...", "warning")
                    retry_url = self._build_authenticated_oauth_url()
                    if retry_url:
                        callback_url = self._follow_redirects(retry_url)
                    if callback_url:
                        return callback_url
                if recreate_session:
                    self._log("新 OAuth 会话登录后仍停留在 add_phone", "warning")
                else:
                    self._log("当前会话登录后仍停留在 add_phone，切换新 OAuth 会话重试", "warning")
                continue

            if self.oauth_start:
                self._log(f"{attempt_name} 未直接拿到回调，尝试重新跟随 OAuth URL...", "warning")
                retry_url = self._build_authenticated_oauth_url()
                if not retry_url:
                    continue
                callback_url = self._follow_redirects(retry_url)
                if callback_url:
                    return callback_url

        return None

    def _handle_oauth_callback(self, callback_url: str) -> Optional[Dict[str, Any]]:
        """处理 OAuth 回调"""
        try:
            if not self.oauth_start:
                self._log("OAuth 流程未初始化", "error")
                return None

            token_info = self.oauth_manager.handle_callback(
                callback_url=callback_url,
                expected_state=self.oauth_start.state,
                code_verifier=self.oauth_start.code_verifier
            )

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

            # 6. 检查 Sentinel 拦截
            self._log("6. 检查 Sentinel 拦截...")
            sen_token = self._check_sentinel(did)
            if sen_token:
                self._log("Sentinel 检查通过")
            else:
                self._log("Sentinel 检查失败或未启用", "warning")

            # 7. 提交注册表单 + 解析响应判断账号状态
            self._log("7. 提交注册表单...")
            signup_result = self._submit_signup_form(did, sen_token)
            if not signup_result.success:
                result.error_message = f"提交注册表单失败: {signup_result.error_message}"
                return result

            # 8. [已注册账号跳过] 注册密码
            if self._is_existing_account:
                self._log("8. [已注册账号] 跳过密码设置，OTP 已自动发送")
            else:
                self._log("8. 注册密码...")
                password_ok, password = self._register_password()
                if not password_ok:
                    result.error_message = "注册密码失败"
                    return result

            # 9. [已注册账号跳过] 发送验证码
            if self._is_existing_account:
                self._log("9. [已注册账号] 跳过发送验证码，使用自动发送的 OTP")
                # 已注册账号的 OTP 在提交表单时已自动发送，记录时间戳
                self._otp_sent_at = time.time()
            else:
                self._log("9. 发送验证码...")
                if not self._send_verification_code():
                    result.error_message = "发送验证码失败"
                    return result

            # 10. 获取验证码
            self._log("10. 等待验证码...")
            code = self._wait_for_signup_verification_code()
            if not code:
                result.error_message = "获取验证码失败"
                return result

            # 11. 验证验证码
            self._log("11. 验证验证码...")
            if not self._validate_signup_verification_code_with_retry(code):
                result.error_message = "验证验证码失败"
                return result

            # 12. [已注册账号跳过] 创建用户账户
            if self._is_existing_account:
                self._log("12. [已注册账号] 跳过创建用户账户")
            else:
                self._log("12. 创建用户账户...")
                if not self._create_user_account():
                    result.error_message = "创建用户账户失败"
                    return result
                if (
                    self._post_create_page_type == "add_phone"
                    or "add-phone" in self._post_create_continue_url
                ):
                    callback_url = self._attempt_add_phone_login_bypass(did, sen_token)
                    if not callback_url:
                        result.metadata = {
                            "blocked_step": "add_phone",
                            "continue_url": self._post_create_continue_url,
                        }
                        self._log(
                            "OpenAI 注册后进入手机号验证，登录回退未直接拿到 OAuth 回调；继续尝试从当前会话提取 Workspace",
                            "warning",
                        )

            if callback_url:
                self._log("13. 已通过登录回退链路拿到 OAuth 回调，跳过 Workspace 预选择")
            else:
                # 13. 获取 Workspace ID（新版流程可能不再提前下发）
                self._log("13. 获取 Workspace ID...")
                workspace_id = self._get_workspace_id()
                if workspace_id:
                    result.workspace_id = workspace_id
                    self._log("14. 选择 Workspace...")
                    continue_url = self._select_workspace(workspace_id)
                    if not continue_url:
                        self._log("workspace/select 失败，回退到原始 OAuth URL 继续流程", "warning")
                        continue_url = self._build_authenticated_oauth_url()
                else:
                    self._log("未提前拿到 Workspace ID，回退到原始 OAuth URL 继续流程", "warning")
                    continue_url = self._build_authenticated_oauth_url()

                # 15. 跟随重定向链
                self._log("15. 跟随重定向链...")
                callback_url = self._follow_redirects(continue_url)
                if not callback_url:
                    result.error_message = "跟随重定向链失败"
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
