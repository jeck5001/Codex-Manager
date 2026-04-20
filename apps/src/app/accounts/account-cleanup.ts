const INVALID_AUTH_STATUS_REASONS = new Set([
  "Refresh 已过期",
  "Refresh 已复用",
  "Refresh 已失效",
  "Refresh 刷新失败",
  "登录态已失效",
  "授权失效",
  "Refresh 连续失效",
]);

export function isInvalidAuthCleanupStatusReason(
  value: string | null | undefined,
): boolean {
  const label = String(value || "").trim();
  return label.length > 0 && INVALID_AUTH_STATUS_REASONS.has(label);
}

export function collectInvalidAuthCleanupAccountIds(
  accounts: Array<{ id: string; lastStatusReason: string | null | undefined }>,
): string[] {
  return accounts
    .filter((account) => isInvalidAuthCleanupStatusReason(account.lastStatusReason))
    .map((account) => account.id);
}
