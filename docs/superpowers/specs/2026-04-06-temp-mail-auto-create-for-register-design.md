# Temp-Mail Auto Create For Register Design

## Goal

在注册弹窗里，当用户选择 `Temp Mail` 时，允许本次注册自动创建一条带随机域名的临时 `temp_mail` 邮箱服务，并在注册任务结束后自动删除，省去先去“邮箱服务”页面手工新建配置的步骤。

## Observed Problem

当前 `temp_mail` 的固定随机域名是在“创建邮箱服务配置”时由后端预配出来的。注册流程本身只会消费“现有邮箱服务列表”：

- 前端注册弹窗先拉取可用邮箱服务分组和具体服务
- 单次注册接口 `/registration/start` 和批量注册接口 `/registration/batch` 都依赖现有 `email_service_id` 或现有服务池轮询
- `temp_mail` 的域名预配逻辑位于 `email_services.py` 的创建接口里，不在注册接口里

这导致用户想用随机域名 `Temp Mail` 注册时，必须先进入“邮箱服务”页面手动创建一条 `temp_mail` 配置，再回到注册界面选择它，流程过长。

## Recommended Approach

在注册界面增加一个仅对 `Temp Mail` 生效的开关：`自动创建临时 Temp-Mail 服务`。

当且仅当满足以下条件时：

- `email_service_type == "temp_mail"`
- `auto_create_temp_mail_service == true`

注册后端会在任务启动前自动创建一条临时 `temp_mail` 服务，并在任务生命周期结束时自动清理。

### Rule 1: Existing Temp-Mail Services Are Ignored When Switch Is On

即使当前已经存在可用的 `temp_mail` 服务，只要开关开启，仍然优先新建临时服务，而不是复用旧配置。

原因：

- 用户显式打开开关时，预期就是“这次注册用新的随机域名”
- 避免旧服务的固定域名被误复用，导致行为和界面文案不一致

### Rule 2: Single Register Gets Its Own Temporary Service

单次注册时：

- 提交注册前创建 1 条临时 `temp_mail` 服务
- 该服务仅绑定当前任务
- 任务成功、失败、取消时都删除

### Rule 3: Batch Register Shares One Temporary Service

批量注册时：

- 批次启动前只创建 1 条临时 `temp_mail` 服务
- 整批任务共用这 1 条服务
- 批次成功、失败、取消时统一删除

这样可以避免一批任务创建大量临时服务，减少 Cloudflare 侧预配动作和资源残留风险。

### Rule 4: Cleanup Is Best-Effort But Mandatory To Attempt

自动创建出来的临时服务必须在以下场景都触发删除：

- 单次任务完成
- 单次任务失败
- 单次任务取消
- 批次完成
- 批次失败
- 批次取消
- 创建成功但任务初始化中途异常

删除失败不能覆盖主任务结果，但必须明确写日志，便于追查残留服务。

## Execution Flow

### Frontend Flow

注册弹窗中，当邮箱服务类型选中 `Temp Mail` 时：

1. 显示开关 `自动创建临时 Temp-Mail 服务`
2. 显示说明文案
3. 开关开启时，隐藏或禁用“具体服务”下拉，避免和现有服务选择产生歧义
4. 提交单次注册或批量注册时，把 `autoCreateTempMailService` 一并发给后端

说明文案：

- 单次注册：`开启后，本次注册会自动创建随机域名的临时邮箱服务，并在任务结束后自动删除。`
- 批量注册：`开启后，整批任务会共用 1 条随机域名的临时邮箱服务，并在批次结束后自动删除。`

### Backend Flow

在 `registration.py` 的单次和批量入口里处理新字段。

单次注册：

1. 检查 `email_service_type == "temp_mail"` 且 `auto_create_temp_mail_service == true`
2. 复用现有 `email_services.py` 中的 `temp_mail` 创建与 Cloudflare 预配逻辑，创建 1 条临时服务
3. 获取新的 `email_service_id`
4. 使用该 `email_service_id` 创建注册任务
5. 在任务执行结束路径统一触发删除

批量注册：

1. 在创建整批任务前，先创建 1 条临时服务
2. 将同一个 `email_service_id` 分配给整批任务
3. 后台批量执行结束后统一删除这条服务
4. 如果批次启动前后出现异常，立即回滚删除

### Temporary Service Metadata

自动创建的服务应写入最小可追踪元数据，避免和手工配置混淆。推荐写入配置字段：

- `auto_created_for_registration = true`
- `auto_cleanup = true`
- `owner_task_uuid` 或 `owner_batch_id`

这些字段主要用于日志、清理和后续排障，不作为用户必须配置项暴露。

## Non-Goals

- 不修改 `outlook`、`tempmail`、`custom_domain`、`browserbase_ddg` 的既有流程
- 不移除现有“具体服务”选择和自动轮询逻辑
- 不改变现有 Temp-Mail 域名预配规则，只复用当前 Cloudflare 预配能力
- 不在本次设计里处理“异常残留服务的离线清扫任务”

## Files To Change

- `apps/src/components/modals/add-account-modal.tsx`
- `apps/src/lib/api/account-client.ts`
- `apps/src/types/index.ts`
- `crates/service/src/account/account_register.rs`
- `crates/service/src/rpc_dispatch/account.rs`
- `vendor/codex-register/src/web/routes/registration.py`
- `vendor/codex-register/src/web/routes/email_services.py`
- `vendor/codex-register/src/database/crud.py`
- `vendor/codex-register/src/database/models.py`
- 对应前后端测试文件

## API Changes

为注册接口新增布尔字段：

- `auto_create_temp_mail_service`

适用范围：

- 单次注册请求
- 批量注册请求

行为约束：

- 仅当 `email_service_type == "temp_mail"` 时生效
- 开关开启时，后端忽略用户传入的 `email_service_id`
- 开关关闭时，后端沿用当前逻辑

## Error Handling

- 若临时服务创建失败，注册直接失败，并返回明确错误：`Temp-Mail 临时服务创建失败: ...`
- 若注册流程失败，同时删除临时服务也失败，主错误仍然是注册失败，删除失败仅写日志
- 若批量流程中途取消，先标记批量任务取消，再执行临时服务回收

## Test Plan

### Frontend

- 只有邮箱服务类型为 `Temp Mail` 时显示自动创建开关
- 开关开启时，“具体服务”选择被隐藏或禁用
- 切换服务类型后，开关显隐与提交参数正确

### Backend

- 单次注册开启开关时，会创建临时 `temp_mail` 服务并绑定到当前任务
- 单次注册完成、失败、取消时都会尝试删除临时服务
- 批量注册开启开关时，只创建 1 条临时服务并复用给整批任务
- 批量注册完成、失败、取消时都会尝试删除临时服务
- 创建成功但任务初始化失败时会立即回滚删除
- 开关关闭时，完全沿用现有逻辑

### Validation

- 相关前后端单测通过
- `pnpm --dir apps run build:desktop`

## Expected Outcome

- 使用随机域名 `Temp Mail` 注册时，不再需要先手工创建邮箱服务
- 用户可显式控制“本次是否自动创建临时服务”
- 单次与批量注册都能自动回收临时服务，减少配置残留
- 开关关闭时，现有注册体验和行为保持不变
