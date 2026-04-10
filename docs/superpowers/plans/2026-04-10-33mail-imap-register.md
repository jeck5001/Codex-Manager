# 33mail IMAP Register Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `33mail + IMAP` email-service type that plugs into the existing registration flow and unified management UI.

**Architecture:** Extend `vendor/codex-register` with a new `BaseEmailService` implementation and surface it through the existing email-service catalog, available-service payload, and stats endpoints. Then update the frontend normalization and summaries so the new type can be configured and selected without introducing a separate workflow.

**Tech Stack:** FastAPI, existing Codex Register service abstractions, Python IMAP libraries, React, TypeScript, TanStack Query, Tauri RPC bridge.

---

## File Map

- Create: `vendor/codex-register/src/services/mail_33_imap.py`
- Modify: `vendor/codex-register/src/config/constants.py`
- Modify: `vendor/codex-register/src/services/__init__.py`
- Modify: `vendor/codex-register/src/web/routes/email_services.py`
- Modify: `vendor/codex-register/src/web/routes/registration.py`
- Create: `vendor/codex-register/tests/test_mail_33_imap_service.py`
- Create: `vendor/codex-register/tests/test_mail_33_email_service_routes.py`
- Modify: `apps/src/types/index.ts`
- Modify: `apps/src/lib/api/account-client.ts`
- Modify: `apps/src/app/email-services/page.tsx`

### Task 1: Add failing backend tests for the new email-service type

**Files:**
- Create: `vendor/codex-register/tests/test_mail_33_imap_service.py`
- Create: `vendor/codex-register/tests/test_mail_33_email_service_routes.py`

- [ ] **Step 1: Write failing service tests covering alias generation and OTP extraction**
- [ ] **Step 2: Run `python3 -m pytest vendor/codex-register/tests/test_mail_33_imap_service.py -q`**
Expected: FAIL because `mail_33_imap.py` does not exist.
- [ ] **Step 3: Write failing route tests covering `/api/email-services/types`, `/api/email-services/stats`, and `/api/registration/available-services`**
- [ ] **Step 4: Run `python3 -m pytest vendor/codex-register/tests/test_mail_33_email_service_routes.py -q`**
Expected: FAIL because the new service type is not yet exposed.

### Task 2: Implement the `mail_33_imap` email service

**Files:**
- Create: `vendor/codex-register/src/services/mail_33_imap.py`
- Modify: `vendor/codex-register/src/config/constants.py`
- Modify: `vendor/codex-register/src/services/__init__.py`

- [ ] **Step 1: Add `MAIL_33_IMAP = "mail_33_imap"` to the email service enum**
- [ ] **Step 2: Implement `Mail33ImapService` with `create_email`, `get_verification_code`, `check_health`, and no-op listing/deletion semantics**
- [ ] **Step 3: Register the service in the service factory exports**
- [ ] **Step 4: Run `python3 -m pytest vendor/codex-register/tests/test_mail_33_imap_service.py -q`**
Expected: PASS

### Task 3: Surface the new service type through the register-service API

**Files:**
- Modify: `vendor/codex-register/src/web/routes/email_services.py`
- Modify: `vendor/codex-register/src/web/routes/registration.py`

- [ ] **Step 1: Add the new type metadata to `/api/email-services/types` with all required config fields**
- [ ] **Step 2: Include `mail_33_imap` in service stats aggregation**
- [ ] **Step 3: Include enabled `mail_33_imap` services in `/api/registration/available-services`**
- [ ] **Step 4: Allow registration task service selection to resolve `mail_33_imap` the same way as other DB-backed services**
- [ ] **Step 5: Run `python3 -m pytest vendor/codex-register/tests/test_mail_33_email_service_routes.py -q`**
Expected: PASS

### Task 4: Update frontend normalization and summaries

**Files:**
- Modify: `apps/src/types/index.ts`
- Modify: `apps/src/lib/api/account-client.ts`
- Modify: `apps/src/app/email-services/page.tsx`

- [ ] **Step 1: Extend frontend types for the new available-service group and stats count**
- [ ] **Step 2: Update normalizers to read the new backend payload keys**
- [ ] **Step 3: Update email-service dashboard summaries so `mail_33_imap` counts as a managed custom mailbox service**
- [ ] **Step 4: Run targeted frontend tests if present**

### Task 5: Verify end-to-end integration

**Files:**
- Modify only the files above

- [ ] **Step 1: Run backend targeted tests**
Run: `python3 -m pytest vendor/codex-register/tests/test_mail_33_imap_service.py vendor/codex-register/tests/test_mail_33_email_service_routes.py -q`
Expected: PASS
- [ ] **Step 2: Run desktop build verification**
Run: `pnpm run build:desktop`
Expected: PASS
