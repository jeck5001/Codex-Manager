# Embed Service Into Desktop App (Single EXE) Plan

Goal: Run gpttools-service in-process so portable output is a single `CodexManager.exe` and no sidecar exe is required.

## Tasks
1) Add failing test for new shutdown flag helpers in `crates/gpttools-service/tests/shutdown_flag.rs`.
2) Implement shutdown flag + shutdown request handling in `crates/gpttools-service` (lib + http server).
3) Switch desktop app to in-process service:
   - add `gpttools-service` dependency in `apps/src-tauri/Cargo.toml`
   - remove sidecar metadata and `externalBin` in `apps/src-tauri/tauri.conf.json`
   - update `apps/src-tauri/src/lib.rs` to spawn/stop in-process thread
4) Update packaging scripts: `rebuild.ps1` and `rebuild.test.ps1`.
5) Verify: `cargo test -p gpttools-service --test shutdown_flag` (and re-run existing node test if needed).

## Rollback
- Restore sidecar config and spawn_service_with_addr to external process.
- Remove shutdown flag helpers and tests.
- Restore rebuild script to copy sidecar exe.
