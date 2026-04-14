"""
CPA-oriented register runtime helpers.

This module starts the migration by normalizing callback handling around the
`openai-cpa` parsing semantics, while leaving the rest of the current register
flow intact for now.
"""

from __future__ import annotations

import urllib.parse
from typing import Any, Dict


def resolve_callback_payload(callback_url: str) -> Dict[str, str]:
    candidate = str(callback_url or "").strip()
    if not candidate:
        return {"code": "", "state": "", "error": "", "error_description": ""}

    if "://" not in candidate:
        if candidate.startswith("?"):
            candidate = f"http://localhost{candidate}"
        elif any(ch in candidate for ch in "/?#") or ":" in candidate:
            candidate = f"http://{candidate}"
        elif "=" in candidate:
            candidate = f"http://localhost/?{candidate}"

    parsed = urllib.parse.urlparse(candidate)
    query = urllib.parse.parse_qs(parsed.query, keep_blank_values=True)
    fragment = urllib.parse.parse_qs(parsed.fragment, keep_blank_values=True)

    for key, values in fragment.items():
        if key not in query or not query[key] or not (query[key][0] or "").strip():
            query[key] = values

    def get1(key: str) -> str:
        values = query.get(key, [""])
        return str(values[0] or "").strip()

    code = get1("code")
    state = get1("state")
    error = get1("error")
    error_description = get1("error_description")

    if code and not state and "#" in code:
        code, state = code.split("#", 1)

    if not error and error_description:
        error, error_description = error_description, ""

    return {
        "code": code,
        "state": state,
        "error": error,
        "error_description": error_description,
    }


class CPARegisterRuntime:
    def __init__(self, engine: Any):
        self.engine = engine

    def normalize_callback_url(self, callback_url: str) -> str:
        raw = str(callback_url or "").strip()
        payload = resolve_callback_payload(raw)
        if not payload["code"] and not payload["error"]:
            return raw

        oauth_start = getattr(self.engine, "oauth_start", None)
        redirect_uri = str(
            getattr(oauth_start, "redirect_uri", "")
            or getattr(getattr(self.engine, "oauth_manager", None), "redirect_uri", "")
            or "http://localhost:1455/auth/callback"
        ).strip()

        params = {}
        for key in ("code", "state", "error", "error_description"):
            value = payload.get(key)
            if value:
                params[key] = value

        encoded = urllib.parse.urlencode(params)
        if not encoded:
            return raw
        return f"{redirect_uri}?{encoded}"

    def handle_oauth_callback(self, callback_url: str) -> Dict[str, Any]:
        normalized = self.normalize_callback_url(callback_url)
        return self.engine.oauth_manager.handle_callback(
            callback_url=normalized,
            expected_state=self.engine.oauth_start.state,
            code_verifier=self.engine.oauth_start.code_verifier,
        )
