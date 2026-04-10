from __future__ import annotations

import asyncio
import json
import os
import re
import sys
import threading
from ipaddress import ip_address
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse, Response

ROOT = Path(__file__).resolve().parents[2]
REGISTER_ROOT = ROOT / "vendor" / "codex-register"
if str(REGISTER_ROOT) not in sys.path:
    sys.path.insert(0, str(REGISTER_ROOT))

from src.services.hotmail.local_handoff_cli import (  # noqa: E402
    get_chromium_executable_path,
    launch_local_handoff,
)

DEFAULT_HOST = os.environ.get("CODEXMANAGER_HOTMAIL_HELPER_HOST", "127.0.0.1")
DEFAULT_PORT = int(os.environ.get("CODEXMANAGER_HOTMAIL_HELPER_PORT", "16788"))
HANDOFF_ROOT = Path(
    os.environ.get("CODEXMANAGER_HOTMAIL_HELPER_ROOT", "/tmp/codex-hotmail-handoff")
)
SERVICE_NAME = "hotmail-local-helper"
PRIVATE_HOST_PATTERN = re.compile(
    r"^("
    r"localhost|"
    r"127(?:\.\d{1,3}){3}|"
    r"10(?:\.\d{1,3}){3}|"
    r"192\.168(?:\.\d{1,3}){2}|"
    r"172\.(?:1[6-9]|2\d|3[0-1])(?:\.\d{1,3}){2}"
    r")$"
)


def _load_extra_allowed_origins() -> set[str]:
    raw = os.environ.get("CODEXMANAGER_HOTMAIL_HELPER_ALLOWED_ORIGINS", "")
    return {item.strip() for item in raw.split(",") if item.strip()}


def is_origin_allowed(origin: str) -> bool:
    origin = str(origin or "").strip()
    if not origin:
        return False
    if origin in _load_extra_allowed_origins():
        return True

    parsed = urlparse(origin)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        return False
    host = parsed.hostname.strip().lower()
    if PRIVATE_HOST_PATTERN.match(host):
        return True
    try:
        address = ip_address(host)
        return address.is_private or address.is_loopback
    except ValueError:
        return host.endswith(".local")


def build_cors_headers(origin: str) -> dict[str, str]:
    if not is_origin_allowed(origin):
        return {}
    return {
        "Access-Control-Allow-Origin": origin,
        "Access-Control-Allow-Methods": "GET,POST,OPTIONS",
        "Access-Control-Allow-Headers": "Content-Type",
        "Vary": "Origin",
    }


def build_json_response(status_code: int, content: dict[str, Any], origin: str = "") -> JSONResponse:
    return JSONResponse(status_code=status_code, content=content, headers=build_cors_headers(origin))


def check_playwright_ready() -> bool:
    executable_path = get_chromium_executable_path()
    return bool(executable_path and Path(executable_path).exists())


async def check_playwright_ready_async() -> bool:
    return await asyncio.to_thread(check_playwright_ready)
def launch_local_handoff_background(payload_path: str, profile_dir: str) -> None:
    thread = threading.Thread(
        target=launch_local_handoff,
        args=(payload_path, profile_dir),
        kwargs={"wait_for_seconds": 300},
        daemon=True,
        name=f"hotmail-handoff-{Path(profile_dir).name}",
    )
    thread.start()


def create_app() -> FastAPI:
    app = FastAPI()

    @app.middleware("http")
    async def apply_dynamic_cors(request: Request, call_next):
        origin = request.headers.get("origin", "")
        if request.method == "OPTIONS":
            if not is_origin_allowed(origin):
                return build_json_response(
                    403,
                    {"ok": False, "error": "origin_not_allowed", "message": "Origin is not allowed"},
                    origin,
                )
            return Response(status_code=204, headers=build_cors_headers(origin))

        response = await call_next(request)
        for name, value in build_cors_headers(origin).items():
            response.headers.setdefault(name, value)
        return response

    @app.get("/health")
    async def health() -> JSONResponse:
        playwright_ready = await check_playwright_ready_async()
        return build_json_response(
            200,
            {
                "ok": True,
                "service": SERVICE_NAME,
                "version": "1",
                "playwright_ready": playwright_ready,
            },
        )

    @app.post("/open-handoff")
    async def open_handoff(request: Request) -> JSONResponse:
        origin = request.headers.get("origin", "")
        if not is_origin_allowed(origin):
            return build_json_response(
                403,
                {"ok": False, "error": "origin_not_allowed", "message": "Origin is not allowed"},
                origin,
            )

        content_type = request.headers.get("content-type", "")
        if "application/json" not in content_type:
            return build_json_response(
                415,
                {
                    "ok": False,
                    "error": "invalid_payload",
                    "message": "Content-Type must be application/json",
                },
                origin,
            )

        payload = await request.json()
        if not isinstance(payload, dict):
            return build_json_response(
                400,
                {"ok": False, "error": "invalid_payload", "message": "Payload must be an object"},
                origin,
            )

        handoff_id = str(payload.get("handoff_id") or payload.get("handoffId") or "").strip()
        target_url = str(payload.get("url") or "").strip()
        if not handoff_id or not target_url:
            return build_json_response(
                400,
                {
                    "ok": False,
                    "error": "invalid_payload",
                    "message": "handoff_id and url are required",
                },
                origin,
            )

        if not await check_playwright_ready_async():
            return build_json_response(
                503,
                {
                    "ok": False,
                    "error": "playwright_browser_missing",
                    "message": "Chromium is not installed for the local helper",
                },
                origin,
            )

        handoff_dir = HANDOFF_ROOT / handoff_id
        payload_path = handoff_dir / "payload.json"
        profile_dir = handoff_dir / "profile"
        handoff_dir.mkdir(parents=True, exist_ok=True)
        payload_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

        try:
            launch_local_handoff_background(str(payload_path), str(profile_dir))
        except Exception as exc:
            return build_json_response(
                500,
                {
                    "ok": False,
                    "error": "browser_launch_failed",
                    "message": str(exc),
                },
                origin,
            )

        return build_json_response(
            200,
            {
                "ok": True,
                "handoff_id": handoff_id,
                "profile_dir": str(profile_dir),
                "message": "browser launched",
            },
            origin,
        )

    return app


def main() -> int:
    import uvicorn

    uvicorn.run(create_app(), host=DEFAULT_HOST, port=DEFAULT_PORT, log_level="info")
    return 0
