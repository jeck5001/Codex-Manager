import argparse
import json
import sys
from pathlib import Path
from urllib.parse import urlparse


def _build_proxy_config(proxy_url: str):
    proxy_url = str(proxy_url or "").strip()
    if not proxy_url:
        return None
    parsed = urlparse(proxy_url)
    if not parsed.scheme or not parsed.hostname or not parsed.port:
        return None
    proxy = {
        "server": f"{parsed.scheme}://{parsed.hostname}:{parsed.port}",
    }
    if parsed.username:
        proxy["username"] = parsed.username
    if parsed.password:
        proxy["password"] = parsed.password
    return proxy


def _normalize_cookie(cookie):
    if not isinstance(cookie, dict):
        return None
    name = str(cookie.get("name") or "").strip()
    value = str(cookie.get("value") or "").strip()
    if not name or not value:
        return None

    normalized = {
        "name": name,
        "value": value,
        "path": str(cookie.get("path") or "/").strip() or "/",
        "secure": bool(cookie.get("secure")),
        "httpOnly": bool(cookie.get("http_only") or cookie.get("httpOnly")),
    }
    domain = str(cookie.get("domain") or "").strip()
    if domain:
        normalized["domain"] = domain
    expires = cookie.get("expires")
    if isinstance(expires, (int, float)) and int(expires) > 0:
        normalized["expires"] = int(expires)
    same_site = str(cookie.get("same_site") or cookie.get("sameSite") or "").strip()
    if same_site:
        normalized["sameSite"] = same_site
    return normalized


def _load_payload(payload_path: str):
    path = Path(payload_path)
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError("payload must be a JSON object")
    return data


def launch_local_handoff(payload_path: str, profile_dir: str) -> None:
    from playwright.sync_api import sync_playwright

    payload = _load_payload(payload_path)
    target_url = str(payload.get("url") or "").strip()
    if not target_url:
        raise ValueError("payload.url is required")

    launch_options = {
        "headless": False,
        "user_data_dir": str(Path(profile_dir)),
        "ignore_https_errors": True,
        "args": [
            "--disable-blink-features=AutomationControlled",
        ],
    }
    proxy = _build_proxy_config(str(payload.get("proxy_url") or ""))
    if proxy:
        launch_options["proxy"] = proxy

    with sync_playwright() as playwright:
        context = playwright.chromium.launch_persistent_context(**launch_options)
        try:
            cookies = []
            for item in payload.get("cookies") or []:
                normalized = _normalize_cookie(item)
                if normalized is not None:
                    cookies.append(normalized)
            if cookies:
                context.add_cookies(cookies)

            page = context.pages[0] if context.pages else context.new_page()
            for origin_state in payload.get("origins") or []:
                if not isinstance(origin_state, dict):
                    continue
                origin = str(origin_state.get("origin") or "").strip()
                if not origin.startswith("http"):
                    continue
                page.goto(origin, wait_until="domcontentloaded", timeout=60_000)
                entries = origin_state.get("local_storage") or origin_state.get("localStorage") or []
                if isinstance(entries, list) and entries:
                    page.evaluate(
                        """entries => {
                            for (const entry of entries) {
                                if (!entry || !entry.name) {
                                    continue;
                                }
                                window.localStorage.setItem(entry.name, entry.value ?? "");
                            }
                        }""",
                        entries,
                    )

            page.goto(target_url, wait_until="domcontentloaded", timeout=60_000)
            page.wait_for_timeout(300_000)
        finally:
            context.close()


def main() -> int:
    parser = argparse.ArgumentParser(description="Launch Hotmail local handoff browser")
    parser.add_argument("--payload", required=True)
    parser.add_argument("--profile-dir", required=True)
    args = parser.parse_args()
    try:
        launch_local_handoff(args.payload, args.profile_dir)
        return 0
    except Exception as exc:
        print(f"local handoff launch failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
