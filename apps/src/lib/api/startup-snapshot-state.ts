type StartupSnapshotSignalInput = {
  accounts?: unknown[];
  usageAggregateSummary?: {
    primaryBucketCount?: number | null;
    primaryKnownCount?: number | null;
    secondaryBucketCount?: number | null;
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
  routeAccountIds?: string[] | null,
): string | null {
  if (!accounts.length) return null;

  const normalizedRouteAccountIds = Array.isArray(routeAccountIds)
    ? routeAccountIds
        .map((accountId) => String(accountId || "").trim())
        .filter(Boolean)
    : [];
  const scopedAccounts =
    normalizedRouteAccountIds.length > 0
      ? accounts.filter((item) => normalizedRouteAccountIds.includes(item.id))
      : accounts;
  if (!scopedAccounts.length) return null;

  const latestId = String(latestRequestAccountId || "").trim();
  if (latestId) {
    const fromLatest = scopedAccounts.find((item) => item.id === latestId);
    if (fromLatest && canParticipateInRouting(fromLatest.availabilityLevel)) {
      return fromLatest.id;
    }
  }

  return (
    scopedAccounts.find((item) => canParticipateInRouting(item.availabilityLevel))?.id ||
    scopedAccounts[0]?.id ||
    null
  );
}
