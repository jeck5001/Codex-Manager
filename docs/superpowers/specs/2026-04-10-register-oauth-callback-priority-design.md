# Register OAuth Callback Priority Design

## Goal

Change the OpenAI/Codex registration authorization flow so that `consent + callback` resolution stays on the primary path, while `add_phone` is treated as a low-priority signal instead of a blocking state-machine branch.

## Problem

The current backend gives `add_phone` too much weight:

- post-registration callback resolution attempts an `add_phone` login bypass before normal OAuth convergence
- login OTP recovery stops early on `add_phone`
- auth result resolution short-circuits instead of continuing into `continue_url` / authenticated OAuth fallback

This differs from the reference browser extension flow, which keeps driving toward OAuth consent and loopback callback capture and does not model `add_phone` as a first-class flow step.

## Decision

Adopt these rules for the backend registration flow:

1. `add_phone` does not preempt workspace / OAuth callback resolution.
2. When a `continue_url` points to `/add-phone`, the runner should still try:
   - workspace authorization discovery
   - authenticated OAuth URL fallback
3. Any-Auto should continue trying current-session ChatGPT session reuse before OAuth convergence, even if post-create state indicates `add_phone`.
4. `add_phone` may still be logged or surfaced in metadata elsewhere, but it should not be the primary branching condition for post-registration callback handling.

## Scope

In scope:

- `vendor/codex-register/src/core/register_flow_runner.py`
- `vendor/codex-register/src/core/any_auto_register.py`
- tests covering callback resolution and Any-Auto ordering

Out of scope:

- local-first browser consent executor
- localhost callback listener helper for OpenAI registration
- UI changes

## Expected Behavior

- post-registration callback resolution prefers workspace selection or authenticated OAuth redirect convergence
- login OTP recovery can still land on callback even if intermediate payload reports `add_phone`
- Any-Auto no longer forces add-phone bypass before session reuse

## Verification

- targeted pytest coverage for register flow runner
- targeted pytest coverage for Any-Auto runner
- regression coverage for existing add-phone-related fallback tests that should continue to pass
