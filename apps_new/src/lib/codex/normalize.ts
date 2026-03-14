import type {
  RequestLogRecord,
  RequestLogTodaySummary,
  StartupSnapshot,
  UsageAggregateSummary,
} from "@/lib/codex/types";
import { EMPTY_REQUEST_LOG_TODAY_SUMMARY, EMPTY_USAGE_AGGREGATE_SUMMARY } from "@/lib/codex/format";

function readPath(source: unknown, path: string) {
  const steps = path.split(".");
  let cursor = source;
  for (const step of steps) {
    if (!cursor || typeof cursor !== "object" || !(step in cursor)) {
      return undefined;
    }
    cursor = (cursor as Record<string, unknown>)[step];
  }
  return cursor;
}

function toFiniteNumber(value: unknown) {
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

function pickNumber(source: unknown, paths: string[], fallback = 0) {
  for (const path of paths) {
    const parsed = toFiniteNumber(readPath(source, path));
    if (parsed != null) {
      return parsed;
    }
  }
  return fallback;
}

export function normalizeRequestLogTodaySummary(source: unknown): RequestLogTodaySummary {
  const inputTokens = pickNumber(source, ["inputTokens", "promptTokens", "tokens.input"], 0);
  const outputTokens = pickNumber(source, ["outputTokens", "completionTokens", "tokens.output"], 0);
  const cachedInputTokens = pickNumber(source, ["cachedInputTokens", "cachedTokens", "tokens.cachedInput"], 0);
  const reasoningOutputTokens = pickNumber(
    source,
    ["reasoningOutputTokens", "reasoningTokens", "tokens.reasoningOutput"],
    0,
  );
  const todayTokens = pickNumber(
    source,
    ["todayTokens", "totalTokens", "tokenTotal", "tokens.total"],
    Math.max(0, inputTokens - cachedInputTokens) + outputTokens,
  );
  const estimatedCost = pickNumber(source, ["estimatedCost", "cost", "costEstimate", "todayCost"], 0);

  return {
    todayTokens: Math.max(0, todayTokens),
    cachedInputTokens: Math.max(0, cachedInputTokens),
    reasoningOutputTokens: Math.max(0, reasoningOutputTokens),
    estimatedCost: Math.max(0, estimatedCost),
  };
}

export function normalizeUsageAggregateSummary(source: unknown): UsageAggregateSummary {
  const primaryBucketCount = pickNumber(source, ["primaryBucketCount"], 0);
  const primaryKnownCount = pickNumber(source, ["primaryKnownCount"], 0);
  const primaryUnknownCount = pickNumber(
    source,
    ["primaryUnknownCount"],
    Math.max(0, primaryBucketCount - primaryKnownCount),
  );
  const secondaryBucketCount = pickNumber(source, ["secondaryBucketCount"], 0);
  const secondaryKnownCount = pickNumber(source, ["secondaryKnownCount"], 0);
  const secondaryUnknownCount = pickNumber(
    source,
    ["secondaryUnknownCount"],
    Math.max(0, secondaryBucketCount - secondaryKnownCount),
  );
  const primaryRemainPercent = toFiniteNumber(readPath(source, "primaryRemainPercent"));
  const secondaryRemainPercent = toFiniteNumber(readPath(source, "secondaryRemainPercent"));

  return {
    primaryBucketCount: Math.max(0, Math.trunc(primaryBucketCount)),
    primaryKnownCount: Math.max(0, Math.trunc(primaryKnownCount)),
    primaryUnknownCount: Math.max(0, Math.trunc(primaryUnknownCount)),
    primaryRemainPercent:
      primaryRemainPercent == null ? null : Math.max(0, Math.min(100, Math.round(primaryRemainPercent))),
    secondaryBucketCount: Math.max(0, Math.trunc(secondaryBucketCount)),
    secondaryKnownCount: Math.max(0, Math.trunc(secondaryKnownCount)),
    secondaryUnknownCount: Math.max(0, Math.trunc(secondaryUnknownCount)),
    secondaryRemainPercent:
      secondaryRemainPercent == null ? null : Math.max(0, Math.min(100, Math.round(secondaryRemainPercent))),
  };
}

export function buildRequestLogIdentity(item: RequestLogRecord, index: number) {
  if (item.id && String(item.id).trim()) {
    return String(item.id);
  }
  return [
    item.createdAt ?? "",
    item.method ?? "",
    item.statusCode ?? "",
    item.accountId ?? "",
    item.keyId ?? "",
    index,
  ].join("|");
}

export function normalizeStartupSnapshot(source: StartupSnapshot | null | undefined): StartupSnapshot {
  if (!source) {
    return {
      accounts: [],
      usageSnapshots: [],
      usageAggregateSummary: { ...EMPTY_USAGE_AGGREGATE_SUMMARY },
      apiKeys: [],
      apiModelOptions: [],
      manualPreferredAccountId: "",
      requestLogTodaySummary: { ...EMPTY_REQUEST_LOG_TODAY_SUMMARY },
      requestLogs: [],
    };
  }

  const requestLogs = Array.isArray(source.requestLogs) ? source.requestLogs : [];
  return {
    accounts: Array.isArray(source.accounts) ? source.accounts : [],
    usageSnapshots: Array.isArray(source.usageSnapshots) ? source.usageSnapshots : [],
    usageAggregateSummary: normalizeUsageAggregateSummary(source.usageAggregateSummary),
    apiKeys: Array.isArray(source.apiKeys) ? source.apiKeys : [],
    apiModelOptions: Array.isArray(source.apiModelOptions) ? source.apiModelOptions : [],
    manualPreferredAccountId: String(source.manualPreferredAccountId || ""),
    requestLogTodaySummary: normalizeRequestLogTodaySummary(source.requestLogTodaySummary),
    requestLogs: requestLogs.map((item, index) => ({
      ...item,
      __identity: item.__identity || buildRequestLogIdentity(item, index),
    })),
  };
}
