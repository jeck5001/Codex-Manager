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
from urllib.parse import quote, unquote, urlsplit

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
DEFAULT_BROWSERBASE_API_BASE = "https://gemini.browserbase.com"
DEFAULT_BROWSERBASE_AGENT_MODEL = "google/gemini-2.5-computer-use-preview-10-2025"
MIN_BROWSERBASE_PHASE_TIMEOUT_SECONDS = 900


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
        task_uuid: Optional[str] = None,
    ):
        self.profile_id = profile_id
        self.profile_name = profile_name
        self.profile_config = profile_config or {}
        self.proxy_url = proxy_url
        self.callback_logger = callback_logger or (lambda *_args, **_kwargs: None)
        self.task_uuid = task_uuid
        self.logs: list[str] = []

    def _log(self, message: str) -> None:
        timestamp = datetime.now().strftime("%H:%M:%S")
        entry = f"[{timestamp}] {message}"
        self.logs.append(entry)
        self.callback_logger(entry)
        if self.task_uuid:
            try:
                with get_db() as db:
                    crud.append_task_log(db, self.task_uuid, entry)
            except Exception as exc:
                logger.warning(f"记录 Browserbase-DDG 任务日志失败: {exc}")

    @staticmethod
    def _response_text_snippet(response: Any, limit: int = 200) -> str:
        text = str(getattr(response, "text", "") or "").strip()
        return text[:limit]

    def _parse_json_response(self, response: Any, stage: str) -> Dict[str, Any]:
        try:
            payload = response.json()
        except json.JSONDecodeError as exc:
            snippet = self._response_text_snippet(response)
            detail = "返回了空响应或非 JSON 响应"
            if snippet:
                detail = f"{detail}: {snippet}"
            raise RuntimeError(f"{stage}: {detail}") from exc
        return payload or {}

    def _parse_json_text(self, raw: str, stage: str) -> Dict[str, Any]:
        text = str(raw or "").strip()
        if not text:
            raise RuntimeError(f"{stage}: 返回了空响应或非 JSON 响应")
        try:
            payload = json.loads(text)
        except json.JSONDecodeError as exc:
            raise RuntimeError(f"{stage}: 返回了空响应或非 JSON 响应: {text[:200]}") from exc
        if not isinstance(payload, dict):
            raise RuntimeError(f"{stage}: 返回的 JSON 不是对象")
        return payload

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

    def _browserbase_api_base(self) -> str:
        raw = self._config_str(
            "browserbase_api_base",
            "browserbaseApiBase",
            default=DEFAULT_BROWSERBASE_API_BASE,
        ).rstrip("/")
        if not raw:
            return DEFAULT_BROWSERBASE_API_BASE

        try:
            parsed = urlsplit(raw)
        except Exception:
            return DEFAULT_BROWSERBASE_API_BASE

        host = (parsed.netloc or "").strip().lower()
        path = (parsed.path or "").strip().lower()
        if host in {"www.browserbase.com", "browserbase.com"}:
            self._log(
                f"检测到 Browserbase 官网地址配置 {raw}，自动改用 Gemini API 地址 {DEFAULT_BROWSERBASE_API_BASE}"
            )
            return DEFAULT_BROWSERBASE_API_BASE

        if host == "gemini.browserbase.com" and path in {"", "/"}:
            return DEFAULT_BROWSERBASE_API_BASE

        return raw

    def _generate_password(self, length: int = 16) -> str:
        return "".join(random.choice(PASSWORD_CHARSET) for _ in range(length)) + "A1!"

    def _agent_model(self) -> str:
        raw = self._config_str(
            "agent_model",
            "agentModel",
            default=DEFAULT_BROWSERBASE_AGENT_MODEL,
        )
        normalized = raw.strip()
        if not normalized:
            return DEFAULT_BROWSERBASE_AGENT_MODEL
        if normalized == "computer-use-preview":
            self._log(
                f"检测到旧 Agent 模型配置 {normalized}，自动改用 {DEFAULT_BROWSERBASE_AGENT_MODEL}"
            )
            return DEFAULT_BROWSERBASE_AGENT_MODEL
        return normalized

    def _phase_timeout_seconds(self) -> int:
        configured = self._config_int("max_wait_seconds", "maxWaitSeconds", default=1800)
        if configured < MIN_BROWSERBASE_PHASE_TIMEOUT_SECONDS:
            self._log(
                f"检测到过短的 Browserbase 等待时间 {configured}s，自动提升到 {MIN_BROWSERBASE_PHASE_TIMEOUT_SECONDS}s"
            )
            return MIN_BROWSERBASE_PHASE_TIMEOUT_SECONDS
        return configured

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
        if not 200 <= response.status_code < 300:
            raise RuntimeError(f"DDG 邮箱别名生成失败: HTTP {response.status_code}")
        payload = self._parse_json_response(response, "DDG 邮箱别名生成失败")
        address = str((payload or {}).get("address") or "").strip()
        if not address:
            raise RuntimeError("DDG 邮箱别名生成失败: 缺少 address")
        email = f"{address}@duck.com"
        self._log(f"已生成 DDG 邮箱: {email}")
        return email

    def _create_browserbase_session(self) -> BrowserbaseSession:
        api_base = self._browserbase_api_base()
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
        payload = self._parse_json_response(response, "Browserbase 会话创建失败")
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

    def _send_agent_goal(self, session_id: str, goal: str):
        api_base = self._browserbase_api_base()
        model = self._agent_model()
        url = (
            f"{api_base}/api/agent/stream?sessionId={quote(session_id)}"
            f"&goal={quote(goal)}&model={quote(model)}"
        )
        self._log(f"正在下发 Browserbase Agent 目标，模型: {model}")
        response = self._http("get", url, stream=True, timeout=20)
        status_code = int(getattr(response, "status_code", 0) or 0)
        if not 200 <= status_code < 300:
            snippet = self._response_text_snippet(response)
            detail = f"HTTP {status_code}"
            if snippet:
                detail = f"{detail}: {snippet}"
            raise RuntimeError(f"Browserbase Agent 目标下发失败: {detail}")
        self._log(f"Browserbase Agent 目标下发成功: HTTP {status_code}")
        return response

    def _open_target_url(self, ws_url: str, target_url: str) -> str:
        normalized_ws_url = ws_url if ws_url.startswith("ws") else f"wss://{ws_url}"
        conn = websocket.create_connection(normalized_ws_url, timeout=10)
        try:
            message_id = 1
            conn.send(json.dumps({
                "id": message_id,
                "method": "Target.createTarget",
                "params": {"url": target_url},
            }))
            while True:
                raw = conn.recv()
                text = str(raw or "").strip()
                if not text:
                    continue
                try:
                    payload = json.loads(text)
                except json.JSONDecodeError:
                    continue
                if payload.get("id") != message_id:
                    continue
                result = payload.get("result") if isinstance(payload.get("result"), dict) else {}
                target_id = str(result.get("targetId") or "").strip()
                if not target_id:
                    raise RuntimeError("Browserbase 打开授权页失败: 缺少 targetId")
                self._log(f"已在 Browserbase 会话中打开目标页: {target_url}")
                return target_id
        finally:
            conn.close()

    @staticmethod
    def _url_matches_target_keyword(current_url: str, target_keyword: str) -> bool:
        if not current_url or not target_keyword:
            return False
        try:
            parsed = urlsplit(current_url)
        except Exception:
            return target_keyword in current_url

        searchable_parts = [parsed.scheme, parsed.netloc, parsed.path]
        return any(target_keyword in part for part in searchable_parts if part)

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
                    text = str(raw or "").strip()
                    if not text:
                        self._log("收到空 WebSocket 消息，已忽略")
                        continue
                    try:
                        payload = json.loads(text)
                    except json.JSONDecodeError:
                        self._log(f"收到非 JSON WebSocket 消息，已忽略: {text[:120]}")
                        continue
                    if payload.get("id") != expected_id:
                        continue
                    targets = (payload.get("result") or {}).get("targetInfos") or []
                    for target in targets:
                        if target.get("type") not in (None, "", "page"):
                            continue
                        current_url = str(target.get("url") or "").strip()
                        current_title = str(target.get("title") or "").strip()
                        if not current_url or current_url == "about:blank":
                            if target_keyword and current_title and target_keyword in current_title:
                                self._log(f"监控到页面标题命中目标: {current_title}")
                                return current_url or current_title
                            continue
                        if current_url not in seen_urls:
                            seen_urls.add(current_url)
                            self._log(f"监控到页面 URL: {current_url}")
                        if current_title and target_keyword in current_title:
                            self._log(f"监控到页面标题命中目标: {current_title}")
                            return current_url
                        if self._url_matches_target_keyword(current_url, target_keyword):
                            return current_url
                    break
                time.sleep(3)
        finally:
            conn.close()
        raise RuntimeError(f"等待目标页面超时: {target_keyword}")

    def _build_phase1_goal(
        self,
        auth_url: str,
        email: str,
        password: str,
        full_name: str,
        birthdate: str,
    ) -> str:
        mail_inbox_url = self._config_str("mail_inbox_url", "mailInboxUrl")
        if not mail_inbox_url:
            raise RuntimeError("browserbase 配置缺少 mail_inbox_url")
        return (
            f"请直接在地址栏打开 {auth_url}，不要使用 Google、DuckDuckGo 或任何搜索引擎，也不要先搜索 ChatGPT。"
            "如果页面显示登录入口，请点击 Sign up / Create account 进入注册流程，并继续完成 Codex 授权流程。"
            f"使用 {email} 作为邮箱，{password} 作为密码，然后在显示验证码发送后立即前往 {mail_inbox_url} 接收自己的邮箱验证码。"
            f"接下来使用 {full_name} 作为全名，{birthdate} 作为出生日期（如果表单要求年龄则换算成年龄）。"
            "如果流程要求确认继续授权、选择 workspace、或者继续跳转到 Codex，请继续完成。"
            "最终目标是让页面跳转到 localhost 回调链接；当出现 localhost 回调页或浏览器地址栏变成 localhost 回调地址后立即停止。"
            "每次等待时间不得超过 3 秒；如果页面卡住，优先刷新当前页面或重新打开上面的授权链接，而不是重新搜索。"
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
        self._open_target_url(phase1_session.ws_url, oauth_start.auth_url)
        phase1_stream = self._send_agent_goal(
            phase1_session.session_id,
            self._build_phase1_goal(
                auth_url=oauth_start.auth_url,
                email=email,
                password=password,
                full_name=user_info["name"],
                birthdate=user_info["birthdate"],
            ),
        )
        timeout_seconds = self._phase_timeout_seconds()
        try:
            callback_url = self._wait_for_target_url(phase1_session.ws_url, "localhost", timeout_seconds)
        finally:
            try:
                phase1_stream.close()
            except Exception:
                pass
        self._log("Browserbase 单会话注册授权阶段完成")
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
        token_payload = self._parse_json_text(token_json, "OAuth token 交换失败")
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
