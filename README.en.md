<p align="center">
  <img src="assets/logo/logo.png" alt="CodexManager Logo" width="220" />
</p>

<h1 align="center">CodexManager</h1>

<p align="center">A local desktop + service toolkit for Codex-compatible account and gateway management.</p>

<p align="center">
  <a href="README.md">中文</a>
</p>

## Overview

CodexManager manages Codex-compatible account pools, usage, platform keys, and exposes a local OpenAI-compatible gateway.

Supported runtime modes:

- Desktop mode: Tauri desktop app + local service
- Service mode: `codexmanager-service` + `codexmanager-web`

## Quick Start

### Desktop

1. Launch the app.
2. Click `Start Service`.
3. Import accounts or complete login in the Accounts page.
4. Adjust listener address, gateway policy, upstream proxy, and related settings when needed.

### Service Edition

1. Download the matching `CodexManager-service-<platform>-<arch>.zip` from Releases.
2. Start `codexmanager-start` first when possible.
3. Default addresses: service `localhost:48760`, Web UI `http://localhost:48761/`.

## Common Entry Points

- Version history: [CHANGELOG.md](CHANGELOG.md)
- Contribution guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- Testing baseline: [TESTING.md](TESTING.md)
- Security policy: [SECURITY.md](SECURITY.md)
- Docs index: [docs/README.md](docs/README.md)

## Release and Assets

- Unified workflow: `.github/workflows/release-all.yml`
- Release guide and asset matrix: [docs/release/20260309195355216_发布与产物说明.md](docs/release/20260309195355216_发布与产物说明.md)
- Environment variables and runtime config: [docs/report/20260309195355187_环境变量与运行配置说明.md](docs/report/20260309195355187_环境变量与运行配置说明.md)

Current primary deliverables:

- Desktop: Windows installer / portable, macOS dmg, Linux AppImage / deb
- Service: Windows / macOS / Linux archives

## Doc Index

### Governance and Plans

- [docs/plan/20260309191759589_长期维护治理TODO.md](docs/plan/20260309191759589_长期维护治理TODO.md)
- [docs/report/20260309191759589_长期维护结构优化建议.md](docs/report/20260309191759589_长期维护结构优化建议.md)

### Operations and Troubleshooting

- [docs/report/20260307234235414_最小排障手册.md](docs/report/20260307234235414_最小排障手册.md)
- [docs/report/20260309195355187_环境变量与运行配置说明.md](docs/report/20260309195355187_环境变量与运行配置说明.md)

### Release

- [docs/release/20260309195355216_发布与产物说明.md](docs/release/20260309195355216_发布与产物说明.md)

## Contact

<p align="center">
  <img src="assets/images/group.jpg" alt="Telegram Group QR Code" width="280" />
  <img src="assets/images/qq_group.jpg" alt="QQ Group QR Code" width="280" />
</p>

- Telegram group: <https://t.me/+8o2Eu7GPMIFjNDM1>
- QQ group: scan the QR code
- WeChat Official Account: 七线牛马
