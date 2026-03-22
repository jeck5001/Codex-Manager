import { invoke, withAddr } from "./transport";
import {
  normalizeAppSettings,
  normalizeDashboardHealth,
  normalizeDashboardTrend,
  normalizeFreeProxySyncResult,
  normalizeGatewayResponseCacheStats,
  normalizeCostExportResult,
  normalizeCostSummary,
  normalizeModelPricingList,
  normalizeRequestLogExportResult,
  normalizeRequestLogFilterSummary,
  normalizeRequestLogListResult,
  normalizeStartupSnapshot,
  normalizeTodaySummary,
} from "./normalize";
import {
  BackgroundTaskSettings,
  CostExportResult,
  CostSummaryResult,
  DashboardHealth,
  DashboardTrend,
  FreeProxySyncResult,
  GatewayResponseCacheStats,
  ModelPricingItem,
  RequestLogExportResult,
  RequestLogFilterSummary,
  RequestLogListResult,
  RequestLogTodaySummary,
  ServiceInitializationResult,
  StartupSnapshot,
} from "../../types";
import { readInitializeResult } from "@/lib/utils/service";

function readStringField(payload: unknown, key: string): string {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return "";
  }
  const value = (payload as Record<string, unknown>)[key];
  return typeof value === "string" ? value.trim() : "";
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
  getRouteStrategy: () =>
    invoke<string>("service_gateway_route_strategy_get", withAddr()),
  setRouteStrategy: (strategy: string) =>
    invoke("service_gateway_route_strategy_set", withAddr({ strategy })),
  async getManualPreferredAccountId(): Promise<string> {
    const result = await invoke<unknown>("service_gateway_manual_account_get", withAddr());
    return readStringField(result, "accountId");
  },
  setManualPreferredAccount: (accountId: string) =>
    invoke("service_gateway_manual_account_set", withAddr({ accountId })),
  clearManualPreferredAccount: () =>
    invoke("service_gateway_manual_account_clear", withAddr()),
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
  async getGatewayCacheStats(): Promise<GatewayResponseCacheStats> {
    const result = await invoke<unknown>("service_gateway_cache_stats", withAddr());
    return normalizeGatewayResponseCacheStats(result);
  },
  clearGatewayCache: () => invoke("service_gateway_cache_clear", withAddr()),

  async listRequestLogs(params?: {
    query?: string;
    statusFilter?: string;
    keyId?: string;
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

  getListenConfig: () => invoke<unknown>("service_listen_config_get", withAddr()),
  setListenConfig: (mode: string) =>
    invoke("service_listen_config_set", withAddr({ mode })),

  getEnvOverrides: async () => {
    const result = await invoke<unknown>("app_settings_get");
    return normalizeAppSettings(result).envOverrides;
  },
};
