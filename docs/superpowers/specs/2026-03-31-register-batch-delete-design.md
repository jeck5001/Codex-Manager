# 注册中心当前页多选批量删除设计

## 目标
在 `/register` 注册中心页面增加“当前页多选 + 批量删除”能力，方便用户一次清理多条注册任务记录。

## 范围
- 仅支持注册中心 `/register`
- 仅支持当前页任务勾选
- 支持单页全选 / 单条勾选 / 清空选择
- 支持批量删除二次确认
- 运行中的任务不可删除；批量删除时跳过并反馈结果
- 删除完成后刷新任务列表、统计、最近任务工作台

## 不做
- 不做跨页全选
- 不做“按筛选条件删除全部”
- 不改账号池 `/accounts`
- 不改现有单条删除语义

## 方案
### 前端
在 `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/app/register/page.tsx`：
- 增加当前页选中状态 `selectedTaskUuids`
- 表头增加全选 Checkbox
- 每行增加 Checkbox
- 顶部增加批量操作栏，显示已选数量和“批量删除”按钮
- 点击批量删除后复用确认弹窗，但文案切换为批量删除
- 删除成功后清空已选项并刷新页面数据

在 `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/hooks/useRegisterTasks.ts`：
- 增加 `deleteTasks(taskUuids: string[])`
- 统一失效 `register-tasks` / `register-stats` / `startup-snapshot`

在 `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/lib/api/account-client.ts`：
- 增加批量删除注册任务客户端方法

### 后端
在 `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/web/routes/registration.py`：
- 新增批量删除请求模型
- 新增批量删除路由
- 对每个任务执行：
  - 不存在 -> 记录失败
  - running -> 跳过并记录失败
  - 其他状态 -> 删除成功
- 返回成功数、失败数、失败明细

在 `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/vendor/codex-register/src/database/crud.py`：
- 增加批量删除注册任务的 CRUD 帮助方法，或复用单条删除循环

## 交互细节
- 表头全选仅影响当前页 `filteredTasks`
- 如果当前页无数据，表头 Checkbox 置灰/不可选
- 如果切页或切换筛选，已选项只保留当前页仍存在的任务；更简单的实现可直接清空已选项
- 批量删除 toast：
  - 全部成功：`已删除 N 条任务`
  - 部分成功：`已删除 X 条，Y 条删除失败`

## 风险与处理
- 运行中任务误删：后端硬拦截
- 当前页筛选切换导致选中错乱：筛选变化时清空选择
- UI 复杂度上涨：只在有选择时显示批量工具条
