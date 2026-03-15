"use client";

import { StartupSnapshot } from "@/types";

export const STARTUP_SNAPSHOT_REQUEST_LOG_LIMIT = 120;
export const STARTUP_SNAPSHOT_STALE_TIME = 15_000;
export const STARTUP_SNAPSHOT_WARMUP_INTERVAL_MS = 2_500;
export const STARTUP_SNAPSHOT_WARMUP_TIMEOUT_MS = 45_000;

export function buildStartupSnapshotQueryKey(
  addr: string | null | undefined,
  requestLogLimit = STARTUP_SNAPSHOT_REQUEST_LOG_LIMIT
) {
  return ["startup-snapshot", addr || null, requestLogLimit] as const;
}

export function hasStartupSnapshotSignal(
  snapshot: StartupSnapshot | undefined
): boolean {
  if (!snapshot) return false;
  if (snapshot.usageSnapshots.length > 0) return true;
  if (snapshot.requestLogs.length > 0) return true;
  if (snapshot.requestLogTodaySummary.todayTokens > 0) return true;
  return (
    snapshot.usageAggregateSummary.primaryKnownCount > 0 ||
    snapshot.usageAggregateSummary.secondaryKnownCount > 0
  );
}
