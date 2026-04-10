# Hotmail Web Local Helper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a localhost helper flow so the Docker-hosted Hotmail page can launch the manual takeover browser on the operator's current machine through a small local HTTP service.

**Architecture:** Keep the existing register-side `local_handoff` payload unchanged and add a separate Python helper that runs on `127.0.0.1:16788`. The web frontend probes the helper, launches it with the current batch payload, and shows precise setup guidance when the helper is missing or not ready.

**Tech Stack:** Python 3, Playwright, pytest, Next.js App Router, TypeScript, TanStack Query, Tauri-safe frontend APIs

---

### Task 1: Build The Local Helper Service

**Files:**
- Create: `tools/hotmail_local_helper/__init__.py`
- Create: `tools/hotmail_local_helper/server.py`
- Create: `tools/hotmail_local_helper/__main__.py`
- Create: `tools/hotmail_local_helper/README.md`
- Create: `tools/hotmail_local_helper/tests/test_server.py`
- Modify: `vendor/codex-register/src/services/hotmail/local_handoff_cli.py`

- [ ] **Step 1: Write the failing helper tests**

```python
from fastapi.testclient import TestClient

from tools.hotmail_local_helper.server import create_app


def test_health_reports_ready_when_browser_check_passes(monkeypatch):
    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.check_playwright_ready",
        lambda: True,
    )
    client = TestClient(create_app())

    response = client.get("/health", headers={"Origin": "http://127.0.0.1:48761"})

    assert response.status_code == 200
    assert response.json()["ok"] is True
    assert response.json()["playwright_ready"] is True


def test_open_handoff_rejects_disallowed_origin():
    client = TestClient(create_app())

    response = client.post(
        "/open-handoff",
        headers={"Origin": "http://evil.example"},
        json={"handoff_id": "abc", "url": "https://signup.live.com"},
    )

    assert response.status_code == 403
    assert response.json()["error"] == "origin_not_allowed"


def test_open_handoff_invokes_launcher(monkeypatch, tmp_path):
    launched = {}

    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.check_playwright_ready",
        lambda: True,
    )

    def fake_launch(payload_path: str, profile_dir: str) -> None:
        launched["payload_path"] = payload_path
        launched["profile_dir"] = profile_dir

    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.launch_local_handoff",
        fake_launch,
    )
    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.HANDOFF_ROOT",
        tmp_path,
    )

    client = TestClient(create_app())
    response = client.post(
        "/open-handoff",
        headers={"Origin": "http://127.0.0.1:48761"},
        json={"handoff_id": "abc", "url": "https://signup.live.com"},
    )

    assert response.status_code == 200
    assert response.json()["ok"] is True
    assert launched["payload_path"].endswith("payload.json")
```

- [ ] **Step 2: Run the helper tests to confirm they fail**

Run: `python -m pytest tools/hotmail_local_helper/tests/test_server.py -q`

Expected: FAIL with import errors because the helper package does not exist yet.

- [ ] **Step 3: Add the helper server and CLI entrypoint**

```python
from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from urllib.parse import urlparse

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse
from fastapi.middleware.cors import CORSMiddleware

ROOT = Path(__file__).resolve().parents[2]
REGISTER_ROOT = ROOT / "vendor" / "codex-register"
if str(REGISTER_ROOT) not in sys.path:
    sys.path.insert(0, str(REGISTER_ROOT))

from src.services.hotmail.local_handoff_cli import launch_local_handoff

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 16788
HANDOFF_ROOT = Path(os.environ.get("CODEXMANAGER_HOTMAIL_HELPER_ROOT", "/tmp/codex-hotmail-handoff"))


def check_playwright_ready() -> bool:
    try:
        from playwright.sync_api import sync_playwright

        with sync_playwright() as playwright:
            return bool(playwright.chromium)
    except Exception:
        return False


def is_origin_allowed(origin: str) -> bool:
    parsed = urlparse(origin)
    if parsed.scheme != "http":
        return False
    return parsed.hostname in {"127.0.0.1", "localhost"}


def create_app() -> FastAPI:
    app = FastAPI()
    app.add_middleware(
        CORSMiddleware,
        allow_origins=[
            "http://127.0.0.1",
            "http://localhost",
            "http://127.0.0.1:48761",
            "http://localhost:48761",
        ],
        allow_credentials=False,
        allow_methods=["GET", "POST"],
        allow_headers=["Content-Type"],
    )

    @app.get("/health")
    def health() -> dict[str, object]:
        return {
            "ok": True,
            "service": "hotmail-local-helper",
            "version": "1",
            "playwright_ready": check_playwright_ready(),
        }

    @app.post("/open-handoff")
    async def open_handoff(request: Request) -> JSONResponse:
        origin = request.headers.get("origin", "")
        if not is_origin_allowed(origin):
            return JSONResponse(
                status_code=403,
                content={"ok": False, "error": "origin_not_allowed", "message": "Origin is not allowed"},
            )

        payload = await request.json()
        handoff_id = str(payload.get("handoff_id") or "").strip()
        target_url = str(payload.get("url") or "").strip()
        if not handoff_id or not target_url:
            return JSONResponse(
                status_code=400,
                content={"ok": False, "error": "invalid_payload", "message": "handoff_id and url are required"},
            )

        handoff_dir = HANDOFF_ROOT / handoff_id
        payload_path = handoff_dir / "payload.json"
        profile_dir = handoff_dir / "profile"
        handoff_dir.mkdir(parents=True, exist_ok=True)
        payload_path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")

        try:
            launch_local_handoff(str(payload_path), str(profile_dir))
        except Exception as exc:
            return JSONResponse(
                status_code=500,
                content={"ok": False, "error": "browser_launch_failed", "message": str(exc)},
            )

        return JSONResponse(
            status_code=200,
            content={
                "ok": True,
                "handoff_id": handoff_id,
                "profile_dir": str(profile_dir),
                "message": "browser launched",
            },
        )

    return app
```

- [ ] **Step 4: Make the launcher reusable from the helper**

```python
def launch_local_handoff(payload_path: str, profile_dir: str, wait_for_seconds: int = 300) -> None:
    from playwright.sync_api import sync_playwright

    payload = _load_payload(payload_path)
    ...
    with sync_playwright() as playwright:
        context = playwright.chromium.launch_persistent_context(**launch_options)
        try:
            ...
            page.goto(target_url, wait_until="domcontentloaded", timeout=60_000)
            page.wait_for_timeout(wait_for_seconds * 1000)
        finally:
            context.close()
```

The helper path should call the same function with the default wait window so desktop handoff behavior stays unchanged.

- [ ] **Step 5: Run helper tests and fix failures**

Run: `python -m pytest tools/hotmail_local_helper/tests/test_server.py -q`

Expected: PASS with 3 passing tests.

- [ ] **Step 6: Commit the helper service**

```bash
git add tools/hotmail_local_helper vendor/codex-register/src/services/hotmail/local_handoff_cli.py
git commit -m "feat: add hotmail web local helper service"
```

### Task 2: Add Frontend Helper Client And State Utilities

**Files:**
- Create: `apps/src/lib/api/hotmail-local-helper.ts`
- Create: `apps/src/lib/api/hotmail-local-helper.test.ts`
- Modify: `apps/src/app/hotmail/hotmail-batch-state.ts`
- Modify: `apps/src/types/index.ts`

- [ ] **Step 1: Write the failing frontend utility tests**

```ts
import { describe, expect, it } from "node:test";

import {
  buildHotmailWebLocalHelperUrl,
  buildHotmailWebLocalHandoffActionState,
} from "./hotmail-batch-state";

describe("buildHotmailWebLocalHandoffActionState", () => {
  it("enables web local handoff for browser runtime batches", () => {
    expect(
      buildHotmailWebLocalHandoffActionState(
        {
          status: "action_required",
          handoffId: "abc",
          localHandoff: { handoffId: "abc", url: "https://signup.live.com" },
        },
        false,
      ),
    ).toEqual({ enabled: true, reason: "" });
  });
});

describe("buildHotmailWebLocalHelperUrl", () => {
  it("uses the default localhost helper endpoint", () => {
    expect(buildHotmailWebLocalHelperUrl("/health")).toBe("http://127.0.0.1:16788/health");
  });
});
```

- [ ] **Step 2: Run the targeted frontend tests to confirm they fail**

Run: `cd apps && node --test src/lib/api/hotmail-local-helper.test.ts src/app/hotmail/hotmail-batch-state.test.ts`

Expected: FAIL because the helper client and web handoff state helpers do not exist yet.

- [ ] **Step 3: Add a dedicated localhost helper client**

```ts
import { RegisterHotmailLocalHandoff } from "../../types";

export interface HotmailLocalHelperHealth {
  ok: boolean;
  service: string;
  version: string;
  playwrightReady: boolean;
}

export interface HotmailLocalHelperLaunchResult {
  ok: boolean;
  handoffId?: string;
  profileDir?: string;
  message?: string;
  error?: string;
}

const HOTMAIL_LOCAL_HELPER_BASE = "http://127.0.0.1:16788";

export function buildHotmailLocalHelperUrl(path: string) {
  return `${HOTMAIL_LOCAL_HELPER_BASE}${path}`;
}

export const hotmailLocalHelperClient = {
  async health(): Promise<HotmailLocalHelperHealth> {
    const response = await fetch(buildHotmailLocalHelperUrl("/health"));
    return response.json();
  },
  async openHandoff(payload: RegisterHotmailLocalHandoff) {
    const response = await fetch(buildHotmailLocalHelperUrl("/open-handoff"), {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });
    return response.json() as Promise<HotmailLocalHelperLaunchResult>;
  },
};
```

- [ ] **Step 4: Extend batch-state helpers for web runtime**

```ts
export function buildHotmailWebLocalHandoffActionState(
  batch: Pick<
    HotmailBatchStatusLike,
    "status" | "actionRequiredReason" | "handoffId" | "localHandoff"
  > | null,
  isDesktopRuntime: boolean,
) {
  if (!hasHotmailPendingLocalHandoff(batch)) {
    return {
      enabled: false,
      reason: "当前批次没有可用的本机接管数据",
    };
  }
  if (isDesktopRuntime) {
    return {
      enabled: false,
      reason: "桌面版应使用本地接管入口",
    };
  }
  return {
    enabled: true,
    reason: "",
  };
}

export function buildHotmailWebLocalHelperUrl(path: string) {
  return `http://127.0.0.1:16788${path}`;
}
```

- [ ] **Step 5: Run frontend tests**

Run: `cd apps && node --test src/lib/api/hotmail-local-helper.test.ts src/app/hotmail/hotmail-batch-state.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit the frontend helper utilities**

```bash
git add apps/src/lib/api/hotmail-local-helper.ts apps/src/lib/api/hotmail-local-helper.test.ts apps/src/app/hotmail/hotmail-batch-state.ts apps/src/types/index.ts
git commit -m "feat: add hotmail web helper client"
```

### Task 3: Wire The Hotmail Page To The Web Helper

**Files:**
- Modify: `apps/src/app/hotmail/page.tsx`
- Test: `apps/src/app/hotmail/navigation.test.ts`
- Test: `apps/src/app/hotmail/hotmail-batch-state.test.ts`

- [ ] **Step 1: Write the failing page interaction test**

```ts
it("shows web local handoff guidance when helper is unavailable", async () => {
  mockHotmailLocalHelperHealth.rejects(new Error("fetch failed"));
  render(<HotmailPage />);

  await user.click(screen.getByRole("button", { name: "本机接管（Web）" }));

  expect(
    await screen.findByText("请先在当前访问页面的这台机器上启动 Hotmail 本机接管 helper"),
  ).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the page tests and confirm they fail**

Run: `cd apps && node --test src/app/hotmail/navigation.test.ts src/app/hotmail/hotmail-batch-state.test.ts`

Expected: FAIL because the Hotmail page does not yet expose the web helper flow.

- [ ] **Step 3: Add the web handoff mutation and setup guidance UI**

```tsx
const webLocalHandoffMutation = useMutation({
  mutationFn: async () => {
    const payload = batchQuery.data?.localHandoff;
    if (!payload || typeof window === "undefined") {
      throw new Error("当前批次没有可用的本机接管数据");
    }

    const health = await hotmailLocalHelperClient.health();
    if (!health.ok || !health.playwrightReady) {
      throw new Error("hotmail_local_helper_not_ready");
    }

    return hotmailLocalHelperClient.openHandoff(payload);
  },
  onSuccess: () => {
    toast.success("已在本机启动接管浏览器，请处理微软验证后回到这里继续注册");
  },
  onError: (error: unknown) => {
    setShowWebHelperGuide(true);
    toast.error(`本机接管启动失败: ${getAppErrorMessage(error)}`);
  },
});
```

```tsx
{showWebHelperGuide ? (
  <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm">
    <p>请先在当前访问页面的这台机器上启动 Hotmail 本机接管 helper。</p>
    <p>健康检查地址：`http://127.0.0.1:16788/health`</p>
    <p>启动后再点击 `本机接管（Web）`。</p>
  </div>
) : null}
```

- [ ] **Step 4: Keep desktop handoff behavior intact**

```tsx
{isDesktopRuntime ? (
  <Button
    onClick={() => localHandoffMutation.mutate()}
    disabled={!localHandoffAction.enabled || localHandoffMutation.isPending}
  >
    本地接管（推荐）
  </Button>
) : (
  <Button
    onClick={() => webLocalHandoffMutation.mutate()}
    disabled={!webLocalHandoffAction.enabled || webLocalHandoffMutation.isPending}
  >
    本机接管（Web）
  </Button>
)}
```

- [ ] **Step 5: Run lint and frontend build verification**

Run: `cd apps && pnpm exec eslint src/app/hotmail/page.tsx src/app/hotmail/hotmail-batch-state.ts src/lib/api/hotmail-local-helper.ts src/lib/api/app-client.ts`

Expected: PASS.

Run: `cd apps && pnpm run build:desktop`

Expected: PASS.

- [ ] **Step 6: Commit the page wiring**

```bash
git add apps/src/app/hotmail/page.tsx apps/src/app/hotmail/navigation.test.ts apps/src/app/hotmail/hotmail-batch-state.test.ts
git commit -m "feat: add hotmail web local handoff flow"
```

### Task 4: Document Setup And Run Full Verification

**Files:**
- Modify: `tools/hotmail_local_helper/README.md`
- Modify: `docs/superpowers/specs/2026-04-09-hotmail-web-local-helper-design.md`
- Create: `docs/superpowers/plans/2026-04-09-hotmail-web-local-helper.md`

- [ ] **Step 1: Add concrete helper setup instructions**

```md
## Quick Start

```bash
cd tools/hotmail_local_helper
python -m venv .venv
source .venv/bin/activate
pip install fastapi uvicorn playwright
playwright install chromium
python -m tools.hotmail_local_helper
```

Then open `http://127.0.0.1:16788/health` on the same machine that will visit Codex Manager.
```

- [ ] **Step 2: Run the Python and frontend verification suite**

Run: `python -m pytest tools/hotmail_local_helper/tests/test_server.py vendor/codex-register/tests/test_hotmail_engine.py vendor/codex-register/tests/test_hotmail_routes.py -q`

Expected: PASS.

Run: `cd apps && node --test src/app/hotmail/hotmail-batch-state.test.ts src/app/hotmail/navigation.test.ts src/lib/api/hotmail-local-helper.test.ts`

Expected: PASS.

- [ ] **Step 3: Perform manual verification**

Run:

```bash
cd tools/hotmail_local_helper
python -m tools.hotmail_local_helper
```

Expected: helper listens on `127.0.0.1:16788`.

Manual checks:

- visit the Docker-hosted Hotmail page from the same machine
- pause a batch on manual challenge
- click `本机接管（Web）`
- confirm local Chromium opens
- stop the helper and confirm the page shows actionable setup guidance

- [ ] **Step 4: Commit docs and verification touch-ups**

```bash
git add tools/hotmail_local_helper/README.md docs/superpowers/specs/2026-04-09-hotmail-web-local-helper-design.md
git commit -m "docs: add hotmail web helper setup notes"
```
