# Manual Callback Parse Plan

Goal: Allow users to paste the OAuth callback URL and complete login manually when the localhost callback port is occupied.

## Tasks
1) Add failing UI wiring test for manual callback input/button + API method.
2) Add failing backend test for `account/login/complete` when params missing.
3) Implement backend:
   - `auth_tokens::complete_login_with_redirect`
   - new RPC method `account/login/complete`
4) Implement frontend:
   - modal input/button
   - parse callback URL and call new RPC
5) Verify tests: `node --test apps\tests\login-manual-callback.test.mjs` and `cargo test -p gpttools-service login_complete_requires_params`.

## Rollback
- Remove new RPC + UI + tests.
