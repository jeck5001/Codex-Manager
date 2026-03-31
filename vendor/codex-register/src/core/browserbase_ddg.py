"""
Browserbase + DDG 注册模式执行器
"""

from __future__ import annotations

import json
import random
import re
import time
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Any, Callable, Dict, Optional
from urllib.parse import quote, unquote

import websocket
from curl_cffi import requests as cffi_requests

from ..config.constants import PASSWORD_CHARSET, generate_random_user_info
from ..config.settings import get_settings
from ..database import crud
from ..database.session import get_db
from .oauth import generate_oauth_url, submit_callback_url
from .register import RegistrationResult


DEFAULT_REGISTER_MODE = "standard"
BROWSERBASE_DDG_REGISTER_MODE = "browserbase_ddg"


@dataclass
class BrowserbaseSession:
    session_id: str
    session_url: str
    ws_url: str


class BrowserbaseDDGRegistrationRunner:
    def __init__(
        self,
        profile_id: int,
        profile_name: str,
        profile_config: Dict[str, Any],
        proxy_url: Optional[str] = None,
        callback_logger: Optional[Callable[[str], None]] = None,
    ):
        self.profile_id = profile_id
        self.profile_name = profile_name
        self.profile_config = profile_config or {}
        self.proxy_url = proxy_url
        self.callback_logger = callback_logger or (lambda *_args, **_kwargs: None)
        self.logs: list[str] = []

    def _log(self, message: str) -> None:
        timestamp = datetime.now().strftime("%H:%M:%S")
        entry = f"[{timestamp}] {message}"
        self.logs.append(entry)
        self.callback_logger(entry)

    def _http(self, method: str, url: str, **kwargs):
        request_method = getattr(cffi_requests, method.lower())
        proxies = None
        if self.proxy_url:
            proxies = {"http": self.proxy_url, "https": self.proxy_url}
        return request_method(
            url,
            proxies=proxies,
            timeout=kwargs.pop("timeout", 30),
            impersonate="chrome",
            **kwargs,
        )

    def _config_str(self, *keys: str, default: str = "") -> str:
        for key in keys:
            value = self.profile_config.get(key)
            if value is not None:
                text = str(value).strip()
                if text:
                    return text
        return default

    def _config_int(self, *keys: str, default: int) -> int:
        raw = self._config_str(*keys, default="")
        if not raw:
            return default
        try:
            return int(raw)
        except (TypeError, ValueError):
            return default

    def _generate_password(self, length: int = 16) -> str:
        return "".join(random.choice(PASSWORD_CHARSET) for _ in range(length)) + "A1!"

    def _resolve_redirect_uri(self, default_redirect_uri: str) -> str:
        explicit = self._config_str("oauth_redirect_uri", "oauthRedirectUri")
        if explicit:
            return explicit
        port = self._config_int("oauth_redirect_port", "oauthRedirectPort", default=1455)
        return f"http://localhost:{port}/auth/callback" if port > 0 else default_redirect_uri

    def _generate_alias(self) -> str:
        ddg_token = self._config_str("ddg_token", "ddgToken")
        if not ddg_token:
            raise RuntimeError("browserbase 配置缺少 ddg_token")

        self._log("正在向 DDG 申请私有邮箱别名")
        response = self._http(
            "post",
            "https://quack.duckduckgo.com/api/email/addresses",
            json={},
            headers={
                "Authorization": f"Bearer {ddg_token}",
                "Content-Type": "application/json",
            },
            timeout=20,
        )
        if response.status_code != 200:
            raise RuntimeError(f"DDG 邮箱别名生成失败: HTTP {response.status_code}")
        payload = response.json()
        address = str((payload or {}).get("address") or "").strip()
        if not address:
            raise RuntimeError("DDG 邮箱别名生成失败: 缺少 address")
        email = f"{address}@duck.com"
        self._log(f"已生成 DDG 邮箱: {email}")
        return email

    def _create_browserbase_session(self) -> BrowserbaseSession:
        api_base = self._config_str(
            "browserbase_api_base",
            "browserbaseApiBase",
            default="https://gemini.browserbase.com",
        ).rstrip("/")
        timezone = self._config_str("browser_timezone", "browserTimezone", default="HKT")
        self._log(f"正在创建 Browserbase 会话，时区: {timezone}")
        response = self._http(
            "post",
            f"{api_base}/api/session",
            json={"timezone": timezone},
            headers={"Content-Type": "application/json"},
            timeout=20,
        )
        if response.status_code != 200:
            raise RuntimeError(f"Browserbase 会话创建失败: HTTP {response.status_code}")
        payload = response.json() or {}
        if not payload.get("success"):
            raise RuntimeError("Browserbase 会话创建失败: success=false")
        session_url = str(payload.get("sessionUrl") or "").strip()
        session_id = str(payload.get("sessionId") or "").strip()
        ws_match = re.search(r"wss=([^&]+)", session_url)
        ws_url = unquote(ws_match.group(1)) if ws_match else ""
        if not ws_url:
            raise RuntimeError("Browserbase 会话创建失败: 缺少 ws_url")
        self._log(f"Browserbase 会话已创建: {session_id}")
        return BrowserbaseSession(session_id=session_id, session_url=session_url, ws_url=ws_url)

    def _send_agent_goal(self, session_id: str, goal: str) -> None:
        api_base = self._config_str(
            "browserbase_api_base",
            "browserbaseApiBase",
            default="https://gemini.browserbase.com",
        ).rstrip("/")
        model = self._config_str(
            "agent_model",
            "agentModel",
            default="google/gemini-3-flash-preview",
        )
        url = (
            f"{api_base}/api/agent/stream?sessionId={quote(session_id)}"
            f"&goal={quote(goal)}&model={quote(model)}"
        )
        self._log("正在下发 Browserbase Agent 目标")
        response = self._http("get", url, stream=True, timeout=20)
        response.close()

    def _wait_for_target_url(self, ws_url: str, target_keyword: str, timeout_seconds: int) -> str:
        normalized_ws_url = ws_url if ws_url.startswith("ws") else f"wss://{ws_url}"
        deadline = time.time() + timeout_seconds
        message_id = 1
        conn = websocket.create_connection(normalized_ws_url, timeout=10)
        try:
            conn.send(json.dumps({
                "id": message_id,
                "method": "Target.setDiscoverTargets",
                "params": {"discover": True},
            }))
            message_id += 1
            seen_urls: set[str] = set()
            while time.time() < deadline:
                conn.send(json.dumps({"id": message_id, "method": "Target.getTargets", "params": {}}))
                expected_id = message_id
                message_id += 1
                while time.time() < deadline:
                    raw = conn.recv()
                    payload = json.loads(raw)
                    if payload.get("id") != expected_id:
                        continue
                    targets = (payload.get("result") or {}).get("targetInfos") or []
                    for target in targets:
                        if target.get("type") not in (None, "", "page"):
                            continue
                        current_url = str(target.get("url") or "").strip()
                        if not current_url or current_url == "about:blank":
                            continue
                        if current_url not in seen_urls:
                            seen_urls.add(current_url)
                            self._log(f"监控到页面 URL: {current_url}")
                        if target_keyword in current_url:
                            return current_url
                    break
                time.sleep(3)
        finally:
            conn.close()
        raise RuntimeError(f"等待目标页面超时: {target_keyword}")

    def _build_phase1_goal(self, email: str, password: str, full_name: str, birthdate: str) -> str:
        mail_inbox_url = self._config_str("mail_inbox_url", "mailInboxUrl")
        if not mail_inbox_url:
            raise RuntimeError("browserbase 配置缺少 mail_inbox_url")
        return (
            f"请打开 chatgpt 的对话页面，然后点击创建一个账户，使用 {email} 作为邮箱，"
            f"{password} 作为密码，然后在显示验证码发送后在 {mail_inbox_url} 上接收自己的邮箱验证码，"
            f"接下来使用 {full_name} 作为全名，{birthdate} 作为出生日期（如果表单要求年龄则换算成年龄）。"
            "创建账户后导航到 "
            "`data:text/html,<html><head><title>MISSION_ACCOMPLISHED</title></head>"
            "<body style=\"background:black;color:lime;display:flex;justify-content:center;align-items:center;"
            "height:100vh;font-family:monospace;\"><h1>> TASK COMPLETED SUCCESSFULLY _</h1></body></html>`"
            "，等待 3 秒并结束。每次等待时间不得超过 3 秒。"
        )

    def _build_phase2_goal(self, auth_url: str, email: str, password: str) -> str:
        mail_inbox_url = self._config_str("mail_inbox_url", "mailInboxUrl")
        return (
            f"选择导航到 {auth_url}，使用 {email} 作为邮箱，{password} 作为密码登录，"
            f"然后在显示验证码发送后在 {mail_inbox_url} 上接收自己的邮箱验证码，"
            "选择登录到 codex，地址跳转到 localhost 回调链接，出现无法访问的页面后记录当前完整 url 并结束。"
            "每次等待时间不得超过 3 秒。"
        )

    def run(self) -> RegistrationResult:
        email = self._generate_alias()
        password = self._generate_password()
        user_info = generate_random_user_info()
        self._log(f"已生成注册身份: {user_info['name']} / {user_info['birthdate']}")

        settings = get_settings()
        redirect_uri = self._resolve_redirect_uri(settings.openai_redirect_uri)
        oauth_start = generate_oauth_url(
            redirect_uri=redirect_uri,
            client_id=self._config_str("oauth_client_id", "oauthClientId", default=settings.openai_client_id),
            scope=settings.openai_scope,
        )

        phase1_session = self._create_browserbase_session()
        self._send_agent_goal(
            phase1_session.session_id,
            self._build_phase1_goal(
                email=email,
                password=password,
                full_name=user_info["name"],
                birthdate=user_info["birthdate"],
            ),
        )
        timeout_seconds = self._config_int("max_wait_seconds", "maxWaitSeconds", default=1800)
        self._wait_for_target_url(phase1_session.ws_url, "MISSION_ACCOMPLISHED", timeout_seconds)
        self._log("Browserbase 注册阶段完成")

        phase2_session = self._create_browserbase_session()
        self._send_agent_goal(
            phase2_session.session_id,
            self._build_phase2_goal(
                auth_url=oauth_start.auth_url,
                email=email,
                password=password,
            ),
        )
        callback_url = self._wait_for_target_url(phase2_session.ws_url, "localhost", timeout_seconds)
        self._log(f"捕获 OAuth 回调 URL: {callback_url}")

        token_json = submit_callback_url(
            callback_url=callback_url,
            expected_state=oauth_start.state,
            code_verifier=oauth_start.code_verifier,
            redirect_uri=redirect_uri,
            client_id=self._config_str("oauth_client_id", "oauthClientId", default=settings.openai_client_id),
            token_url=settings.openai_token_url,
            proxy_url=self.proxy_url,
        )
        token_payload = json.loads(token_json)
        expired_at = None
        expired_text = str(token_payload.get("expired") or "").strip()
        if expired_text:
            try:
                expired_at = datetime.fromisoformat(expired_text.replace("Z", "+00:00")).replace(tzinfo=None)
            except ValueError:
                expired_at = datetime.utcnow() + timedelta(days=14)

        self._log("OAuth token 交换成功")
        return RegistrationResult(
            success=True,
            email=email,
            password=password,
            account_id=str(token_payload.get("account_id") or "").strip(),
            workspace_id=str(token_payload.get("workspace_id") or "").strip(),
            access_token=str(token_payload.get("access_token") or "").strip(),
            refresh_token=str(token_payload.get("refresh_token") or "").strip(),
            id_token=str(token_payload.get("id_token") or "").strip(),
            error_message="",
            logs=self.logs.copy(),
            metadata={
                "register_mode": BROWSERBASE_DDG_REGISTER_MODE,
                "browserbase_config_id": self.profile_id,
                "browserbase_config_name": self.profile_name,
                "expired": expired_text,
            },
            source="register",
            account_status="active",
            is_usable=True,
        )

    def save_to_database(self, result: RegistrationResult) -> bool:
        if not result.success:
            return False

        try:
            settings = get_settings()
            with get_db() as db:
                crud.create_account(
                    db,
                    email=result.email,
                    password=result.password,
                    client_id=self._config_str("oauth_client_id", "oauthClientId", default=settings.openai_client_id),
                    session_token=result.session_token,
                    cookies=result.cookies,
                    email_service=BROWSERBASE_DDG_REGISTER_MODE,
                    email_service_id=str(self.profile_id),
                    account_id=result.account_id,
                    workspace_id=result.workspace_id,
                    access_token=result.access_token,
                    refresh_token=result.refresh_token,
                    id_token=result.id_token,
                    proxy_used=self.proxy_url,
                    extra_data=result.metadata,
                    status=result.account_status,
                    source=result.source,
                )
                crud.update_browserbase_config_last_used(db, self.profile_id)
            self._log("Browserbase 注册结果已写入数据库")
            return True
        except Exception as exc:
            self._log(f"保存 Browserbase 注册结果失败: {exc}")
            return False
