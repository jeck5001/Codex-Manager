"use client";

import {
  Account,
  AccountListResult,
  AccountUsage,
  AuditLogExportResult,
  AuditLogItem,
  AuditLogListResult,
  AlertChannel,
  AlertChannelTestResult,
  AlertHistoryItem,
  AlertRule,
  CostExportResult,
  CostSummaryDayItem,
  CostSummaryKeyItem,
  CostSummaryModelItem,
  CostSummaryResult,
  CostUsageSummary,
  ApiKey,
  ApiKeyAllowedModelsConfig,
  ApiKeyCreateResult,
  ApiKeyModelFallback,
  ApiKeyResponseCacheConfig,
  ApiKeyRateLimit,
  ApiKeyUsageStat,
  AppSettings,
  AccountHealthTier,
  BackgroundTaskSettings,
  DashboardAccountStatusBucket,
  DashboardHealth,
  DashboardTrend,
  DashboardTrendPoint,
  DeviceAuthInfo,
  EnvOverrideCatalogItem,
  FailureReasonSummaryItem,
  GovernanceSummaryItem,
  FreeProxySyncResult,
  GatewayResponseCacheStats,
  HeatmapCellItem,
  HeatmapTrendResult,
  HealthcheckConfig,
  HealthcheckRunResult,
  LoginStartResult,
  ModelTrendItem,
  ModelTrendResult,
  ModelPricingItem,
  ModelOption,
  RequestTrendItem,
  RequestTrendResult,
  OperationAuditItem,
  PluginItem,
  RequestLog,
  RequestLogExportResult,
  RequestLogFilterSummary,
  RequestLogListResult,
  RequestLogTodaySummary,
  StartupSnapshot,
  UsageAggregateSummary,
  UsagePredictionSummary,
  WebAuthTwoFactorSetupResult,
  WebAuthTwoFactorStatusResult,
} from "@/types";
import { APP_NAV_DEFAULT_VISIBLE_IDS, normalizeVisibleMenuItems } from "@/lib/navigation";
import {
  calcAvailability,
  getUsageDisplayBuckets,
  isLowQuotaUsage,
  toNullableNumber,
} from "@/lib/utils/usage";

const DEFAULT_BACKGROUND_TASKS: BackgroundTaskSettings = {
  usagePollingEnabled: true,
  usagePollIntervalSecs: 600,
  gatewayKeepaliveEnabled: true,
  gatewayKeepaliveIntervalSecs: 180,
  tokenRefreshPollingEnabled: true,
  tokenRefreshPollIntervalSecs: 60,
  sessionProbePollingEnabled: false,
  sessionProbeIntervalSecs: 1800,
  sessionProbeSampleSize: 2,
  usageRefreshWorkers: 4,
  httpWorkerFactor: 4,
  httpWorkerMin: 8,
  httpStreamWorkerFactor: 1,
  httpStreamWorkerMin: 2,
  autoRegisterPoolEnabled: false,
  autoRegisterReadyAccountCount: 2,
  autoRegisterReadyRemainPercent: 20,
  autoDisableRiskyAccountsEnabled: false,
  autoDisableRiskyAccountsFailureThreshold: 3,
  autoDisableRiskyAccountsHealthScoreThreshold: 60,
  autoDisableRiskyAccountsLookbackMins: 60,
  accountCooldownAuthSecs: 300,
  accountCooldownRateLimitedSecs: 45,
  accountCooldownServerErrorSecs: 30,
  accountCooldownNetworkSecs: 20,
  accountCooldownLowQuotaSecs: 1800,
  accountCooldownDeactivatedSecs: 21600,
};

function asObject(payload: unknown): Record<string, unknown> {
  return payload && typeof payload === "object" && !Array.isArray(payload)
    ? (payload as Record<string, unknown>)
    : {};
}

function asArray<T = unknown>(payload: unknown): T[] {
  return Array.isArray(payload) ? payload : [];
}

function asString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value.trim() : fallback;
}

function asBoolean(value: unknown, fallback = false): boolean {
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return value !== 0;
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (["1", "true", "yes", "on"].includes(normalized)) return true;
    if (["0", "false", "no", "off"].includes(normalized)) return false;
  }
  return fallback;
}

function asInteger(value: unknown, fallback: number, min = 0): number {
  const parsed = toNullableNumber(value);
  if (parsed == null) return fallback;
  return Math.max(min, Math.trunc(parsed));
}

function normalizeStringRecord(payload: unknown): Record<string, string> {
  const source = asObject(payload);
  return Object.entries(source).reduce<Record<string, string>>((result, [key, value]) => {
    result[key] = asString(value);
    return result;
  }, {});
}

export function normalizeUsageSnapshot(payload: unknown): AccountUsage | null {
  const source = asObject(payload);
  const accountId = asString(source.accountId ?? source.account_id);
  if (!accountId) return null;

  return {
    accountId,
    availabilityStatus: asString(source.availabilityStatus ?? source.availability_status),
    usedPercent: toNullableNumber(source.usedPercent ?? source.used_percent),
    windowMinutes: toNullableNumber(source.windowMinutes ?? source.window_minutes),
    resetsAt: toNullableNumber(source.resetsAt ?? source.resets_at),
    secondaryUsedPercent: toNullableNumber(
      source.secondaryUsedPercent ?? source.secondary_used_percent
    ),
    secondaryWindowMinutes: toNullableNumber(
      source.secondaryWindowMinutes ?? source.secondary_window_minutes
    ),
    secondaryResetsAt: toNullableNumber(
      source.secondaryResetsAt ?? source.secondary_resets_at
    ),
    creditsJson: asString(source.creditsJson ?? source.credits_json) || null,
    capturedAt: toNullableNumber(source.capturedAt ?? source.captured_at),
  };
}

export function normalizeUsageList(payload: unknown): AccountUsage[] {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload);
  return items
    .map((item) => normalizeUsageSnapshot(item))
    .filter((item): item is AccountUsage => Boolean(item));
}

export function buildUsageMap(usages: AccountUsage[]): Map<string, AccountUsage> {
  return new Map(usages.map((item) => [item.accountId, item]));
}

export function normalizeUsageAggregateSummary(payload: unknown): UsageAggregateSummary {
  const source = asObject(payload);
  return {
    primaryBucketCount: asInteger(source.primaryBucketCount, 0, 0),
    primaryKnownCount: asInteger(source.primaryKnownCount, 0, 0),
    primaryUnknownCount: asInteger(source.primaryUnknownCount, 0, 0),
    primaryRemainPercent: toNullableNumber(source.primaryRemainPercent),
    secondaryBucketCount: asInteger(source.secondaryBucketCount, 0, 0),
    secondaryKnownCount: asInteger(source.secondaryKnownCount, 0, 0),
    secondaryUnknownCount: asInteger(source.secondaryUnknownCount, 0, 0),
    secondaryRemainPercent: toNullableNumber(source.secondaryRemainPercent),
  };
}

export function normalizeUsagePredictionSummary(
  payload: unknown
): UsagePredictionSummary {
  const source = asObject(payload);
  return {
    quotaProtectionEnabled: asBoolean(
      source.quotaProtectionEnabled ?? source.quota_protection_enabled,
      false
    ),
    quotaProtectionThresholdPercent: asInteger(
      source.quotaProtectionThresholdPercent ??
        source.quota_protection_threshold_percent,
      0,
      0
    ),
    readyAccountCount: asInteger(
      source.readyAccountCount ?? source.ready_account_count,
      0,
      0
    ),
    estimatedHoursToThreshold: toNullableNumber(
      source.estimatedHoursToThreshold ?? source.estimated_hours_to_threshold
    ),
    estimatedHoursToPoolExhaustion: toNullableNumber(
      source.estimatedHoursToPoolExhaustion ??
        source.estimated_hours_to_pool_exhaustion
    ),
    thresholdLimitedBy:
      asString(source.thresholdLimitedBy ?? source.threshold_limited_by) || null,
    poolLimitedBy:
      asString(source.poolLimitedBy ?? source.pool_limited_by) || null,
  };
}

export function normalizeFailureReasonSummary(
  payload: unknown
): FailureReasonSummaryItem[] {
  return asArray(payload)
    .map((item) => {
      const source = asObject(item);
      const code = asString(source.code);
      if (!code) return null;
      return {
        code,
        label: asString(source.label) || code,
        count: asInteger(source.count, 0, 0),
        affectedAccounts: asInteger(
          source.affectedAccounts ?? source.affected_accounts,
          0,
          0
        ),
        lastSeenAt: toNullableNumber(source.lastSeenAt ?? source.last_seen_at),
      };
    })
    .filter((item): item is FailureReasonSummaryItem => Boolean(item));
}

export function normalizeGovernanceSummary(
  payload: unknown
): GovernanceSummaryItem[] {
  return asArray(payload)
    .map((item) => {
      const source = asObject(item);
      const code = asString(source.code);
      if (!code) return null;
      return {
        code,
        label: asString(source.label) || code,
        targetStatus:
          asString(source.targetStatus ?? source.target_status) || "disabled",
        count: asInteger(source.count, 0, 0),
        affectedAccounts: asInteger(
          source.affectedAccounts ?? source.affected_accounts,
          0,
          0
        ),
        lastSeenAt: toNullableNumber(source.lastSeenAt ?? source.last_seen_at),
      };
    })
    .filter((item): item is GovernanceSummaryItem => Boolean(item));
}

export function normalizeOperationAudits(payload: unknown): OperationAuditItem[] {
  return asArray(payload)
    .map((item) => {
      const source = asObject(item);
      const action = asString(source.action);
      if (!action) return null;
      return {
        action,
        label: asString(source.label) || action,
        detail: asString(source.detail),
        accountId: asString(source.accountId ?? source.account_id) || null,
        createdAt: toNullableNumber(source.createdAt ?? source.created_at),
      };
    })
    .filter((item): item is OperationAuditItem => Boolean(item));
}

export function normalizeAuditLogItem(item: unknown): AuditLogItem | null {
  const source = asObject(item);
  const id = asInteger(source.id, -1, -1);
  const action = asString(source.action);
  const objectType = asString(source.objectType ?? source.object_type);
  const createdAt = asInteger(source.createdAt ?? source.created_at, 0, 0);
  if (id < 0 || !action || !objectType || createdAt <= 0) {
    return null;
  }
  const changesValue = source.changes;
  return {
    id,
    action,
    objectType,
    objectId: asString(source.objectId ?? source.object_id) || null,
    operator: asString(source.operator) || "unknown",
    changes:
      changesValue && typeof changesValue === "object" && !Array.isArray(changesValue)
        ? (changesValue as Record<string, unknown>)
        : null,
    createdAt,
  };
}

export function normalizeAuditLogListResult(payload: unknown): AuditLogListResult {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload)
    .map((item) => normalizeAuditLogItem(item))
    .filter((item): item is AuditLogItem => Boolean(item));
  return {
    items,
    total: asInteger(source.total, items.length, 0),
    page: asInteger(source.page, 1, 1),
    pageSize: asInteger(source.pageSize, items.length || 20, 1),
  };
}

export function normalizeAuditLogExportResult(payload: unknown): AuditLogExportResult {
  const source = asObject(payload);
  return {
    format: asString(source.format) || "csv",
    fileName: asString(source.fileName ?? source.file_name) || "codexmanager-auditlogs.csv",
    content: asString(source.content),
    recordCount: asInteger(source.recordCount ?? source.record_count, 0, 0),
  };
}

export function normalizeTodaySummary(payload: unknown): RequestLogTodaySummary {
  const source = asObject(payload);
  const inputTokens = asInteger(source.inputTokens, 0, 0);
  const cachedInputTokens = asInteger(source.cachedInputTokens, 0, 0);
  const outputTokens = asInteger(source.outputTokens, 0, 0);
  const reasoningOutputTokens = asInteger(source.reasoningOutputTokens, 0, 0);
  return {
    inputTokens,
    cachedInputTokens,
    outputTokens,
    reasoningOutputTokens,
    todayTokens: asInteger(
      source.todayTokens,
      Math.max(0, inputTokens - cachedInputTokens) + outputTokens,
      0
    ),
    estimatedCost: Math.max(0, toNullableNumber(source.estimatedCost) ?? 0),
  };
}

export function normalizeDashboardHealth(payload: unknown): DashboardHealth {
  const source = asObject(payload);
  const bucketItems = asArray(source.accountStatusBuckets ?? source.account_status_buckets);
  const metricsSource = asObject(source.gatewayMetrics ?? source.gateway_metrics);

  return {
    generatedAt: toNullableNumber(source.generatedAt ?? source.generated_at),
    accountStatusBuckets: bucketItems
      .map((item): DashboardAccountStatusBucket | null => {
        const bucket = asObject(item);
        const key = asString(bucket.key);
        if (!key) return null;
        return {
          key,
          label: asString(bucket.label) || key,
          count: asInteger(bucket.count, 0, 0),
          percent: asInteger(bucket.percent, 0, 0),
        };
      })
      .filter((item): item is DashboardAccountStatusBucket => Boolean(item)),
    gatewayMetrics: {
      windowMinutes: asInteger(metricsSource.windowMinutes ?? metricsSource.window_minutes, 5, 1),
      totalRequests: asInteger(metricsSource.totalRequests ?? metricsSource.total_requests, 0, 0),
      successRequests: asInteger(
        metricsSource.successRequests ?? metricsSource.success_requests,
        0,
        0
      ),
      errorRequests: asInteger(metricsSource.errorRequests ?? metricsSource.error_requests, 0, 0),
      qps: Math.max(0, toNullableNumber(metricsSource.qps) ?? 0),
      successRate: Math.max(
        0,
        Math.min(100, toNullableNumber(metricsSource.successRate ?? metricsSource.success_rate) ?? 0)
      ),
      p50LatencyMs: toNullableNumber(
        metricsSource.p50LatencyMs ?? metricsSource.p50_latency_ms
      ),
      p95LatencyMs: toNullableNumber(
        metricsSource.p95LatencyMs ?? metricsSource.p95_latency_ms
      ),
      p99LatencyMs: toNullableNumber(
        metricsSource.p99LatencyMs ?? metricsSource.p99_latency_ms
      ),
    },
    recentHealthcheck: normalizeHealthcheckRun(
      source.recentHealthcheck ?? source.recent_healthcheck
    ),
  };
}

export function normalizeHealthcheckRun(payload: unknown): HealthcheckRunResult | null {
  const source = asObject(payload);
  const startedAt = toNullableNumber(source.startedAt ?? source.started_at);
  const finishedAt = toNullableNumber(source.finishedAt ?? source.finished_at);
  const sampledAccounts = asInteger(
    source.sampledAccounts ?? source.sampled_accounts,
    0,
    0
  );
  const successCount = asInteger(source.successCount ?? source.success_count, 0, 0);
  const failureCount = asInteger(source.failureCount ?? source.failure_count, 0, 0);
  const failedAccountsSource = source.failedAccounts ?? source.failed_accounts;

  if (
    startedAt == null &&
    finishedAt == null &&
    sampledAccounts === 0 &&
    successCount === 0 &&
    failureCount === 0 &&
    !Array.isArray(failedAccountsSource)
  ) {
    return null;
  }

  return {
    startedAt,
    finishedAt,
    totalAccounts: asInteger(source.totalAccounts ?? source.total_accounts, 0, 0),
    sampledAccounts,
    successCount,
    failureCount,
    failedAccounts: asArray(failedAccountsSource)
      .map((item) => {
        const failed = asObject(item);
        const accountId = asString(failed.accountId ?? failed.account_id);
        if (!accountId) return null;
        return {
          accountId,
          label: asString(failed.label) || null,
          reason: asString(failed.reason),
        };
      })
      .filter(
        (
          item
        ): item is {
          accountId: string;
          label: string | null;
          reason: string;
        } => Boolean(item)
      ),
  };
}

export function normalizeHealthcheckConfig(payload: unknown): HealthcheckConfig {
  const source = asObject(payload);
  return {
    enabled: asBoolean(source.enabled, false),
    intervalSecs: asInteger(source.intervalSecs ?? source.interval_secs, 1800, 1),
    sampleSize: asInteger(source.sampleSize ?? source.sample_size, 2, 1),
    recentRun: normalizeHealthcheckRun(source.recentRun ?? source.recent_run),
  };
}

export function normalizeDashboardTrend(payload: unknown): DashboardTrend {
  const source = asObject(payload);
  const points = asArray(source.points).map((item): DashboardTrendPoint | null => {
    const point = asObject(item);
    const bucketTs = asInteger(point.bucketTs ?? point.bucket_ts, 0, 0);
    if (!bucketTs) return null;
    return {
      bucketTs,
      requestCount: asInteger(point.requestCount ?? point.request_count, 0, 0),
      errorCount: asInteger(point.errorCount ?? point.error_count, 0, 0),
      errorRate: Math.max(
        0,
        Math.min(100, toNullableNumber(point.errorRate ?? point.error_rate) ?? 0)
      ),
    };
  });

  return {
    generatedAt: toNullableNumber(source.generatedAt ?? source.generated_at),
    bucketMinutes: asInteger(source.bucketMinutes ?? source.bucket_minutes, 1, 1),
    points: points.filter((item): item is DashboardTrendPoint => Boolean(item)),
  };
}

export function normalizeAlertRule(item: unknown): AlertRule | null {
  const source = asObject(item);
  const id = asString(source.id);
  if (!id) return null;
  return {
    id,
    name: asString(source.name),
    ruleType: asString(source.ruleType ?? source.rule_type),
    config: asObject(source.config ?? source.config_json),
    enabled: asBoolean(source.enabled, true),
    createdAt: toNullableNumber(source.createdAt ?? source.created_at),
    updatedAt: toNullableNumber(source.updatedAt ?? source.updated_at),
  };
}

export function normalizeAlertRuleList(payload: unknown): AlertRule[] {
  const source = asObject(payload);
  return asArray(source.items ?? payload)
    .map((item) => normalizeAlertRule(item))
    .filter((item): item is AlertRule => Boolean(item));
}

export function normalizeAlertChannel(item: unknown): AlertChannel | null {
  const source = asObject(item);
  const id = asString(source.id);
  if (!id) return null;
  return {
    id,
    name: asString(source.name),
    channelType: asString(source.channelType ?? source.channel_type),
    config: asObject(source.config ?? source.config_json),
    enabled: asBoolean(source.enabled, true),
    createdAt: toNullableNumber(source.createdAt ?? source.created_at),
    updatedAt: toNullableNumber(source.updatedAt ?? source.updated_at),
  };
}

export function normalizeAlertChannelList(payload: unknown): AlertChannel[] {
  const source = asObject(payload);
  return asArray(source.items ?? payload)
    .map((item) => normalizeAlertChannel(item))
    .filter((item): item is AlertChannel => Boolean(item));
}

export function normalizeAlertHistoryItem(item: unknown): AlertHistoryItem | null {
  const source = asObject(item);
  const id = asInteger(source.id, -1, -1);
  if (id < 0) return null;
  return {
    id,
    ruleId: asString(source.ruleId ?? source.rule_id) || null,
    ruleName: asString(source.ruleName ?? source.rule_name) || null,
    channelId: asString(source.channelId ?? source.channel_id) || null,
    channelName: asString(source.channelName ?? source.channel_name) || null,
    status: asString(source.status),
    message: asString(source.message),
    createdAt: toNullableNumber(source.createdAt ?? source.created_at),
  };
}

export function normalizeAlertHistoryList(payload: unknown): AlertHistoryItem[] {
  const source = asObject(payload);
  return asArray(source.items ?? payload)
    .map((item) => normalizeAlertHistoryItem(item))
    .filter((item): item is AlertHistoryItem => Boolean(item));
}

export function normalizeAlertChannelTestResult(
  payload: unknown
): AlertChannelTestResult {
  const source = asObject(payload);
  return {
    channelId: asString(source.channelId ?? source.channel_id),
    status: asString(source.status),
    sentAt: toNullableNumber(source.sentAt ?? source.sent_at),
  };
}

export function normalizePlugin(item: unknown): PluginItem | null {
  const source = asObject(item);
  const id = asString(source.id);
  if (!id) return null;
  return {
    id,
    name: asString(source.name),
    description: asString(source.description) || null,
    runtime: asString(source.runtime),
    hookPoints: asArray(source.hookPoints ?? source.hook_points)
      .map((value) => asString(value))
      .filter((value) => value.length > 0),
    scriptContent: asString(source.scriptContent ?? source.script_content),
    enabled: asBoolean(source.enabled, true),
    timeoutMs: asInteger(source.timeoutMs ?? source.timeout_ms, 100, 1),
    createdAt: toNullableNumber(source.createdAt ?? source.created_at),
    updatedAt: toNullableNumber(source.updatedAt ?? source.updated_at),
  };
}

export function normalizePluginList(payload: unknown): PluginItem[] {
  const source = asObject(payload);
  return asArray(source.items ?? payload)
    .map((item) => normalizePlugin(item))
    .filter((item): item is PluginItem => Boolean(item));
}

export function normalizeAccount(item: unknown, usage?: AccountUsage | null): Account | null {
  const source = asObject(item);
  const id = asString(source.id);
  if (!id) return null;

  const name = asString(source.label || source.name) || id;
  const groupName = asString(source.groupName ?? source.group_name);
  const status = asString(source.status);
  const healthScore = asInteger(source.healthScore ?? source.health_score, 100, 0);
  const cooldownUntil = toNullableNumber(
    source.cooldownUntil ?? source.cooldown_until
  );
  const newAccountProtectionUntil = toNullableNumber(
    source.newAccountProtectionUntil ?? source.new_account_protection_until
  );
  const availability = calcAvailability(usage, { status });
  const usageBuckets = getUsageDisplayBuckets(usage);
  const healthTier = deriveAccountHealthTier({
    status,
    healthScore,
    availabilityLevel: availability.level,
    isLowQuota: isLowQuotaUsage(usage),
  });
  const tags = Array.isArray(source.tags)
    ? source.tags
        .map((item) => asString(item))
        .filter(Boolean)
    : asString(source.tags)
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean);

  return {
    id,
    name,
    group: groupName,
    priority: asInteger(source.sort ?? source.priority, 0, 0),
    label: name,
    groupName,
    tags,
    sort: asInteger(source.sort ?? source.priority, 0, 0),
    status,
    healthScore,
    healthTier,
    lastStatusReason:
      asString(source.lastStatusReason ?? source.last_status_reason) || null,
    lastStatusChangedAt: toNullableNumber(
      source.lastStatusChangedAt ?? source.last_status_changed_at
    ),
    lastGovernanceReason:
      asString(source.lastGovernanceReason ?? source.last_governance_reason) || null,
    lastGovernanceAt: toNullableNumber(
      source.lastGovernanceAt ?? source.last_governance_at
    ),
    lastIsolationReasonCode:
      asString(
        source.lastIsolationReasonCode ?? source.last_isolation_reason_code
      ) || null,
    lastIsolationReason:
      asString(source.lastIsolationReason ?? source.last_isolation_reason) || null,
    lastIsolationAt: toNullableNumber(
      source.lastIsolationAt ?? source.last_isolation_at
    ),
    cooldownUntil,
    cooldownReasonCode:
      asString(source.cooldownReasonCode ?? source.cooldown_reason_code) || null,
    cooldownReason:
      asString(source.cooldownReason ?? source.cooldown_reason) || null,
    newAccountProtectionUntil,
    newAccountProtectionReason:
      asString(
        source.newAccountProtectionReason ?? source.new_account_protection_reason
      ) || null,
    isNewAccountProtected:
      typeof newAccountProtectionUntil === "number" &&
      Number.isFinite(newAccountProtectionUntil) &&
      newAccountProtectionUntil > Math.floor(Date.now() / 1000),
    isInCooldown:
      typeof cooldownUntil === "number" &&
      Number.isFinite(cooldownUntil) &&
      cooldownUntil > Math.floor(Date.now() / 1000),
    isIsolated:
      ["disabled", "inactive", "deactivated", "unavailable"].includes(
        status.toLowerCase()
      ) &&
      Boolean(
        asString(source.lastIsolationReason ?? source.last_isolation_reason)
      ),
    isAvailable: availability.level === "ok",
    isLowQuota: isLowQuotaUsage(usage),
    isDeactivated: status.toLowerCase() === "deactivated",
    lastRefreshAt: usage?.capturedAt ?? null,
    availabilityText: availability.text,
    availabilityLevel: availability.level,
    primaryRemainPercent: usageBuckets.primaryRemainPercent,
    secondaryRemainPercent: usageBuckets.secondaryRemainPercent,
    subscriptionPlanType: asString(
      source.subscriptionPlanType ?? source.subscription_plan_type
    ) || null,
    subscriptionUpdatedAt: toNullableNumber(
      source.subscriptionUpdatedAt ?? source.subscription_updated_at
    ),
    teamManagerUploadedAt: toNullableNumber(
      source.teamManagerUploadedAt ?? source.team_manager_uploaded_at
    ),
    officialPromoLink:
      asString(source.officialPromoLink ?? source.official_promo_link) || null,
    officialPromoLinkUpdatedAt: toNullableNumber(
      source.officialPromoLinkUpdatedAt ?? source.official_promo_link_updated_at
    ),
    usage: usage ?? null,
  };
}

function deriveAccountHealthTier(input: {
  status: string;
  healthScore: number;
  availabilityLevel: string;
  isLowQuota: boolean;
}): AccountHealthTier {
  const normalizedStatus = String(input.status || "").trim().toLowerCase();
  if (
    ["deactivated", "disabled", "inactive", "unavailable"].includes(normalizedStatus) ||
    input.healthScore < 70
  ) {
    return "risky";
  }
  if (
    input.isLowQuota ||
    input.availabilityLevel !== "ok" ||
    input.healthScore < 100
  ) {
    return "warning";
  }
  return "healthy";
}

export function normalizeAccountList(
  payload: unknown,
  usages: AccountUsage[] = []
): AccountListResult {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload);
  const usageMap = buildUsageMap(usages);
  const normalizedItems = items
    .map((item) => normalizeAccount(item, usageMap.get(asString(asObject(item).id))))
    .filter((item): item is Account => Boolean(item));

  return {
    items: normalizedItems,
    total: asInteger(source.total, normalizedItems.length, 0),
    page: asInteger(source.page, 1, 1),
    pageSize: asInteger(source.pageSize, normalizedItems.length || 20, 1),
  };
}

export function attachUsagesToAccounts(
  accounts: Account[],
  usages: AccountUsage[]
): Account[] {
  const usageMap = buildUsageMap(usages);
  return accounts.map((account) => normalizeAccount(account, usageMap.get(account.id)) || account);
}

export function normalizeModelOptions(payload: unknown): ModelOption[] {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload);
  return items
    .map((item) => {
      const current = asObject(item);
      const slug = asString(current.slug);
      if (!slug) return null;
      return {
        slug,
        displayName: asString(current.displayName ?? current.display_name) || slug,
      };
    })
    .filter((item): item is ModelOption => Boolean(item));
}

export function normalizeApiKey(item: unknown): ApiKey | null {
  const source = asObject(item);
  const id = asString(source.id);
  if (!id) return null;

  return {
    id,
    name: asString(source.name) || "未命名",
    model: asString(source.modelSlug ?? source.model_slug),
    modelSlug: asString(source.modelSlug ?? source.model_slug),
    reasoningEffort: asString(source.reasoningEffort ?? source.reasoning_effort),
    protocol: asString(source.protocolType ?? source.protocol_type) || "openai_compat",
    clientType: asString(source.clientType ?? source.client_type),
    authScheme: asString(source.authScheme ?? source.auth_scheme),
    upstreamBaseUrl: asString(source.upstreamBaseUrl ?? source.upstream_base_url),
    staticHeadersJson: asString(source.staticHeadersJson ?? source.static_headers_json),
    status: asString(source.status) || "enabled",
    createdAt: toNullableNumber(source.createdAt ?? source.created_at),
    lastUsedAt: toNullableNumber(source.lastUsedAt ?? source.last_used_at),
    expiresAt: toNullableNumber(source.expiresAt ?? source.expires_at),
  };
}

export function normalizeApiKeyList(payload: unknown): ApiKey[] {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload);
  return items
    .map((item) => normalizeApiKey(item))
    .filter((item): item is ApiKey => Boolean(item));
}

export function normalizeApiKeyCreateResult(payload: unknown): ApiKeyCreateResult {
  const source = asObject(payload);
  return {
    id: asString(source.id),
    key: asString(source.key),
  };
}

export function normalizeApiKeyUsageStats(payload: unknown): ApiKeyUsageStat[] {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload);
  return items
    .map((item) => {
      const current = asObject(item);
      const keyId = asString(current.keyId ?? current.key_id);
      if (!keyId) return null;
      return {
        keyId,
        totalTokens: asInteger(current.totalTokens ?? current.total_tokens, 0, 0),
      };
    })
    .filter((item): item is ApiKeyUsageStat => Boolean(item));
}

export function normalizeApiKeyRateLimit(payload: unknown): ApiKeyRateLimit {
  const source = asObject(payload);
  return {
    keyId: asString(source.keyId ?? source.key_id),
    rpm: toNullableNumber(source.rpm),
    tpm: toNullableNumber(source.tpm),
    dailyLimit: toNullableNumber(source.dailyLimit ?? source.daily_limit),
  };
}

export function normalizeApiKeyModelFallback(payload: unknown): ApiKeyModelFallback {
  const source = asObject(payload);
  return {
    keyId: asString(source.keyId ?? source.key_id),
    modelChain: asArray(source.modelChain ?? source.model_chain)
      .map((value) => asString(value))
      .filter((value) => value.length > 0),
  };
}

export function normalizeApiKeyAllowedModels(
  payload: unknown
): ApiKeyAllowedModelsConfig {
  const source = asObject(payload);
  return {
    keyId: asString(source.keyId ?? source.key_id),
    allowedModels: asArray(source.allowedModels ?? source.allowed_models)
      .map((value) => asString(value))
      .filter((value) => value.length > 0),
  };
}

export function normalizeApiKeyResponseCache(payload: unknown): ApiKeyResponseCacheConfig {
  const source = asObject(payload);
  return {
    keyId: asString(source.keyId ?? source.key_id),
    enabled: asBoolean(source.enabled, false),
  };
}

export function normalizeDeviceAuthInfo(payload: unknown): DeviceAuthInfo | null {
  const source = asObject(payload);
  const verificationUrl = asString(source.verificationUrl ?? source.verification_url);
  if (!verificationUrl) return null;

  return {
    userCodeUrl: asString(source.userCodeUrl ?? source.user_code_url),
    tokenUrl: asString(source.tokenUrl ?? source.token_url),
    verificationUrl,
    redirectUri: asString(source.redirectUri ?? source.redirect_uri),
  };
}

export function normalizeLoginStartResult(payload: unknown): LoginStartResult {
  const source = asObject(payload);
  return {
    authUrl: asString(source.authUrl ?? source.auth_url),
    loginId: asString(source.loginId ?? source.login_id),
    loginType: asString(source.loginType ?? source.login_type),
    issuer: asString(source.issuer),
    clientId: asString(source.clientId ?? source.client_id),
    redirectUri: asString(source.redirectUri ?? source.redirect_uri),
    warning: asString(source.warning),
    device: normalizeDeviceAuthInfo(source.device),
  };
}

export function normalizeRequestLog(item: unknown): RequestLog | null {
  const source = asObject(item);
  const createdAt = toNullableNumber(source.createdAt ?? source.created_at);
  const traceId = asString(source.traceId ?? source.trace_id);
  const keyId = asString(source.keyId ?? source.key_id);
  const accountId = asString(source.accountId ?? source.account_id);
  const requestPath = asString(source.requestPath ?? source.request_path);
  const method = asString(source.method);
  const id = traceId || [createdAt ?? "", method, requestPath, accountId, keyId].join("|");
  if (!id) return null;
  const durationMs = toNullableNumber(
    source.durationMs ??
      source.duration_ms ??
      source.latencyMs ??
      source.latency_ms ??
      source.elapsedMs ??
      source.elapsed_ms ??
      source.responseTimeMs ??
      source.response_time_ms
  );

  return {
    id,
    traceId,
    keyId,
    accountId,
    initialAccountId: asString(source.initialAccountId ?? source.initial_account_id),
    attemptedAccountIds: asArray(source.attemptedAccountIds ?? source.attempted_account_ids)
      .map((value) => asString(value))
      .filter((value) => value.length > 0),
    routeStrategy: asString(source.routeStrategy ?? source.route_strategy),
    requestedModel: asString(source.requestedModel ?? source.requested_model),
    modelFallbackPath: asArray(source.modelFallbackPath ?? source.model_fallback_path)
      .map((value) => asString(value))
      .filter((value) => value.length > 0),
    requestPath,
    originalPath: asString(source.originalPath ?? source.original_path),
    adaptedPath: asString(source.adaptedPath ?? source.adapted_path),
    method,
    path: requestPath,
    model: asString(source.model),
    reasoningEffort: asString(source.reasoningEffort ?? source.reasoning_effort),
    responseAdapter: asString(source.responseAdapter ?? source.response_adapter),
    upstreamUrl: asString(source.upstreamUrl ?? source.upstream_url),
    statusCode: toNullableNumber(source.statusCode ?? source.status_code),
    inputTokens: toNullableNumber(source.inputTokens ?? source.input_tokens),
    cachedInputTokens: toNullableNumber(
      source.cachedInputTokens ?? source.cached_input_tokens
    ),
    outputTokens: toNullableNumber(source.outputTokens ?? source.output_tokens),
    totalTokens: toNullableNumber(source.totalTokens ?? source.total_tokens),
    reasoningOutputTokens: toNullableNumber(
      source.reasoningOutputTokens ?? source.reasoning_output_tokens
    ),
    estimatedCostUsd: toNullableNumber(
      source.estimatedCostUsd ?? source.estimated_cost_usd
    ),
    durationMs,
    error: asString(source.error),
    createdAt,
  };
}

export function normalizeModelPricingList(payload: unknown): ModelPricingItem[] {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload);
  return items
    .map((item) => {
      const entry = asObject(item);
      const modelSlug = asString(entry.modelSlug ?? entry.model_slug);
      if (!modelSlug) return null;
      return {
        modelSlug,
        inputPricePer1k: toNullableNumber(
          entry.inputPricePer1k ?? entry.input_price_per_1k
        ) ?? 0,
        outputPricePer1k: toNullableNumber(
          entry.outputPricePer1k ?? entry.output_price_per_1k
        ) ?? 0,
        updatedAt: toNullableNumber(entry.updatedAt ?? entry.updated_at),
      };
    })
    .filter((item): item is ModelPricingItem => Boolean(item));
}

function normalizeCostUsageSummary(payload: unknown): CostUsageSummary {
  const source = asObject(payload);
  return {
    requestCount: asInteger(source.requestCount ?? source.request_count, 0, 0),
    inputTokens: asInteger(source.inputTokens ?? source.input_tokens, 0, 0),
    cachedInputTokens: asInteger(
      source.cachedInputTokens ?? source.cached_input_tokens,
      0,
      0
    ),
    outputTokens: asInteger(source.outputTokens ?? source.output_tokens, 0, 0),
    totalTokens: asInteger(source.totalTokens ?? source.total_tokens, 0, 0),
    estimatedCostUsd:
      toNullableNumber(source.estimatedCostUsd ?? source.estimated_cost_usd) ?? 0,
  };
}

export function normalizeCostSummary(payload: unknown): CostSummaryResult {
  const source = asObject(payload);
  const byKey = asArray(source.byKey ?? source.by_key)
    .map((item) => {
      const entry = asObject(item);
      const keyId = asString(entry.keyId ?? entry.key_id);
      if (!keyId) return null;
      return {
        keyId,
        ...normalizeCostUsageSummary(entry),
      };
    })
    .filter((item): item is CostSummaryKeyItem => Boolean(item))
    .sort(
      (left, right) =>
        right.estimatedCostUsd - left.estimatedCostUsd ||
        left.keyId.localeCompare(right.keyId)
    );
  const byModel = asArray(source.byModel ?? source.by_model)
    .map((item) => {
      const entry = asObject(item);
      const model = asString(entry.model);
      if (!model) return null;
      return {
        model,
        ...normalizeCostUsageSummary(entry),
      };
    })
    .filter((item): item is CostSummaryModelItem => Boolean(item))
    .sort(
      (left, right) =>
        right.estimatedCostUsd - left.estimatedCostUsd ||
        left.model.localeCompare(right.model)
    );
  const byDay = asArray(source.byDay ?? source.by_day)
    .map((item) => {
      const entry = asObject(item);
      const day = asString(entry.day);
      if (!day) return null;
      return {
        day,
        ...normalizeCostUsageSummary(entry),
      };
    })
    .filter((item): item is CostSummaryDayItem => Boolean(item));

  return {
    preset: asString(source.preset) || "today",
    rangeStart: asInteger(source.rangeStart ?? source.range_start, 0, 0),
    rangeEnd: asInteger(source.rangeEnd ?? source.range_end, 0, 0),
    total: normalizeCostUsageSummary(source.total),
    byKey,
    byModel,
    byDay,
  };
}

export function normalizeCostExportResult(payload: unknown): CostExportResult {
  const source = asObject(payload);
  return {
    fileName:
      asString(source.fileName ?? source.file_name) || "codexmanager-costs.csv",
    content: asString(source.content),
  };
}

export function normalizeRequestTrends(payload: unknown): RequestTrendResult {
  const source = asObject(payload);
  const items = asArray(source.items)
    .map((item) => {
      const entry = asObject(item);
      const bucket = asString(entry.bucket);
      if (!bucket) return null;
      return {
        bucket,
        requestCount: asInteger(entry.requestCount ?? entry.request_count, 0, 0),
        successCount: asInteger(entry.successCount ?? entry.success_count, 0, 0),
        successRate:
          toNullableNumber(entry.successRate ?? entry.success_rate) ?? 0,
      };
    })
    .filter((item): item is RequestTrendItem => Boolean(item));

  return {
    preset: asString(source.preset) || "30d",
    granularity: asString(source.granularity) || "day",
    rangeStart: asInteger(source.rangeStart ?? source.range_start, 0, 0),
    rangeEnd: asInteger(source.rangeEnd ?? source.range_end, 0, 0),
    items,
  };
}

export function normalizeModelTrends(payload: unknown): ModelTrendResult {
  const source = asObject(payload);
  const items = asArray(source.items)
    .map((item) => {
      const entry = asObject(item);
      const model = asString(entry.model);
      if (!model) return null;
      return {
        model,
        requestCount: asInteger(entry.requestCount ?? entry.request_count, 0, 0),
        successCount: asInteger(entry.successCount ?? entry.success_count, 0, 0),
        successRate:
          toNullableNumber(entry.successRate ?? entry.success_rate) ?? 0,
      };
    })
    .filter((item): item is ModelTrendItem => Boolean(item));

  return {
    preset: asString(source.preset) || "30d",
    rangeStart: asInteger(source.rangeStart ?? source.range_start, 0, 0),
    rangeEnd: asInteger(source.rangeEnd ?? source.range_end, 0, 0),
    items,
  };
}

export function normalizeHeatmapTrends(payload: unknown): HeatmapTrendResult {
  const source = asObject(payload);
  const items = asArray(source.items)
    .map((item) => {
      const entry = asObject(item);
      return {
        weekday: asInteger(entry.weekday, 0, 0),
        hour: asInteger(entry.hour, 0, 0),
        requestCount: asInteger(entry.requestCount ?? entry.request_count, 0, 0),
        successCount: asInteger(entry.successCount ?? entry.success_count, 0, 0),
        successRate:
          toNullableNumber(entry.successRate ?? entry.success_rate) ?? 0,
      };
    })
    .filter((item): item is HeatmapCellItem => item.hour >= 0 && item.hour <= 23);

  return {
    preset: asString(source.preset) || "30d",
    rangeStart: asInteger(source.rangeStart ?? source.range_start, 0, 0),
    rangeEnd: asInteger(source.rangeEnd ?? source.range_end, 0, 0),
    items,
  };
}

export function normalizeRequestLogs(payload: unknown): RequestLog[] {
  const source = asObject(payload);
  const items = asArray(source.items ?? payload);
  return items
    .map((item) => normalizeRequestLog(item))
    .filter((item): item is RequestLog => Boolean(item));
}

export function normalizeRequestLogListResult(payload: unknown): RequestLogListResult {
  const source = asObject(payload);
  const items = normalizeRequestLogs(source.items ?? payload);
  return {
    items,
    total: asInteger(source.total, items.length, 0),
    page: asInteger(source.page, 1, 1),
    pageSize: asInteger(source.pageSize, items.length || 20, 1),
  };
}

export function normalizeRequestLogExportResult(payload: unknown): RequestLogExportResult {
  const source = asObject(payload);
  return {
    format: asString(source.format) || "csv",
    fileName: asString(source.fileName ?? source.file_name) || "codexmanager-requestlogs.csv",
    content: asString(source.content),
    recordCount: asInteger(source.recordCount ?? source.record_count, 0, 0),
  };
}

export function normalizeRequestLogFilterSummary(
  payload: unknown
): RequestLogFilterSummary {
  const source = asObject(payload);
  return {
    totalCount: asInteger(source.totalCount, 0, 0),
    filteredCount: asInteger(source.filteredCount, 0, 0),
    successCount: asInteger(source.successCount, 0, 0),
    errorCount: asInteger(source.errorCount, 0, 0),
    totalTokens: asInteger(source.totalTokens, 0, 0),
  };
}

export function normalizeBackgroundTasks(payload: unknown): BackgroundTaskSettings {
  const source = asObject(payload);
  return {
    usagePollingEnabled: asBoolean(
      source.usagePollingEnabled,
      DEFAULT_BACKGROUND_TASKS.usagePollingEnabled
    ),
    usagePollIntervalSecs: asInteger(
      source.usagePollIntervalSecs,
      DEFAULT_BACKGROUND_TASKS.usagePollIntervalSecs,
      1
    ),
    gatewayKeepaliveEnabled: asBoolean(
      source.gatewayKeepaliveEnabled,
      DEFAULT_BACKGROUND_TASKS.gatewayKeepaliveEnabled
    ),
    gatewayKeepaliveIntervalSecs: asInteger(
      source.gatewayKeepaliveIntervalSecs,
      DEFAULT_BACKGROUND_TASKS.gatewayKeepaliveIntervalSecs,
      1
    ),
    tokenRefreshPollingEnabled: asBoolean(
      source.tokenRefreshPollingEnabled,
      DEFAULT_BACKGROUND_TASKS.tokenRefreshPollingEnabled
    ),
    tokenRefreshPollIntervalSecs: asInteger(
      source.tokenRefreshPollIntervalSecs,
      DEFAULT_BACKGROUND_TASKS.tokenRefreshPollIntervalSecs,
      1
    ),
    sessionProbePollingEnabled: asBoolean(
      source.sessionProbePollingEnabled,
      DEFAULT_BACKGROUND_TASKS.sessionProbePollingEnabled
    ),
    sessionProbeIntervalSecs: asInteger(
      source.sessionProbeIntervalSecs,
      DEFAULT_BACKGROUND_TASKS.sessionProbeIntervalSecs,
      1
    ),
    sessionProbeSampleSize: asInteger(
      source.sessionProbeSampleSize,
      DEFAULT_BACKGROUND_TASKS.sessionProbeSampleSize,
      1
    ),
    usageRefreshWorkers: asInteger(
      source.usageRefreshWorkers,
      DEFAULT_BACKGROUND_TASKS.usageRefreshWorkers,
      1
    ),
    httpWorkerFactor: asInteger(
      source.httpWorkerFactor,
      DEFAULT_BACKGROUND_TASKS.httpWorkerFactor,
      1
    ),
    httpWorkerMin: asInteger(
      source.httpWorkerMin,
      DEFAULT_BACKGROUND_TASKS.httpWorkerMin,
      1
    ),
    httpStreamWorkerFactor: asInteger(
      source.httpStreamWorkerFactor,
      DEFAULT_BACKGROUND_TASKS.httpStreamWorkerFactor,
      1
    ),
    httpStreamWorkerMin: asInteger(
      source.httpStreamWorkerMin,
      DEFAULT_BACKGROUND_TASKS.httpStreamWorkerMin,
      1
    ),
    autoRegisterPoolEnabled: asBoolean(
      source.autoRegisterPoolEnabled,
      DEFAULT_BACKGROUND_TASKS.autoRegisterPoolEnabled
    ),
    autoRegisterReadyAccountCount: asInteger(
      source.autoRegisterReadyAccountCount,
      DEFAULT_BACKGROUND_TASKS.autoRegisterReadyAccountCount,
      1
    ),
    autoRegisterReadyRemainPercent: asInteger(
      source.autoRegisterReadyRemainPercent,
      DEFAULT_BACKGROUND_TASKS.autoRegisterReadyRemainPercent,
      0
    ),
    autoDisableRiskyAccountsEnabled: asBoolean(
      source.autoDisableRiskyAccountsEnabled,
      DEFAULT_BACKGROUND_TASKS.autoDisableRiskyAccountsEnabled
    ),
    autoDisableRiskyAccountsFailureThreshold: asInteger(
      source.autoDisableRiskyAccountsFailureThreshold,
      DEFAULT_BACKGROUND_TASKS.autoDisableRiskyAccountsFailureThreshold,
      1
    ),
    autoDisableRiskyAccountsHealthScoreThreshold: asInteger(
      source.autoDisableRiskyAccountsHealthScoreThreshold,
      DEFAULT_BACKGROUND_TASKS.autoDisableRiskyAccountsHealthScoreThreshold,
      1
    ),
    autoDisableRiskyAccountsLookbackMins: asInteger(
      source.autoDisableRiskyAccountsLookbackMins,
      DEFAULT_BACKGROUND_TASKS.autoDisableRiskyAccountsLookbackMins,
      1
    ),
    accountCooldownAuthSecs: asInteger(
      source.accountCooldownAuthSecs,
      DEFAULT_BACKGROUND_TASKS.accountCooldownAuthSecs,
      0
    ),
    accountCooldownRateLimitedSecs: asInteger(
      source.accountCooldownRateLimitedSecs,
      DEFAULT_BACKGROUND_TASKS.accountCooldownRateLimitedSecs,
      0
    ),
    accountCooldownServerErrorSecs: asInteger(
      source.accountCooldownServerErrorSecs,
      DEFAULT_BACKGROUND_TASKS.accountCooldownServerErrorSecs,
      0
    ),
    accountCooldownNetworkSecs: asInteger(
      source.accountCooldownNetworkSecs,
      DEFAULT_BACKGROUND_TASKS.accountCooldownNetworkSecs,
      0
    ),
    accountCooldownLowQuotaSecs: asInteger(
      source.accountCooldownLowQuotaSecs,
      DEFAULT_BACKGROUND_TASKS.accountCooldownLowQuotaSecs,
      0
    ),
    accountCooldownDeactivatedSecs: asInteger(
      source.accountCooldownDeactivatedSecs,
      DEFAULT_BACKGROUND_TASKS.accountCooldownDeactivatedSecs,
      0
    ),
  };
}

export function normalizeEnvOverrideCatalog(payload: unknown): EnvOverrideCatalogItem[] {
  return asArray(payload).reduce<EnvOverrideCatalogItem[]>((result, item) => {
    const source = asObject(item);
    const key = asString(source.key);
    if (!key) return result;
    result.push({
      key,
      label: asString(source.label) || key,
      defaultValue: asString(source.defaultValue ?? source.default_value),
      scope: asString(source.scope),
      applyMode: asString(source.applyMode ?? source.apply_mode),
    });
    return result;
  }, []);
}

export function normalizeAppSettings(payload: unknown): AppSettings {
  const source = asObject(payload);
  const camelVisibleMenuItems = asArray(source.visibleMenuItems)
    .map((item) => asString(item))
    .filter(Boolean);
  const snakeVisibleMenuItems = asArray(source.visible_menu_items)
    .map((item) => asString(item))
    .filter(Boolean);
  return {
    updateAutoCheck: asBoolean(source.updateAutoCheck, true),
    closeToTrayOnClose: asBoolean(source.closeToTrayOnClose, false),
    closeToTraySupported: asBoolean(source.closeToTraySupported, false),
    lowTransparency: asBoolean(source.lowTransparency, false),
    lightweightModeOnCloseToTray: asBoolean(
      source.lightweightModeOnCloseToTray,
      false
    ),
    webAccessPasswordConfigured: asBoolean(
      source.webAccessPasswordConfigured,
      false
    ),
    webAccessTwoFactorEnabled: asBoolean(
      source.webAccessTwoFactorEnabled,
      false
    ),
    webAccessRecoveryCodesRemaining: asInteger(
      source.webAccessRecoveryCodesRemaining,
      0,
      0
    ),
    remoteManagementEnabled: asBoolean(source.remoteManagementEnabled, false),
    remoteManagementSecretConfigured: asBoolean(
      source.remoteManagementSecretConfigured,
      false
    ),
    serviceAddr: asString(source.serviceAddr) || "localhost:48760",
    serviceListenMode: asString(source.serviceListenMode) || "loopback",
    serviceListenModeOptions: asArray(source.serviceListenModeOptions).map((item) =>
      asString(item)
    ),
    mcpEnabled: asBoolean(source.mcpEnabled, true),
    mcpPort: asInteger(source.mcpPort, 48762, 1),
    routeStrategy: asString(source.routeStrategy) || "ordered",
    routeStrategyOptions: asArray(source.routeStrategyOptions).map((item) =>
      asString(item)
    ),
    freeAccountMaxModel: asString(source.freeAccountMaxModel) || "auto",
    freeAccountMaxModelOptions: asArray(source.freeAccountMaxModelOptions).map((item) =>
      asString(item)
    ),
    newAccountProtectionDays: asInteger(source.newAccountProtectionDays, 3, 0),
    quotaProtectionEnabled: asBoolean(source.quotaProtectionEnabled, false),
    quotaProtectionThresholdPercent: asInteger(
      source.quotaProtectionThresholdPercent,
      10,
      0
    ),
    requestCompressionEnabled: asBoolean(source.requestCompressionEnabled, true),
    payloadRewriteRulesJson: asString(source.payloadRewriteRulesJson) || "[]",
    modelAliasPoolsJson: asString(source.modelAliasPoolsJson) || "[]",
    retryPolicyMaxRetries: asInteger(source.retryPolicyMaxRetries, 3, 0),
    retryPolicyBackoffStrategy:
      asString(source.retryPolicyBackoffStrategy) || "exponential",
    retryPolicyRetryableStatusCodes: asArray(
      source.retryPolicyRetryableStatusCodes
    )
      .map((item) => asInteger(item, -1, -1))
      .filter((item) => item >= 100 && item <= 599),
    responseCacheEnabled: asBoolean(source.responseCacheEnabled, false),
    responseCacheTtlSecs: asInteger(source.responseCacheTtlSecs, 3600, 1),
    responseCacheMaxEntries: asInteger(source.responseCacheMaxEntries, 256, 1),
    gatewayOriginator: asString(source.gatewayOriginator) || "codex_cli_rs",
    gatewayResidencyRequirement: asString(source.gatewayResidencyRequirement),
    gatewayResidencyRequirementOptions: asArray(
      source.gatewayResidencyRequirementOptions
    ).map((item) => asString(item)),
    cpaNoCookieHeaderModeEnabled: asBoolean(
      source.cpaNoCookieHeaderModeEnabled,
      false
    ),
    upstreamProxyUrl: asString(source.upstreamProxyUrl),
    upstreamStreamTimeoutMs: asInteger(source.upstreamStreamTimeoutMs, 1_800_000, 0),
    sseKeepaliveIntervalMs: asInteger(source.sseKeepaliveIntervalMs, 15_000, 1),
    teamManagerEnabled: asBoolean(source.teamManagerEnabled, false),
    teamManagerApiUrl: asString(source.teamManagerApiUrl),
    teamManagerHasApiKey: asBoolean(source.teamManagerHasApiKey, false),
    backgroundTasks: normalizeBackgroundTasks(source.backgroundTasks),
    envOverrides: normalizeStringRecord(source.envOverrides),
    envOverrideCatalog: normalizeEnvOverrideCatalog(source.envOverrideCatalog),
    envOverrideReservedKeys: asArray(source.envOverrideReservedKeys).map((item) =>
      asString(item)
    ),
    envOverrideUnsupportedKeys: asArray(source.envOverrideUnsupportedKeys).map((item) =>
      asString(item)
    ),
    theme: asString(source.theme) || "tech",
    appearancePreset: asString(source.appearancePreset) || "classic",
    visibleMenuItems: normalizeVisibleMenuItems(
      camelVisibleMenuItems.length > 0
        ? camelVisibleMenuItems
        : snakeVisibleMenuItems.length > 0
          ? snakeVisibleMenuItems
          : APP_NAV_DEFAULT_VISIBLE_IDS
    ),
  };
}

export function normalizeWebAuthTwoFactorSetupResult(
  payload: unknown
): WebAuthTwoFactorSetupResult {
  const source = asObject(payload);
  return {
    enabled: asBoolean(source.enabled, false),
    secret: asString(source.secret),
    otpAuthUrl: asString(source.otpAuthUrl ?? source.otp_auth_url),
    qrCodeDataUrl: asString(source.qrCodeDataUrl ?? source.qr_code_data_url),
    recoveryCodes: asArray(source.recoveryCodes ?? source.recovery_codes).map((item) =>
      asString(item)
    ),
    setupToken: asString(source.setupToken ?? source.setup_token),
  };
}

export function normalizeWebAuthTwoFactorStatusResult(
  payload: unknown
): WebAuthTwoFactorStatusResult {
  const source = asObject(payload);
  return {
    enabled: asBoolean(source.enabled, false),
    recoveryCodesRemaining: asInteger(source.recoveryCodesRemaining, 0, 0),
    method: asString(source.method),
  };
}

export function normalizeGatewayRetryPolicy(payload: unknown) {
  const source = asObject(payload);
  return {
    maxRetries: asInteger(source.maxRetries, 3, 0),
    backoffStrategy: asString(source.backoffStrategy) || "exponential",
    retryableStatusCodes: asArray(source.retryableStatusCodes)
      .map((item) => asInteger(item, -1, -1))
      .filter((item) => item >= 100 && item <= 599),
  };
}

export function normalizeGatewayResponseCacheStats(
  payload: unknown
): GatewayResponseCacheStats {
  const source = asObject(payload);
  return {
    enabled: asBoolean(source.enabled, false),
    ttlSecs: asInteger(source.ttlSecs ?? source.ttl_secs, 3600, 1),
    maxEntries: asInteger(source.maxEntries ?? source.max_entries, 256, 1),
    entryCount: asInteger(source.entryCount ?? source.entry_count, 0, 0),
    estimatedBytes: asInteger(source.estimatedBytes ?? source.estimated_bytes, 0, 0),
    hitCount: asInteger(source.hitCount ?? source.hit_count, 0, 0),
    missCount: asInteger(source.missCount ?? source.miss_count, 0, 0),
    hitRatePercent: Math.max(
      0,
      toNullableNumber(source.hitRatePercent ?? source.hit_rate_percent) ?? 0
    ),
  };
}

export function normalizeFreeProxySyncResult(payload: unknown): FreeProxySyncResult {
  const source = asObject(payload);
  return {
    sourceUrl: asString(source.sourceUrl),
    sourceUpdatedAt: asString(source.sourceUpdatedAt) || null,
    fetchedCount: asInteger(source.fetchedCount, 0, 0),
    matchedCount: asInteger(source.matchedCount, 0, 0),
    appliedCount: asInteger(source.appliedCount, 0, 0),
    protocol: asString(source.protocol),
    anonymity: asString(source.anonymity),
    countryFilter: asArray(source.countryFilter).map((item) => asString(item)).filter(Boolean),
    limit: asInteger(source.limit, 0, 0),
    clearedUpstreamProxyUrl: asBoolean(source.clearedUpstreamProxyUrl, false),
    singleProxyStillConfigured: asBoolean(source.singleProxyStillConfigured, false),
    previousUpstreamProxyUrl: asString(source.previousUpstreamProxyUrl) || null,
    proxyListValue: asString(source.proxyListValue),
    proxies: asArray(source.proxies).map((item) => asString(item)).filter(Boolean),
    registerProxySyncEnabled: asBoolean(source.registerProxySyncEnabled, false),
    registerProxyCreatedCount: asInteger(source.registerProxyCreatedCount, 0, 0),
    registerProxyUpdatedCount: asInteger(source.registerProxyUpdatedCount, 0, 0),
    registerProxyTotalCount: asInteger(source.registerProxyTotalCount, 0, 0),
  };
}

export function normalizeStartupSnapshot(payload: unknown): StartupSnapshot {
  const source = asObject(payload);
  const usageSnapshots = normalizeUsageList(source.usageSnapshots);
  const usageMap = buildUsageMap(usageSnapshots);
  const accounts = asArray(source.accounts)
    .map((item) => normalizeAccount(item, usageMap.get(asString(asObject(item).id))))
    .filter((item): item is Account => Boolean(item));

  return {
    accounts,
    usageSnapshots,
    usageAggregateSummary: normalizeUsageAggregateSummary(source.usageAggregateSummary),
    usagePredictionSummary: normalizeUsagePredictionSummary(
      source.usagePredictionSummary ?? source.usage_prediction_summary
    ),
    failureReasonSummary: normalizeFailureReasonSummary(
      source.failureReasonSummary ?? source.failure_reason_summary
    ),
    governanceSummary: normalizeGovernanceSummary(
      source.governanceSummary ?? source.governance_summary
    ),
    operationAudits: normalizeOperationAudits(
      source.operationAudits ?? source.operation_audits
    ),
    apiKeys: normalizeApiKeyList(source.apiKeys),
    apiModelOptions: normalizeModelOptions(source.apiModelOptions),
    manualPreferredAccountId: asString(source.manualPreferredAccountId),
    requestLogTodaySummary: normalizeTodaySummary(source.requestLogTodaySummary),
    requestLogs: normalizeRequestLogs(source.requestLogs),
  };
}
