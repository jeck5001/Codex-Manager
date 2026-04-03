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
from typing import Callable, Dict, Optional


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
    for chunk in (cookies_str or "").split(";"):
        raw = chunk.strip()
        if not raw or "=" not in raw:
            continue
        name, value = raw.split("=", 1)
        name = name.strip()
        value = value.strip()
        if not name or not value:
            continue
        cookies.append({
            "name": name,
            "value": value,
            "domain": domain,
            "path": "/",
            "secure": True,
            "httpOnly": False,
        })
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
