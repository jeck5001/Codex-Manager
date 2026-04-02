# Email Service Copy Config Design

## Goal

在邮箱服务页面为单条邮箱服务增加“复制配置”能力，用户可以直接从列表行操作菜单复制该服务的完整配置 JSON。

## Current Context

- 邮箱服务列表位于 `apps/src/app/email-services/page.tsx`。
- 当前列表行的操作菜单已有：
  - 编辑
  - 测试连接
  - 启用/禁用
  - 删除
- 列表中只展示配置概览，敏感字段被隐藏。
- 页面已经具备 `readEmailServiceFull(service.id)` 能力，可读取编辑场景使用的完整配置，包括敏感字段。
- 项目已有通用剪贴板工具：`apps/src/lib/utils/clipboard.ts`。

## Recommended Approach

在邮箱服务列表每行的下拉菜单中新增一个 `复制配置` 操作。点击后：

1. 调用 `readEmailServiceFull(service.id)` 读取单条服务的完整配置。
2. 将完整服务对象整理为稳定的 JSON 结构。
3. 使用现有 `copyTextToClipboard` 将格式化 JSON 写入剪贴板。
4. 成功时显示 toast：`已复制邮箱服务配置`
5. 失败时显示统一错误 toast，不打开编辑弹窗。

## Why This Approach

- 不依赖列表页当前概览数据，避免复制结果缺少敏感字段。
- 复用已有后端读取能力，不新增 API。
- 交互最直接，用户不需要先点“编辑”再复制。
- 对现有邮箱服务页影响面最小。

## UX Design

### Entry

- 位置：邮箱服务列表行级操作菜单
- 文案：`复制配置`
- 图标：可复用复制/剪贴板图标，若当前页未引入则新增

### Success

- 成功复制后显示：`已复制邮箱服务配置`

### Failure

- `readEmailServiceFull` 失败：沿用后端/Hook 返回的错误信息
- 剪贴板失败：显示 `复制配置失败: <错误信息>`

## Copied Payload Shape

复制内容为格式化 JSON，保留完整配置字段，建议结构如下：

```json
{
  "id": 12,
  "name": "主力 Outlook 池",
  "serviceType": "outlook",
  "enabled": true,
  "priority": 10,
  "config": {
    "client_id": "...",
    "refresh_token": "...",
    "tenant": "..."
  }
}
```

## Field Inclusion Rules

复制内容仅包含：

- `id`
- `name`
- `serviceType`
- `enabled`
- `priority`
- `config`

明确不包含：

- `lastUsed`
- `createdAt`
- `updatedAt`
- 列表页的概览文本
- 页面派生状态

原因：复制的目标是“可复用配置”，不是导出整条 UI 展示记录。

## Edge Cases

- 如果后端返回空 `config`，仍复制完整骨架 JSON，`config` 为 `{}`。
- 只支持单条复制，不在本轮增加批量复制。
- 保留敏感字段原值，因为用户需求是“复制整条邮箱服务配置”。

## Testing Requirements

至少覆盖：

1. 列表行菜单出现 `复制配置`
2. 点击后会调用 `readEmailServiceFull(service.id)`
3. 成功时会把格式化 JSON 传给剪贴板工具
4. 成功后显示成功 toast
5. 剪贴板失败时显示错误 toast
6. 现有菜单项：
   - 编辑
   - 测试连接
   - 启用/禁用
   - 删除
   
   行为保持不变

## Risks

- 因为复制结果包含敏感字段，误分享会暴露密钥/密码类配置。这是用户主动触发的导出行为，本轮不额外做脱敏。
- 若后端未来扩充 `RegisterEmailService` 全量字段，复制结构应保持稳定，不要直接把整个对象原样透传给 `JSON.stringify`。

