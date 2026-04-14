# CPA Register Migration Design

## Summary

Refactor the current `Codex-Manager` registration stack so it keeps the existing `Next.js/Tauri + crates/service + vendor/codex-register` outer architecture, but uses the `openai-cpa` registration flow as the primary execution model.

This migration includes:

- migrating the `generator_email` mailbox flow into the current project as a first-class email service
- migrating the CPA registration core, including email verification, callback parsing, token exchange, and retry semantics
- migrating the CPA browser plugin / page-injection assets into the current register execution path
- preserving the current task, batch, RPC, account-import, and management UI shell

The end state is not "run two projects side by side". The end state is: the user stays inside `Codex-Manager`, while the registration engine inside it behaves like `openai-cpa`.

## Goals

- Keep the current project as the only user-facing application.
- Replace the existing register execution semantics with the `openai-cpa` flow semantics.
- Make `generator_email` a formal email-service type inside the current register service catalog.
- Preserve the current task, batch, proxy, logging, and account-import architecture where it still provides value.
- Let desktop/web users create, choose, and run CPA-style registration from the existing UI.

## Non-Goals

- Do not embed the `openai-cpa` standalone HTML/Vue application as a second frontend inside this repo.
- Do not maintain two independent long-term registration engines after the migration.
- Do not redesign the current database model unless the existing model cannot represent a required CPA state.
- Do not treat `generator_email` as the default or only provider; it remains one selectable provider among others.
- Do not add a large "enhancement pass" for observability or health scoring in the first migration phase beyond what is necessary to run and debug the new flow.

## Current Context

- `Codex-Manager` already has a unified register stack based on `vendor/codex-register`, routed through `crates/service`, then exposed through the current web/desktop UI.
- The current register engine has already absorbed some CPA ideas. Recent repository history includes `Align registration flow with openai-cpa`, so this is an in-progress convergence rather than a greenfield rewrite.
- `openai-cpa` contains the mailbox mode `generator_email`, a direct OpenAI registration flow in `utils/register.py`, and browser-plugin assets under `plugin/openai-cpa-plugin`.
- `openai-cpa` implements registration as a browser-driven step flow with stronger page-state detection than the current project's older register path.

## Product Shape

### User-Facing Shape

Users continue to:

- manage email services inside the current `Codex-Manager` UI
- create register tasks and register batches from the current UI
- view task logs and results in the current UI

Users do not switch to the `openai-cpa` standalone UI.

### Registration Engine Shape

Internally, register execution changes to:

- CPA-first page-step progression
- CPA-first callback and token exchange handling
- CPA-first browser plugin / injected-script coordination
- provider-based OTP retrieval including `generator_email`

The current register result objects, task records, and import path remain the outer contract.

## Architecture

### 1. CPA-First Registration Core

The main register execution path in `vendor/codex-register` should be rebuilt around the `openai-cpa` flow semantics.

Responsibilities:

- generate and track OAuth state / PKCE material in the same shape CPA expects
- create or reuse mailbox credentials from the selected email service
- drive OpenAI registration through CPA-style page transitions
- parse callback URLs, exchange tokens, and derive account metadata
- classify failures using CPA-style semantics

Implementation direction:

- keep the current `vendor/codex-register` module boundary
- move the primary registration behavior toward a CPA-derived engine
- leave `register.py` and related current modules as compatibility boundaries or thin wrappers where useful

The migration target is behavioral alignment with `openai-cpa`, not a literal copy-paste layout.

### 2. Browser Automation Assets

The assets under `openai-cpa/plugin/openai-cpa-plugin` become part of the current project's register execution resources.

Responsibilities:

- detect signup pages and key page states
- click or advance the correct signup controls
- prepare the verification flow
- distinguish retryable password-page failures, existing-account states, and normal OTP-wait states
- report actionable step information back into the register backend

This layer should be versioned and loaded by the current register stack, not treated as an unmanaged external folder.

### 3. Email Provider Layer

`generator_email` becomes a first-class email service type in the current register service catalog.

Responsibilities:

- create an inbox by loading `https://generator.email`
- parse the generated mailbox address from the returned page
- derive and store the `surl` mailbox handle
- poll the mailbox page and extract the latest OpenAI / ChatGPT OTP

This service should integrate through the existing `BaseEmailService` / factory pattern so the new register core can consume it through the same abstraction used by other mailbox providers.

### 4. Task Orchestration Adapter

The current task and batch models remain, but they need a CPA adapter layer.

Responsibilities:

- store the CPA execution context required for retries or continuation
- map CPA step outcomes onto current task and batch status fields
- write task logs using CPA step names and page classifications
- preserve current post-registration behaviors such as import, batch artifacts, and task cleanup

This avoids replacing the whole management plane while still making the task system reflect the new engine accurately.

### 5. UI Integration

The current web/desktop frontend remains the user interface, but it must expose the new behavior.

Required additions:

- `generator_email` in email-service type creation and listing flows
- task selection and available-service payload support for the new provider
- clearer log and status rendering for CPA-first step transitions and failure reasons

The goal is "CPA flow inside current UI", not "CPA UI inside current app".

## Data Flow

1. The user creates or selects an email service in the current UI.
2. The frontend sends the request through existing RPC entrypoints in `crates/service`.
3. `crates/service` forwards register requests to `vendor/codex-register`.
4. The CPA-first register engine requests mailbox credentials from the selected provider.
5. If the selected provider is `generator_email`, it returns `email + surl`.
6. The browser automation layer drives the OpenAI signup flow using CPA-style step detection.
7. When OTP is required, the engine asks the provider adapter for a verification code.
8. The engine completes callback parsing and token exchange using CPA-style semantics.
9. The result is mapped back into the current account-import, task-result, and batch-result pipeline.

## Failure Handling

### Retryable Failures

- page load timeout
- temporary network failure
- OTP not yet available
- transient token exchange failure
- recognized retryable signup/password page states from CPA logic

### Non-Retryable Failures

- email already exists
- explicit signup rejection
- invalid email-service configuration
- plugin / injected-script boot failure that makes the flow impossible
- callback/token payloads that are structurally invalid

### Manual-Intervention Failures

- unrecognized new page layouts
- persistent human-verification blockers
- long-running stalls where the engine can no longer determine the next safe action

Logs should record the CPA step or page classification directly so failures are diagnosable from the current task UI.

## Migration Boundaries

### Migrate

- `openai-cpa` mailbox provider behavior for `generator_email`
- `openai-cpa` registration core behavior
- `openai-cpa` callback / token exchange semantics
- `openai-cpa` plugin and injected page-step assets
- current project service catalog, RPC, and UI wiring needed to expose the above

### Do Not Migrate

- the standalone `openai-cpa` index page and its independent dashboard shell
- a second independent configuration system
- a second independent task model or result database

## Rollout Plan

### Phase 1: Provider Migration

- add `generator_email` as a formal email-service type
- add parsing and OTP extraction coverage
- expose it through service-type and available-service APIs

### Phase 2: Single-Task CPA Core

- migrate the CPA register core into `vendor/codex-register`
- make a single register task run end-to-end with the new semantics
- keep the outer current task contract stable

### Phase 3: Browser Plugin / Script Migration

- move CPA browser assets into the current project
- integrate script loading and page-state reporting with current task execution
- surface richer step logs

### Phase 4: Batch and UI Completion

- wire the new provider and engine fully into register batch execution
- complete frontend affordances for provider creation, selection, and log display
- run full regression checks on desktop/web build and register-related tests

## Testing Strategy

### Backend Unit Tests

- `generator_email` address parsing
- `generator_email` `surl` derivation
- `generator_email` OTP extraction
- callback parsing and token-response normalization
- CPA page-state classification and retry classification

### Integration Tests

- email-service type catalog includes `generator_email`
- available-service payload exposes the new type correctly
- register request forwarding remains RPC-compatible
- single-task CPA execution maps cleanly into current task results

### Verification

- targeted `vendor/codex-register` pytest coverage for the migrated provider and flow modules
- register-related Rust tests in `crates/service`
- `pnpm run build:desktop`

## Risks

- The CPA browser/plugin assets may not fit the current runtime boot path without additional adapter work.
- `generator.email` is HTML-scrape based and less stable than self-hosted or IMAP-backed providers.
- Some current register modules already partially diverged from CPA; merging behavior without reintroducing regressions requires careful contract tests.
- Batch retry and task-resume behavior may expose hidden assumptions in the current task model.

## Recommendation

Adopt a CPA-first internal migration while preserving the current project shell:

- keep `Codex-Manager` as the only product surface
- migrate the registration behavior to `openai-cpa` semantics
- integrate `generator_email` as a formal provider
- avoid embedding the standalone CPA frontend or keeping dual long-term register engines

This gives the user the current management UI and deployment model, while aligning the actual registration engine with the flow they explicitly want to standardize on.
