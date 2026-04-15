# Register Local Engine Migration Design

## Summary

Replace the current `Codex-Manager` registration execution model with a local native engine that reproduces the `openai-cpa` registration flow inside this repository.

This migration is specifically about fixing the core registration logic, not just adding mailbox providers. The current project still proxies registration execution to an external register service, so prior `generator_email` migration work did not actually replace the broken register flow. This design corrects that.

The end state is:

- users still use the current `Codex-Manager` UI and RPC entrypoints
- registration tasks and batches are still managed in the current app
- the actual registration execution path now follows the `openai-cpa` flow semantics locally
- `generator_email` becomes the first mailbox provider supported by the new local engine

## Goals

- Replace the current external register-service execution path with a local register engine.
- Rebuild the local register engine around the real `openai-cpa/utils/register.py` flow.
- Preserve the current frontend and RPC contract as much as possible.
- Preserve current task, batch, account-import, and task-log surfaces where feasible.
- Make `generator_email` the first fully supported mailbox provider in the new local engine.

## Non-Goals

- Do not keep the current external register service as the primary execution engine.
- Do not embed or run the standalone `openai-cpa` application as a sidecar product.
- Do not fully migrate every `openai-cpa` mailbox provider in the first pass.
- Do not fully migrate HeroSMS, Browserbase, Hotmail, or all phone-verification branches in phase one.
- Do not redesign the frontend registration UX beyond what is required for compatibility with the new engine.

## Problem Statement

### Current Behavior

The current registration entrypoints in [`crates/service/src/account/account_register.rs`](/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/.worktrees/cpa-sync-scheduler/crates/service/src/account/account_register.rs) still forward registration work to an external register service:

- `start_register_task()` -> `POST /api/registration/start`
- `start_register_batch()` -> `POST /api/registration/batch`
- task reads / logs / batch reads also depend on external APIs

This means the app is still using the old execution engine even if some provider-related pieces have already been migrated.

### Root Cause

The previous migration effort moved parts of the integration surface:

- service catalog exposure
- email-service type support
- `generator_email` provider availability

But it did not replace the underlying register execution engine. As a result, the actual registration flow logic never changed.

### Required Correction

To truly "change our registration flow logic to his project's flow", the app must stop depending on the external register-service execution path and instead run the CPA-style flow locally.

## Reference Flow to Migrate

The source-of-truth execution model is `openai-cpa/utils/register.py`.

The core path to reproduce locally is:

1. allocate mailbox credentials
2. start OAuth / authorize session
3. submit signup email via `authorize/continue`
4. submit password via `user/register`
5. send and validate email OTP when required
6. submit profile data via `create_account`
7. perform delayed silent OAuth login
8. handle second OTP verification when the login chain requires it
9. handle consent / workspace selection
10. extract callback URL
11. exchange code for tokens
12. map tokens into the current account-import pipeline

This flow is the migration target. Behavioral equivalence matters more than matching `openai-cpa` file structure line-for-line.

## Target Product Shape

### User-Facing Shape

Users continue to:

- create register tasks from the current modal and frontend
- create register batches from the current modal and frontend
- read task logs, statuses, and import results in the current UI
- manage mailbox services in the current UI

Users do not switch to a second application or separate registration dashboard.

### Execution Shape

Internally, registration changes from:

- `Codex-Manager UI -> crates/service -> external register service`

to:

- `Codex-Manager UI -> crates/service -> local register runtime -> local CPA-style engine`

This is the critical behavioral replacement.

## Architecture

### 1. Keep RPC and Frontend Contracts Stable

The following current entrypoints remain the public interface:

- `account/register/start`
- `account/register/batch/start`
- `account/register/tasks/list`
- `account/register/task/*`
- existing frontend mutations in [`apps/src/lib/api/account-client.ts`](/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/.worktrees/cpa-sync-scheduler/apps/src/lib/api/account-client.ts)
- existing frontend flows in [`apps/src/components/modals/add-account-modal.tsx`](/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/.worktrees/cpa-sync-scheduler/apps/src/components/modals/add-account-modal.tsx)

The UI should not need a product redesign just because the engine changes.

### 2. Replace External Register-Service Execution With Local Runtime

[`crates/service/src/account/account_register.rs`](/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/.worktrees/cpa-sync-scheduler/crates/service/src/account/account_register.rs) currently acts as an HTTP proxy to the external register service. This must be refactored into a local task/batch orchestrator.

Responsibilities after migration:

- accept start / batch / cancel / read RPC requests
- create and store local task state
- spawn local task workers
- collect structured step logs
- manage import handoff
- expose current task and batch snapshots without external HTTP dependency

Compatibility fallback to the old external service may exist temporarily for uncovered scenarios, but it must not remain the default engine.

### 3. Introduce a Local Register Engine Layer

Add a focused local engine layer under `crates/service/src/account/` with clear module boundaries:

- `register_engine.rs`
  - single-task orchestration
  - CPA-style flow transitions
  - failure classification
- `register_http.rs`
  - OpenAI/Auth HTTP helpers
  - retry wrappers
  - redirect following
  - callback parsing
  - token exchange
- `register_email/`
  - provider trait and provider-specific adapters
  - mailbox allocation
  - OTP polling / extraction
- `register_runtime.rs`
  - in-memory task registry
  - batch orchestration
  - cancellation state
  - progress snapshots

The intent is to avoid one giant `account_register.rs` file that mixes HTTP, mailbox, runtime, and RPC glue.

### 4. Introduce a Structured Task State Machine

Local task status should reflect real execution stages instead of opaque external-service task strings.

Recommended status progression:

- `queued`
- `preparing_email`
- `submitting_signup`
- `waiting_email_otp`
- `validating_email_otp`
- `creating_account`
- `oauth_login`
- `waiting_login_otp`
- `selecting_workspace`
- `extracting_tokens`
- `importing`
- `succeeded`
- `failed`
- `canceled`

These states map directly to the migrated CPA flow and make failures diagnosable.

### 5. First-Class Structured Failure Reasons

The runtime should distinguish at least these machine-readable failure codes:

- `email_provider_failed`
- `signup_blocked`
- `password_submit_failed`
- `otp_timeout`
- `otp_invalid`
- `create_account_failed`
- `oauth_failed`
- `workspace_select_failed`
- `token_extract_failed`
- `import_failed`
- `canceled`

The UI may still show human-readable text, but the runtime should not rely on raw log text as the only error contract.

### 6. Migrate Mailbox Abstractions, Starting With `generator_email`

The new local engine needs a unified mailbox abstraction derived from `openai-cpa/utils/email_providers/mail_service.py`.

The first mandatory provider is `generator_email`, using the behavior from:

- `openai-cpa/utils/email_providers/generator_email_service.py`
- the OTP polling flow from `openai-cpa/utils/email_providers/mail_service.py`

Provider responsibilities:

- create mailbox
- return `email + provider credential` payload
- poll for the newest OpenAI/ChatGPT OTP
- return normalized six-digit OTP

The provider interface should be generic enough to absorb later providers without reworking the core engine.

### 7. Preserve Account Import as the Final Handoff

The final output of successful registration should still feed the current account import / storage pipeline rather than inventing a second account persistence mechanism.

The new engine should produce the equivalent normalized token payload, then import it into the existing account storage flow.

## Data Flow

### Single Registration

1. frontend starts a register task through existing RPC
2. service creates a local task record
3. local engine resolves selected email provider
4. local provider allocates mailbox credentials
5. engine executes CPA-style signup sequence
6. provider polling is used when OTP is required
7. engine continues through create-account and silent OAuth login
8. engine extracts final callback URL and exchanges tokens
9. engine imports the resulting account
10. task status and logs are finalized locally

### Batch Registration

1. frontend starts a register batch through existing RPC
2. service expands the batch into local queued tasks
3. batch runtime applies configured concurrency and interval rules
4. each task runs through the same local single-task engine
5. task results roll up into the batch snapshot
6. batch cancellation sets local cancellation flags and stops new work

## Logging Model

Each task should write step logs with explicit stage names, for example:

- mailbox allocated
- signup email submitted
- password submitted
- email OTP requested
- email OTP received
- email OTP validated
- account profile submitted
- silent OAuth login started
- login OTP requested
- login OTP validated
- workspace selected
- callback extracted
- tokens exchanged
- account imported

This is required both for debugging and for parity with the observability users expect from the current task UI.

## Compatibility Strategy

### Public Compatibility

- keep frontend request payloads stable where possible
- keep RPC names stable
- keep task read/list response structure stable where feasible
- preserve current account import expectations

### Migration Compatibility

The external register-service code path may remain behind a temporary fallback flag during migration, but:

- it must not be the default path after the feature lands
- it must not be used for `generator_email`
- it should be removed after local engine coverage is sufficient

## Phased Delivery

### Phase 1: Local Single-Task Core + `generator_email`

- create local task runtime skeleton
- create register engine state machine
- implement local HTTP/auth helpers
- implement local `generator_email` provider
- make `account/register/start` use the local engine
- import successful accounts locally

### Phase 2: Local Batch Runtime

- implement local batch creation and scheduling
- implement concurrency and interval handling
- implement batch cancellation
- map batch snapshots to current frontend expectations

### Phase 3: Task Read / Logs / Import Parity

- replace external task read/list/log queries
- expose local logs and failure codes
- preserve current frontend compatibility

### Phase 4: Cleanup and Fallback Reduction

- make local engine the default everywhere
- narrow or remove external fallback paths
- remove obsolete proxy-only code once no longer needed

## Testing Strategy

### Rust Unit Tests

- task state machine transitions
- failure code mapping
- callback parsing
- token-response normalization
- `generator_email` mailbox parsing
- `generator_email` OTP extraction

### Rust Integration Tests

- `account/register/start` creates local tasks instead of external HTTP dependency
- local task snapshots expose expected status/log fields
- successful local registration result feeds account import pipeline
- batch runtime respects concurrency, interval, and cancel semantics

### Frontend Compatibility Verification

- existing register modal still works with current RPC shape
- task list / batch list rendering remains compatible with the new local statuses
- desktop build still succeeds

### Required Verification Commands

- targeted Rust tests for new register engine modules
- targeted Rust tests for RPC/register behavior
- `pnpm run build:desktop`

## Risks

- `openai-cpa` is Python and imperative; translating behavior into Rust without losing edge-case handling requires contract tests around the migrated flow.
- `generator.email` is scrape-based and may change markup.
- keeping public compatibility while replacing the entire engine will surface hidden assumptions in current task readers and frontend progress UI.
- batch scheduling can easily regress cancellation or progress reporting if runtime state is not modeled explicitly.

## Recommendation

Proceed with a local-engine migration that keeps the current UI/RPC shell but replaces the actual registration execution path inside `Codex-Manager`.

This is the only approach that actually fixes the user's complaint:

- not merely "provider migrated"
- not merely "interface aligned"
- but the real registration flow logic changed to match `openai-cpa`

## Relationship to Prior Spec

This document supersedes the incomplete execution interpretation from [`docs/superpowers/specs/2026-04-14-cpa-register-migration-design.md`](/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/.worktrees/cpa-sync-scheduler/docs/superpowers/specs/2026-04-14-cpa-register-migration-design.md) where the intended engine replacement was not fully enforced in implementation.

The key clarification is:

- previous work moved some integration surfaces
- this spec requires replacing the local execution engine itself
