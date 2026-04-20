type StartupSnapshotSignalInput = {
  accounts?: unknown[];
  usageAggregateSummary?: {
    primaryKnownCount?: number | null;
    secondaryKnownCount?: number | null;
  } | null;
  requestLogTodaySummary?: {
    todayTokens?: number | null;
  } | null;
  recentRequestLogCount?: number | null;
} | null | undefined;

type CurrentAccountCandidate = {
  id: string;
  availabilityLevel?: string | null;
};

function canParticipateInRouting(level: string | null | undefined): boolean {
  const normalized = String(level || "").trim().toLowerCase();
  return normalized !== "warn" && normalized !== "bad";
}

export function hasStartupSnapshotSignal(snapshot: StartupSnapshotSignalInput): boolean {
  if (!snapshot) return false;
  if ((snapshot.recentRequestLogCount ?? 0) > 0) return true;
  if ((snapshot.requestLogTodaySummary?.todayTokens ?? 0) > 0) return true;
  return (
    (snapshot.usageAggregateSummary?.primaryKnownCount ?? 0) > 0 ||
    (snapshot.usageAggregateSummary?.secondaryKnownCount ?? 0) > 0
  );
}

export function pickCurrentAccountId(
  accounts: CurrentAccountCandidate[],
  latestRequestAccountId?: string | null,
  manualPreferredAccountId?: string | null,
): string | null {
  if (!accounts.length) return null;

  const preferredId = String(manualPreferredAccountId || "").trim();
  if (preferredId) {
    const preferred = accounts.find((item) => item.id === preferredId);
    if (preferred && canParticipateInRouting(preferred.availabilityLevel)) {
      return preferred.id;
    }
  }

  const latestId = String(latestRequestAccountId || "").trim();
  if (latestId) {
    const fromLatest = accounts.find((item) => item.id === latestId);
    if (fromLatest && canParticipateInRouting(fromLatest.availabilityLevel)) {
      return fromLatest.id;
    }
  }

  return (
    accounts.find((item) => canParticipateInRouting(item.availabilityLevel))?.id ||
    (preferredId ? accounts.find((item) => item.id === preferredId)?.id ?? null : null) ||
    accounts[0]?.id ||
    null
  );
}
