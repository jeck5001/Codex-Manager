<p align="center">
  <img src="assets/logo/logo.png" alt="CodexManager Logo" width="220" />
</p>

<h1 align="center">CodexManager</h1>

<p align="center">面向 Codex 账号池、网关转发与运维管理的一体化桌面端 / Web 控制台</p>

<p align="center">
  <a href="README.en.md">English</a>
</p>

CodexManager 用来统一管理账号、平台 Key、请求转发、用量统计和运维设置。它既可以作为本地桌面端使用，也可以以 service + web 或 Docker 的方式部署，适合个人、本地实验环境和轻量团队共用场景。

## 核心能力

- 账号池管理：导入、导出、标签、分组、优先级、状态切换、批量刷新
- 注册中心协同：注册任务追踪、到号状态标识、手动加入账号池
- 网关转发：提供 OpenAI 兼容入口，支持多账号路由、失败回退、请求日志追踪
- 平台 Key 管理：创建、禁用、删除、模型绑定、访问控制扩展
- 运维与观测：请求日志、用量快照、费用统计、健康巡检、告警能力持续增强
- 扩展能力：MCP Server 模式、插件管理 / Hook 系统正在持续收口

## 运行模式

| 模式 | 适合场景 | 说明 |
| --- | --- | --- |
| 桌面端 | 本机使用、最省心 | Tauri 应用内直接管理账号、服务与设置 |
| Service + Web | 服务器 / NAS / 远程主机 | 后端进程提供 RPC 与网关，浏览器访问 Web UI |
| Docker Compose | 本地联调、容器化部署 | 一次拉起 register、service、web 三个组件 |

## 快速开始

### 桌面端

1. 启动应用后点击“启动服务”。
2. 进入“账号管理”导入账号，或在“注册中心”完成注册任务。
3. 刷新账号状态与用量，确认账号已可用。
4. 在客户端中把 OpenAI Base URL 指向 CodexManager 网关地址。

### 本地源码运行

```bash
pnpm install
pnpm run build:desktop
```

如果你只需要前端类型检查或桌面前端静态构建，可参考：

```bash
pnpm exec tsc --noEmit
pnpm exec next build --webpack
```

### Docker 本地构建

仓库内已经提供本地构建版 compose：

```bash
docker compose -f docker/docker-compose.localbuild.yml up -d --build
```

默认端口：

- `48761`：Web 控制台
- `48760`：service / RPC / 网关
- `9000`：register 服务

更多环境变量、目录挂载和部署建议请看 [运行与部署指南](docs/report/20260310122606850_运行与部署指南.md)。

## 常见使用路径

| 你现在要做什么 | 直接看这里 |
| --- | --- |
| 首次启动、桌面端 / Web / Docker 部署 | [运行与部署指南](docs/report/20260310122606850_运行与部署指南.md) |
| 配置端口、数据库、代理、Web 密码、环境变量 | [环境变量与运行配置说明](docs/report/20260309195355187_环境变量与运行配置说明.md) |
| 联调 RPC / HTTP / Web 登录接口 | [API 说明](docs/API.md) |
| 把 CodexManager 接到 Claude Code / Cursor | [MCP 接入指南](docs/report/20260323161000000_MCP接入指南.md) |
| 使用插件管理、Lua 模板和 Hook 能力 | [插件管理与 Lua 开发指南](docs/report/20260323193000000_插件管理与Lua开发指南.md) |
| 排查账号不命中、导入失败、挑战拦截、请求异常 | [FAQ 与账号命中规则](docs/report/20260310122606852_FAQ与账号命中规则.md) |
| 本地构建、打包、发版、脚本使用 | [构建发布与脚本说明](docs/release/20260310122606851_构建发布与脚本说明.md) |

更完整的文档索引见 [docs/README.md](docs/README.md)。

## 截图

![仪表盘](assets/images/dashboard.png)
![账号管理](assets/images/accounts.png)
![平台 Key](assets/images/platform-key.png)
![请求日志](assets/images/log.png)
![设置页](assets/images/themes.png)

## 项目结构

```text
.
├─ apps/                # Next.js 前端与 Tauri 桌面端
├─ crates/core/         # 类型、存储、migration、共享核心能力
├─ crates/service/      # 账号管理、网关、RPC、调度与业务服务
├─ crates/web/          # Web UI 静态资源与服务端桥接
├─ crates/start/        # Service 版一键启动器
├─ docker/              # Dockerfile 与 compose 配置
├─ docs/                # 正式文档、运行手册、发布说明、治理文档
└─ assets/              # Logo、截图、平台补充资源
```

## 开发与验证

- 前端变更默认至少验证 `pnpm run build:desktop`
- service 相关变更优先运行最小 `cargo test` / `cargo check`
- 版本历史和对外可见变更统一维护在 [CHANGELOG.md](CHANGELOG.md)
- 协作、架构、安全与测试基线分别见 [CONTRIBUTING.md](CONTRIBUTING.md)、[ARCHITECTURE.md](ARCHITECTURE.md)、[SECURITY.md](SECURITY.md)、[TESTING.md](TESTING.md)

## 免责声明

- 本项目仅用于学习、开发与合法合规的内部工具场景。
- 使用者需要自行遵守 OpenAI、Anthropic 等上游平台的服务条款与相关法律法规。
- 仓库不提供账号、API Key、代理服务或任何绕过平台限制的能力。

## 社区与反馈

- 社区讨论：[Linux.do 话题](https://linux.do/t/topic/1688401)
- 问题反馈：优先使用 GitHub Issues / Discussions
- 交流群入口：答案是项目名 `CodexManager`

<p align="center">
  <img src="assets/images/qq_group.jpg" alt="交流群二维码" width="280" />
</p>
