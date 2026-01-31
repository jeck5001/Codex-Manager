# 平台 Key 独立页面入口 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 新增“平台 Key”独立页面入口并迁移 API Key 列表区块。

**Architecture:** 复用现有单页切换机制（page + nav active）。新增 pageApiKeys 容器并在 switchPage 中切换；API Key 渲染继续使用现有 renderApiKeys 与 DOM id，不改动数据刷新逻辑。

**Tech Stack:** Vite + 原生 HTML/CSS/JS。

### Task 1: 新增页面容器与导航切换（@superpowers:test-driven-development）

**Files:**
- Modify: `apps/gpttools-desktop/index.html`
- Modify: `apps/gpttools-desktop/src/ui/dom.js`
- Modify: `apps/gpttools-desktop/src/main.js`
- Verify: `apps/gpttools-desktop/src/views/apikeys.js`
- Modify (optional): `apps/gpttools-desktop/src/styles/layout.css`

**Step 1: Write the failing test**

```javascript
// apps/gpttools-desktop/tests/apikeys-page.test.mjs
import { readFileSync } from "node:fs";
import { strict as assert } from "node:assert";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(here, "..");

const indexHtml = readFileSync(join(projectRoot, "index.html"), "utf8");
const domJs = readFileSync(join(projectRoot, "src", "ui", "dom.js"), "utf8");
const mainJs = readFileSync(join(projectRoot, "src", "main.js"), "utf8");

assert(indexHtml.includes('id="navApiKeys"'));
assert(indexHtml.includes('id="pageApiKeys"'));
assert(domJs.includes("navApiKeys"));
assert(domJs.includes("pageApiKeys"));
assert(mainJs.includes("apikeys"));
```

**Step 2: Run test to verify it fails**

Run: `node apps/gpttools-desktop/tests/apikeys-page.test.mjs`
Expected: FAIL with assertion about missing nav/page wiring.

**Step 3: Write minimal implementation**

- 在侧边栏新增 `#navApiKeys` 按钮。
- 新增 `#pageApiKeys` 容器并迁移 API Key 区块。
- dom.js 增加 `navApiKeys` / `pageApiKeys` 映射。
- main.js 在 `switchPage` 和事件绑定中增加 apikeys。

**Step 4: Run test to verify it passes**

Run: `node apps/gpttools-desktop/tests/apikeys-page.test.mjs`
Expected: PASS with "apikeys page wiring present".

**Step 5: Commit**

```bash
git add apps/gpttools-desktop/index.html apps/gpttools-desktop/src/ui/dom.js apps/gpttools-desktop/src/main.js docs/plans/2026-01-30-platform-key-page-design.md
 git commit -m "feat: add apikeys standalone page"
```

(若无 git 仓库，跳过此步。)

### 说明：保留页面连线测试

为便于回归检查，本次保留 `apps/gpttools-desktop/tests/apikeys-page.test.mjs`，
不再执行删除步骤。
