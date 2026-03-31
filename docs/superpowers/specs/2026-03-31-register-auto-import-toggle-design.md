# 注册中心自动入池开关设计

**目标**：在注册界面的“启动注册”弹窗中增加“注册成功后自动入池”开关，默认开启；关闭后注册流程只创建注册任务，不自动导入本地账号池。

## 范围
- 单个注册
- 批量注册
- Outlook 批量注册

## 设计
- 开关位置放在注册弹窗里，归属于注册参数区域。
- 默认值为 `true`，保持现有用户体验。
- 当开关关闭时：
  - 前端仍然发起注册/轮询任务；
  - 注册完成后不再调用 `importRegisterTask` / `importRegisterAccountByEmail`；
  - UI 给出“已完成，未自动入池，可在注册中心手动加入号池”的提示；
  - 注册中心列表继续依赖既有 `待入池/已入池` 展示逻辑。
- 方案 1 只改前端，不改后端任务协议，避免扩大改动面。

## 影响文件
- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/apps/src/components/modals/add-account-modal.tsx`
- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/docs/superpowers/specs/2026-03-31-register-auto-import-toggle-design.md`
- `/Users/jfwang/IdeaProjects/CascadeProjects/Codex-Manager/docs/superpowers/plans/2026-03-31-register-auto-import-toggle.md`

## 验证
- 代码层：补最小测试覆盖“关闭自动入池时不走自动导入分支”。
- 集成层：`pnpm run build:desktop` 通过。
