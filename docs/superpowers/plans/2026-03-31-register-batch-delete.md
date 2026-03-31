# Register Batch Delete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在注册中心 `/register` 页面支持当前页多选并批量删除注册任务。

**Architecture:** 前端在注册中心页面维护当前页选中任务集合，并通过新增批量删除 API 提交任务 UUID 列表。后端逐条校验并删除任务，对运行中任务返回失败明细，前端基于返回结果展示成功/失败提示并刷新数据。

**Tech Stack:** Next.js App Router、TypeScript、TanStack Query、shadcn/ui、FastAPI、SQLAlchemy。

---

### Task 1: 后端批量删除注册任务接口

**Files:**
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/web/routes/registration.py`
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/database/crud.py`
- Test: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/tests/test_registration_batch_delete.py`

- [ ] Step 1: 写失败测试，覆盖成功删除、运行中跳过、任务不存在
- [ ] Step 2: 运行测试并确认失败
- [ ] Step 3: 实现批量删除请求模型与路由
- [ ] Step 4: 实现 CRUD 批量删除辅助逻辑
- [ ] Step 5: 运行测试并确认通过

### Task 2: 前端 API 与 hook 支持批量删除

**Files:**
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/lib/api/account-client.ts`
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/hooks/useRegisterTasks.ts`

- [ ] Step 1: 为批量删除客户端方法补失败测试（若无现成测试框架，则在实现后通过类型检查与构建验证）
- [ ] Step 2: 增加批量删除 API 封装
- [ ] Step 3: 增加 `deleteTasks` mutation，并统一刷新查询缓存
- [ ] Step 4: 运行构建验证

### Task 3: 注册中心 UI 增加当前页多选与批量删除

**Files:**
- Modify: `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/app/register/page.tsx`

- [ ] Step 1: 写/补最小行为验证（如无前端测试框架，则通过手动逻辑分解 + 构建验证）
- [ ] Step 2: 增加 `selectedTaskUuids` 状态与当前页全选逻辑
- [ ] Step 3: 在表头/表格行渲染 Checkbox
- [ ] Step 4: 增加批量删除工具条与确认弹窗文案分支
- [ ] Step 5: 删除后清空选择、刷新列表、显示结果 toast
- [ ] Step 6: 运行构建验证

### Task 4: 全量验证

**Files:**
- Verify only

- [ ] Step 1: 运行 Python 后端相关测试
- [ ] Step 2: 运行 `pnpm run build:desktop`
- [ ] Step 3: 手动检查 `/register` 页面交互是否符合设计
- [ ] Step 4: 提交代码
