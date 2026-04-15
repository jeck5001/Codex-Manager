import type { AccountCpaSyncStatusResult } from "../../types/index.ts";

export function parseCpaSyncScheduleInterval(value: string): number | null {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return null;
  const rounded = Math.trunc(numeric);
  if (rounded < 1) return null;
  return rounded;
}

export function formatCpaSyncStatusLabel(
  status: AccountCpaSyncStatusResult | null | undefined
): string {
  switch (status?.status) {
    case "running":
      return "同步中";
    case "misconfigured":
      return "待补配置";
    case "error":
      return "最近失败";
    case "idle":
      return "待机";
    case "disabled":
    default:
      return "已关闭";
  }
}

export function shouldPollCpaSyncStatus(
  status: AccountCpaSyncStatusResult | null | undefined
): boolean {
  return Boolean(status?.scheduleEnabled || status?.isRunning);
}
