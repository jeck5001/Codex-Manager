# GPTTools 桌面端前端整理与端口管理 /spec

日期：2026-01-30

## 背景与现状
- 桌面端为 Tauri 应用，前端入口目前是单文件：`apps/gpttools-desktop/src/index.html`，CSS/JS 均内联。
- `apps/gpttools-desktop/dist/index.html` 与 `src/index.html` 内容接近，当前未见明确前端构建流程。
- 后端为 `gpttools-service`，主服务默认监听 `127.0.0.1:5050`（可通过 `GPTTOOLS_SERVICE_ADDR` 覆盖）。
- 登录回调使用内置登录 server，默认端口 `1455`（可通过 `GPTTOOLS_LOGIN_ADDR`/`GPTTOOLS_REDIRECT_URI` 影响）。

## 目标
1) 前端代码结构化：把 HTML/CSS/JS 拆分为清晰的目录与文件，便于维护。
2) 服务与端口尽量合并或减少，避免多端口占用；在确有必要时，也要有明确的用途划分。
3) 端口可在程序启动时配置或自动调整，避免端口冲突造成启动失败。
4) Rust 后端结构整理，清理不需要/未使用的代码与模块。
5) 代码风格更规范，并补充中文行级注释（说明关键逻辑）。
6) Rust 后端按业务模块拆分（例如 auth / callback / gateway 等），提升可读性与维护性。

## 非目标（待确认）
- 不改变现有 UI/交互风格（除非整理结构需要微调）。

## 已确认事项
- 前端整理方式：引入构建工具（如 Vite），由 `src/` 构建产出 `dist/`。
- 构建工具选择：Vite。
- 允许新增前端构建依赖并修改 `package.json` / `*.lock`。
- 端口策略：所有服务合并为单端口；应用启动时由用户在页面设置端口后再启动。
- 允许调整 OAuth Redirect URI（可改平台/控制台设置）。
- 接受将 service 从纯 TCP RPC 调整为 HTTP（例如 `POST /rpc`），以便与登录回调 / gateway 共用同一端口。
- 端口不持久化：每次启动在页面手动填写。
- 端口冲突策略：直接提示“端口被占用”，要求用户修改。
- 清理范围：整个仓库（含 `crates/gpttools-core`、`crates/gpttools-cli` 等）。
- 保留入口：仅桌面端（Tauri GUI + service）。其他入口（如 CLI）可移除。
- 中文注释范围：主要函数/模块都加中文行级注释。
- Rust 模块拆分粒度：按“领域+用例”更细拆分（如 auth_login.rs / auth_tokens.rs / usage_refresh.rs 等）。
- daemon 重命名：`gpttools-daemon` → `gpttools-service`（目录/包名/可执行名全部调整）。
- 环境变量与命令名：所有含 daemon 字样的变量/命令统一改为 service（如 `GPTTOOLS_SERVICE_ADDR`、`service_*`）。

## 需求细化（待确认）
- 前端整理方式：
  - 构建工具选择（Vite/其它）与目录规范（src 结构、入口文件、静态资源）。
- 端口配置策略：
  - 启动页端口输入的 UX（输入校验、占用检查、启动流程反馈）。
- 端口合并范围：
  - 已确认：登录回调与 gateway 与主 service 统一到同一 HTTP 端口。
- Rust 模块拆分粒度：
  - 更细：按“领域+用例”拆分（如 auth_login.rs / auth_tokens.rs / usage_refresh.rs 等）。

## 验收标准（草案）
- 前端代码拆分到多个文件，结构清晰（HTML/样式/脚本分离，功能模块可定位）。
- 启动应用时可以设置或自动选择端口，且前端与 service 使用同一配置来源。
- 端口冲突时不出现启动失败或“无响应”，有明确的提示或自动回退机制。
- Rust 后端目录结构清晰，未使用代码被移除或集中归档，整体可读性提升。
- 关键逻辑处有中文行级注释，便于维护。
- Rust 后端模块职责清晰，按功能拆分（auth / callback / gateway 等），文件结构可直观定位功能。

## 风险与注意事项
- 引入构建工具会触发依赖变更与额外构建步骤。
- 登录回调端口与外部服务的 Redirect URI 相关，合并/调整需评估兼容性。
- 端口动态化需要同步更新 Tauri 与 service 的通讯配置。
- “未使用代码”判定需明确范围与口径，避免误删仍有用途的功能。
