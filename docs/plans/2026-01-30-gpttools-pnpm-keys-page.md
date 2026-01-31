# PnPM + API Key Page Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将前端构建与 Tauri dev/build 统一切到 pnpm，并把“平台 Key”列表/创建改为独立页面入口。

**Architecture:** 保留现有 Vite 构建与模块化 JS 结构，更新 Tauri build 命令为 pnpm；前端新增“平台 Key”导航页，将原账号页的 API Key 面板迁移到独立页面并复用现有 modal。

**Tech Stack:** Tauri 2.x, Vite, Vanilla JS, pnpm

---

### Task 1: 切换 Tauri 构建命令到 pnpm 并更新文档

**Files:**
- Modify: `apps/gpttools-desktop/src-tauri/tauri.conf.json`
- Modify: `docs/plans/2026-01-30-gpttools-single-port-refactor.md`

**Step 1: 更新 Tauri build 命令**

```json
"build": {
  "beforeDevCommand": "pnpm run dev",
  "beforeBuildCommand": "pnpm run build",
  "devUrl": "http://localhost:5173",
  "frontendDist": "../dist"
}
```

**Step 2: 更新计划文档中的 npm 指令为 pnpm**

将：
- `npm install` → `pnpm install`
- `npm run dev` → `pnpm run dev`
- `npm run build` → `pnpm run build`

**Step 3: 手动验证（用户执行）**

Run: `pnpm install`
Expected: 安装成功并生成 `node_modules`

---

### Task 2: 新增“平台 Key”独立页面入口

**Files:**
- Modify: `apps/gpttools-desktop/index.html`
- Modify: `apps/gpttools-desktop/src/ui/dom.js`
- Modify: `apps/gpttools-desktop/src/main.js`
- Modify: `apps/gpttools-desktop/src/views/apikeys.js`
- Modify: `apps/gpttools-desktop/src/styles/layout.css` (如需样式微调)

**Step 1: HTML 新增导航与页面容器**

在侧边栏新增按钮：
```html
<button id="navApiKeys">平台 Key</button>
```

新增独立页面容器（从账号页迁移原 API Key 区块）：
```html
<section id="pageApiKeys" class="page">
  <div class="panel">
    <div class="panel-header">
      <div>
        <h3>平台 Key</h3>
        <p>创建/管理网关调用所需的 Key</p>
      </div>
      <button id="createApiKey" class="secondary">创建 Key</button>
    </div>
    <div class="table table-header api-table">
      <div>Key ID</div>
      <div>名称</div>
      <div>状态</div>
      <div>最近使用</div>
      <div>操作</div>
    </div>
    <div class="table api-table" id="apiKeyRows"></div>
  </div>
</section>
```

**Step 2: JS 绑定新页面与导航切换**

```js
// dom.js
navApiKeys: document.getElementById("navApiKeys"),
pageApiKeys: document.getElementById("pageApiKeys"),

// main.js
function switchPage(page) {
  dom.navApiKeys.classList.toggle("active", page === "apikeys");
  dom.pageApiKeys.classList.toggle("active", page === "apikeys");
  dom.pageTitle.textContent = page === "apikeys" ? "平台 Key" : ...;
}

// 事件绑定
 dom.navApiKeys.addEventListener("click", () => switchPage("apikeys"));
```

**Step 3: 迁移 API Key 渲染与入口**

- 保持 `renderApiKeys` 逻辑不变
- 确保 `createApiKey` 按钮位于新页面并继续弹出 modal

**Step 4: 手动验证**

- 点击“平台 Key”导航后显示 Key 列表
- “创建 Key”按钮能弹出 modal 并生成 key

---

### Task 3: 前端运行验证（用户执行）

**Step 1: 启动 Vite**
Run: `pnpm run dev`
Expected: Vite 输出本地地址

**Step 2: 手动验证页面**
- 侧边栏显示“平台 Key”
- 页面切换正常，Key 列表与创建功能可用

---

### Task 4: 更新实现后验证（可选）

**Step 1: 手动检查 Tauri dev**
Run: `pnpm run dev` 后启动 Tauri
Expected: Tauri 能加载前端并显示新页面

**Step 2: Commit**
```bash
# git add apps/gpttools-desktop apps/gpttools-desktop/src-tauri docs/plans/2026-01-30-gpttools-single-port-refactor.md
# git commit -m "refactor: switch to pnpm and move api keys to dedicated page"
```
