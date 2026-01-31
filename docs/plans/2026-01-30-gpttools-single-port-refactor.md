# GPTTools Single-Port + Frontend/Backend Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 service 改为单端口 HTTP 服务（/rpc + /auth/callback + gateway），前端使用 Vite 拆分结构并提供“启动端口”入口，同时清理 CLI 等未用入口、整理 Rust 代码结构并补充中文注释。

**Architecture:** 以 `tiny_http` 作为单端口 HTTP 服务器，统一路由到 `POST /rpc`（JSON-RPC）、`GET /auth/callback`（OAuth 回调）、以及 `gateway` 请求；Tauri 端在 UI 选择端口后再启动 service，并通过 HTTP 调用 RPC。Rust 业务逻辑按“领域+用例”拆分到更细模块，并在关键逻辑处补中文行级注释。

**Tech Stack:** Rust (Tauri, tiny_http), Vite (Vanilla JS), Cargo, PowerShell

---

### Task 1: 重命名 daemon 为 gpttools-service

**Files:**
- Modify: `Cargo.toml`
- Rename: `crates/gpttools-daemon/` → `crates/gpttools-service/`
- Modify: `crates/gpttools-service/Cargo.toml`
- Modify: `crates/gpttools-service/src/main.rs`
- Modify: `apps/gpttools-desktop/src-tauri/src/lib.rs`
- Modify: `apps/gpttools-desktop/src-tauri/tauri.conf.json`

**Step 1: 明确重命名范围并更新 Cargo workspace**

```toml
# Cargo.toml
[workspace]
members = [
  "crates/gpttools-core",
  "crates/gpttools-service"
]
```

**Step 2: 修改 crate 名称与二进制名称**

```toml
# crates/gpttools-service/Cargo.toml
[package]
name = "gpttools-service"
```

**Step 3: 更新代码引用**

- Rust 代码中所有 `gpttools_daemon::` → `gpttools_service::`
- Tauri 的 `externalBin` 名称与路径同步更新
- 启动时寻找的 sidecar 文件名同步更新
- 环境变量：`GPTTOOLS_DAEMON_ADDR` → `GPTTOOLS_SERVICE_ADDR`、`GPTTOOLS_NO_DAEMON` → `GPTTOOLS_NO_SERVICE`
- Tauri 命令与前端调用：`daemon_*` → `service_*`

**Step 4: 验证编译入口**

Run: `cargo check -p gpttools-service`
Expected: PASS

**Step 5: Commit**

```bash
# git add -A
# git commit -m "refactor: rename service crate to gpttools-service"
```

---

### Task 2: 清理入口与工作区（只保留桌面端 + service）

**Files:**
- Modify: `Cargo.toml`
- Delete: `crates/gpttools-cli/**`
- Modify: `README.md`（如有 CLI 说明）

**Step 1: 写“期望失败”的检查脚本（确保 CLI 被移除）**

```powershell
# 期望在移除后找不到 gpttools-cli
Get-ChildItem -Recurse -File .\crates\gpttools-cli -ErrorAction SilentlyContinue
```

**Step 2: 先运行一次（当前应能找到 CLI，作为失败基线）**

Run: `Get-ChildItem -Recurse -File .\crates\gpttools-cli`
Expected: 能看到文件列表（失败基线）

**Step 3: 移除 CLI 与 workspace 引用**

```toml
# Cargo.toml
[workspace]
members = [
  "crates/gpttools-core",
  "crates/gpttools-service"
]
```

并删除 `crates/gpttools-cli/` 目录与 README 中 CLI 相关描述。

**Step 4: 重新运行检查脚本**

Run: `Get-ChildItem -Recurse -File .\crates\gpttools-cli -ErrorAction SilentlyContinue`
Expected: 无输出

**Step 5: Commit**

```bash
git add Cargo.toml README.md
# 若已删除目录
# git add -A crates/gpttools-cli
# git commit -m "chore: remove unused cli entry"
```

---

### Task 3: 更新 service 测试为 HTTP RPC（先让测试失败）

**Files:**
- Modify: `crates/gpttools-service/tests/rpc.rs`
- Modify: `crates/gpttools-service/tests/e2e.rs`
- Modify: `crates/gpttools-service/src/lib.rs`（仅加 test helper 声明）

**Step 1: 新增 HTTP 请求辅助函数（测试内）**

```rust
fn post_rpc(addr: &str, body: &str) -> serde_json::Value {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(addr).expect("connect server");
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(request.as_bytes()).expect("write");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read");

    // 取 HTTP body
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or("");
    serde_json::from_str(body).expect("parse response")
}
```

**Step 2: 替换测试中的 TCP 直连为 HTTP /rpc**

示例（rpc_initialize_roundtrip）：
```rust
let req = JsonRpcRequest { id: 1, method: "initialize".to_string(), params: None };
let json = serde_json::to_string(&req).expect("serialize");
let v = post_rpc(&server.addr, &json);
```

**Step 3: 运行测试，确认失败（因为服务端尚未支持 /rpc）**

Run: `cargo test -p gpttools-service --test rpc`
Expected: FAIL（连接/解析 HTTP 失败）

**Step 4: Commit（仅当后续实现完成前不提交，可跳过）**

```bash
# git add crates/gpttools-service/tests/rpc.rs crates/gpttools-service/tests/e2e.rs
# git commit -m "test: switch service rpc tests to http"
```

---

### Task 4: 单端口 HTTP 服务器（/rpc + /auth/callback + gateway）

**Files:**
- Modify: `crates/gpttools-service/src/lib.rs`
- Modify: `crates/gpttools-service/src/main.rs`
- Create: `crates/gpttools-service/src/http/mod.rs`
- Create: `crates/gpttools-service/src/http/server.rs`
- Create: `crates/gpttools-service/src/http/rpc_endpoint.rs`
- Create: `crates/gpttools-service/src/http/callback_endpoint.rs`
- Create: `crates/gpttools-service/src/http/gateway_endpoint.rs`

**Step 1: 新建 HTTP 路由骨架**

```rust
// crates/gpttools-service/src/http/mod.rs
pub mod server;
pub mod rpc_endpoint;
pub mod callback_endpoint;
pub mod gateway_endpoint;
```

**Step 2: 实现 server 路由**

```rust
// crates/gpttools-service/src/http/server.rs
use tiny_http::{Request, Response, Server};

pub fn start_http(addr: &str) -> std::io::Result<()> {
    let server = Server::http(addr)?;
    for request in server.incoming_requests() {
        route_request(request);
    }
    Ok(())
}

fn route_request(request: Request) {
    let path = request.url().to_string();
    if request.method().as_str() == "POST" && path == "/rpc" {
        crate::http::rpc_endpoint::handle_rpc(request);
        return;
    }
    if request.method().as_str() == "GET" && path.starts_with("/auth/callback") {
        crate::http::callback_endpoint::handle_callback(request);
        return;
    }
    // 其余交给 gateway
    crate::http::gateway_endpoint::handle_gateway(request);
}
```

**Step 3: 实现 /rpc 处理**

```rust
// crates/gpttools-service/src/http/rpc_endpoint.rs
use tiny_http::{Request, Response};
use std::io::Read;

pub fn handle_rpc(mut request: Request) {
    let mut body = String::new();
    let _ = request.as_reader().read_to_string(&mut body);
    if body.trim().is_empty() {
        let _ = request.respond(Response::from_string("{}").with_status_code(400));
        return;
    }
    let req: gpttools_core::rpc::types::JsonRpcRequest = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            let _ = request.respond(Response::from_string("{}").with_status_code(400));
            return;
        }
    };
    let resp = crate::handle_request(req);
    let json = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string());
    let _ = request.respond(Response::from_string(json));
}
```

**Step 4: 实现 /auth/callback 与 gateway 转发**

```rust
// crates/gpttools-service/src/http/callback_endpoint.rs
use tiny_http::{Request, Response};

pub fn handle_callback(request: Request) {
    if let Err(err) = crate::auth_callback::handle_login_request(request) {
        let _ = request.respond(Response::from_string(err).with_status_code(500));
    }
}
```

```rust
// crates/gpttools-service/src/http/gateway_endpoint.rs
use tiny_http::Request;

pub fn handle_gateway(request: Request) {
    if let Err(err) = crate::gateway::handle_gateway_request(request) {
        eprintln!("gateway request error: {err}");
    }
}
```

**Step 5: 将 start_server 改为调用 HTTP 服务器**

```rust
// crates/gpttools-service/src/lib.rs
pub fn start_server(addr: &str) -> std::io::Result<()> {
    ensure_usage_polling();
    crate::http::server::start_http(addr)
}
```

**Step 6: 运行测试，确认 /rpc 通过**

Run: `cargo test -p gpttools-service --test rpc`
Expected: PASS

**Step 7: Commit**

```bash
# git add crates/gpttools-service/src
# git commit -m "feat: single-port http server with /rpc and callback"
```

---

### Task 5: 登录回调与网关合并到同端口（移除独立监听）

**Files:**
- Modify: `crates/gpttools-service/src/lib.rs`
- Modify: `crates/gpttools-service/src/http/gateway_endpoint.rs`
- Modify: `crates/gpttools-service/src/http/callback_endpoint.rs`

**Step 1: 移除独立 login server/gateway server 启动逻辑**

删除或停用：
- `ensure_login_server` / `bind_login_server` / `run_login_server`
- `ensure_gateway` / `gateway_server_loop`

**Step 2: 更新 redirect URI 生成逻辑**

```rust
fn resolve_redirect_uri() -> Option<String> {
    if let Ok(uri) = std::env::var("GPTTOOLS_REDIRECT_URI") {
        return Some(uri);
    }
    // 统一由主端口提供回调地址
    let addr = std::env::var("GPTTOOLS_SERVICE_ADDR").ok()?;
    Some(format!("http://{addr}/auth/callback"))
}
```

**Step 3: 运行 e2e 测试**

Run: `cargo test -p gpttools-service --test e2e`
Expected: PASS

**Step 4: Commit**

```bash
# git add crates/gpttools-service/src
# git commit -m "refactor: unify login/gateway on single port"
```

---

### Task 6: Rust 业务逻辑细粒度拆分 + 中文行级注释

**Files:**
- Create: `crates/gpttools-service/src/auth_login.rs`
- Create: `crates/gpttools-service/src/auth_tokens.rs`
- Create: `crates/gpttools-service/src/auth_callback.rs`
- Create: `crates/gpttools-service/src/usage_refresh.rs`
- Create: `crates/gpttools-service/src/usage_read.rs`
- Create: `crates/gpttools-service/src/usage_list.rs`
- Create: `crates/gpttools-service/src/account_list.rs`
- Create: `crates/gpttools-service/src/account_update.rs`
- Create: `crates/gpttools-service/src/account_delete.rs`
- Create: `crates/gpttools-service/src/apikey_list.rs`
- Create: `crates/gpttools-service/src/apikey_create.rs`
- Create: `crates/gpttools-service/src/apikey_delete.rs`
- Create: `crates/gpttools-service/src/apikey_disable.rs`
- Create: `crates/gpttools-service/src/gateway.rs`
- Create: `crates/gpttools-service/src/storage_helpers.rs`
- Modify: `crates/gpttools-service/src/lib.rs`

**Step 1: 为每个用例模块创建文件并移动对应函数**

例（auth_login.rs）：
```rust
// 登录发起相关逻辑
pub fn login_start(...) -> Result<serde_json::Value, String> {
    // 中文注释：构造 OAuth 登录地址与状态
}
```

**Step 2: 在 lib.rs 中集中路由 RPC 到新模块**

```rust
pub fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "account/login/start" => auth_login::login_start(req),
        "account/login/status" => auth_login::login_status(req),
        // ...其余用例
        _ => JsonRpcResponse::error(req.id, "unsupported method"),
    }
}
```

**Step 3: 为主要函数添加中文行级注释**

- 说明业务意图（例如“校验参数”、“查库”、“拼装返回”）
- 避免对显而易见的语句重复注释

**Step 4: 运行 service 测试**

Run: `cargo test -p gpttools-service`
Expected: PASS

**Step 5: Commit**

```bash
# git add crates/gpttools-service/src
# git commit -m "refactor: split service logic into fine-grained modules"
```

---

### Task 7: Tauri 后端支持手动端口启动 + HTTP RPC

**Files:**
- Modify: `apps/gpttools-desktop/src-tauri/src/lib.rs`

**Step 1: 新增启动/停止命令，并统一命名为 service_***

```rust
#[tauri::command]
fn service_start(app: tauri::AppHandle, addr: String) -> Result<(), String> {
  // 中文注释：保存地址到环境变量，启动 service
  std::env::set_var("GPTTOOLS_SERVICE_ADDR", &addr);
  std::env::set_var("GPTTOOLS_REDIRECT_URI", format!("http://{addr}/auth/callback"));
  spawn_service_with_addr(&app, &addr)?;
  Ok(())
}

// 其余命令示例：service_initialize / service_account_list / service_usage_read ...
```

**Step 2: 将 rpc_call 改为 HTTP /rpc**

```rust
fn rpc_call(method: &str, addr: Option<String>, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
  let addr = addr
    .or_else(|| std::env::var("GPTTOOLS_SERVICE_ADDR").ok())
    .unwrap_or_else(|| "127.0.0.1:5050".to_string());
  let req = JsonRpcRequest { id: 1, method: method.to_string(), params };
  let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;

  // 简单 HTTP 请求
  let mut stream = TcpStream::connect(&addr).map_err(|e| e.to_string())?;
  let http = format!(
    "POST /rpc HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
    body.len(),
    body
  );
  stream.write_all(http.as_bytes()).map_err(|e| e.to_string())?;
  let mut buf = String::new();
  stream.read_to_string(&mut buf).map_err(|e| e.to_string())?;
  let body = buf.split("\r\n\r\n").nth(1).unwrap_or("");
  let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
  Ok(v)
}
```

**Step 3: 移除应用启动时自动 spawn**

- 在 `.setup()` 中去掉 `spawn_service`
- 改为只有 `service_start` 被调用才启动

**Step 4: 运行（手动验证）**

- 运行桌面端后，确认未自动启动 service

**Step 5: Commit**

```bash
# git add apps/gpttools-desktop/src-tauri/src/lib.rs
# git commit -m "feat: start service on demand and use http rpc"
```

---

### Task 8: Vite 前端重构与端口 UI

**Files:**
- Create: `apps/gpttools-desktop/package.json`
- Create: `apps/gpttools-desktop/vite.config.js`
- Create: `apps/gpttools-desktop/index.html`
- Create: `apps/gpttools-desktop/src/main.js`
- Create: `apps/gpttools-desktop/src/styles/*.css`
- Create: `apps/gpttools-desktop/src/scripts/*.js`
- Delete: `apps/gpttools-desktop/src/index.html`（旧版）

**Step 0: 依赖安装由用户执行（禁止自动安装）**

Run (用户执行并贴输出): `pnpm install`
Expected: 成功安装 Vite 依赖

**Step 1: 初始化 Vite 配置（仅文件，不运行安装）**

```json
{
  "name": "gpttools-desktop",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "vite": "^5.0.0"
  }
}
```

**Step 2: 迁移 HTML/CSS/JS**

- `index.html` 仅保留结构与 `<script type="module" src="/src/main.js"></script>`
- CSS 拆分到 `src/styles/` 并在 `main.js` 引用
- JS 拆分为模块：`api.js`、`state.js`、`views/*.js`、`ui/*.js`
- 所有 Tauri invoke 从 `daemon_*` 改为 `service_*`

**Step 3: 添加“端口输入 + 启动”UI**

- 新增输入框与按钮
- 点击按钮调用 Tauri `service_start(addr)`
- 端口占用时提示错误

**Step 4: 手动验证**

- 打开应用 → 输入端口 → 点击启动 → 状态显示“已连接”
- 登录回调成功后可刷新账号/用量

**Step 5: Commit**

```bash
# git add apps/gpttools-desktop
# git commit -m "refactor: move frontend to vite and add port start ui"
```

---

### Task 9: Tauri 构建配置更新

**Files:**
- Modify: `apps/gpttools-desktop/src-tauri/tauri.conf.json`

**Step 1: 更新构建命令**

```json
"build": {
  "beforeDevCommand": "pnpm run dev",
  "beforeBuildCommand": "pnpm run build",
  "devUrl": "http://localhost:5173",
  "frontendDist": "../dist"
}
```

**Step 2: 手动验证**

- 运行 `pnpm run dev` 后 Tauri 能打开前端

**Step 3: Commit**

```bash
# git add apps/gpttools-desktop/src-tauri/tauri.conf.json
# git commit -m "chore: update tauri build config for vite"
```

---

### Task 10: 全量验证（含授权回调）

**Files:**
- (无)

**Step 1: Rust 测试**

Run: `cargo test -p gpttools-service`
Expected: PASS

**Step 2: 手动授权回调验证**

- 启动应用 → 输入端口 → 启动 service
- 进行 OpenAI 登录并确认回调成功（/auth/callback）
- 账号列表与用量可正常读取

**Step 3: Commit（如需）**

```bash
# git add -A
# git commit -m "chore: verify single-port flow"
```
