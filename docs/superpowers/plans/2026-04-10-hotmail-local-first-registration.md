# Hotmail Local-First Registration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Hotmail registration run in a local helper browser from the first page by default, while the backend only orchestrates tasks, stores state, and tracks results.

**Architecture:** Replace backend-owned Playwright execution with a task-oriented Hotmail orchestration API. The web page checks the local helper before starting a batch, sends task payloads to that helper, and continues to render batch state from backend callbacks and persisted records.

**Tech Stack:** Python 3, FastAPI, Playwright, pytest, Next.js App Router, TypeScript, TanStack Query

---

### Task 1: Convert Hotmail Backend Into A Task Orchestrator

**Files:**
- Modify: `vendor/codex-register/src/web/routes/hotmail.py`
- Create: `vendor/codex-register/tests/test_hotmail_local_first_routes.py`
- Modify: `vendor/codex-register/src/services/hotmail/types.py`

- [ ] **Step 1: Write the failing backend orchestration tests**

```python
from fastapi import FastAPI
from fastapi.testclient import TestClient

from src.web.routes.hotmail import router


def build_client():
    app = FastAPI()
    app.include_router(router, prefix="/api/hotmail")
    return TestClient(app)


def test_create_batch_requires_local_first_mode():
    client = build_client()

    response = client.post(
        "/api/hotmail/batches",
        json={
            "count": 1,
            "concurrency": 1,
            "interval_min": 0,
            "interval_max": 0,
            "execution_mode": "local_first",
        },
    )

    assert response.status_code == 200
    assert response.json()["status"] == "pending_local_start"
    assert response.json()["current_task"]["task_id"]


def test_helper_progress_updates_batch_state():
    client = build_client()
    batch = client.post(
        "/api/hotmail/batches",
        json={"count": 1, "concurrency": 1, "interval_min": 0, "interval_max": 0, "execution_mode": "local_first"},
    ).json()
    task_id = batch["current_task"]["task_id"]

    response = client.post(
        f"/api/hotmail/batches/{batch['batch_id']}/tasks/{task_id}/progress",
        json={"status": "running", "current_step": "submitting_profile", "log_line": "profile submitted"},
    )

    assert response.status_code == 200
    payload = client.get(f"/api/hotmail/batches/{batch['batch_id']}").json()
    assert payload["status"] == "running"
    assert payload["current_task"]["current_step"] == "submitting_profile"
    assert "profile submitted" in payload["logs"][-1]
```

- [ ] **Step 2: Run the new backend test file and confirm it fails**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_local_first_routes.py -q`

Expected: FAIL because the local-first batch/task endpoints and fields do not exist yet.

- [ ] **Step 3: Add task-oriented Hotmail batch state and callback routes**

```python
class HotmailBatchCreateRequest(BaseModel):
    count: int
    concurrency: int = 1
    interval_min: int = 1
    interval_max: int = 2
    proxy: Optional[str] = None
    execution_mode: str = "local_first"


class HotmailTaskProgressRequest(BaseModel):
    status: str
    current_step: str = ""
    manual_action_required: bool = False
    log_line: str = ""


class HotmailTaskResultRequest(BaseModel):
    success: bool
    failure_code: str = ""
    failure_message: str = ""
    artifact: Optional[dict] = None
```

```python
def _new_task(batch_id: str, request: HotmailBatchCreateRequest, index: int) -> dict:
    return {
        "task_id": str(uuid.uuid4()),
        "batch_id": batch_id,
        "status": "pending_local_start",
        "current_step": "queued",
        "manual_action_required": False,
        "failure_code": "",
        "failure_message": "",
        "proxy": request.proxy,
        "target_email": "",
        "verification_email": "",
        "artifact_path": "",
        "index": index,
    }
```

```python
@router.post("/batches")
async def create_hotmail_batch(request: HotmailBatchCreateRequest):
    batch_id = str(uuid.uuid4())
    batch = _default_batch(batch_id, request)
    batch["status"] = "pending_local_start"
    batch["current_task"] = _new_task(batch_id, request, 0)
    hotmail_batches[batch_id] = batch
    return _public_hotmail_batch(batch)


@router.post("/batches/{batch_id}/tasks/{task_id}/progress")
async def update_hotmail_task_progress(batch_id: str, task_id: str, request: HotmailTaskProgressRequest):
    batch = _get_batch_or_404(batch_id)
    task = _get_task_or_409(batch, task_id)
    task["status"] = request.status
    task["current_step"] = request.current_step
    task["manual_action_required"] = request.manual_action_required
    batch["status"] = request.status
    if request.log_line:
        batch["logs"].append(request.log_line)
    return _public_hotmail_batch(batch)
```

- [ ] **Step 4: Add result handling for helper success/failure**

```python
@router.post("/batches/{batch_id}/tasks/{task_id}/result")
async def finish_hotmail_task(batch_id: str, task_id: str, request: HotmailTaskResultRequest):
    batch = _get_batch_or_404(batch_id)
    task = _get_task_or_409(batch, task_id)
    task["status"] = "success" if request.success else "failed"
    task["failure_code"] = request.failure_code
    task["failure_message"] = request.failure_message

    if request.success and request.artifact:
        result = HotmailRegistrationResult(
            success=True,
            artifact=HotmailAccountArtifact(**request.artifact),
        )
        _record_result(batch, result)
    else:
        result = HotmailRegistrationResult(
            success=False,
            reason_code=request.failure_code,
            error_message=request.failure_message,
        )
        _record_result(batch, result)

    batch["current_task"] = _maybe_schedule_next_task(batch)
    batch["finished"] = batch["current_task"] is None
    batch["status"] = "finished" if batch["finished"] else "pending_local_start"
    return _public_hotmail_batch(batch)
```

- [ ] **Step 5: Run backend Hotmail route tests**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_local_first_routes.py vendor/codex-register/tests/test_hotmail_routes.py -q`

Expected: PASS.

- [ ] **Step 6: Commit backend orchestration changes**

```bash
git add vendor/codex-register/src/web/routes/hotmail.py vendor/codex-register/src/services/hotmail/types.py vendor/codex-register/tests/test_hotmail_local_first_routes.py
git commit -m "feat: add hotmail local-first task orchestration"
```

### Task 2: Upgrade The Local Helper Into A Full Hotmail Executor

**Files:**
- Modify: `tools/hotmail_local_helper/server.py`
- Create: `tools/hotmail_local_helper/hotmail_runner.py`
- Create: `tools/hotmail_local_helper/tests/test_hotmail_runner.py`
- Modify: `vendor/codex-register/src/services/hotmail/engine.py`

- [ ] **Step 1: Write the failing helper task tests**

```python
from fastapi.testclient import TestClient

from tools.hotmail_local_helper.server import create_app


def test_start_task_accepts_local_first_hotmail_payload(monkeypatch):
    accepted = {}

    def fake_start(payload):
        accepted.update(payload)

    monkeypatch.setattr("tools.hotmail_local_helper.server.start_hotmail_task_background", fake_start)
    monkeypatch.setattr("tools.hotmail_local_helper.server.check_playwright_ready", lambda: True)
    client = TestClient(create_app())

    response = client.post(
        "/hotmail/start-task",
        headers={"Origin": "http://192.168.5.35:48761"},
        json={"batch_id": "batch-1", "task_id": "task-1", "password": "pw", "profile": {"first_name": "A"}},
    )

    assert response.status_code == 200
    assert response.json()["ok"] is True
    assert accepted["task_id"] == "task-1"
```

- [ ] **Step 2: Run the helper tests and confirm they fail**

Run: `python3 -m pytest tools/hotmail_local_helper/tests/test_hotmail_runner.py tools/hotmail_local_helper/tests/test_server.py -q`

Expected: FAIL because `/hotmail/start-task` and helper runner code do not exist.

- [ ] **Step 3: Add a dedicated helper runner for local-first Hotmail tasks**

```python
from dataclasses import dataclass


@dataclass
class HotmailLocalTask:
    batch_id: str
    task_id: str
    password: str
    profile: dict
    proxy: str = ""
    backend_callback_base: str = ""
    backend_callback_token: str = ""
    verification_mailbox: dict | None = None


def run_hotmail_local_task(task: HotmailLocalTask) -> None:
    callback_client = build_backend_callback_client(task)
    callback_client.report_progress(task, status="running", current_step="opening_signup")
    ...
    callback_client.report_result(task, success=True, artifact=artifact)
```

- [ ] **Step 4: Expose task-oriented helper endpoints**

```python
@app.post("/hotmail/start-task")
async def start_hotmail_task(request: Request) -> JSONResponse:
    origin = request.headers.get("origin", "")
    payload = await request.json()
    task = validate_hotmail_task_payload(payload)
    if not check_playwright_ready():
        return build_json_response(503, {"ok": False, "error": "playwright_browser_missing"}, origin)
    start_hotmail_task_background(task)
    return build_json_response(200, {"ok": True, "task_id": task.task_id, "message": "task accepted"}, origin)
```

```python
@app.post("/hotmail/cancel-task")
async def cancel_hotmail_task(request: Request) -> JSONResponse:
    payload = await request.json()
    task_id = str(payload.get("task_id") or "").strip()
    cancel_hotmail_task_state(task_id)
    return build_json_response(200, {"ok": True, "task_id": task_id, "message": "cancel requested"})
```

- [ ] **Step 5: Refactor the Playwright Hotmail engine for helper-owned execution**

```python
class HotmailRegistrationEngine:
    def run_local_first(self, *, callback_reporter=None) -> HotmailRegistrationResult:
        with self.browser_factory(proxy_url=self.proxy_url, callback_logger=self.callback_logger) as session:
            callback_reporter and callback_reporter("running", "opening_signup")
            session.open_signup()
            ...
            callback_reporter and callback_reporter("running", "waiting_email_code")
```

Keep the older `run()` path temporarily as a wrapper or compatibility shim until Task 4 removes container-side browser ownership from the UI path.

- [ ] **Step 6: Run helper and Hotmail engine tests**

Run: `python3 -m pytest tools/hotmail_local_helper/tests/test_hotmail_runner.py tools/hotmail_local_helper/tests/test_server.py vendor/codex-register/tests/test_hotmail_engine.py -q`

Expected: PASS.

- [ ] **Step 7: Commit helper execution changes**

```bash
git add tools/hotmail_local_helper/server.py tools/hotmail_local_helper/hotmail_runner.py tools/hotmail_local_helper/tests/test_hotmail_runner.py vendor/codex-register/src/services/hotmail/engine.py
git commit -m "feat: add hotmail local-first helper execution"
```

### Task 3: Switch The Hotmail Page To Local-First Start Semantics

**Files:**
- Modify: `apps/src/app/hotmail/page.tsx`
- Modify: `apps/src/lib/api/account-client.ts`
- Modify: `apps/src/types/index.ts`
- Modify: `apps/src/lib/api/hotmail-local-helper.ts`
- Modify: `apps/src/app/hotmail/hotmail-batch-state.ts`
- Create: `apps/src/lib/api/hotmail-local-helper-task.test.ts`

- [ ] **Step 1: Write the failing frontend tests**

```ts
import test from "node:test";
import assert from "node:assert/strict";

import { buildHotmailBatchStatusText } from "./hotmail-batch-state.ts";

test("local-first batches show running-on-local-machine hint", () => {
  assert.equal(
    buildHotmailBatchStatusText({
      status: "running",
      currentTask: { status: "running", currentStep: "opening_signup" },
      executionMode: "local_first",
    }),
    "正在本机执行",
  );
});
```

- [ ] **Step 2: Run the targeted frontend tests and confirm they fail**

Run: `cd apps && node --test src/app/hotmail/hotmail-batch-state.test.ts src/lib/api/hotmail-local-helper.test.ts src/lib/api/hotmail-local-helper-task.test.ts`

Expected: FAIL because local-first batch/task fields and helper task APIs do not exist.

- [ ] **Step 3: Add task-aware frontend types and helper client methods**

```ts
export interface RegisterHotmailTaskSnapshot {
  taskId: string;
  status: string;
  currentStep: string;
  manualActionRequired: boolean;
  failureCode: string;
  failureMessage: string;
}
```

```ts
export const hotmailLocalHelperClient = {
  ...
  async startTask(payload: HotmailLocalFirstTaskPayload) {
    const response = await fetch(buildHotmailLocalHelperUrl("/hotmail/start-task"), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    return normalizeHotmailLocalHelperLaunchResult(await readJsonResponse(response));
  },
};
```

- [ ] **Step 4: Change `开始批次` to require helper health before backend batch creation**

```tsx
const createMutation = useMutation({
  mutationFn: async () => {
    const health = await hotmailLocalHelperClient.health();
    if (!health.ok) {
      throw new Error("Hotmail helper 不可用");
    }
    if (!health.playwrightReady) {
      throw new Error("Hotmail helper 缺少 Playwright Chromium");
    }

    const batch = await accountClient.startRegisterHotmailBatch({
      count: ...,
      concurrency: ...,
      intervalMin: ...,
      intervalMax: ...,
      proxy: proxy.trim() || null,
      executionMode: "local_first",
    });

    if (batch.currentTask) {
      await hotmailLocalHelperClient.startTask(batch.currentTaskPayload);
    }
    return batch;
  },
```

- [ ] **Step 5: Replace handoff-first UI copy with local-first state copy**

```tsx
{currentBatch?.executionMode === "local_first" && currentBatch.currentTask ? (
  <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm">
    <p className="font-medium">
      {currentBatch.currentTask.manualActionRequired
        ? "请在已打开的本机浏览器窗口中继续处理微软验证"
        : "正在本机执行，请勿关闭本机浏览器窗口"}
    </p>
  </div>
) : null}
```

Keep the existing handoff UI only as a compatibility fallback while old batches are still readable.

- [ ] **Step 6: Run lint and desktop build**

Run: `cd apps && pnpm exec eslint src/app/hotmail/page.tsx src/app/hotmail/hotmail-batch-state.ts src/lib/api/hotmail-local-helper.ts src/lib/api/account-client.ts`

Expected: PASS.

Run: `cd apps && pnpm run build:desktop`

Expected: PASS.

- [ ] **Step 7: Commit frontend local-first changes**

```bash
git add apps/src/app/hotmail/page.tsx apps/src/lib/api/account-client.ts apps/src/types/index.ts apps/src/lib/api/hotmail-local-helper.ts apps/src/app/hotmail/hotmail-batch-state.ts apps/src/lib/api/hotmail-local-helper-task.test.ts
git commit -m "feat: switch hotmail page to local-first execution"
```

### Task 4: Finalize Helper Setup Docs And Full Verification

**Files:**
- Modify: `tools/hotmail_local_helper/README.md`
- Modify: `docs/superpowers/specs/2026-04-10-hotmail-local-first-registration-design.md`
- Create: `docs/superpowers/plans/2026-04-10-hotmail-local-first-registration.md`

- [ ] **Step 1: Update helper README for local-first usage**

```md
## Hotmail Local-First Mode

Start the helper on the same machine that opens the Codex Manager Hotmail page:

```bash
cd tools/hotmail_local_helper
python3 -m venv .venv
source .venv/bin/activate
pip install -r ../../vendor/codex-register/requirements.txt
playwright install chromium
cd ../..
python3 -m tools.hotmail_local_helper
```

Then open `http://127.0.0.1:16788/health`.
```

- [ ] **Step 2: Run Python verification**

Run: `python3 -m pytest tools/hotmail_local_helper/tests/test_server.py tools/hotmail_local_helper/tests/test_hotmail_runner.py vendor/codex-register/tests/test_hotmail_engine.py vendor/codex-register/tests/test_hotmail_routes.py vendor/codex-register/tests/test_hotmail_local_first_routes.py -q`

Expected: PASS.

- [ ] **Step 3: Run frontend verification**

Run: `cd apps && node --test src/app/hotmail/hotmail-batch-state.test.ts src/lib/api/hotmail-local-helper.test.ts src/lib/api/hotmail-local-helper-task.test.ts`

Expected: PASS.

Run: `cd apps && pnpm run build:desktop`

Expected: PASS.

- [ ] **Step 4: Perform manual verification**

Manual checks:

1. Start the helper locally
2. Open the Docker-hosted Hotmail page from the same machine
3. Click `开始批次`
4. Confirm the page refuses to start if helper is unavailable
5. Confirm Chromium opens locally before any Microsoft challenge page appears
6. Confirm the page shows `正在本机执行`
7. Confirm a manual verification page, if it appears, is already local and no `本机接管（Web）` step is needed

- [ ] **Step 5: Commit documentation touch-ups**

```bash
git add tools/hotmail_local_helper/README.md docs/superpowers/specs/2026-04-10-hotmail-local-first-registration-design.md
git commit -m "docs: add hotmail local-first helper usage notes"
```
