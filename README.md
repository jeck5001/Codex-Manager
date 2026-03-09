<p align="center">
  <img src="assets/logo/logo.png" alt="CodexManager Logo" width="220" />
</p>

<h1 align="center">CodexManager</h1>

<p align="center">本地桌面端 + 服务进程的 Codex 账号池管理器</p>

<p align="center">
  <a href="README.en.md">English</a>
</p>

## 项目介绍

CodexManager 用于统一管理 Codex 兼容账号池、用量、平台 Key，并提供本地 OpenAI 兼容网关入口。

支持两种运行形态：

- 桌面模式：Tauri 桌面端 + 本地 service
- Service 模式：`codexmanager-service` + `codexmanager-web`

## 快速开始

### 桌面端

1. 启动应用。
2. 点击“启动服务”。
3. 进入账号管理完成导入或登录。
4. 在设置页按需调整监听地址、网关策略、上游代理等配置。

### Service 版本

1. 下载 Release 中对应平台的 `CodexManager-service-<platform>-<arch>.zip`。
2. 推荐先启动 `codexmanager-start`。
3. 默认地址：service `localhost:48760`，Web UI `http://localhost:48761/`。

## 常用入口

- 版本历史：[CHANGELOG.md](CHANGELOG.md)
- 协作规范：[CONTRIBUTING.md](CONTRIBUTING.md)
- 架构说明：[ARCHITECTURE.md](ARCHITECTURE.md)
- 测试基线：[TESTING.md](TESTING.md)
- 安全说明：[SECURITY.md](SECURITY.md)
- 文档索引：[docs/README.md](docs/README.md)

## 发布与产物

- 统一发布入口：`.github/workflows/release-all.yml`
- 发布说明与产物清单：[docs/release/20260309195355216_发布与产物说明.md](docs/release/20260309195355216_发布与产物说明.md)
- 环境变量与运行配置：[docs/report/20260309195355187_环境变量与运行配置说明.md](docs/report/20260309195355187_环境变量与运行配置说明.md)

当前主要发布产物：

- Desktop：Windows 安装版 / 便携版、macOS dmg、Linux AppImage / deb
- Service：Windows / macOS / Linux 压缩包

## 文档索引

### 治理与计划

- [docs/plan/20260309191759589_长期维护治理TODO.md](docs/plan/20260309191759589_长期维护治理TODO.md)
- [docs/report/20260309191759589_长期维护结构优化建议.md](docs/report/20260309191759589_长期维护结构优化建议.md)

### 排障与运行

- [docs/report/20260307234235414_最小排障手册.md](docs/report/20260307234235414_最小排障手册.md)
- [docs/report/20260309195355187_环境变量与运行配置说明.md](docs/report/20260309195355187_环境变量与运行配置说明.md)

### 发布

- [docs/release/20260309195355216_发布与产物说明.md](docs/release/20260309195355216_发布与产物说明.md)

## 联系方式

<p align="center">
  <img src="assets/images/group.jpg" alt="交流群二维码" width="280" />
  <img src="assets/images/qq_group.jpg" alt="QQ 交流群二维码" width="280" />
</p>

- Telegram 交流群：<https://t.me/+8o2Eu7GPMIFjNDM1>
- QQ 交流群：扫码加入
- 微信公众号：七线牛马
