# 33mail IMAP Register Design

## Summary

Add a new `33mail + IMAP` email-service type to the existing Codex Register stack so NAS-hosted Docker deployments can run OpenAI registration unattended while keeping `33mail` as the alias provider.

The new flow will:

- generate a fresh `33mail` alias for each registration attempt
- forward mail to a real inbox configured with IMAP credentials
- poll IMAP for the latest OpenAI verification code
- reuse the current standard and `any_auto` OpenAI registration engines
- appear inside the existing email-service management and register-task UI

## Goals

- Keep registration under the current unified `Codex-Manager -> crates/service -> vendor/codex-register` architecture.
- Avoid any browser-extension dependency or interactive mailbox UI.
- Reuse the current registration task, retry, batch, proxy, and account-import flows.
- Make `33mail` a first-class email-service type instead of a one-off task mode.

## Non-Goals

- Do not reimplement the whole StepFlow browser-extension flow in this change.
- Do not add a brand-new register mode just for `33mail`.
- Do not add schema migrations; the generic `email_services.config` JSON column is sufficient.
- Do not implement 33mail account creation or dashboard automation; the user still prepares the 33mail account and target inbox externally.

## Product Shape

### Email Service Type

Add a new type: `mail_33_imap`.

It is configured in the existing email-service page and participates in the current priority and enabled/disabled scheduling.

### Config Fields

- `alias_domain`
  - required, the 33mail alias suffix such as `example.33mail.com`
- `real_inbox_email`
  - required, the real destination inbox receiving forwarded messages
- `imap_host`
  - required
- `imap_port`
  - required, default `993`
- `imap_username`
  - required
- `imap_password`
  - required secret
- `imap_mailbox`
  - optional, default `INBOX`
- `imap_ssl`
  - optional boolean, default `true`
- `subject_keyword`
  - optional, default `OpenAI`
- `from_filter`
  - optional, default `openai.com`
- `otp_pattern`
  - optional, default six-digit OTP regex
- `poll_interval`
  - optional, default `3`
- `timeout`
  - optional, default `120`
- `alias_length`
  - optional, default `12`

## Architecture

### 1. New 33mail IMAP Email Service

Add `Mail33ImapService` under `vendor/codex-register/src/services`.

Responsibilities:

- generate random alias local-parts and build `alias@domain`
- expose the alias as the created email
- poll the configured IMAP mailbox
- filter mail by recipient, sender, subject, and sent time
- extract the newest matching OTP

It should implement the existing `BaseEmailService` contract so the registration engine can use it without mode-specific logic.

### 2. Register Service Type Catalog

Expose `mail_33_imap` through:

- `GET /api/email-services/types`
- `GET /api/registration/available-services`
- email-service stats aggregation

The register page already builds its selector dynamically from the available-service payload, so this change should slot into the current UI with minimal special handling.

### 3. Registration Flow Reuse

No new register engine is required for this phase.

The current standard registration flow already expects:

1. `create_email()`
2. submit email and password
3. `get_verification_code()`

`Mail33ImapService` can satisfy that contract directly, which is enough to make unattended NAS registration possible while keeping `33mail`.

## Data Flow

1. User creates a `mail_33_imap` email service in the UI.
2. Frontend saves it through the existing RPC and register-service route.
3. A register task selects that service directly or by priority-based rotation.
4. `Mail33ImapService.create_email()` returns a new 33mail alias.
5. OpenAI sends the OTP to that alias; 33mail forwards it to the configured real inbox.
6. `Mail33ImapService.get_verification_code()` polls IMAP, finds the latest matching message, extracts the OTP, and returns it.
7. The current registration engine completes the task and the rest of the pipeline remains unchanged.

## Error Handling

### Service Configuration Errors

- missing required IMAP settings
- invalid alias domain
- invalid numeric values for timeout, interval, or port

These should fail at service creation/update time when possible, or during service test.

### Runtime Errors

- IMAP login failure
- mailbox selection failure
- OTP timeout
- no matching mail found
- OTP extraction failure

These should surface through the existing task logs and error messages so the current retry tooling keeps working.

## Testing Strategy

### Backend

- unit tests for alias generation and domain normalization
- unit tests for IMAP message filtering and OTP extraction
- route tests for email-service type catalog and available-service listing
- route tests for stats aggregation including the new type

### Frontend

- type normalization coverage for the new stats field and available-service group
- a lightweight rendering test is optional; the form is already dynamic and the main regression risk is normalization, not JSX branching

## Rollout Notes

- Existing deployments remain compatible because the new type is additive.
- Users must still prepare a real IMAP-capable inbox and bind it to 33mail.
- Docker images may later need extra healthcheck or CA bundle verification for IMAP providers, but that is outside this change.
