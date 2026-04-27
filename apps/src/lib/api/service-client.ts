import { invoke, withAddr } from "./transport";
import {
  normalizeAuditLogExportResult,
  normalizeAuditLogListResult,
  normalizeAlertChannel,
  normalizeAlertChannelList,
  normalizeAlertChannelTestResult,
  normalizeAlertHistoryList,
  normalizeAlertRule,
  normalizeAlertRuleList,
  normalizeAppSettings,
  normalizeCacheAnalyticsByKey,
  normalizeCacheAnalyticsByModel,
  normalizeCacheAnalyticsSummary,
  normalizeCacheAnalyticsTrend,
  normalizeConsumerModelBreakdown,
  normalizeConsumerOverview,
  normalizeConsumerRanking,
  normalizeConsumerTrend,
  normalizeDashboardHealth,
  normalizeDashboardTrend,
  normalizeFreeProxyClearResult,
  normalizeFreeProxySyncResult,
  normalizeGatewayRouteStrategy,
  normalizeGatewayRetryPolicy,
  normalizeGatewayResponseCacheStats,
  normalizeHeatmapTrends,
  normalizeHealthcheckConfig,
  normalizeHealthcheckRun,
  normalizeCostExportResult,
  normalizeCostSummary,
  normalizeModelTrends,
  normalizeModelPricingList,
  normalizePlugin,
  normalizePluginList,
  normalizeRequestTrends,
  normalizeRequestLogExportResult,
  normalizeRequestLogFilterSummary,
  normalizeRequestLogListResult,
  normalizeStartupSnapshot,
  normalizeTodaySummary,
  normalizeWebAuthTwoFactorSetupResult,
  normalizeWebAuthTwoFactorStatusResult,
} from "./normalize";
import {
  AuditLogExportResult,
  AuditLogListResult,
  AlertChannel,
  AlertChannelTestResult,
  AlertHistoryItem,
  AlertRule,
  BackgroundTaskSettings,
  CacheAnalyticsByKeyResult,
  CacheAnalyticsByModelResult,
  CacheAnalyticsSummaryResult,
  CacheAnalyticsTrendResult,
  ConsumerModelBreakdownResult,
  ConsumerOverviewResult,
  ConsumerRankingResult,
  ConsumerTrendResult,
  CostExportResult,
  CostSummaryResult,
  DashboardHealth,
  DashboardTrend,
  FreeProxyClearResult,
  FreeProxySyncResult,
  GatewayRouteStrategyInfo,
  GatewayRetryPolicy,
  GatewayResponseCacheStats,
  HeatmapTrendResult,
  HealthcheckConfig,
  HealthcheckRunResult,
  ModelTrendResult,
  ModelPricingItem,
  PluginItem,
  RequestTrendResult,
  RequestLogExportResult,
  RequestLogFilterSummary,
  RequestLogListResult,
  RequestLogTodaySummary,
  ServiceInitializationResult,
  StartupSnapshot,
  WebAuthTwoFactorSetupResult,
  WebAuthTwoFactorStatusResult,
} from "../../types";
import { readInitializeResult } from "@/lib/utils/service";

function readStringArrayField(payload: unknown, key: string): string[] {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return [];
  }
  const value = (payload as Record<string, unknown>)[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .map((item) => (typeof item === "string" ? item.trim() : ""))
    .filter(Boolean);
}

export const serviceClient = {
  start: (addr?: string) => invoke("service_start", { addr }),
  stop: () => invoke("service_stop"),
  async initialize(addr?: string): Promise<ServiceInitializationResult> {
    const result = await invoke<unknown>(
      "service_initialize",
      addr ? { addr } : withAddr()
    );
    return readInitializeResult(result);
  },
  async getStartupSnapshot(
    params?: Record<string, unknown>
  ): Promise<StartupSnapshot> {
    const result = await invoke<unknown>(
      "service_startup_snapshot",
      withAddr(params)
    );
    return normalizeStartupSnapshot(result);
  },

  getGatewayTransport: () => invoke<unknown>("service_gateway_transport_get", withAddr()),
  setGatewayTransport: (settings: Record<string, unknown>) =>
    invoke("service_gateway_transport_set", withAddr(settings)),
  async getGatewayRetryPolicy(): Promise<GatewayRetryPolicy> {
    const result = await invoke<unknown>(
      "service_gateway_retry_policy_get",
      withAddr()
    );
    return normalizeGatewayRetryPolicy(result);
  },
  async setGatewayRetryPolicy(params: {
    maxRetries?: number;
    backoffStrategy?: string;
    retryableStatusCodes?: number[];
  }): Promise<GatewayRetryPolicy> {
    const result = await invoke<unknown>(
      "service_gateway_retry_policy_set",
      withAddr(params)
    );
    return normalizeGatewayRetryPolicy(result);
  },
  getUpstreamProxy: () =>
    invoke<string>("service_gateway_upstream_proxy_get", withAddr()),
  setUpstreamProxy: (proxyUrl: string) =>
    invoke("service_gateway_upstream_proxy_set", withAddr({ proxyUrl })),
  async syncFreeProxyPool(params?: {
    protocol?: string;
    anonymity?: string;
    country?: string;
    limit?: number;
    clearUpstreamProxyUrl?: boolean;
    syncRegisterProxyPool?: boolean;
    sourceUrl?: string;
  }): Promise<FreeProxySyncResult> {
    const result = await invoke<unknown>(
      "service_gateway_freeproxy_sync",
      withAddr(params ?? {})
    );
    return normalizeFreeProxySyncResult(result);
  },
  async clearFreeProxyPool(): Promise<FreeProxyClearResult> {
    const result = await invoke<unknown>(
      "service_gateway_freeproxy_clear",
      withAddr()
    );
    return normalizeFreeProxyClearResult(result);
  },
  async getRouteStrategy(): Promise<GatewayRouteStrategyInfo> {
    const result = await invoke<unknown>(
      "service_gateway_route_strategy_get",
      withAddr()
    );
    return normalizeGatewayRouteStrategy(result);
  },
  setRouteStrategy: (strategy: string) =>
    invoke("service_gateway_route_strategy_set", withAddr({ strategy })),
  async getRouteAccountIds(): Promise<string[]> {
    const result = await invoke<unknown>("service_gateway_route_accounts_get", withAddr());
    return readStringArrayField(result, "accountIds");
  },
  setRouteAccounts: (accountIds: string[]) =>
    invoke("service_gateway_route_accounts_set", withAddr({ accountIds })),
  clearRouteAccounts: () =>
    invoke("service_gateway_route_accounts_clear", withAddr()),
  async getManualPreferredAccountId(): Promise<string> {
    const accountIds = await serviceClient.getRouteAccountIds();
    return accountIds[0] ?? "";
  },
  setManualPreferredAccount: (accountId: string) =>
    serviceClient.setRouteAccounts(accountId ? [accountId] : []),
  clearManualPreferredAccount: () =>
    serviceClient.clearRouteAccounts(),
  getHeaderPolicy: () =>
    invoke<string>("service_gateway_header_policy_get", withAddr()),
  setHeaderPolicy: (cpaNoCookieHeaderModeEnabled: boolean) =>
    invoke(
      "service_gateway_header_policy_set",
      withAddr({ cpaNoCookieHeaderModeEnabled })
    ),

  getBackgroundTasks: () =>
    invoke<BackgroundTaskSettings>("service_gateway_background_tasks_get", withAddr()),
  setBackgroundTasks: (settings: BackgroundTaskSettings) =>
    invoke(
      "service_gateway_background_tasks_set",
      withAddr({ ...(settings as unknown as Record<string, unknown>) })
    ),
  async getHealthcheckConfig(): Promise<HealthcheckConfig> {
    const result = await invoke<unknown>("service_healthcheck_config_get", withAddr());
    return normalizeHealthcheckConfig(result);
  },
  async setHealthcheckConfig(params: {
    enabled?: boolean;
    intervalSecs?: number;
    sampleSize?: number;
  }): Promise<HealthcheckConfig> {
    const result = await invoke<unknown>(
      "service_healthcheck_config_set",
      withAddr(params)
    );
    return normalizeHealthcheckConfig(result);
  },
  async runHealthcheck(): Promise<HealthcheckRunResult> {
    const result = await invoke<unknown>("service_healthcheck_run", withAddr());
    return (
      normalizeHealthcheckRun(result) ?? {
        startedAt: null,
        finishedAt: null,
        totalAccounts: 0,
        sampledAccounts: 0,
        successCount: 0,
        failureCount: 0,
        failedAccounts: [],
      }
    );
  },
  async setupWebAuthTwoFactor(): Promise<WebAuthTwoFactorSetupResult> {
    const result = await invoke<unknown>(
      "service_web_auth_two_factor_setup",
      withAddr()
    );
    return normalizeWebAuthTwoFactorSetupResult(result);
  },
  async verifyWebAuthTwoFactor(params: {
    setupToken?: string;
    code?: string;
    recoveryCode?: string;
  }): Promise<WebAuthTwoFactorStatusResult> {
    const result = await invoke<unknown>(
      "service_web_auth_two_factor_verify",
      withAddr(params)
    );
    return normalizeWebAuthTwoFactorStatusResult(result);
  },
  async disableWebAuthTwoFactor(params: {
    code?: string;
    recoveryCode?: string;
  }): Promise<WebAuthTwoFactorStatusResult> {
    const result = await invoke<unknown>(
      "service_web_auth_two_factor_disable",
      withAddr(params)
    );
    return normalizeWebAuthTwoFactorStatusResult(result);
  },
  async getGatewayCacheStats(): Promise<GatewayResponseCacheStats> {
    const result = await invoke<unknown>("service_gateway_cache_stats", withAddr());
    return normalizeGatewayResponseCacheStats(result);
  },
  clearGatewayCache: () => invoke("service_gateway_cache_clear", withAddr()),
  async listAlertRules(): Promise<AlertRule[]> {
    const result = await invoke<unknown>("service_alert_rules_list", withAddr());
    return normalizeAlertRuleList(result);
  },
  async upsertAlertRule(params: {
    id?: string | null;
    name: string;
    type: string;
    config: Record<string, unknown>;
    enabled: boolean;
  }): Promise<AlertRule> {
    const result = await invoke<unknown>("service_alert_rules_upsert", withAddr(params));
    const item = normalizeAlertRule(result);
    if (!item) throw new Error("告警规则返回数据无效");
    return item;
  },
  deleteAlertRule: (id: string) =>
    invoke("service_alert_rules_delete", withAddr({ id })),
  async listAlertChannels(): Promise<AlertChannel[]> {
    const result = await invoke<unknown>("service_alert_channels_list", withAddr());
    return normalizeAlertChannelList(result);
  },
  async upsertAlertChannel(params: {
    id?: string | null;
    name: string;
    type: string;
    config: Record<string, unknown>;
    enabled: boolean;
  }): Promise<AlertChannel> {
    const result = await invoke<unknown>("service_alert_channels_upsert", withAddr(params));
    const item = normalizeAlertChannel(result);
    if (!item) throw new Error("告警渠道返回数据无效");
    return item;
  },
  deleteAlertChannel: (id: string) =>
    invoke("service_alert_channels_delete", withAddr({ id })),
  async testAlertChannel(id: string): Promise<AlertChannelTestResult> {
    const result = await invoke<unknown>("service_alert_channels_test", withAddr({ id }));
    return normalizeAlertChannelTestResult(result);
  },
  async listAlertHistory(limit = 50): Promise<AlertHistoryItem[]> {
    const result = await invoke<unknown>("service_alert_history_list", withAddr({ limit }));
    return normalizeAlertHistoryList(result);
  },
  async listPlugins(): Promise<PluginItem[]> {
    const result = await invoke<unknown>("service_plugin_list", withAddr());
    return normalizePluginList(result);
  },
  async upsertPlugin(params: {
    id?: string | null;
    name: string;
    description?: string | null;
    runtime?: string;
    hookPoints: string[];
    scriptContent: string;
    enabled: boolean;
    timeoutMs?: number;
  }): Promise<PluginItem> {
    const result = await invoke<unknown>("service_plugin_upsert", withAddr(params));
    const item = normalizePlugin(result);
    if (!item) throw new Error("插件返回数据无效");
    return item;
  },
  deletePlugin: (id: string) => invoke("service_plugin_delete", withAddr({ id })),
  async listAuditLogs(params?: {
    action?: string;
    objectType?: string;
    objectId?: string;
    timeFrom?: number | null;
    timeTo?: number | null;
    page?: number;
    pageSize?: number;
  }): Promise<AuditLogListResult> {
    const result = await invoke<unknown>(
      "service_audit_list",
      withAddr({
        action: params?.action || "",
        objectType: params?.objectType || "",
        objectId: params?.objectId || "",
        timeFrom: params?.timeFrom ?? null,
        timeTo: params?.timeTo ?? null,
        page: params?.page ?? 1,
        pageSize: params?.pageSize ?? 20,
      })
    );
    return normalizeAuditLogListResult(result);
  },
  async exportAuditLogs(params?: {
    format?: string;
    action?: string;
    objectType?: string;
    objectId?: string;
    timeFrom?: number | null;
    timeTo?: number | null;
  }): Promise<AuditLogExportResult> {
    const result = await invoke<unknown>(
      "service_audit_export",
      withAddr({
        format: params?.format || "csv",
        action: params?.action || "",
        objectType: params?.objectType || "",
        objectId: params?.objectId || "",
        timeFrom: params?.timeFrom ?? null,
        timeTo: params?.timeTo ?? null,
      })
    );
    return normalizeAuditLogExportResult(result);
  },
  async downloadAuditLogsViaHttp(params?: {
    format?: string;
    action?: string;
    objectType?: string;
    objectId?: string;
    timeFrom?: number | null;
    timeTo?: number | null;
  }): Promise<void> {
    if (typeof document === "undefined") {
      throw new Error("当前环境不支持浏览器导出");
    }

    const searchParams = new URLSearchParams();
    searchParams.set("format", params?.format || "csv");
    searchParams.set("action", params?.action || "");
    searchParams.set("objectType", params?.objectType || "");
    searchParams.set("objectId", params?.objectId || "");
    if (params?.timeFrom != null) {
      searchParams.set("timeFrom", String(params.timeFrom));
    }
    if (params?.timeTo != null) {
      searchParams.set("timeTo", String(params.timeTo));
    }

    const anchor = document.createElement("a");
    anchor.href = `/api/export/auditlogs?${searchParams.toString()}`;
    anchor.rel = "noopener";
    anchor.style.display = "none";
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  },

  async listRequestLogs(params?: {
    query?: string;
    statusFilter?: string;
    keyId?: string;
    keyIds?: string[];
    model?: string;
    timeFrom?: number | null;
    timeTo?: number | null;
    page?: number;
    pageSize?: number;
  }): Promise<RequestLogListResult> {
    const result = await invoke<unknown>(
      "service_requestlog_list",
      withAddr({
        query: params?.query || "",
        statusFilter: params?.statusFilter || "all",
        keyId: params?.keyId || "",
        keyIds: params?.keyIds ?? [],
        model: params?.model || "",
        timeFrom: params?.timeFrom ?? null,
        timeTo: params?.timeTo ?? null,
        page: params?.page ?? 1,
        pageSize: params?.pageSize ?? 20,
      })
    );
    return normalizeRequestLogListResult(result);
  },
  async getRequestLogSummary(params?: {
    query?: string;
    statusFilter?: string;
    keyId?: string;
    keyIds?: string[];
    model?: string;
    timeFrom?: number | null;
    timeTo?: number | null;
  }): Promise<RequestLogFilterSummary> {
    const result = await invoke<unknown>(
      "service_requestlog_summary",
      withAddr({
        query: params?.query || "",
        statusFilter: params?.statusFilter || "all",
        keyId: params?.keyId || "",
        keyIds: params?.keyIds ?? [],
        model: params?.model || "",
        timeFrom: params?.timeFrom ?? null,
        timeTo: params?.timeTo ?? null,
      })
    );
    return normalizeRequestLogFilterSummary(result);
  },
  async exportRequestLogs(params?: {
    format?: string;
    query?: string;
    statusFilter?: string;
    keyId?: string;
    keyIds?: string[];
    model?: string;
    timeFrom?: number | null;
    timeTo?: number | null;
  }): Promise<RequestLogExportResult> {
    const result = await invoke<unknown>(
      "service_requestlog_export",
      withAddr({
        format: params?.format || "csv",
        query: params?.query || "",
        statusFilter: params?.statusFilter || "all",
        keyId: params?.keyId || "",
        keyIds: params?.keyIds ?? [],
        model: params?.model || "",
        timeFrom: params?.timeFrom ?? null,
        timeTo: params?.timeTo ?? null,
      })
    );
    return normalizeRequestLogExportResult(result);
  },
  async downloadRequestLogsViaHttp(params?: {
    format?: string;
    query?: string;
    statusFilter?: string;
    keyId?: string;
    keyIds?: string[];
    model?: string;
    timeFrom?: number | null;
    timeTo?: number | null;
  }): Promise<void> {
    if (typeof document === "undefined") {
      throw new Error("当前环境不支持浏览器导出");
    }

    const searchParams = new URLSearchParams();
    searchParams.set("format", params?.format || "csv");
    searchParams.set("query", params?.query || "");
    searchParams.set("statusFilter", params?.statusFilter || "all");
    searchParams.set("keyId", params?.keyId || "");
    for (const keyId of params?.keyIds ?? []) {
      const normalized = String(keyId || "").trim();
      if (!normalized) continue;
      searchParams.append("keyIds", normalized);
    }
    searchParams.set("model", params?.model || "");
    if (params?.timeFrom != null) {
      searchParams.set("timeFrom", String(params.timeFrom));
    }
    if (params?.timeTo != null) {
      searchParams.set("timeTo", String(params.timeTo));
    }

    const anchor = document.createElement("a");
    anchor.href = `/api/export/requestlogs?${searchParams.toString()}`;
    anchor.rel = "noopener";
    anchor.style.display = "none";
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  },
  clearRequestLogs: () => invoke("service_requestlog_clear", withAddr()),
  async getTodaySummary(): Promise<RequestLogTodaySummary> {
    const result = await invoke<unknown>(
      "service_requestlog_today_summary",
      withAddr()
    );
    return normalizeTodaySummary(result);
  },
  async getDashboardHealth(): Promise<DashboardHealth> {
    const result = await invoke<unknown>("service_dashboard_health", withAddr());
    return normalizeDashboardHealth(result);
  },
  async getDashboardTrend(): Promise<DashboardTrend> {
    const result = await invoke<unknown>("service_dashboard_trend", withAddr());
    return normalizeDashboardTrend(result);
  },
  async getCostSummary(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<CostSummaryResult> {
    const result = await invoke<unknown>(
      "service_stats_cost_summary",
      withAddr({
        preset: params?.preset || "month",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeCostSummary(result);
  },
  async exportCostSummary(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<CostExportResult> {
    const result = await invoke<unknown>(
      "service_stats_cost_export",
      withAddr({
        preset: params?.preset || "month",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeCostExportResult(result);
  },
  async getCostModelPricing(): Promise<ModelPricingItem[]> {
    const result = await invoke<unknown>(
      "service_stats_cost_model_pricing_get",
      withAddr()
    );
    return normalizeModelPricingList(result);
  },
  setCostModelPricing: (items: ModelPricingItem[]) =>
    invoke("service_stats_cost_model_pricing_set", withAddr({ items })),
  async getRequestTrends(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
    granularity?: string;
  }): Promise<RequestTrendResult> {
    const result = await invoke<unknown>(
      "service_stats_trends_requests",
      withAddr({
        preset: params?.preset || "30d",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
        granularity: params?.granularity || "day",
      })
    );
    return normalizeRequestTrends(result);
  },
  async getModelTrends(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<ModelTrendResult> {
    const result = await invoke<unknown>(
      "service_stats_trends_models",
      withAddr({
        preset: params?.preset || "30d",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeModelTrends(result);
  },
  async getHeatmapTrends(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<HeatmapTrendResult> {
    const result = await invoke<unknown>(
      "service_stats_trends_heatmap",
      withAddr({
        preset: params?.preset || "30d",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeHeatmapTrends(result);
  },

  getListenConfig: () => invoke<unknown>("service_listen_config_get", withAddr()),
  setListenConfig: (mode: string) =>
    invoke("service_listen_config_set", withAddr({ mode })),

  getEnvOverrides: async () => {
    const result = await invoke<unknown>("app_settings_get");
    return normalizeAppSettings(result).envOverrides;
  },

  // -- Consumer Analytics --
  async getConsumerOverview(params: {
    keyId: string;
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<ConsumerOverviewResult> {
    const result = await invoke<unknown>(
      "service_stats_consumer_overview",
      withAddr({
        keyId: params.keyId,
        preset: params.preset || "month",
        startTs: params.startTs ?? null,
        endTs: params.endTs ?? null,
      })
    );
    return normalizeConsumerOverview(result);
  },
  async getConsumerTrend(params: {
    keyId: string;
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<ConsumerTrendResult> {
    const result = await invoke<unknown>(
      "service_stats_consumer_trend",
      withAddr({
        keyId: params.keyId,
        preset: params.preset || "month",
        startTs: params.startTs ?? null,
        endTs: params.endTs ?? null,
      })
    );
    return normalizeConsumerTrend(result);
  },
  async getConsumerModelBreakdown(params: {
    keyId: string;
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<ConsumerModelBreakdownResult> {
    const result = await invoke<unknown>(
      "service_stats_consumer_model_breakdown",
      withAddr({
        keyId: params.keyId,
        preset: params.preset || "month",
        startTs: params.startTs ?? null,
        endTs: params.endTs ?? null,
      })
    );
    return normalizeConsumerModelBreakdown(result);
  },
  async getConsumerRanking(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
    limit?: number;
  }): Promise<ConsumerRankingResult> {
    const result = await invoke<unknown>(
      "service_stats_consumer_ranking",
      withAddr({
        preset: params?.preset || "month",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
        limit: params?.limit ?? 20,
      })
    );
    return normalizeConsumerRanking(result);
  },

  // -- Cache Analytics --
  async getCacheAnalyticsSummary(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<CacheAnalyticsSummaryResult> {
    const result = await invoke<unknown>(
      "service_stats_cache_summary",
      withAddr({
        preset: params?.preset || "month",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeCacheAnalyticsSummary(result);
  },
  async getCacheAnalyticsTrend(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<CacheAnalyticsTrendResult> {
    const result = await invoke<unknown>(
      "service_stats_cache_trend",
      withAddr({
        preset: params?.preset || "month",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeCacheAnalyticsTrend(result);
  },
  async getCacheAnalyticsByModel(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<CacheAnalyticsByModelResult> {
    const result = await invoke<unknown>(
      "service_stats_cache_by_model",
      withAddr({
        preset: params?.preset || "month",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeCacheAnalyticsByModel(result);
  },
  async getCacheAnalyticsByKey(params?: {
    preset?: string;
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<CacheAnalyticsByKeyResult> {
    const result = await invoke<unknown>(
      "service_stats_cache_by_key",
      withAddr({
        preset: params?.preset || "month",
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      })
    );
    return normalizeCacheAnalyticsByKey(result);
  },
};
