<p align="center">
  <img src="assets/logo/logo.png" alt="CodexManager Logo" width="220" />
</p>

<h1 align="center">CodexManager</h1>

<p align="center">A local desktop + service toolkit for Codex-compatible account and gateway management.</p>

<p align="center">
  <a href="README.en.md">中文</a>|
  <a href="https://github.com/qxcnm/Codex-Manager">GitHub Repo</a>|
  <a href="https://qxnm.top">Official Website</a>|
  <a href="#sponsor">Sponsor</a>
</p>

A local desktop + service toolkit for managing Codex-compatible accounts, usage, platform keys, and a built-in local gateway.

## Star Chart
<p align="center">
  <img src="assets/images/star-history.png" alt="Star Chart" width="900" />
</p>

## Source Code Description:
> This product is completely by my command + AI to build Codex (98%) Gemini (2%) If the use of the process of generating problems please friendly exchanges, because the open source just think that someone can use, the basic function is no problem, do not like do not spray.
> Secondly, I do not have enough environment to verify that each package has no problem, I have to go to work (I'm just a poor bastard can not afford to buy macs and so on), I only guarantee the availability of Win desktop, if there are problems with the other end, please feedback in the exchange group or submit the Issues after sufficient testing, I will deal with it when I have time.
> Finally, I would like to thank all the users in the A-flow group for their feedback on the various platforms and their participation in some of the tests. 


## Disclaimer

- This project is for learning and development purposes only.

- Users must comply with the terms of service of all relevant platforms (e.g., OpenAI, Anthropic).

- The author does not provide or distribute any accounts, API keys, or proxy services, and is not responsible for how this software is used.

- Do not use this project to bypass rate limits or service restrictions

## Landing Guide
| What you want to do | Go here |
| --- | --- |
| First launch, deployment, Docker, macOS allowlist | [Runtime and deployment guide](docs/report/运行与部署指南.md) |
| Configure port, proxy, database, Web password, environment variables | [Environment variables and runtime config](docs/report/环境变量与运行配置说明.md) |
| Troubleshoot account selection, import failures, challenge blocks, request issues | [FAQ and account-hit rules](docs/report/FAQ与账号命中规则.md) |
| Fast plugin-center integration and minimal onboarding | [Plugin center minimal integration guide](docs/report/插件中心最小接入说明.md) |
| Integrate the plugin center, API list, market modes, and Rhai interfaces | [Plugin center integration and interface list](docs/report/插件中心对接与接口清单.md) |
| Full internal interface inventory | [System internal interface inventory](docs/report/系统内部接口总表.md) |
| Build locally, package, publish, run scripts | [Build, release, and script guide](docs/release/构建发布与脚本说明.md) |

## Recent Changes
  - Current latest version: `v0.1.14` (2026-03-30)
- This round of high-concurrency protection is now in place: the Settings page includes a System Derive button and a single-account concurrency limit, it applies CPU / memory-based recommendations immediately, and the ingress path now uses short queue waits plus fast overload degradation to keep the service alive under heavy traffic.
- The Chinese and English READMEs now both cover the concurrency-protection flow, and the recent commit highlights have been refreshed to the latest commit.
- Added an “Aggregate API” management page: manage multiple third-party relay providers as minimal upstreams, with `Codex / Claude` categorization, supplier name, sort order, URL, key, and connection testing.
- Platform-key rotation now supports `Account Rotation` and `Aggregate API Rotation`; Aggregate API rotation prefers the configured provider order first, then forwards upstream requests by protocol while keeping account rotation unchanged.
  - `v0.1.14` continues the high-concurrency protection and docs cleanup from this round: the ingress path now uses short queue waits plus fast overload degradation, the Settings page adds System Derive and a single-account concurrency limit, and the README reflects the latest version notes.
- Account management adds the most practical governance features from this round: `account_deactivated` and `workspace_deactivated` are now recognized as unavailable signals, the list supports a dedicated `Banned` filter, and the actions menu can clean banned accounts in one click.
- The 5-hour and 7-day quota columns now show reset timestamps under each progress bar. Free accounts that only expose a 7-day window also render the reset time under the 7-day column instead of the wrong bucket.
- Platform keys now support service tier overrides with `Follow Request`, `Fast`, and `Flex`. `Fast` maps to upstream `priority`, while `Flex` is forwarded as `flex`; the desktop create/edit flow now saves and round-trips these values correctly.
- The Settings page restores the service listen-mode switch so you can choose between `localhost` and `0.0.0.0`; the `Check for Updates` button now shows loading only for manual checks.
- Web and desktop interaction bugs were also cleaned up: refreshing non-home Web routes no longer downloads the wrong file, and clipboard actions now degrade gracefully when `navigator.clipboard.writeText` is unavailable.
  - The release path stays unified: the product version is now `0.1.14`, and the workspace, frontend package, Tauri desktop app, release-version checks, and README version notes are all kept in sync. See [CHANGELOG.md](CHANGELOG.md) for the full history.

### Recent Commit Highlights
- `85022b9`: improve high-concurrency protection and docs. The ingress path now uses short queue waits plus fast overload degradation, the Settings page adds System Derive and a single-account concurrency limit, and both READMEs were refreshed.
- `a6a96d6`: add a plugin-center preview image to the README. Both the Chinese and English README screenshot sections now include `plugin.png`.
- `ec03f2c`: remove date prefixes from long-lived docs. Long-term documents now use stable filenames, and README links were updated accordingly.
- `927142a`: adjust the default interval for the scheduled script. The cleanup script now runs every minute by default, while users can still customize it.
- `028c8c8`: add the scheduled-script entry and the internal interface inventory. The Accounts page now exposes the scheduled-script action, and the docs include the full system interface list.
- `885edd0`: improve plugin-center docs and onboarding. The minimal integration guide and the full interface list are both completed.

## Features
- Account pool management: groups, tags, sorting, notes, banned detection, and banned filtering
- Bulk import / export: multi-file import, recursive desktop folder import for JSON, one-file-per-account export
- Usage dashboard: 5-hour + 7-day windows, plus accounts that only expose a 7-day window, with per-window reset timestamps
- OAuth login: browser flow + manual callback parsing
- Platform keys: create, disable, delete, model binding, reasoning effort, and service tier overrides (`Follow Request` / `Fast` / `Flex`)
- Aggregate API: manage third-party minimal upstream relays with create/edit, connection testing, supplier name, sort priority, and `Codex / Claude` categorization
- Settings page concurrency controls: a System Derive button, a single-account concurrency limit, and a short-queue overload-degradation path
- Plugin center minimal integration: route `/plugins/`, builtin/private/custom market modes, and the smallest manifest/RPC/Rhai contract for quick onboarding
- Plugin center: route `/plugins/`, supports builtin curated, private, and custom market modes, with plugin manifests, tasks, logs, and Rhai integration
- System internal interface inventory: a single place for desktop commands, RPC methods, and plugin built-ins
- Local service with configurable port and listen address
- Local OpenAI-compatible gateway for CLI and third-party tools

## Screenshots
![Dashboard](assets/images/dashboard.png)
![Accounts](assets/images/accounts.png)
![Platform Key](assets/images/platform-key.png)
![Aggregate API](assets/images/aggregate-api.png)
![Plugin Center](assets/images/plug.png)
![Logs](assets/images/log.png)
![Settings](assets/images/themes.png)

## Quick Start
1. Launch the desktop app and click `Start Service`.
2. Go to Accounts, add an account, and complete authorization.
3. If callback parsing fails, paste the callback URL manually.
4. Refresh usage and confirm the account status.

## Page Overview
### Desktop
- Accounts: bulk import/export, refresh accounts and usage, plus low-quota / banned filters and reset-time display
- Platform Keys: bind keys by model, reasoning effort, and service tier, then inspect request logs
- Plugin center minimal integration: `/plugins/` route, smallest plugin manifest, RPC surface, and Rhai built-ins
- Plugin center: `/plugins/` route, builtin/private/custom market switching, plugin install/enable/disable, tasks, logs, and Rhai integration
- Settings: manage ports, listen address, proxy, theme, auto-update, background behavior, and concurrency tuning

### Service Edition
- `codexmanager-service`: local OpenAI-compatible gateway
- `codexmanager-web`: browser-based management UI
- `codexmanager-start`: one command to launch service + web

## Core Docs
- Version history: [CHANGELOG.md](CHANGELOG.md)
- Contribution guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- Testing baseline: [TESTING.md](TESTING.md)
- Security: [SECURITY.md](SECURITY.md)
- Docs index: [docs/README.md](docs/README.md)

## Topic Pages
| Page | Content |
| --- | --- |
| [Runtime and deployment guide](docs/report/运行与部署指南.md) | First launch, Docker, Service edition, macOS allowlist |
| [Environment variables and runtime config](docs/report/环境变量与运行配置说明.md) | App config, proxy, listen address, database, Web security |
| [FAQ and account-hit rules](docs/report/FAQ与账号命中规则.md) | Account hit logic, challenge blocks, import/export, common issues |
| [Minimal troubleshooting guide](docs/report/最小排障手册.md) | Fast path for service startup, forwarding, and model refresh issues |
| [Plugin center minimal integration guide](docs/report/插件中心最小接入说明.md) | Plugin center minimal manifest, RPC surface, and Rhai built-ins |
| [Plugin center integration and interface list](docs/report/插件中心对接与接口清单.md) | Plugin center route, market modes, Tauri/RPC interfaces, manifest fields, Rhai built-ins |
| [System internal interface inventory](docs/report/系统内部接口总表.md) | Desktop commands, service RPC methods, and plugin built-ins |
| [Build, release, and script guide](docs/release/构建发布与脚本说明.md) | Local build, Tauri packaging, Release workflow, script flags |
| [Release assets guide](docs/release/发布与产物说明.md) | Platform artifacts, naming, release vs pre-release |
| [Script and release responsibility matrix](docs/report/脚本与发布职责对照.md) | Which script owns which step |
| [Protocol regression checklist](docs/report/协议兼容回归清单.md) | `/v1/chat/completions`, `/v1/responses`, tools regression items |
| [CHANGELOG.md](CHANGELOG.md) | Latest release notes, unreleased changes, and full version history |

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
└─ README.en.md
```

## Acknowledgements And References

- Codex (OpenAI): this project references its implementation and source layout for request-path behavior, login semantics, and upstream compatibility <https://github.com/openai/codex>

## Recognized Community
<p align="center">
  <a href="https://linux.do/t/topic/1688401" title="LINUX DO">
    <img
      src="https://cdn3.linux.do/original/4X/d/1/4/d146c68151340881c884d95e0da4acdf369258c6.png?style=for-the-badge&logo=discourse&logoColor=white"
      alt="LINUX DO"
      width="100"
      hight="100"
    />
  </a>
</p>

## Sponsor

Thanks to everyone supporting CodexManager. Your sponsorship and donations help keep the project actively maintained and steadily improved.

Special thanks to Fang Mumu, [Wonderdch](https://github.com/Wonderdch), and Catch_Bat for their support.

- Fang Mumu: thanks for providing token support for the project. His GPT card service supports self-service purchase and activation, offers stable access, a 30-day guarantee, and support for Codex 5.4. Website: [https://www.aixiamo.com/](https://www.aixiamo.com/)
- Donation acknowledgements: [Wonderdch](https://github.com/Wonderdch), Catch_Bat

If this project helps you, you are welcome to support its ongoing maintenance and updates.

<p align="left">
  <img src="assets/images/wechatPay.jpg" alt="WeChat sponsor QR code" width="180" />
  <img src="assets/images/AliPay.jpg" alt="Alipay sponsor QR code" width="180" />
</p>

## Contact Information
- Official Account: 七线牛马
- WeChat: ProsperGao
- Telegram group: [CodexManager TG Group](https://t.me/+OdpFa9GvjxhjMDhl)
- Community Group:

  <img src="assets/images/qq_group.jpg" alt="Community Group QR Code" width="280" />
