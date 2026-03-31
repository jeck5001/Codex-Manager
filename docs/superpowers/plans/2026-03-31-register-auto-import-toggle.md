# Register Auto Import Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在注册弹窗中增加“注册成功后自动入池”开关，默认开启，关闭后只注册不自动导入号池。

**Architecture:** 该功能保持在前端实现：注册仍按原链路启动并轮询，自动入池仅作为轮询完成后的分支控制。这样不改 RPC/后端协议，最小化风险，并继续复用注册中心已有“待入池/手动加入号池”逻辑。

**Tech Stack:** Next.js App Router, TypeScript, React hooks, TanStack Query, Tauri invoke.

---

### Task 1: 落地开关状态与界面

**Files:**
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/components/modals/add-account-modal.tsx`

- [ ] 新增 `registerAutoImport` 状态，默认 `true`，并在 reset 时恢复默认值。
- [ ] 在注册表单区域加入 `Switch` + 文案 `注册成功后自动入池`。
- [ ] 在单个注册 / 批量注册 / Outlook 批量注册的提交与轮询逻辑中读取该开关。

### Task 2: 用最小改动切断自动导入分支

**Files:**
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/components/modals/add-account-modal.tsx`

- [ ] 单个注册完成时：若关闭自动入池，直接展示“注册完成，可在注册中心手动加入号池”。
- [ ] 批量注册完成时：若关闭自动入池，不调用批量导入逻辑，改为汇总提示。
- [ ] Outlook 批量注册完成时：若关闭自动入池，不按邮箱导入，改为汇总提示。

### Task 3: 验证

**Files:**
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/components/modals/add-account-modal.tsx`

- [ ] 运行类型检查/构建验证：`pnpm run build:desktop`
- [ ] 自查：默认值为开启；关闭后不再调用自动导入分支；手动入池入口仍保留在注册中心。
