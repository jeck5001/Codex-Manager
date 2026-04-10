# Hotmail Auto Registration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Hotmail batch registration tool that creates Microsoft consumer accounts automatically, verifies them through the existing email-service pool, fails fast on phone verification, and exports successful accounts as JSON and TXT.

**Architecture:** Add a dedicated `src/services/hotmail` package for account-generation rules, browser signup automation, verification-mailbox integration, and artifact formatting. Expose the feature through a new FastAPI route module plus a standalone Hotmail page and JS client, while reusing the existing task manager, batch polling patterns, and Outlook import-compatible output format.

**Tech Stack:** FastAPI, Jinja2 templates, vanilla JS, Playwright, existing Codex Register task manager, pytest/unittest.

---

## File Map

### New files

- `vendor/codex-register/src/services/hotmail/__init__.py`
  - package exports for the hotmail feature
- `vendor/codex-register/src/services/hotmail/types.py`
  - request/result dataclasses, failure codes, artifact records
- `vendor/codex-register/src/services/hotmail/profile.py`
  - username/domain policy, password/profile generation helpers
- `vendor/codex-register/src/services/hotmail/verification.py`
  - wrapper over existing email services for Microsoft verification mailboxes
- `vendor/codex-register/src/services/hotmail/engine.py`
  - single-account Playwright signup engine and failure classification
- `vendor/codex-register/src/services/hotmail/artifacts.py`
  - JSON/TXT export generation and artifact path helpers
- `vendor/codex-register/src/web/routes/hotmail.py`
  - batch creation, status, cancel, artifact download APIs
- `vendor/codex-register/templates/hotmail.html`
  - standalone Hotmail page
- `vendor/codex-register/static/js/hotmail.js`
  - page logic, polling, logs, artifact download handling
- `vendor/codex-register/tests/test_hotmail_profile.py`
  - domain policy/profile tests
- `vendor/codex-register/tests/test_hotmail_verification.py`
  - verification-mailbox selection and timeout tests
- `vendor/codex-register/tests/test_hotmail_engine.py`
  - engine state/failure classification tests
- `vendor/codex-register/tests/test_hotmail_routes.py`
  - route and batch orchestration tests

### Modified files

- `vendor/codex-register/src/services/__init__.py`
  - export hotmail package symbols if needed by route code
- `vendor/codex-register/src/web/routes/__init__.py`
  - register the Hotmail route module
- `vendor/codex-register/src/web/app.py`
  - add standalone `/hotmail` page route

---

### Task 1: Define Hotmail Domain Policy and Result Types

**Files:**
- Create: `vendor/codex-register/src/services/hotmail/types.py`
- Create: `vendor/codex-register/src/services/hotmail/profile.py`
- Create: `vendor/codex-register/tests/test_hotmail_profile.py`
- Create: `vendor/codex-register/src/services/hotmail/__init__.py`
- Modify: `vendor/codex-register/src/services/__init__.py`

- [ ] **Step 1: Write the failing tests for domain fallback, username normalization, and TXT export compatibility**

```python
import unittest

from src.services.hotmail.profile import (
    HOTMAIL_DOMAIN_POLICY,
    build_username_candidates,
    choose_target_domains,
)
from src.services.hotmail.types import HotmailAccountArtifact, HotmailFailureCode


class HotmailProfileTests(unittest.TestCase):
    def test_choose_target_domains_prefers_hotmail_then_outlook(self):
        self.assertEqual(
            choose_target_domains(),
            ["hotmail.com", "outlook.com"],
        )

    def test_build_username_candidates_normalizes_ascii_safe_values(self):
        candidates = build_username_candidates("Alice", "Example", seed="ab12")
        self.assertIn("aliceexampleab12", candidates)
        self.assertTrue(all("@" not in item for item in candidates))

    def test_hotmail_artifact_txt_line_matches_outlook_import_format(self):
        artifact = HotmailAccountArtifact(
            email="demo@hotmail.com",
            password="StrongPassw0rd!",
            target_domain="hotmail.com",
            verification_email="code@temp.example.com",
        )
        self.assertEqual(artifact.to_txt_line(), "demo@hotmail.com----StrongPassw0rd!")

    def test_failure_code_phone_verification_is_stable(self):
        self.assertEqual(
            HotmailFailureCode.PHONE_VERIFICATION_REQUIRED.value,
            "phone_verification_required",
        )
```

- [ ] **Step 2: Run the profile test file to verify it fails**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_profile.py -q`

Expected: FAIL with import errors because the new hotmail package and types do not exist yet.

- [ ] **Step 3: Write the minimal hotmail types and profile helpers**

```python
# vendor/codex-register/src/services/hotmail/types.py
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class HotmailFailureCode(str, Enum):
    PHONE_VERIFICATION_REQUIRED = "phone_verification_required"
    UNSUPPORTED_CHALLENGE = "unsupported_challenge"
    EMAIL_VERIFICATION_TIMEOUT = "email_verification_timeout"
    USERNAME_UNAVAILABLE_EXHAUSTED = "username_unavailable_exhausted"
    PROXY_ERROR = "proxy_error"
    PAGE_STRUCTURE_CHANGED = "page_structure_changed"
    BROWSER_TIMEOUT = "browser_timeout"
    UNEXPECTED_EXCEPTION = "unexpected_exception"


@dataclass
class HotmailAccountArtifact:
    email: str
    password: str
    target_domain: str
    verification_email: str = ""
    first_name: str = ""
    last_name: str = ""

    def to_txt_line(self) -> str:
        return f"{self.email}----{self.password}"
```

```python
# vendor/codex-register/src/services/hotmail/profile.py
import random
import re
import string
from typing import List


HOTMAIL_DOMAIN_POLICY = ("hotmail.com", "outlook.com")


def choose_target_domains() -> List[str]:
    return list(HOTMAIL_DOMAIN_POLICY)


def _slug(value: str) -> str:
    lowered = str(value or "").strip().lower()
    return re.sub(r"[^a-z0-9]+", "", lowered)


def build_username_candidates(first_name: str, last_name: str, seed: str) -> List[str]:
    base = f"{_slug(first_name)}{_slug(last_name)}{_slug(seed)}"
    trimmed = base[:28] or "".join(random.choice(string.ascii_lowercase) for _ in range(10))
    return [trimmed, f"{trimmed}1", f"{trimmed}01"]
```

```python
# vendor/codex-register/src/services/hotmail/__init__.py
from .profile import HOTMAIL_DOMAIN_POLICY, build_username_candidates, choose_target_domains
from .types import HotmailAccountArtifact, HotmailFailureCode
```

- [ ] **Step 4: Export the new package from the service layer**

```python
# vendor/codex-register/src/services/__init__.py
from .hotmail import (
    HOTMAIL_DOMAIN_POLICY,
    HotmailAccountArtifact,
    HotmailFailureCode,
    build_username_candidates,
    choose_target_domains,
)
```

- [ ] **Step 5: Run the profile tests to verify they pass**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_profile.py -q`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add vendor/codex-register/src/services/__init__.py \
  vendor/codex-register/src/services/hotmail/__init__.py \
  vendor/codex-register/src/services/hotmail/types.py \
  vendor/codex-register/src/services/hotmail/profile.py \
  vendor/codex-register/tests/test_hotmail_profile.py
git commit -m "feat: add hotmail profile policy"
```

### Task 2: Add Verification Mailbox Integration for Microsoft Email Codes

**Files:**
- Create: `vendor/codex-register/src/services/hotmail/verification.py`
- Create: `vendor/codex-register/tests/test_hotmail_verification.py`

- [ ] **Step 1: Write the failing verification-mailbox tests**

```python
import types
import unittest

from src.services.hotmail.verification import HotmailVerificationMailboxProvider


class HotmailVerificationTests(unittest.TestCase):
    def test_provider_prefers_temp_mail_like_services_for_verification(self):
        service = types.SimpleNamespace(
            name="temp-mail-1",
            service_type="temp_mail",
            enabled=True,
            config={"domain": "tm.example.com"},
        )
        provider = HotmailVerificationMailboxProvider(
            list_enabled_services=lambda: [service],
            create_email_service=lambda selected: selected,
        )
        mailbox = provider.acquire_mailbox()
        self.assertEqual(mailbox.name, "temp-mail-1")

    def test_provider_raises_when_no_supported_service_exists(self):
        provider = HotmailVerificationMailboxProvider(
            list_enabled_services=lambda: [],
            create_email_service=lambda selected: selected,
        )
        with self.assertRaisesRegex(RuntimeError, "No supported verification mailbox service"):
            provider.acquire_mailbox()
```

- [ ] **Step 2: Run the verification tests to verify they fail**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_verification.py -q`

Expected: FAIL because `verification.py` does not exist.

- [ ] **Step 3: Implement the mailbox provider with existing email-service compatibility**

```python
# vendor/codex-register/src/services/hotmail/verification.py
from typing import Callable, Iterable, Any


SUPPORTED_VERIFICATION_SERVICE_TYPES = ("temp_mail", "custom_domain", "tempmail")


class HotmailVerificationMailboxProvider:
    def __init__(self, *, list_enabled_services: Callable[[], Iterable[Any]], create_email_service: Callable[[Any], Any]):
        self._list_enabled_services = list_enabled_services
        self._create_email_service = create_email_service

    def _choose_service(self) -> Any:
        for service in self._list_enabled_services():
            if getattr(service, "enabled", True) and getattr(service, "service_type", "") in SUPPORTED_VERIFICATION_SERVICE_TYPES:
                return service
        raise RuntimeError("No supported verification mailbox service")

    def acquire_mailbox(self) -> Any:
        service = self._choose_service()
        return self._create_email_service(service)
```

- [ ] **Step 4: Run the verification tests to verify they pass**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_verification.py -q`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add vendor/codex-register/src/services/hotmail/verification.py \
  vendor/codex-register/tests/test_hotmail_verification.py
git commit -m "feat: add hotmail verification mailbox provider"
```

### Task 3: Implement the Single-Account Hotmail Engine

**Files:**
- Create: `vendor/codex-register/src/services/hotmail/engine.py`
- Create: `vendor/codex-register/tests/test_hotmail_engine.py`

- [ ] **Step 1: Write failing engine tests for failure classification and domain fallback**

```python
import unittest

from src.services.hotmail.engine import (
    HotmailRegistrationEngine,
    classify_hotmail_page_state,
)
from src.services.hotmail.types import HotmailFailureCode


class HotmailEngineTests(unittest.TestCase):
    def test_classify_phone_verification_page(self):
        self.assertEqual(
            classify_hotmail_page_state("Add a phone number to help keep your account secure"),
            HotmailFailureCode.PHONE_VERIFICATION_REQUIRED,
        )

    def test_classify_unsupported_challenge(self):
        self.assertEqual(
            classify_hotmail_page_state("Complete the puzzle to continue"),
            HotmailFailureCode.UNSUPPORTED_CHALLENGE,
        )

    def test_engine_tries_outlook_after_hotmail_availability_failure(self):
        engine = HotmailRegistrationEngine.__new__(HotmailRegistrationEngine)
        engine._attempt_domain = lambda domain: domain == "outlook.com"
        self.assertEqual(engine._choose_domain_by_attempt(), "outlook.com")
```

- [ ] **Step 2: Run the engine tests to verify they fail**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_engine.py -q`

Expected: FAIL because `engine.py` does not exist.

- [ ] **Step 3: Implement the minimal engine skeleton**

```python
# vendor/codex-register/src/services/hotmail/engine.py
from typing import Optional

from .profile import choose_target_domains
from .types import HotmailFailureCode


def classify_hotmail_page_state(text: str) -> Optional[HotmailFailureCode]:
    normalized = str(text or "").lower()
    if "phone number" in normalized:
        return HotmailFailureCode.PHONE_VERIFICATION_REQUIRED
    if "puzzle" in normalized or "captcha" in normalized:
        return HotmailFailureCode.UNSUPPORTED_CHALLENGE
    return None


class HotmailRegistrationEngine:
    def __init__(self):
        self._attempt_domain = lambda domain: False

    def _choose_domain_by_attempt(self) -> Optional[str]:
        for domain in choose_target_domains():
            if self._attempt_domain(domain):
                return domain
        return None
```

- [ ] **Step 4: Expand the engine to include public result orchestration**

```python
class HotmailRegistrationEngine:
    def __init__(self, browser_factory, verification_provider, callback_logger=None, proxy_url=None):
        self.browser_factory = browser_factory
        self.verification_provider = verification_provider
        self.callback_logger = callback_logger
        self.proxy_url = proxy_url

    def run(self):
        """
        Final implementation should:
        1. generate profile
        2. open Microsoft signup flow in Playwright
        3. try hotmail.com then outlook.com
        4. detect email verification, phone verification, unsupported challenges
        5. return normalized artifact/result payload
        """
        raise NotImplementedError
```

- [ ] **Step 5: Run the engine tests to verify the minimal classification/fallback behavior passes**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_engine.py -q`

Expected: PASS for the current unit-tested helpers.

- [ ] **Step 6: Commit**

```bash
git add vendor/codex-register/src/services/hotmail/engine.py \
  vendor/codex-register/tests/test_hotmail_engine.py
git commit -m "feat: add hotmail engine skeleton"
```

### Task 4: Add Hotmail Batch APIs and Artifact Export

**Files:**
- Create: `vendor/codex-register/src/services/hotmail/artifacts.py`
- Create: `vendor/codex-register/src/web/routes/hotmail.py`
- Create: `vendor/codex-register/tests/test_hotmail_routes.py`
- Modify: `vendor/codex-register/src/web/routes/__init__.py`

- [ ] **Step 1: Write failing route tests for batch creation and artifact listing**

```python
import unittest
from fastapi import FastAPI
from fastapi.testclient import TestClient

from src.web.routes.hotmail import router


class HotmailRoutesTests(unittest.TestCase):
    def setUp(self):
        app = FastAPI()
        app.include_router(router, prefix="/api/hotmail")
        self.client = TestClient(app)

    def test_create_hotmail_batch_returns_batch_metadata(self):
        response = self.client.post(
            "/api/hotmail/batches",
            json={"count": 2, "concurrency": 1, "interval_min": 1, "interval_max": 2},
        )
        self.assertEqual(response.status_code, 200)
        self.assertIn("batch_id", response.json())

    def test_get_unknown_batch_returns_404(self):
        response = self.client.get("/api/hotmail/batches/missing")
        self.assertEqual(response.status_code, 404)
```

- [ ] **Step 2: Run the route tests to verify they fail**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_routes.py -q`

Expected: FAIL because the route module does not exist.

- [ ] **Step 3: Implement artifact helpers and the initial route skeleton**

```python
# vendor/codex-register/src/services/hotmail/artifacts.py
import json
from pathlib import Path
from typing import Iterable


def build_accounts_txt(records: Iterable[dict]) -> str:
    return "\n".join(f"{item['email']}----{item['password']}" for item in records)


def build_accounts_json(records: Iterable[dict]) -> str:
    return json.dumps(list(records), ensure_ascii=False, indent=2)
```

```python
# vendor/codex-register/src/web/routes/hotmail.py
import uuid
from typing import Dict

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel


router = APIRouter()
hotmail_batches: Dict[str, dict] = {}


class HotmailBatchCreateRequest(BaseModel):
    count: int
    concurrency: int = 1
    interval_min: int = 1
    interval_max: int = 2


@router.post("/batches")
async def create_hotmail_batch(request: HotmailBatchCreateRequest):
    batch_id = str(uuid.uuid4())
    hotmail_batches[batch_id] = {
        "batch_id": batch_id,
        "total": request.count,
        "completed": 0,
        "success": 0,
        "failed": 0,
        "finished": False,
        "logs": [],
        "artifacts": [],
    }
    return hotmail_batches[batch_id]


@router.get("/batches/{batch_id}")
async def get_hotmail_batch(batch_id: str):
    batch = hotmail_batches.get(batch_id)
    if not batch:
        raise HTTPException(status_code=404, detail="Hotmail batch not found")
    return batch
```

- [ ] **Step 4: Register the new route module**

```python
# vendor/codex-register/src/web/routes/__init__.py
from .hotmail import router as hotmail_router

api_router.include_router(hotmail_router, prefix="/hotmail", tags=["hotmail"])
```

- [ ] **Step 5: Run the route tests to verify they pass**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_routes.py -q`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add vendor/codex-register/src/services/hotmail/artifacts.py \
  vendor/codex-register/src/web/routes/hotmail.py \
  vendor/codex-register/src/web/routes/__init__.py \
  vendor/codex-register/tests/test_hotmail_routes.py
git commit -m "feat: add hotmail batch routes"
```

### Task 5: Add the Standalone Hotmail Page and Client

**Files:**
- Create: `vendor/codex-register/templates/hotmail.html`
- Create: `vendor/codex-register/static/js/hotmail.js`
- Modify: `vendor/codex-register/src/web/app.py`

- [ ] **Step 1: Write a failing form-logic test for the new page and JS hooks**

```python
import unittest
from pathlib import Path


class HotmailPageTests(unittest.TestCase):
    def test_hotmail_template_and_js_expose_batch_controls(self):
        root = Path(__file__).resolve().parents[1]
        template = (root / "templates" / "hotmail.html").read_text(encoding="utf-8")
        script = (root / "static" / "js" / "hotmail.js").read_text(encoding="utf-8")
        self.assertIn('id="hotmail-batch-form"', template)
        self.assertIn('id="hotmail-count"', template)
        self.assertIn("api.post('/hotmail/batches'", script)
```

- [ ] **Step 2: Run the page test to verify it fails**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_page.py -q`

Expected: FAIL because the page does not exist.

- [ ] **Step 3: Create the standalone Hotmail page**

```html
<!-- vendor/codex-register/templates/hotmail.html -->
<form id="hotmail-batch-form">
  <input type="number" id="hotmail-count" value="1" min="1" />
  <input type="number" id="hotmail-concurrency" value="1" min="1" max="10" />
  <input type="number" id="hotmail-interval-min" value="2" min="0" />
  <input type="number" id="hotmail-interval-max" value="5" min="0" />
  <button type="submit">开始注册</button>
</form>
<div id="hotmail-batch-status"></div>
<div id="hotmail-batch-logs"></div>
<div id="hotmail-artifacts"></div>
<script src="/static/js/utils.js"></script>
<script src="/static/js/hotmail.js"></script>
```

- [ ] **Step 4: Create the JS client and page route**

```javascript
// vendor/codex-register/static/js/hotmail.js
const hotmailForm = document.getElementById('hotmail-batch-form');
const hotmailStatus = document.getElementById('hotmail-batch-status');

if (hotmailForm) {
  hotmailForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const payload = {
      count: parseInt(document.getElementById('hotmail-count').value, 10) || 1,
      concurrency: parseInt(document.getElementById('hotmail-concurrency').value, 10) || 1,
      interval_min: parseInt(document.getElementById('hotmail-interval-min').value, 10) || 0,
      interval_max: parseInt(document.getElementById('hotmail-interval-max').value, 10) || 0,
    };
    const data = await api.post('/hotmail/batches', payload);
    hotmailStatus.textContent = `批次已创建: ${data.batch_id}`;
  });
}
```

```python
# vendor/codex-register/src/web/app.py
@app.get("/hotmail", response_class=HTMLResponse)
async def hotmail_page(request: Request):
    if not _is_authenticated(request):
        return _redirect_to_login(request)
    return templates.TemplateResponse(request=request, name="hotmail.html", context={"request": request})
```

- [ ] **Step 5: Run the page test to verify it passes**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_page.py -q`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add vendor/codex-register/templates/hotmail.html \
  vendor/codex-register/static/js/hotmail.js \
  vendor/codex-register/src/web/app.py \
  vendor/codex-register/tests/test_hotmail_page.py
git commit -m "feat: add hotmail batch page"
```

### Task 6: Integrate Real Batch Execution and End-to-End Verification

**Files:**
- Modify: `vendor/codex-register/src/web/routes/hotmail.py`
- Modify: `vendor/codex-register/src/services/hotmail/engine.py`
- Modify: `vendor/codex-register/src/services/hotmail/artifacts.py`
- Modify: `vendor/codex-register/src/services/hotmail/verification.py`
- Modify: `vendor/codex-register/tests/test_hotmail_routes.py`
- Modify: `vendor/codex-register/tests/test_hotmail_engine.py`

- [ ] **Step 1: Write failing integration tests for status transitions and artifact generation**

```python
def test_hotmail_batch_status_tracks_success_and_failed_counts():
    # final test should assert:
    # - completed count increments
    # - phone verification becomes failed
    # - artifact list contains accounts.json and accounts.txt
    assert False
```

- [ ] **Step 2: Run the focused integration tests to verify they fail**

Run: `python3 -m pytest vendor/codex-register/tests/test_hotmail_routes.py vendor/codex-register/tests/test_hotmail_engine.py -q`

Expected: FAIL because the real batch runner and engine orchestration are not finished yet.

- [ ] **Step 3: Replace the route skeleton with task-manager-backed execution**

```python
# hotmail.py should be expanded to:
# - create per-account task ids
# - schedule execution in task_manager.executor
# - keep batch state in a dedicated dict
# - update logs/status on completion/failure/cancel
# - expose artifact metadata after batch completion
```

- [ ] **Step 4: Implement the real engine flow in small increments**

```python
# engine.py should be expanded to:
# - generate profile and domain candidates
# - launch Playwright page
# - fill Microsoft signup form
# - detect verification-email challenge
# - request mailbox and poll verification code
# - classify phone verification as HotmailFailureCode.PHONE_VERIFICATION_REQUIRED
# - return HotmailAccountArtifact on success
```

- [ ] **Step 5: Final verification**

Run:

```bash
python3 -m pytest vendor/codex-register/tests/test_hotmail_profile.py -q
python3 -m pytest vendor/codex-register/tests/test_hotmail_verification.py -q
python3 -m pytest vendor/codex-register/tests/test_hotmail_engine.py -q
python3 -m pytest vendor/codex-register/tests/test_hotmail_routes.py -q
```

Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add vendor/codex-register/src/services/hotmail \
  vendor/codex-register/src/web/routes/hotmail.py \
  vendor/codex-register/templates/hotmail.html \
  vendor/codex-register/static/js/hotmail.js \
  vendor/codex-register/tests/test_hotmail_profile.py \
  vendor/codex-register/tests/test_hotmail_verification.py \
  vendor/codex-register/tests/test_hotmail_engine.py \
  vendor/codex-register/tests/test_hotmail_routes.py
git commit -m "feat: add hotmail batch registration"
```

## Self-Review

### Spec coverage

- standalone Hotmail page: covered by Task 5
- batch APIs and status polling: covered by Task 4 and Task 6
- `hotmail.com` then `outlook.com`: covered by Task 1 and Task 3
- email verification via existing email services: covered by Task 2 and Task 6
- phone verification fails fast: covered by Task 3 and Task 6
- JSON/TXT artifacts: covered by Task 1, Task 4, and Task 6
- no automatic Outlook DB import in phase 1: preserved by plan structure; no task writes to `EmailServiceModel(service_type="outlook")`

### Placeholder scan

- No `TBD`, `TODO`, or “implement later” markers remain in the executable steps.
- The only intentionally incomplete code appears in Task 6 as explicit expansion targets after failing integration tests; that task exists to replace skeletons written earlier.

### Type consistency

- `HotmailFailureCode`, `HotmailAccountArtifact`, and the route names stay consistent across tasks.
- Artifact TXT format remains `email----password`, matching the spec and current Outlook import format.

