# Hotmail Auto Registration Design

## Summary

Add a standalone Hotmail account production tool to the existing Codex Register system.

The tool will:

- create Microsoft consumer email accounts automatically
- prefer `@hotmail.com`, then fall back to `@outlook.com`
- use the existing temporary email service pool for email verification
- run fully automatically
- mark tasks as failed immediately when Microsoft requires phone verification or other unsupported risk checks
- export successful accounts as `json` and `txt`

This feature is intentionally independent from the current email-service management flow. It produces accounts first. Importing them into the existing `outlook` email-service table is a later phase.

## Goals

- Provide a dedicated batch tool for Hotmail account creation.
- Reuse the existing batch task and status-polling architecture where practical.
- Keep the first version fully automatic, with no manual takeover path.
- Make exported results directly compatible with the current Outlook import workflow.
- Minimize coupling with the OpenAI registration flow.

## Non-Goals

- Do not automatically convert successful accounts into OAuth-backed Outlook services in phase 1.
- Do not add any manual takeover UI.
- Do not support phone-verification solving, SMS platforms, or captcha-solving platforms in phase 1.
- Do not merge this feature into the current email-service management page.

## Product Shape

Add a new standalone Hotmail registration page and task type.

The page will let the user configure:

- target account count
- concurrency
- min/max interval
- proxy
- domain policy: `hotmail.com` first, then `outlook.com`
- verification email service source: existing temp email services
- export format options

The page will show:

- batch progress
- per-task status
- live logs
- failure classification
- download links for exported artifacts

## High-Level Architecture

### 1. HotmailRegistrationEngine

Responsible for one Microsoft signup attempt.

Responsibilities:

- generate username, password, profile data
- acquire a verification mailbox from the existing temp-mail/custom-domain/outlook-capable email-service selection rules only when needed for Microsoft verification
- drive the Microsoft signup flow
- detect success, unsupported verification, and hard failure
- return a normalized result object

Implementation direction:

- use Playwright as the primary browser automation layer for Microsoft signup
- use HTTP helpers only for supporting tasks when they are clearly stable and lower risk
- keep browser/session logic isolated from batch orchestration

### 2. HotmailBatchRunner

Responsible for orchestration.

Responsibilities:

- create batch ids and task ids
- schedule concurrent account-creation jobs
- apply interval throttling
- persist progress and logs
- support cancellation
- collect successful account artifacts

This should reuse the current registration batch patterns rather than invent a second scheduling model.

### 3. HotmailArtifactStore

Responsible for output files.

Outputs:

- `accounts.json`
- `accounts.txt`
- optional raw per-task logs for diagnostics

`accounts.txt` should use the current Outlook import-compatible format:

```text
email----password
```

That keeps phase 1 useful even without direct DB import.

### 4. Hotmail Web UI

A separate page or dedicated section, not part of the existing email-service page.

Reason:

- current email-service UI manages existing mailbox providers
- Hotmail registration is account production, not service configuration
- keeping it separate avoids muddying the semantics of the Outlook management flow

## Detailed Flow

### Batch Flow

1. User creates a Hotmail batch task.
2. Backend validates parameters and creates batch state.
3. Runner starts `N` concurrent signup jobs.
4. Each job creates one account attempt through `HotmailRegistrationEngine`.
5. Successful jobs append exportable account records.
6. Failed jobs append failure reason and logs.
7. When the batch finishes or is cancelled, artifacts are finalized and exposed to the UI.

### Single Account Flow

1. Generate account profile
   - first name
   - last name
   - birthdate
   - username candidates
   - password

2. Start Microsoft signup in browser.

3. Try `@hotmail.com`.
   - if unavailable for domain-specific reasons, retry with `@outlook.com`
   - if username collision occurs, retry username generation within the same domain policy

4. Fill registration form and submit.

5. If Microsoft requests email verification:
   - provision a verification mailbox through the existing email-service layer
   - wait for code
   - submit code

6. Detect outcome:
   - success: account created
   - phone verification requested: fail with `phone_verification_required`
   - captcha or unsupported challenge: fail with `unsupported_challenge`
   - timeout/network/page drift: fail with corresponding technical category

7. Return normalized result.

## Domain Policy

Phase 1 policy:

- preferred domain: `hotmail.com`
- fallback domain: `outlook.com`

Failure handling:

- if `hotmail.com` is unavailable due to username/domain availability issues, retry as `outlook.com`
- if the failure is unrelated to domain availability, do not automatically rerun the whole flow on another domain unless the page state clearly supports switching without a full restart

## Verification Strategy

### Supported

- email verification through existing temporary mailbox services

### Unsupported in Phase 1

- phone verification
- SMS provider integration
- external captcha solving
- manual takeover

Unsupported verification must produce a terminal failed task with an explicit reason. The system should not hang waiting for impossible recovery.

## Failure Model

Each task must end in exactly one of these states:

- `success`
- `failed`
- `cancelled`

Each failed task should also have a machine-friendly reason code, for example:

- `phone_verification_required`
- `unsupported_challenge`
- `email_verification_timeout`
- `username_unavailable_exhausted`
- `proxy_error`
- `page_structure_changed`
- `browser_timeout`
- `unexpected_exception`

This failure taxonomy matters because later tuning will depend on it.

## Data Model

Phase 1 should avoid schema-heavy changes where possible, but it still needs batch persistence comparable to the existing registration batch flow.

Minimum stored result fields per Hotmail task:

- `task_id`
- `batch_id`
- `status`
- `reason_code`
- `email`
- `password`
- `target_domain`
- `verification_email`
- `started_at`
- `finished_at`
- `log_excerpt`

Batch summary fields:

- `batch_id`
- `total`
- `running`
- `success`
- `failed`
- `cancelled`
- `created_at`
- `finished_at`
- `artifact_paths`

## API Shape

Add dedicated Hotmail endpoints under registration or a new dedicated route group.

Phase 1 API set:

- `POST /api/hotmail/batches`
  - create batch
- `GET /api/hotmail/batches/{batch_id}`
  - batch status
- `POST /api/hotmail/batches/{batch_id}/cancel`
  - cancel batch
- `GET /api/hotmail/batches/{batch_id}/artifacts`
  - list/download export artifacts

If it is simpler to colocate with existing registration routes, that is acceptable, but the route names should still clearly identify this as Hotmail production rather than OpenAI registration.

## UI Design

The UI should mirror existing task pages in interaction style, but remain separate in purpose.

Sections:

- batch configuration form
- live task table
- progress summary
- log stream
- artifact download area

Important UX details:

- show that phone verification is unsupported and will fail the task
- show which verification mailbox was used for each successful or failed account
- show explicit failure reasons instead of generic `failed`

## Implementation Constraints

- Preserve the current Outlook import flow unchanged.
- Do not automatically write new Hotmail accounts into `EmailServiceModel(service_type="outlook")` in phase 1.
- Keep Hotmail-specific logic out of temp-mail Cloudflare provisioning code.
- Reuse existing logging, task status, and interval-control patterns where feasible.

## Testing Strategy

### Unit Tests

- username/domain selection policy
- failure reason mapping
- export formatting
- verification-email routing
- unsupported verification detection

### Integration Tests

- batch creation and polling
- successful artifact generation
- cancellation behavior
- fallback from `hotmail.com` to `outlook.com`
- email verification success path
- phone verification terminal failure path

### Browser Flow Tests

Use deterministic fakes/mocks around Playwright page transitions where possible.

Do not make the test suite depend on live Microsoft signup pages.

## Risks

### 1. Microsoft Risk Controls

This is the dominant risk.

Phone verification, challenge escalation, and account throttling may make success rate highly dependent on:

- proxy quality
- region consistency
- browser fingerprint consistency
- creation rate

### 2. Page Drift

Microsoft signup UI may change frequently.

Mitigation:

- centralize page selectors and page-state detection
- classify unsupported pages explicitly instead of letting flows hang

### 3. Temporary Email Latency

Verification email delay can reduce success rate.

Mitigation:

- reuse existing mailbox polling abstractions
- classify timeout separately from hard denial

## Phasing

### Phase 1

- standalone Hotmail batch tool
- full automation only
- email verification supported
- phone verification fails fast
- export `json` and import-compatible `txt`

### Phase 2

- optional direct import into Outlook email services
- optional OAuth token acquisition pipeline

### Phase 3

- proxy strategy tuning
- experiment toggles for anti-risk-control improvements

## Recommendation

Build phase 1 now as a dedicated Hotmail production line with its own UI and task routes, while reusing the existing project’s batch orchestration patterns and email-service abstractions.

This gives a usable first release without polluting current email-service management or forcing OAuth acquisition into the first milestone.
