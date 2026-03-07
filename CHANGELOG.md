# 更新日志

本文件用于记录 CodexManager 的对外可见变更，作为版本历史的唯一事实源。
格式参考 Keep a Changelog，并结合当前项目的实际维护方式做最小收敛。

## [Unreleased]

### Changed
- 暂无。

## [0.1.6] - 2026-03-07

### Fixed
- 修复 `release-all.yml` 在手动关闭 `run_verify` 时仍强依赖预构建前端工件的问题；各平台任务缺少 `codexmanager-frontend-dist` 时会自动回退到本地 `pnpm install + build`。

### Changed
- Windows 桌面端发布产物继续收敛，仅保留 `CodexManager-portable.exe` 便携版，不再额外生成 `CodexManager-windows-portable.zip`。
- 完善 SOCKS5 上游代理支持与归一化，并补充设置页中的代理协议提示文案。

## [0.1.5] - 2026-03-06

### Added
- 新增“按文件夹导入”：桌面端可直接选择目录，递归扫描其中 `.json` 文件并批量导入账号。
- 新增 OpenAI 上游代理配置与请求头收敛策略开关，可在设置页直接保存并即时生效。
- 补充 chat tools 命中探针脚本，便于本地验证工具调用是否真正命中与透传。

### Fixed
- 修复 `tool_calls` / `tools` 相关回归：补齐 chat 聚合路径中的工具调用保留、工具名缩短与响应还原链路，避免工具调用在 OpenAI 兼容返回、流式增量和适配转换中丢失或名称错乱。
- 完善 OpenClaw / Anthropic 兼容返回适配，确保工具调用、SSE 增量和非流式 JSON 响应都能按兼容格式正确还原。
- 请求日志追踪增强，补充原始路径、适配路径和更多上下文，便于定位 `/v1/chat/completions -> /v1/responses` 转发与协议适配问题。

### Changed
- 网关协议适配进一步对齐 Codex CLI：`/v1/chat/completions` 与 `/v1/responses` 两条链路统一收敛到 Codex `responses` 语义，上游流式/非流式行为与官方更接近，兼容 Cherry Studio 等客户端的 OpenAI 兼容调用。
- 设置页顶部常用配置改为统一的三列行布局，代理配置与其保持一致；同时支持关闭窗口后隐藏到系统托盘运行。
- 发布流程整合为单一一键多平台 workflow，并收敛桌面端产物形态；Windows 直接提供 portable exe，macOS 统一使用 DMG 分发。

## [0.1.4] - 2026-03-03

### Added
- 新增“一键移除不可用 Free 账号”：批量清理“不可用 + free 计划”账号，并返回扫描/跳过/删除统计。
- 新增“导出用户”：支持选择本地目录并按“一个账号一个 JSON 文件”导出。
- 导入兼容增强：支持 `tokens.*`、顶层 `*_token`、camelCase 字段（如 `accessToken` / `idToken` / `refreshToken`）自动识别。

### Fixed
- 兼容旧 service：前端导入前会自动归一化顶层 token 格式，避免旧版后端报 `missing field: tokens`。

### Changed
- 账号管理页操作区整合为单一“账号操作”下拉菜单，替代右侧多按钮堆叠，界面更简洁。

[Unreleased]: https://github.com/qxcnm/Codex-Manager/compare/v0.1.6...HEAD
[0.1.6]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.6
[0.1.5]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.5
[0.1.4]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.4