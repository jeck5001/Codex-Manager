export function normalizeRouteAccountIds(accountIds: string[]): string[] {
  const normalizedIds: string[] = [];
  const seen = new Set<string>();
  for (const accountId of accountIds) {
    const normalized = String(accountId || "").trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    normalizedIds.push(normalized);
  }
  return normalizedIds;
}

export function isRouteAccountSelected(
  routeAccountIds: string[],
  accountId: string,
): boolean {
  const normalizedAccountId = String(accountId || "").trim();
  if (!normalizedAccountId) {
    return false;
  }
  return normalizeRouteAccountIds(routeAccountIds).includes(normalizedAccountId);
}

export function mergeRouteAccountIds(
  routeAccountIds: string[],
  accountIdsToAdd: string[],
): string[] {
  return normalizeRouteAccountIds([
    ...normalizeRouteAccountIds(routeAccountIds),
    ...normalizeRouteAccountIds(accountIdsToAdd),
  ]);
}

export function removeRouteAccountIds(
  routeAccountIds: string[],
  accountIdsToRemove: string[],
): string[] {
  const currentIds = normalizeRouteAccountIds(routeAccountIds);
  if (!currentIds.length) {
    return [];
  }
  const idsToRemove = new Set(normalizeRouteAccountIds(accountIdsToRemove));
  if (!idsToRemove.size) {
    return currentIds;
  }
  return currentIds.filter((accountId) => !idsToRemove.has(accountId));
}

export function describeRouteAccountScope(
  routeAccountIds: string[],
  knownAccountIds: string[],
): string {
  const normalizedRouteAccountIds = normalizeRouteAccountIds(routeAccountIds);
  if (!normalizedRouteAccountIds.length) {
    return "全部可用账号参与路由";
  }

  const knownIds = new Set(
    knownAccountIds
      .map((accountId) => String(accountId || "").trim())
      .filter(Boolean),
  );
  const effectiveCount = normalizedRouteAccountIds.filter((accountId) =>
    knownIds.has(accountId),
  ).length;
  if (effectiveCount === 0) {
    return "全部可用账号参与路由";
  }

  return `已限制为 ${effectiveCount} 个账号参与路由`;
}
