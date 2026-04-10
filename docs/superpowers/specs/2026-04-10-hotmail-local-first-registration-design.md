# Hotmail Local-First Registration Design

## Goal

Replace the current Hotmail registration execution path so that the browser runs on the operator's local machine from the first registration page, instead of starting in the container and only switching to local control after Microsoft presents a human verification challenge.

## Relationship To Existing Designs

This design supersedes the operational role of:

- `2026-04-09-hotmail-local-handoff-design.md`
- `2026-04-09-hotmail-web-local-helper-design.md`

Those designs improved the manual takeover experience after a challenge page had already appeared. They remain useful background, but they do not solve the root problem the user observed: Microsoft is still evaluating a session that originally started in the container or remote automation environment. This design keeps the helper concept, but moves it earlier in the lifecycle so that Hotmail browser execution is local-first by default.

## Problem

The current Hotmail flow still starts inside a backend-controlled Playwright session and only hands the browser over locally after Microsoft has already challenged the account creation flow.

That architecture improves input quality, but not session reputation. By the time the operator sees the local browser, the request chain has already been shaped by the remote execution environment, and Microsoft may have already marked the registration as high-risk. The observed symptom is that the local browser still lands on a "prove you're human" page that repeatedly returns "try again."

This means the real problem is not just remote interaction latency. The core problem is that session ownership changes too late.

## Recommended Approach

Switch Hotmail registration to a local-first execution model.

The register backend should stop owning the browser session for Hotmail. Instead, it becomes a task orchestrator:

- generate batch metadata
- allocate mailbox resources
- prepare account profile data
- persist logs and final results
- receive progress and failure updates from the local helper

The local helper becomes the browser executor from the first registration page onward:

- open Chromium locally
- visit the Microsoft signup page
- fill and submit the registration flow
- wait for email verification
- stop for manual interaction if required
- report progress and results back to the backend

From the operator's perspective, Hotmail "开始批次" should now mean "start this on my current machine." The system should not silently fall back to container-side execution.

## Alternatives Considered

### 1. Keep challenge-page handoff only

This is not recommended. It addresses usability of the challenge interaction, but not the upstream risk score that likely triggers or hardens the challenge.

### 2. Start remotely, hand off earlier

This is better than waiting until the challenge page, but it still introduces a session migration boundary. That boundary is exactly what this redesign aims to remove.

### 3. Local-first helper execution

This is the recommended approach. It keeps the full browser chain on the operator's machine and matches the user's stated goal most directly.

## Architecture

The backend and helper should be separated into orchestration and execution roles.

### Backend role

`codexmanager-register` continues to own batch lifecycle and persisted state. For Hotmail, it should no longer open Playwright and drive pages directly. Instead it should:

- create a batch record
- generate one registration task per account attempt
- reserve a verification mailbox for that task
- return task payloads to the web UI or helper-facing API
- accept task progress, completion, and failure updates
- store artifacts, logs, and derived status

### Local helper role

The helper should own browser execution for Hotmail. It should:

- start a dedicated local Chromium profile
- run the Microsoft signup flow from the first page
- use the mailbox information provided by the backend
- poll or fetch verification state as needed
- send progress snapshots back to the backend
- surface manual challenge moments in the already-open local browser

### Frontend role

The Hotmail page becomes a coordinator between backend and local helper:

- check helper health before starting a batch
- refuse to start if the helper is unavailable
- start batch orchestration in the backend
- send task payloads to the helper
- poll backend batch state as it already does
- display local-execution-specific guidance when the helper reports manual interaction required

## Execution Flow

The new default flow should be:

1. User opens `/hotmail`
2. User clicks `开始批次`
3. Frontend checks `GET http://127.0.0.1:16788/health`
4. If helper is healthy, frontend asks backend to create a Hotmail local-first batch
5. Backend creates the batch and a first registration task payload
6. Frontend sends that task to the local helper
7. Helper opens local Chromium and executes registration
8. Helper reports status back to backend during key transitions
9. Frontend keeps showing the batch using backend state
10. If manual verification appears, the browser is already local, so the page only prompts the user to continue in the open local window
11. Helper reports final success or failure
12. Backend stores artifact/log result and schedules the next attempt if needed

## Helper API Changes

The helper currently exposes only health and challenge handoff launch endpoints. It should grow task-oriented endpoints for full registration execution.

Recommended endpoints:

### `GET /health`

Keeps its current role and still reports helper availability plus Chromium readiness.

### `POST /hotmail/start-task`

Accepts a complete registration task payload from the frontend.

Expected payload fields:

- `batch_id`
- `task_id`
- `email_candidate` or username/domain inputs
- `password`
- `profile`
- `proxy`
- `verification_mailbox`
- `backend_callback_base`
- `backend_callback_token` or similar scoped credential

Expected response:

```json
{
  "ok": true,
  "task_id": "task-123",
  "message": "task accepted"
}
```

### `POST /hotmail/cancel-task`

Allows the frontend or backend to cancel a currently running local task if the user stops the batch.

### Internal backend callback use

The helper should call backend callback routes for:

- task started
- page advanced
- waiting for manual verification
- verification code submitted
- task succeeded
- task failed

The helper should not directly mutate the database. It reports through backend APIs so the backend remains the source of truth.

## Backend API Changes

The backend should add task-oriented Hotmail endpoints and data models.

Recommended additions:

- create local-first Hotmail batch
- read next task payload for a batch
- accept helper progress events
- accept helper success/failure result events

The backend should keep the existing batch read, artifacts read, cancel, and abandon endpoints where they still make sense. The user-facing batch tracking model should remain stable where possible.

## Data Model

A Hotmail batch record should continue to expose aggregate status, but individual attempts need a local-execution task model.

Recommended task-level fields:

- `task_id`
- `batch_id`
- `status`
- `current_step`
- `manual_action_required`
- `failure_code`
- `failure_message`
- `verification_email`
- `target_email`
- `artifact_path`
- `started_at`
- `updated_at`

The helper should not own durable state beyond temporary browser profiles and transient execution context. Durable task state lives in the backend.

## Web UI Changes

The Hotmail page should keep a single primary action, but its semantics change.

### Start behavior

`开始批次` should:

- check helper health first
- show an actionable setup block if helper is missing
- only proceed when local execution is available

### Running behavior

While a task is running locally, the page should make that explicit with text such as:

- `正在本机执行`
- `请勿关闭本机浏览器窗口`

### Manual verification behavior

If a task hits a human verification page:

- do not show `本机接管（Web）`
- instead show a passive reminder that the browser is already open locally
- keep `继续注册` only if a backend checkpoint still requires an explicit continue action

### Failure behavior

If the helper reports:

- `unsupported_challenge`
- `account_creation_blocked`
- `phone_verification_required`

the page should show those reasons directly in the batch log/status area instead of implying that another handoff is needed.

## Error Handling

The system should fail early and explicitly.

- If helper is offline: do not create the Hotmail batch
- If helper lacks Chromium: do not create the Hotmail batch
- If helper accepts the task but local browser launch fails: mark that task failed with a helper launch error
- If backend callback submission fails transiently: helper should retry with bounded backoff
- If helper process dies mid-task: backend should eventually mark the task as stale or failed after timeout

The user should never be left guessing whether the browser is supposed to be local or remote. All status text should consistently describe the local-first model.

## Security Boundaries

The helper remains localhost-only. It should not become a general remote browser service.

- bind only to `127.0.0.1`
- accept only narrow Hotmail task payloads
- require explicit backend callback credentials scoped to the current batch/task
- keep origin restrictions on browser-initiated requests
- do not expose filesystem or arbitrary shell access

## Testing

### Backend

Add tests for:

- local-first batch creation
- task payload generation
- helper progress/result callback handling
- stale task timeout behavior

### Helper

Add tests for:

- start-task payload validation
- task execution bootstrap
- backend callback formatting
- launch failure reporting

The execution layer should be tested with mocked browser primitives in CI.

### Frontend

Add tests for:

- helper health gating before batch start
- helper-missing guidance
- local-running status presentation
- manual verification message changes under local-first execution

Run `pnpm run build:desktop` to confirm the Hotmail page still builds in desktop mode.

### Manual verification

Minimum manual checks:

1. Start helper on the operator machine
2. Open the Docker-hosted Hotmail page from that same machine
3. Start a Hotmail batch
4. Confirm Chromium opens locally before any Microsoft challenge appears
5. Confirm logs update in the page while the local browser runs
6. Confirm a challenge page, if any, is already local and no secondary takeover step is required

## Scope Boundaries

Included in this design:

- make local-first helper execution the default for Hotmail
- move Hotmail browser ownership from backend to helper
- add backend task orchestration and helper callbacks
- update the Hotmail page to require helper availability before start

Out of scope:

- changing non-Hotmail registration flows
- packaging the helper into Docker
- browser extension support
- native installer packaging
- automatic challenge solving

## Success Criteria

This redesign is successful if:

1. Hotmail registration begins in a local browser from the first page
2. The system no longer depends on post-challenge session migration
3. The page clearly indicates helper availability, local execution, and local manual action states
4. Backend batch tracking still works without owning the live browser
