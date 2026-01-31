# Login Callback Port Warning Plan

Goal: When localhost:1455 is occupied, return a clear error so UI can prompt the user.

## Tasks
1) Add failing test for port-in-use warning in `crates/gpttools-service/src/auth_callback.rs`.
2) Implement login server binding with explicit AddrInUse message.
3) Ensure login_start surfaces login server errors.
4) Verify with `cargo test -p gpttools-service login_server_reports_port_in_use`.

## Rollback
- Revert auth_callback/auth_login changes and remove the new test.
