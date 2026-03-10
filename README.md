<p align="center">
  <img src="assets/logo/logo.png" alt="CodexManager Logo" width="220" />
</p>

<h1 align="center">CodexManager</h1>

<p align="center">本地桌面端 + 服务进程的 Codex 账号池管理器</p>

<p align="center">
  <a href="README.en.md">English</a>
</p>

本地桌面端 + 服务进程的 Codex 账号池管理器，用于统一管理账号、用量与平台 Key，并提供本地网关能力。

## 最近变更
- 当前最新版本：`v0.1.6`（2026-03-07）
- 当前主分支已继续补齐 Web 安全链路：`codexmanager-web` 的访问密码仍会持久化，但登录会话现在会绑定当前 Web 进程；关闭并重新打开后，旧 Cookie 不再继续生效，必须重新验证密码。
- 协议适配继续对齐 Codex / OpenAI 兼容生态：`/v1/chat/completions` 与 `/v1/responses` 转发链路进一步统一，`tools` / `tool_calls` 聚合、工具名缩短与响应还原链路已补齐，并覆盖 Cherry Studio、OpenClaw、Claude Code 等兼容场景。
- 网关诊断能力增强：失败响应增加结构化 `errorCode` / `errorDetail` 字段，并补充 `X-CodexManager-Error-Code`、`X-CodexManager-Trace-Id` 头；请求日志也补充了原始路径、适配路径和更多上游上下文，便于精确排障。
- 发布体系继续收敛到单一入口：`release-all.yml` 统一负责 Windows / macOS / Linux 一键发布；当 `run_verify=false` 时会自动回退到本地前端构建，不再强依赖预构建工件，同时继续复用前端产物与协议回归基线。
- 完整版本历史请查看 [CHANGELOG.md](CHANGELOG.md)。

## 功能概览
- 账号池管理：分组、标签、排序、备注
- 批量导入 / 导出：支持多文件导入、桌面端文件夹递归导入 JSON、按账号导出单文件
- 用量展示：兼容 5 小时 + 7 日双窗口，以及仅返回 7 日单窗口的账号
- 授权登录：浏览器授权 + 手动回调解析
- 平台 Key：生成、禁用、删除、模型绑定
- 本地服务：自动拉起、可自定义端口
- 本地网关：为 CLI 和第三方工具提供统一 OpenAI 兼容入口

## 截图
![仪表盘](assets/images/dashboard.png)
![账号管理](assets/images/accounts.png)
![平台 Key](assets/images/platform-key.png)
![日志视图](assets/images/log.png)
![设置页](assets/images/themes.png)

## 快速开始
1. 启动桌面端，点击“启动服务”。
2. 进入“账号管理”，添加账号并完成授权。
3. 如回调失败，粘贴回调链接手动完成解析。
4. 刷新用量并确认账号状态。

## 常用文档
- 版本历史：[CHANGELOG.md](CHANGELOG.md)
- 协作约定：[CONTRIBUTING.md](CONTRIBUTING.md)
- 架构说明：[ARCHITECTURE.md](ARCHITECTURE.md)
- 测试基线：[TESTING.md](TESTING.md)
- 安全说明：[SECURITY.md](SECURITY.md)
- 文档索引：[docs/README.md](docs/README.md)

## 使用与部署
- 运行与部署指南：[docs/report/20260310122606850_运行与部署指南.md](docs/report/20260310122606850_运行与部署指南.md)
- 环境变量与运行配置：[docs/report/20260309195355187_环境变量与运行配置说明.md](docs/report/20260309195355187_环境变量与运行配置说明.md)
- FAQ 与账号命中规则：[docs/report/20260310122606852_FAQ与账号命中规则.md](docs/report/20260310122606852_FAQ与账号命中规则.md)
- 最小排障手册：[docs/report/20260307234235414_最小排障手册.md](docs/report/20260307234235414_最小排障手册.md)

## 构建与发布
- 构建发布与脚本说明：[docs/release/20260310122606851_构建发布与脚本说明.md](docs/release/20260310122606851_构建发布与脚本说明.md)
- 发布与产物说明：[docs/release/20260309195355216_发布与产物说明.md](docs/release/20260309195355216_发布与产物说明.md)
- 脚本与发布职责对照：[docs/report/20260309195735631_脚本与发布职责对照.md](docs/report/20260309195735631_脚本与发布职责对照.md)
- 协议兼容回归清单：[docs/report/20260309195735632_协议兼容回归清单.md](docs/report/20260309195735632_协议兼容回归清单.md)

## 目录结构
```text
.
├─ apps/                # 前端与 Tauri 桌面端
│  ├─ src/
│  ├─ src-tauri/
│  └─ dist/
├─ crates/              # Rust core/service
│  ├─ core
│  ├─ service
│  ├─ start              # Service 版本一键启动器（拉起 service + web）
│  └─ web                # Service 版本 Web UI（可内嵌静态资源 + /api/rpc 代理）
├─ docs/                # 正式文档目录
├─ scripts/             # 构建与发布脚本
├─ portable/            # 便携版输出目录
└─ README.md
```

## 联系方式

<p align="center">
  <img src="assets/images/group.jpg" alt="交流群二维码" width="280" />
  <img src="assets/images/qq_group.jpg" alt="QQ 交流群二维码" width="280" />
</p>

- Telegram 交流群：<https://t.me/+8o2Eu7GPMIFjNDM1>
- QQ 交流群：扫码加入
- 微信公众号：七线牛马
