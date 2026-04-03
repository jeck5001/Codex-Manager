"""
通过 Playwright 在浏览器上下文中获取完整 Sentinel token。

这里优先贴近浏览器真实行为：
- 打开 auth.openai.com/about-you
- 等待 window.SentinelSDK.token 可用
- 直接调用 SentinelSDK.token(flow)

返回值统一为带 p / t / c / id / flow 的字典，供注册流程直接组装
openai-sentinel-token 请求头。
"""

from __future__ import annotations

import logging
import urllib.parse
from typing import Any, Callable, Dict, Optional


logger = logging.getLogger(__name__)

DEFAULT_SENTINEL_TARGET_URL = "https://auth.openai.com/about-you"
DEFAULT_SENTINEL_USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/136.0.7103.92 Safari/537.36"
)


def _log(callback_logger: Optional[Callable[[str], None]], message: str):
    if callback_logger:
        try:
            callback_logger(message)
            return
        except Exception:
            pass
    logger.info(message)


def _parse_cookie_str(cookies_str: str, domain: str) -> list[dict]:
    cookies: list[dict] = []
    normalized_domain = str(domain or "").strip()
    host_domain = normalized_domain.lstrip(".")
    host_url = "https://chatgpt.com/"
    if host_domain == "openai.com":
        host_url = "https://auth.openai.com/"
    elif host_domain:
        host_url = f"https://{host_domain}/"

    for chunk in (cookies_str or "").split(";"):
        raw = chunk.strip()
        if not raw or "=" not in raw:
            continue
        name, value = raw.split("=", 1)
        name = name.strip()
        value = value.strip()
        if not name or not value:
            continue
        cookie = {
            "name": name,
            "value": value,
            "secure": True,
            "httpOnly": False,
        }
        if name.startswith("__Host-"):
            cookie["url"] = host_url
        else:
            cookie["domain"] = normalized_domain
            cookie["path"] = "/"
        cookies.append(cookie)
    return cookies


def _build_playwright_proxy(proxy_url: Optional[str]) -> Optional[Dict[str, str]]:
    raw = (proxy_url or "").strip()
    if not raw:
        return None

    parsed = urllib.parse.urlsplit(raw)
    if not parsed.scheme or not parsed.hostname:
        return None

    server = f"{parsed.scheme}://{parsed.hostname}"
    if parsed.port:
        server = f"{server}:{parsed.port}"

    proxy: Dict[str, str] = {"server": server}
    if parsed.username:
        proxy["username"] = urllib.parse.unquote(parsed.username)
    if parsed.password:
        proxy["password"] = urllib.parse.unquote(parsed.password)
    return proxy


def fetch_browser_sentinel_token(
    *,
    did: str,
    flow: str,
    referer: str,
    cookies_str: str = "",
    proxy_url: Optional[str] = None,
    callback_logger: Optional[Callable[[str], None]] = None,
) -> Optional[Dict[str, str]]:
    """在浏览器页面中调用 SentinelSDK.token(flow)，返回完整 token 载荷。"""
    try:
        from playwright.sync_api import TimeoutError as PlaywrightTimeoutError
        from playwright.sync_api import sync_playwright
    except ImportError:
        _log(callback_logger, "playwright 未安装，无法使用浏览器 Sentinel")
        return None

    target_url = (referer or "").strip() or DEFAULT_SENTINEL_TARGET_URL
    domain = ".openai.com"
    proxy = _build_playwright_proxy(proxy_url)

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            proxy=proxy,
            args=[
                "--no-sandbox",
                "--disable-blink-features=AutomationControlled",
            ],
        )
        context = browser.new_context(
            viewport={"width": 1920, "height": 1080},
            user_agent=DEFAULT_SENTINEL_USER_AGENT,
            ignore_https_errors=True,
        )

        try:
            cookies = _parse_cookie_str(cookies_str, domain)
            if cookies:
                context.add_cookies(cookies)

            page = context.new_page()
            page.goto(target_url, wait_until="domcontentloaded", timeout=30_000)
            page.wait_for_timeout(1_500)
            page.wait_for_function(
                """
                () => typeof window.SentinelSDK !== 'undefined'
                  && typeof window.SentinelSDK.token === 'function'
                """,
                timeout=15_000,
            )
            _log(callback_logger, f"浏览器 Sentinel SDK 已加载，开始请求 flow={flow}")

            result = page.evaluate(
                """
                async (flow) => {
                    try {
                        const token = await window.SentinelSDK.token(flow);
                        return { success: true, token };
                    } catch (error) {
                        return {
                            success: false,
                            error: error?.message || String(error || "unknown error"),
                        };
                    }
                }
                """,
                flow,
            )
            if not isinstance(result, dict):
                _log(callback_logger, "浏览器 Sentinel 返回值不是对象")
                return None
            if not result.get("success") or not result.get("token"):
                _log(callback_logger, f"浏览器 Sentinel 失败: {result.get('error', 'unknown error')}")
                return None

            token = result["token"]
            if isinstance(token, str):
                try:
                    import json
                    payload = json.loads(token)
                except Exception:
                    payload = {"c": token}
            elif isinstance(token, dict):
                payload = token
            else:
                _log(callback_logger, "浏览器 Sentinel token 类型不支持")
                return None

            normalized = {
                "p": str(payload.get("p") or "").strip(),
                "t": str(payload.get("t") or "").strip(),
                "c": str(payload.get("c") or payload.get("token") or "").strip(),
                "id": str(payload.get("id") or did).strip(),
                "flow": str(payload.get("flow") or flow).strip(),
            }
            if not normalized["c"]:
                _log(callback_logger, "浏览器 Sentinel 未返回 c 字段")
                return None
            _log(
                callback_logger,
                "浏览器 Sentinel 成功"
                f" | flow={normalized['flow']}"
                f" | p={'yes' if normalized['p'] else 'no'}"
                f" | t={'yes' if normalized['t'] else 'no'}"
                f" | c={'yes' if normalized['c'] else 'no'}",
            )
            return normalized
        except PlaywrightTimeoutError:
            _log(callback_logger, "浏览器 Sentinel 等待超时")
            return None
        except Exception as e:
            _log(callback_logger, f"浏览器 Sentinel 异常: {e}")
            return None
        finally:
            try:
                context.close()
            finally:
                browser.close()


def _extract_browser_session_token(cookies: list[dict]) -> str:
    cookie_map = {
        str(item.get("name") or "").strip(): str(item.get("value") or "").strip()
        for item in cookies
        if str(item.get("name") or "").strip() and str(item.get("value") or "").strip()
    }

    for cookie_name in (
        "__Secure-next-auth.session-token",
        "next-auth.session-token",
    ):
        direct = cookie_map.get(cookie_name)
        if direct:
            return direct

        prefix = f"{cookie_name}."
        chunks: list[tuple[int, str]] = []
        for name, value in cookie_map.items():
            if not name.startswith(prefix):
                continue
            suffix = name[len(prefix):]
            if suffix.isdigit():
                chunks.append((int(suffix), value))
        if chunks:
            chunks.sort(key=lambda item: item[0])
            return "".join(value for _, value in chunks)

    return ""


def fetch_browser_chatgpt_session_payload(
    *,
    cookies_str: str = "",
    cookies: Optional[list[dict[str, Any]]] = None,
    proxy_url: Optional[str] = None,
    callback_logger: Optional[Callable[[str], None]] = None,
) -> Optional[Dict[str, Any]]:
    """在浏览器上下文中落地 chatgpt.com 会话并读取 /api/auth/session。"""
    try:
        from playwright.sync_api import TimeoutError as PlaywrightTimeoutError
        from playwright.sync_api import sync_playwright
    except ImportError:
        _log(callback_logger, "playwright 未安装，无法使用浏览器 ChatGPT Session")
        return None

    proxy = _build_playwright_proxy(proxy_url)

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            proxy=proxy,
            args=[
                "--no-sandbox",
                "--disable-blink-features=AutomationControlled",
            ],
        )
        context = browser.new_context(
            viewport={"width": 1920, "height": 1080},
            user_agent=DEFAULT_SENTINEL_USER_AGENT,
            ignore_https_errors=True,
        )

        try:
            context_cookies = list(cookies or [])
            if not context_cookies and cookies_str:
                context_cookies.extend(_parse_cookie_str(cookies_str, ".openai.com"))
            if context_cookies:
                context.add_cookies(context_cookies)

            page = context.new_page()
            page.goto("https://auth.openai.com/", wait_until="domcontentloaded", timeout=30_000)
            page.wait_for_timeout(1_000)
            page.goto("https://chatgpt.com/", wait_until="domcontentloaded", timeout=30_000)
            page.wait_for_timeout(2_000)

            session_payload = page.evaluate(
                """
                async () => {
                    const response = await fetch("https://chatgpt.com/api/auth/session", {
                        credentials: "include",
                        headers: { "accept": "application/json" },
                    });
                    let payload = null;
                    try {
                        payload = await response.json();
                    } catch (error) {
                        payload = null;
                    }
                    return {
                        ok: response.ok,
                        status: response.status,
                        payload,
                    };
                }
                """
            )
            if not isinstance(session_payload, dict):
                _log(callback_logger, "浏览器 ChatGPT Session 返回值不是对象")
                return None
            if not session_payload.get("ok"):
                _log(
                    callback_logger,
                    f"浏览器 ChatGPT Session 失败: HTTP {session_payload.get('status')}",
                )
                return None

            payload = session_payload.get("payload")
            if not isinstance(payload, dict):
                _log(callback_logger, "浏览器 ChatGPT Session 缺少 JSON payload")
                return None

            browser_cookies = context.cookies(["https://chatgpt.com", "https://auth.openai.com"])
            session_token = _extract_browser_session_token(browser_cookies)
            if session_token and not str(payload.get("sessionToken") or "").strip():
                payload["sessionToken"] = session_token

            _log(
                callback_logger,
                "浏览器 ChatGPT Session 成功"
                f" | accessToken={'yes' if str(payload.get('accessToken') or '').strip() else 'no'}"
                f" | sessionToken={'yes' if str(payload.get('sessionToken') or '').strip() else 'no'}"
                f" | account={'yes' if isinstance(payload.get('account'), dict) else 'no'}",
            )
            return payload
        except PlaywrightTimeoutError:
            _log(callback_logger, "浏览器 ChatGPT Session 等待超时")
            return None
        except Exception as e:
            _log(callback_logger, f"浏览器 ChatGPT Session 异常: {e}")
            return None
        finally:
            try:
                context.close()
            finally:
                browser.close()
