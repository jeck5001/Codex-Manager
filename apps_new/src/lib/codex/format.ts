import type {
  AccountRecord,
  AvailabilityStatus,
  RequestLogTodaySummary,
  UsageAggregateSummary,
  UsageSnapshot,
} from "@/lib/codex/types";

const dateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  year: "numeric",
  month: "numeric",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
  second: "2-digit",
});

const COMPACT_NUMBER_UNITS = [
  { value: 1e18, suffix: "E" },
  { value: 1e15, suffix: "P" },
  { value: 1e12, suffix: "T" },
  { value: 1e9, suffix: "B" },
  { value: 1e6, suffix: "M" },
  { value: 1e3, suffix: "K" },
] as const;

const MINUTES_PER_HOUR = 60;
const MINUTES_PER_DAY = 24 * MINUTES_PER_HOUR;
const ROUNDING_BIAS = 3;

export const EMPTY_REQUEST_LOG_TODAY_SUMMARY: RequestLogTodaySummary = {
  todayTokens: 0,
  cachedInputTokens: 0,
  reasoningOutputTokens: 0,
  estimatedCost: 0,
};

export const EMPTY_USAGE_AGGREGATE_SUMMARY: UsageAggregateSummary = {
  primaryBucketCount: 0,
  primaryKnownCount: 0,
  primaryUnknownCount: 0,
  primaryRemainPercent: null,
  secondaryBucketCount: 0,
  secondaryKnownCount: 0,
  secondaryUnknownCount: 0,
  secondaryRemainPercent: null,
};

function formatDateTime(date: Date) {
  return dateTimeFormatter.format(date);
}

function trimTrailingZeros(text: string) {
  return String(text)
    .replace(/\.0+$/, "")
    .replace(/(\.\d*[1-9])0+$/, "$1");
}

function parseFiniteNumber(value: unknown) {
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }
  if (typeof value === "string") {
    const normalized = value.trim();
    if (!normalized) {
      return null;
    }
    const parsed = Number(normalized);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

export function formatTimestamp(timestamp: number | null | undefined, emptyLabel = "未知") {
  if (!timestamp) {
    return emptyLabel;
  }
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return emptyLabel;
  }
  return formatDateTime(date);
}

export function formatCompactNumber(value: number | string | null | undefined, fallback = "-", maxFractionDigits = 1) {
  const parsed = parseFiniteNumber(value);
  if (parsed == null) {
    return fallback;
  }

  const normalized = Math.max(0, parsed);
  if (normalized < 1000) {
    return `${Math.round(normalized)}`;
  }

  for (const unit of COMPACT_NUMBER_UNITS) {
    if (normalized < unit.value) {
      continue;
    }
    const scaled = normalized / unit.value;
    return `${trimTrailingZeros(scaled.toFixed(maxFractionDigits))}${unit.suffix}`;
  }

  return `${Math.round(normalized)}`;
}

export function formatLimitLabel(windowMinutes: number | null | undefined, fallback: string) {
  if (windowMinutes == null) {
    return fallback;
  }

  const minutes = Math.max(0, windowMinutes);
  const minutesPerWeek = 7 * MINUTES_PER_DAY;
  const minutesPerMonth = 30 * MINUTES_PER_DAY;

  if (minutes <= MINUTES_PER_DAY + ROUNDING_BIAS) {
    const hours = Math.max(1, Math.floor((minutes + ROUNDING_BIAS) / MINUTES_PER_HOUR));
    return `${hours}小时用量`;
  }
  if (minutes <= minutesPerWeek + ROUNDING_BIAS) {
    return "7天用量";
  }
  if (minutes <= minutesPerMonth + ROUNDING_BIAS) {
    return "30天用量";
  }
  return "年度用量";
}

export function formatResetLabel(timestamp: number | null | undefined) {
  if (!timestamp) {
    return "重置：--";
  }
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return "重置：--";
  }

  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear()
    && date.getMonth() === now.getMonth()
    && date.getDate() === now.getDate();
  const hh = String(date.getHours()).padStart(2, "0");
  const mm = String(date.getMinutes()).padStart(2, "0");

  if (sameDay) {
    return `重置：${hh}:${mm}`;
  }

  const month = date.getMonth() + 1;
  const day = date.getDate();
  return `重置：${month}月${day}日 ${hh}:${mm}`;
}

function isInactiveAccount(account: AccountRecord | null | undefined) {
  return String(account?.status || "").trim().toLowerCase() === "inactive";
}

export function calculateAvailability(
  usage: UsageSnapshot | null | undefined,
  account: AccountRecord | null | undefined = null,
): AvailabilityStatus {
  if (isInactiveAccount(account)) {
    return { text: "不可用", level: "bad" };
  }
  if (!usage) {
    return { text: "未知", level: "unknown" };
  }

  const normalizedStatus = String(usage.availabilityStatus || "").trim().toLowerCase();
  if (normalizedStatus === "available") {
    return { text: "可用", level: "ok" };
  }
  if (normalizedStatus === "primary_window_available_only") {
    return { text: "单窗口可用", level: "ok" };
  }
  if (normalizedStatus === "unavailable") {
    return { text: "不可用", level: "bad" };
  }
  if (normalizedStatus === "unknown") {
    return { text: "未知", level: "unknown" };
  }

  const primaryMissing = usage.usedPercent == null || usage.windowMinutes == null;
  const secondaryPresent = usage.secondaryUsedPercent != null || usage.secondaryWindowMinutes != null;
  const secondaryMissing = usage.secondaryUsedPercent == null || usage.secondaryWindowMinutes == null;

  if (primaryMissing) {
    return { text: "用量缺失", level: "bad" };
  }
  if ((usage.usedPercent || 0) >= 100) {
    return { text: "5小时已用尽", level: "warn" };
  }
  if (!secondaryPresent) {
    return { text: "单窗口可用", level: "ok" };
  }
  if (secondaryMissing) {
    return { text: "用量缺失", level: "bad" };
  }
  if ((usage.secondaryUsedPercent || 0) >= 100) {
    return { text: "7日已用尽", level: "bad" };
  }
  return { text: "可用", level: "ok" };
}

function normalizePercent(value: number | null | undefined) {
  if (value == null) {
    return null;
  }
  return Math.max(0, Math.min(100, value));
}

export function remainingPercent(value: number | null | undefined) {
  const used = normalizePercent(value);
  if (used == null) {
    return null;
  }
  return Math.max(0, 100 - used);
}

export function computeUsageStats(accounts: AccountRecord[], usageSnapshots: UsageSnapshot[]) {
  const usageMap = new Map(usageSnapshots.map((item) => [item.accountId, item]));
  let total = 0;
  let okCount = 0;
  let unavailableCount = 0;
  let lowCount = 0;

  for (const account of accounts) {
    total += 1;
    const usage = usageMap.get(account.id);
    const availability = calculateAvailability(usage, account);
    if (availability.level === "ok") {
      okCount += 1;
    }
    if (availability.level === "warn" || availability.level === "bad") {
      unavailableCount += 1;
    }

    const primaryRemain = remainingPercent(usage?.usedPercent);
    const secondaryRemain = remainingPercent(usage?.secondaryUsedPercent);
    if ((primaryRemain != null && primaryRemain <= 20) || (secondaryRemain != null && secondaryRemain <= 20)) {
      lowCount += 1;
    }
  }

  return { total, okCount, unavailableCount, lowCount };
}

function isLongWindow(windowMinutes: number | null | undefined) {
  const minutes = parseFiniteNumber(windowMinutes);
  return minutes != null && minutes > MINUTES_PER_DAY + ROUNDING_BIAS;
}

function extractPlanTypeRecursive(value: unknown): string | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      const nested = extractPlanTypeRecursive(item);
      if (nested) {
        return nested;
      }
    }
    return null;
  }

  const keys = [
    "plan_type",
    "planType",
    "subscription_tier",
    "subscriptionTier",
    "tier",
    "account_type",
    "accountType",
    "type",
  ] as const;

  for (const key of keys) {
    const candidate = (value as Record<string, unknown>)[key];
    if (typeof candidate === "string" && candidate.trim()) {
      return candidate.trim().toLowerCase();
    }
  }

  for (const candidate of Object.values(value as Record<string, unknown>)) {
    const nested = extractPlanTypeRecursive(candidate);
    if (nested) {
      return nested;
    }
  }

  return null;
}

export function parseCredits(raw: string | null | undefined) {
  if (!raw) {
    return null;
  }
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return null;
  }
}

function isFreePlanUsage(usage: UsageSnapshot | null | undefined) {
  const planType = extractPlanTypeRecursive(parseCredits(usage?.creditsJson));
  return typeof planType === "string" && planType.includes("free");
}

export function computeAggregateRemainingStats(accounts: AccountRecord[], usageSnapshots: UsageSnapshot[]) {
  const usageMap = new Map(usageSnapshots.map((item) => [item.accountId, item]));

  let primaryBucketCount = 0;
  let primaryKnownCount = 0;
  let primaryRemainingTotal = 0;
  let secondaryBucketCount = 0;
  let secondaryKnownCount = 0;
  let secondaryRemainingTotal = 0;

  for (const account of accounts) {
    const usage = usageMap.get(account.id);
    const hasPrimarySignal = usage?.usedPercent != null || usage?.windowMinutes != null;
    const hasSecondarySignal = usage?.secondaryUsedPercent != null || usage?.secondaryWindowMinutes != null;
    const primaryRemain = remainingPercent(usage?.usedPercent);
    const primaryBelongsToSecondary = !hasSecondarySignal && (isLongWindow(usage?.windowMinutes) || isFreePlanUsage(usage));

    if (hasPrimarySignal) {
      if (primaryBelongsToSecondary) {
        secondaryBucketCount += 1;
      } else {
        primaryBucketCount += 1;
      }
    }

    if (primaryRemain != null) {
      if (primaryBelongsToSecondary) {
        secondaryKnownCount += 1;
        secondaryRemainingTotal += primaryRemain;
      } else {
        primaryKnownCount += 1;
        primaryRemainingTotal += primaryRemain;
      }
    }

    const secondaryRemain = remainingPercent(usage?.secondaryUsedPercent);
    if (hasSecondarySignal) {
      secondaryBucketCount += 1;
    }
    if (secondaryRemain != null) {
      secondaryKnownCount += 1;
      secondaryRemainingTotal += secondaryRemain;
    }
  }

  return {
    primaryBucketCount,
    primaryKnownCount,
    primaryUnknownCount: Math.max(0, primaryBucketCount - primaryKnownCount),
    primaryRemainPercent: primaryKnownCount > 0 ? Math.round(primaryRemainingTotal / primaryKnownCount) : null,
    secondaryBucketCount,
    secondaryKnownCount,
    secondaryUnknownCount: Math.max(0, secondaryBucketCount - secondaryKnownCount),
    secondaryRemainPercent: secondaryKnownCount > 0 ? Math.round(secondaryRemainingTotal / secondaryKnownCount) : null,
  } satisfies UsageAggregateSummary;
}
