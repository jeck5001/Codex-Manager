"use client";

import { StartupSnapshot } from "@/types";
import { hasStartupSnapshotSignal as hasStartupSnapshotSignalImpl } from "./startup-snapshot-state";

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
  return hasStartupSnapshotSignalImpl(snapshot);
}
