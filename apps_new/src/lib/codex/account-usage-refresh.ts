import type { AccountUsageRefreshProgress } from "./types.ts";

export const EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS: AccountUsageRefreshProgress = Object.freeze({
  active: false,
  completed: 0,
  total: 0,
  remaining: 0,
  lastAccountLabel: "",
});

export function normalizeAccountUsageRefreshProgress(
  progress?: Partial<AccountUsageRefreshProgress> | null,
): AccountUsageRefreshProgress {
  const total = Math.max(0, Number(progress?.total || 0));
  const completed = Math.min(total, Math.max(0, Number(progress?.completed || 0)));
  const fallbackRemaining = Math.max(0, total - completed);

  return {
    active: Boolean(progress?.active) && total > 0,
    completed,
    total,
    remaining: Math.max(0, Number(progress?.remaining ?? fallbackRemaining)),
    lastAccountLabel: String(progress?.lastAccountLabel || "").trim(),
  };
}

export function buildAccountUsageRefreshText(progress?: Partial<AccountUsageRefreshProgress> | null) {
  const normalized = normalizeAccountUsageRefreshProgress(progress);
  if (!normalized.active) {
    return "";
  }

  const primaryText = `刷新进度 ${normalized.completed}/${normalized.total}，剩余 ${normalized.remaining} 项`;
  return normalized.lastAccountLabel
    ? `${primaryText} · 最近完成：${normalized.lastAccountLabel}`
    : primaryText;
}
