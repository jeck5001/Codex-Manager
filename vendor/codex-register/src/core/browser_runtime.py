"""Shared helpers for Playwright/patchright browser launches in the register flow."""

from __future__ import annotations

import os
from typing import Optional

REGISTER_CHROME_EXECUTABLE_PATH_ENV = "REGISTER_CHROME_EXECUTABLE_PATH"

_DEFAULT_CHROME_CANDIDATES = (
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/opt/google/chrome/chrome",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
)


def resolve_register_chrome_executable_path() -> Optional[str]:
    candidate = (os.environ.get(REGISTER_CHROME_EXECUTABLE_PATH_ENV) or "").strip()
    if candidate and os.path.exists(candidate):
        return candidate
    for default in _DEFAULT_CHROME_CANDIDATES:
        if os.path.exists(default):
            return default
    return None
