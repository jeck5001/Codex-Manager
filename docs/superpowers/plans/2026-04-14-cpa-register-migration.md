# CPA Register Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the current register stack so `Codex-Manager` keeps its existing UI/RPC shell while running the `openai-cpa` registration flow and exposing `generator_email` as a first-class email service.

**Architecture:** Keep `apps/src -> crates/service -> vendor/codex-register` as the public contract, then migrate the internal register engine to CPA-first behavior. Add `generator_email` to the service catalog, adapt task/batch execution to CPA semantics, and load the CPA browser plugin assets through the existing register runtime instead of embedding the standalone CPA dashboard.

**Tech Stack:** Next.js 14 App Router, TypeScript, TanStack Query, Rust (`crates/service`), Python/FastAPI (`vendor/codex-register`), pytest, cargo test, pnpm.

---

## File Map

- Modify: `vendor/codex-register/src/config/constants.py`
  - Add `generator_email` to `EmailServiceType` and defaults.
- Create: `vendor/codex-register/src/services/generator_email.py`
  - First-class provider wrapping Generator.email mailbox creation and OTP extraction.
- Modify: `vendor/codex-register/src/services/__init__.py`
  - Register the new provider with `EmailServiceFactory`.
- Modify: `vendor/codex-register/src/web/routes/email_services.py`
  - Expose `generator_email` config metadata and sanitize any new secret fields.
- Modify: `vendor/codex-register/src/web/routes/registration.py`
  - Allow the new provider in available-service payloads and task creation.
- Modify: `vendor/codex-register/src/core/register.py`
  - Replace remaining legacy register steps with CPA-first orchestration.
- Create: `vendor/codex-register/src/core/cpa_register_runtime.py`
  - Concentrate imported CPA flow helpers instead of continuing to grow `register.py`.
- Create: `vendor/codex-register/src/core/cpa_page_driver.py`
  - Bridge backend state and injected browser script events.
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/manifest.json`
  - Vendor CPA plugin assets into the current project tree.
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/background.js`
  - Adapted from CPA plugin background coordinator.
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/content/register.js`
  - Adapted page-step logic for signup and verification.
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/content/utils.js`
  - Shared page helpers.
- Modify: `crates/service/src/account/account_register.rs`
  - Preserve RPC payload compatibility while surfacing the new provider and any richer task data.
- Modify: `crates/service/tests/register_payload_forwarding.rs`
  - Lock the new service type and task payload forwarding behavior.
- Modify: `apps/src/types/index.ts`
  - Ensure `generator_email` and any new task fields normalize correctly.
- Modify: `apps/src/lib/api/account-client.ts`
  - Keep frontend normalization compatible with the new provider/task payloads.
- Modify: `apps/src/app/email-services/page.tsx`
  - Make `generator_email` creatable/editable through the existing form flow.
- Modify: `apps/src/components/modals/add-account-modal.tsx`
  - Surface `generator_email` and CPA-first register mode behavior in task creation.
- Modify: `apps/src/app/register/page.tsx`
  - Render richer CPA-first task statuses and logs without changing the page shell.
- Create: `vendor/codex-register/tests/test_generator_email_service.py`
  - Provider parsing and OTP extraction coverage.
- Create: `vendor/codex-register/tests/test_cpa_register_runtime.py`
  - CPA-first backend flow coverage.
- Modify: `vendor/codex-register/tests/test_register_flow_runner.py`
  - Align runner behavior with new CPA redirect and page-state semantics.
- Modify: `vendor/codex-register/tests/test_web_app.py`
  - Cover service catalog / available-services integration.

### Task 1: Add `generator_email` As A First-Class Provider

**Files:**
- Modify: `vendor/codex-register/src/config/constants.py`
- Create: `vendor/codex-register/src/services/generator_email.py`
- Modify: `vendor/codex-register/src/services/__init__.py`
- Modify: `vendor/codex-register/src/web/routes/email_services.py`
- Create: `vendor/codex-register/tests/test_generator_email_service.py`

- [ ] **Step 1: Write the failing provider tests**

```python
from src.services.generator_email import GeneratorEmailService


def test_create_email_parses_generator_homepage(monkeypatch):
    html = """
    <html>
      <span id="email_ch_text">tester.box@generator.local</span>
    </html>
    """

    class FakeResponse:
        status_code = 200
        text = html

    monkeypatch.setattr(
        "src.services.generator_email.requests.get",
        lambda *args, **kwargs: FakeResponse(),
    )

    service = GeneratorEmailService(name="generator-email")
    email_info = service.create_email()

    assert email_info["email"] == "tester.box@generator.local"
    assert email_info["email_id"] == "generator.local/tester.box"
    assert email_info["credentials"]["surl"] == "generator.local/tester.box"


def test_get_verification_code_reads_latest_openai_otp(monkeypatch):
    html = """
    <html>
      <div>OpenAI verification message</div>
      <div>Your ChatGPT code is 654321</div>
    </html>
    """

    class FakeResponse:
        status_code = 200
        text = html

    monkeypatch.setattr(
        "src.services.generator_email.requests.get",
        lambda *args, **kwargs: FakeResponse(),
    )

    service = GeneratorEmailService(name="generator-email")
    code = service.get_verification_code(
        email="tester.box@generator.local",
        email_id="generator.local/tester.box",
    )

    assert code == "654321"
```

- [ ] **Step 2: Run the provider tests to verify they fail**

Run: `cd vendor/codex-register && python -m pytest tests/test_generator_email_service.py -q`
Expected: FAIL with `ModuleNotFoundError` or missing `GeneratorEmailService`.

- [ ] **Step 3: Add `generator_email` to the service type catalog**

```python
class EmailServiceType(str, Enum):
    TEMPMAIL = "tempmail"
    OUTLOOK = "outlook"
    CUSTOM_DOMAIN = "custom_domain"
    TEMP_MAIL = "temp_mail"
    MAIL_33_IMAP = "mail_33_imap"
    GENERATOR_EMAIL = "generator_email"


EMAIL_SERVICE_DEFAULTS = {
    # ...
    "generator_email": {
        "base_url": "https://generator.email",
        "timeout": 30,
        "poll_interval": 3,
    },
}
```

- [ ] **Step 4: Implement the provider with the current `BaseEmailService` contract**

```python
class GeneratorEmailService(BaseEmailService):
    def __init__(self, name: str = "generator_email", config: Optional[Dict[str, Any]] = None):
        super().__init__(EmailServiceType.GENERATOR_EMAIL, name)
        merged = {**EMAIL_SERVICE_DEFAULTS["generator_email"], **(config or {})}
        self.base_url = str(merged.get("base_url") or "https://generator.email").rstrip("/")
        self.timeout = int(merged.get("timeout") or 30)
        self.poll_interval = int(merged.get("poll_interval") or 3)

    def create_email(self, config: Dict[str, Any] = None) -> Dict[str, Any]:
        response = requests.get(self.base_url, headers=self._headers(), timeout=self.timeout, impersonate="chrome110")
        if response.status_code != 200:
            raise EmailServiceError(f"Generator.email create inbox failed: HTTP {response.status_code}")

        email = self._parse_email(response.text or "")
        surl = self._build_surl(email)
        if not email or not surl:
            raise EmailServiceError("Generator.email did not return a usable mailbox")

        return {
            "email": email,
            "email_id": surl,
            "service_id": surl,
            "credentials": {"surl": surl},
        }

    def get_verification_code(self, email: str, email_id: str = None, timeout: int = 120, poll_interval: Optional[int] = None, pattern: str = r"(?<!\d)(\d{6})(?!\d)", otp_sent_at: Optional[float] = None) -> Optional[str]:
        mailbox_id = str(email_id or "").strip()
        if not mailbox_id:
            raise EmailServiceError("Generator.email requires surl/email_id")
        response = requests.get(f"{self.base_url}/{mailbox_id}", headers=self._headers(), cookies={"surl": mailbox_id}, timeout=self.timeout, impersonate="chrome110")
        if response.status_code != 200:
            raise EmailServiceError(f"Generator.email mailbox read failed: HTTP {response.status_code}")
        return self._extract_code(response.text or "", pattern)
```

- [ ] **Step 5: Register the provider and expose form metadata**

```python
# vendor/codex-register/src/services/__init__.py
from .generator_email import GeneratorEmailService
EmailServiceFactory.register(EmailServiceType.GENERATOR_EMAIL, GeneratorEmailService)

# vendor/codex-register/src/web/routes/email_services.py
SENSITIVE_FIELDS = {"password", "api_key", "refresh_token", "access_token", "admin_password"}

GENERATOR_EMAIL_FIELDS = [
    {"name": "base_url", "label": "Base URL", "required": False, "default_value": "https://generator.email"},
    {"name": "timeout", "label": "请求超时", "required": False, "default_value": 30},
    {"name": "poll_interval", "label": "轮询间隔", "required": False, "default_value": 3},
]
```

- [ ] **Step 6: Re-run the provider tests to verify they pass**

Run: `cd vendor/codex-register && python -m pytest tests/test_generator_email_service.py -q`
Expected: PASS with `2 passed`.

- [ ] **Step 7: Commit the provider slice**

```bash
git add vendor/codex-register/src/config/constants.py vendor/codex-register/src/services/generator_email.py vendor/codex-register/src/services/__init__.py vendor/codex-register/src/web/routes/email_services.py vendor/codex-register/tests/test_generator_email_service.py
git commit -m "feat: add generator email register provider"
```

### Task 2: Migrate The CPA Browser Assets Into The Register Runtime

**Files:**
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/manifest.json`
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/background.js`
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/content/register.js`
- Create: `vendor/codex-register/src/browser_assets/openai_cpa_plugin/content/utils.js`
- Create: `vendor/codex-register/src/core/cpa_page_driver.py`
- Create: `vendor/codex-register/tests/test_cpa_register_runtime.py`

- [ ] **Step 1: Write a failing backend test for CPA page-state bridging**

```python
from src.core.cpa_page_driver import classify_signup_state


def test_classify_signup_state_prefers_existing_account_signal():
    snapshot = {
        "is_signup_password_page": True,
        "page_text": "Account associated with this email address already exists",
        "has_retry_button": False,
    }

    state = classify_signup_state(snapshot)

    assert state["kind"] == "email_exists"
    assert state["retryable"] is False
```

- [ ] **Step 2: Run the CPA page-state test to verify it fails**

Run: `cd vendor/codex-register && python -m pytest tests/test_cpa_register_runtime.py -q`
Expected: FAIL because `cpa_page_driver` does not exist yet.

- [ ] **Step 3: Vendor the CPA plugin assets into the current repo**

```text
Copy:
- /Users/jfwang/IdeaProjects/CascadeProjects/openai-cpa/plugin/openai-cpa-plugin/manifest.json
- /Users/jfwang/IdeaProjects/CascadeProjects/openai-cpa/plugin/openai-cpa-plugin/background.js
- /Users/jfwang/IdeaProjects/CascadeProjects/openai-cpa/plugin/openai-cpa-plugin/content/register.js
- /Users/jfwang/IdeaProjects/CascadeProjects/openai-cpa/plugin/openai-cpa-plugin/content/utils.js

Target:
- vendor/codex-register/src/browser_assets/openai_cpa_plugin/...
```

- [ ] **Step 4: Add a small Python adapter that turns CPA DOM snapshots into backend actions**

```python
def classify_signup_state(snapshot: dict[str, Any]) -> dict[str, Any]:
    text = str(snapshot.get("page_text") or "").lower()
    if "email address already exists" in text:
        return {"kind": "email_exists", "retryable": False}
    if "something went wrong" in text or "timed out" in text:
        return {"kind": "password_retry", "retryable": True}
    if snapshot.get("is_signup_password_page"):
        return {"kind": "awaiting_password_submit", "retryable": False}
    return {"kind": "unknown", "retryable": False}
```

- [ ] **Step 5: Re-run the CPA page-state test to verify it passes**

Run: `cd vendor/codex-register && python -m pytest tests/test_cpa_register_runtime.py -q`
Expected: PASS with `1 passed`.

- [ ] **Step 6: Commit the browser-asset slice**

```bash
git add vendor/codex-register/src/browser_assets/openai_cpa_plugin vendor/codex-register/src/core/cpa_page_driver.py vendor/codex-register/tests/test_cpa_register_runtime.py
git commit -m "feat: vendor cpa browser register assets"
```

### Task 3: Replace The Remaining Register Core With CPA-First Flow Semantics

**Files:**
- Modify: `vendor/codex-register/src/core/register.py`
- Create: `vendor/codex-register/src/core/cpa_register_runtime.py`
- Modify: `vendor/codex-register/src/core/register_flow_runner.py`
- Modify: `vendor/codex-register/src/web/routes/registration.py`
- Modify: `vendor/codex-register/tests/test_register_flow_runner.py`
- Modify: `vendor/codex-register/tests/test_register_any_auto_mode.py`

- [ ] **Step 1: Write a failing test for CPA-style callback resolution**

```python
from src.core.cpa_register_runtime import resolve_callback_payload


def test_resolve_callback_payload_accepts_query_fragment_mix():
    payload = resolve_callback_payload(
        "http://localhost:1455/auth/callback?code=abc123#state=xyz456"
    )

    assert payload["code"] == "abc123"
    assert payload["state"] == "xyz456"
```

- [ ] **Step 2: Run the callback test to verify it fails**

Run: `cd vendor/codex-register && python -m pytest tests/test_cpa_register_runtime.py::test_resolve_callback_payload_accepts_query_fragment_mix -q`
Expected: FAIL because `resolve_callback_payload` is missing.

- [ ] **Step 3: Move CPA callback, token, and retry helpers into a dedicated runtime module**

```python
def resolve_callback_payload(callback_url: str) -> Dict[str, str]:
    candidate = str(callback_url or "").strip()
    parsed = urllib.parse.urlparse(candidate)
    query = urllib.parse.parse_qs(parsed.query, keep_blank_values=True)
    fragment = urllib.parse.parse_qs(parsed.fragment, keep_blank_values=True)
    for key, values in fragment.items():
        if key not in query or not query[key] or not (query[key][0] or "").strip():
            query[key] = values
    return {
        "code": (query.get("code", [""])[0] or "").strip(),
        "state": (query.get("state", [""])[0] or "").strip(),
        "error": (query.get("error", [""])[0] or "").strip(),
    }
```

- [ ] **Step 4: Make `RegistrationEngine` delegate signup progression to the CPA runtime**

```python
class RegistrationEngine:
    def __init__(...):
        # ...
        self.cpa_runtime = CPARegisterRuntime(self)

    def run_registration(self) -> RegistrationResult:
        email_info = self.email_service.create_email()
        self.email = email_info["email"]
        self.email_info = email_info
        return self.cpa_runtime.run_signup_flow()
```

- [ ] **Step 5: Align route-level task execution with the new engine outputs**

```python
engine = RegistrationEngine(
    email_service=email_service,
    proxy_url=proxy,
    callback_logger=logger_callback,
    task_uuid=task_uuid,
)
result = engine.run_registration()
crud.update_registration_task_result(db, task_uuid, result.to_dict())
```

- [ ] **Step 6: Re-run the targeted CPA/runtime tests**

Run: `cd vendor/codex-register && python -m pytest tests/test_cpa_register_runtime.py tests/test_register_flow_runner.py tests/test_register_any_auto_mode.py -q`
Expected: PASS with no failures in the targeted runtime suite.

- [ ] **Step 7: Commit the register-core slice**

```bash
git add vendor/codex-register/src/core/register.py vendor/codex-register/src/core/cpa_register_runtime.py vendor/codex-register/src/core/register_flow_runner.py vendor/codex-register/src/web/routes/registration.py vendor/codex-register/tests/test_cpa_register_runtime.py vendor/codex-register/tests/test_register_flow_runner.py vendor/codex-register/tests/test_register_any_auto_mode.py
git commit -m "feat: migrate register core to cpa flow"
```

### Task 4: Wire The New Provider And CPA Runtime Through Rust RPC And The Current Frontend

**Files:**
- Modify: `crates/service/src/account/account_register.rs`
- Modify: `crates/service/tests/register_payload_forwarding.rs`
- Modify: `apps/src/types/index.ts`
- Modify: `apps/src/lib/api/account-client.ts`
- Modify: `apps/src/app/email-services/page.tsx`
- Modify: `apps/src/components/modals/add-account-modal.tsx`
- Modify: `apps/src/app/register/page.tsx`

- [ ] **Step 1: Write a failing Rust forwarding test for `generator_email`**

```rust
#[test]
fn rpc_register_payload_supports_generator_email() {
    let params = serde_json::json!({
        "emailServiceType": "generator_email",
        "registerMode": "standard"
    });

    let payload = super::build_register_payload_for_test(params).unwrap();

    assert_eq!(payload.get("email_service_type").and_then(|v| v.as_str()), Some("generator_email"));
}
```

- [ ] **Step 2: Run the Rust forwarding test to verify it fails**

Run: `cargo test -p codexmanager-service rpc_register_payload_supports_generator_email -- --nocapture`
Expected: FAIL because the new provider value is not covered yet.

- [ ] **Step 3: Extend Rust payload normalization without changing the external RPC contract**

```rust
let service_type = email_service_type.trim();
let payload = json!({
    "email_service_type": service_type,
    "email_service_id": email_service_id,
    "email_service_config": email_service_config,
    "register_mode": register_mode.unwrap_or("standard"),
});
```

- [ ] **Step 4: Update frontend type normalization and forms to accept `generator_email`**

```ts
export interface RegisterEmailServiceType {
  value: string;
  label: string;
  description?: string | null;
  configFields: RegisterEmailServiceField[];
}

const canUseGeneratorEmail = selectedServiceType === "generator_email";
```

- [ ] **Step 5: Update the email-service and register modals to surface the new provider and CPA-first logs**

```tsx
<SelectItem value="generator_email">Generator.email</SelectItem>

{task.logs?.map((entry) => (
  <div key={entry} className="text-xs text-muted-foreground">
    {entry}
  </div>
))}
```

- [ ] **Step 6: Re-run the Rust and frontend targeted tests**

Run: `cargo test -p codexmanager-service register_payload_forwarding -- --nocapture`
Expected: PASS for register forwarding coverage.

Run: `pnpm test -- --runInBand apps/src/components/modals/register-mode-options.test.ts apps/src/components/modals/register-temp-mail-auto-create.test.ts apps/src/app/email-services/temp-mail-domain-config-state.test.ts`
Expected: PASS for existing frontend targeted tests plus any new type-normalization coverage you add.

- [ ] **Step 7: Commit the RPC/frontend slice**

```bash
git add crates/service/src/account/account_register.rs crates/service/tests/register_payload_forwarding.rs apps/src/types/index.ts apps/src/lib/api/account-client.ts apps/src/app/email-services/page.tsx apps/src/components/modals/add-account-modal.tsx apps/src/app/register/page.tsx
git commit -m "feat: expose cpa register flow in current ui"
```

### Task 5: Run Full Verification And Stabilize The Migration

**Files:**
- Modify: `vendor/codex-register/tests/test_web_app.py`
- Modify: `vendor/codex-register/tests/test_register_flow_runner.py`
- Modify: `crates/service/tests/rpc.rs`
- Modify: `docs/superpowers/specs/2026-04-14-cpa-register-migration-design.md` (only if implementation realities require a documented adjustment)

- [ ] **Step 1: Add an integration test covering the new provider in the service catalog**

```python
def test_email_service_types_include_generator_email(client):
    response = client.get("/api/email-services/types")
    payload = response.json()

    values = {item["value"] for item in payload["types"]}
    assert "generator_email" in values
```

- [ ] **Step 2: Run the catalog integration test to verify it fails before the route adjustments**

Run: `cd vendor/codex-register && python -m pytest tests/test_web_app.py::test_email_service_types_include_generator_email -q`
Expected: FAIL until the route payload includes the new type.

- [ ] **Step 3: Finish any route/test adjustments required to make the full migrated stack green**

```python
available_services["generator_email"] = {
    "available": len(generator_services) > 0,
    "count": len(generator_services),
    "services": [serialize_service(item) for item in generator_services],
}
```

- [ ] **Step 4: Run the Python register suite**

Run: `cd vendor/codex-register && python -m pytest tests/test_generator_email_service.py tests/test_cpa_register_runtime.py tests/test_register_flow_runner.py tests/test_web_app.py -q`
Expected: PASS with zero failures in the migrated Python suite.

- [ ] **Step 5: Run the Rust register suite**

Run: `cargo test -p codexmanager-service register_payload_forwarding rpc -- --nocapture`
Expected: PASS for register-related Rust coverage.

- [ ] **Step 6: Run the desktop build**

Run: `pnpm run build:desktop`
Expected: exit code `0`.

- [ ] **Step 7: Commit the final stabilization slice**

```bash
git add vendor/codex-register/tests/test_web_app.py vendor/codex-register/tests/test_register_flow_runner.py crates/service/tests/rpc.rs docs/superpowers/specs/2026-04-14-cpa-register-migration-design.md
git commit -m "test: verify cpa register migration end to end"
```
