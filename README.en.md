<p align="center">
  <img src="assets/logo/logo.png" alt="CodexManager Logo" width="220" />
</p>

<h1 align="center">CodexManager</h1>

<p align="center">A local desktop + service toolkit for Codex-compatible account and gateway management.</p>

<p align="center">
  <a href="README.md">中文</a>
</p>

A local desktop + service toolkit for managing Codex-compatible accounts, usage, platform keys, and a built-in local gateway.

## Recent Changes
- Current latest version: `v0.1.6` (2026-03-07)
- The current `main` branch continues to harden the Web auth flow: `codexmanager-web` still persists the password, but authenticated sessions are now scoped to the current Web process, so old cookies no longer survive a full close-and-reopen cycle.
- Protocol adaptation keeps moving closer to Codex / OpenAI-compatible behavior: `/v1/chat/completions` and `/v1/responses` forwarding are more aligned, and the `tools` / `tool_calls` aggregation, shortened tool-name mapping, and response restoration paths are now preserved across Cherry Studio, OpenClaw, Claude Code, and similar clients.
- Gateway diagnostics are richer: failure responses now expose structured `errorCode` / `errorDetail` fields, plus `X-CodexManager-Error-Code` and `X-CodexManager-Trace-Id` headers.
- The release pipeline remains consolidated under `release-all.yml` for one-click Windows / macOS / Linux publishing.
- Full history is available in [CHANGELOG.md](CHANGELOG.md).

## Features
- Account pool management: groups, tags, sorting, notes
- Bulk import / export: multi-file import, recursive desktop folder import for JSON, one-file-per-account export
- Usage dashboard: 5-hour + 7-day windows, plus accounts that only expose a 7-day window
- OAuth login: browser flow + manual callback parsing
- Platform keys: create, disable, delete, model binding
- Local service with configurable port
- Local OpenAI-compatible gateway for CLI and third-party tools

## Screenshots
![Dashboard](assets/images/dashboard.png)
![Accounts](assets/images/accounts.png)
![Platform Key](assets/images/platform-key.png)
![Logs](assets/images/log.png)
![Settings](assets/images/themes.png)

## Quick Start
1. Launch the desktop app and click `Start Service`.
2. Go to Accounts, add an account, and complete authorization.
3. If callback parsing fails, paste the callback URL manually.
4. Refresh usage and confirm the account status.

## Core Docs
- Version history: [CHANGELOG.md](CHANGELOG.md)
- Contribution guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- Testing baseline: [TESTING.md](TESTING.md)
- Security: [SECURITY.md](SECURITY.md)
- Docs index: [docs/README.md](docs/README.md)

## Usage and Deployment
- Runtime and deployment guide: [docs/report/20260310122606850_运行与部署指南.md](docs/report/20260310122606850_运行与部署指南.md)
- Environment variables and runtime config: [docs/report/20260309195355187_环境变量与运行配置说明.md](docs/report/20260309195355187_环境变量与运行配置说明.md)
- FAQ and account-hit rules: [docs/report/20260310122606852_FAQ与账号命中规则.md](docs/report/20260310122606852_FAQ与账号命中规则.md)
- Minimal troubleshooting guide: [docs/report/20260307234235414_最小排障手册.md](docs/report/20260307234235414_最小排障手册.md)

## Build and Release
- Build, release, and script guide: [docs/release/20260310122606851_构建发布与脚本说明.md](docs/release/20260310122606851_构建发布与脚本说明.md)
- Release assets guide: [docs/release/20260309195355216_发布与产物说明.md](docs/release/20260309195355216_发布与产物说明.md)
- Script and release responsibility matrix: [docs/report/20260309195735631_脚本与发布职责对照.md](docs/report/20260309195735631_脚本与发布职责对照.md)
- Protocol regression checklist: [docs/report/20260309195735632_协议兼容回归清单.md](docs/report/20260309195735632_协议兼容回归清单.md)

## Project Structure
```text
.
├─ apps/                # Frontend and Tauri desktop app
│  ├─ src/
│  ├─ src-tauri/
│  └─ dist/
├─ crates/              # Rust core/service crates
│  ├─ core
│  ├─ service
│  ├─ start              # Service starter (launches service + web)
│  └─ web                # Service Web UI (optional embedded assets + /api/rpc proxy)
├─ docs/                # Formal project documentation
├─ scripts/             # Build and release scripts
├─ portable/            # Portable output directory
└─ README.en.md
```

## Contact

<p align="center">
  <img src="assets/images/group.jpg" alt="Telegram Group QR Code" width="280" />
  <img src="assets/images/qq_group.jpg" alt="QQ Group QR Code" width="280" />
</p>

- Telegram group: <https://t.me/+8o2Eu7GPMIFjNDM1>
- QQ group: scan the QR code
- WeChat Official Account: 七线牛马
