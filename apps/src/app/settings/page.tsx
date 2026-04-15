"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useTheme } from "next-themes";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import { accountClient } from "@/lib/api/account-client";
import { appClient } from "@/lib/api/app-client";
import { serviceClient } from "@/lib/api/service-client";
import { getAppErrorMessage, isTauriRuntime } from "@/lib/api/transport";
import { useAppStore } from "@/lib/store/useAppStore";
import {
  APPEARANCE_PRESETS,
  applyAppearancePreset,
  normalizeAppearancePreset,
} from "@/lib/appearance";
import {
  AlertChannel,
  AlertRule,
  AccountCpaSyncResult,
  AccountCpaSyncStatusResult,
  AppSettings,
  BackgroundTaskSettings,
  FreeProxySyncResult,
  GatewayResponseCacheStats,
  HealthcheckConfig,
  HealthcheckRunResult,
  PluginItem,
} from "@/types";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import {
  AppWindow,
  BellRing,
  Check,
  Cpu,
  Database,
  Download,
  ExternalLink,
  Globe,
  History,
  Info,
  PlugZap,
  Palette,
  Play,
  PlusCircle,
  RefreshCw,
  RotateCcw,
  Save,
  Search,
  Settings as SettingsIcon,
  Trash2,
  Variable,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { formatTsFromSeconds } from "@/lib/utils/usage";
import {
  APP_NAV_ALWAYS_VISIBLE_IDS,
  APP_NAV_ITEMS,
  type AppNavItemId,
  normalizeVisibleMenuItems,
  sanitizeVisibleMenuItems,
} from "@/lib/navigation";
import { describeFreeProxyClearResult } from "./freeproxy-clear-state";
import {
  formatCpaSyncStatusLabel,
  parseCpaSyncScheduleInterval,
  shouldPollCpaSyncStatus,
} from "./cpa-sync-state";

const ENV_DESCRIPTION_MAP: Record<string, string> = {
  CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS:
    "控制单次上游请求允许持续的最长时间，单位毫秒；超过后会主动结束请求并返回超时错误。",
  CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS:
    "控制流式上游请求允许持续的最长时间，单位毫秒；填 0 可关闭流式超时上限。",
  CODEXMANAGER_SSE_KEEPALIVE_INTERVAL_MS:
    "控制向下游补发 SSE keep-alive 帧的间隔，单位毫秒；上游长时间安静时可避免客户端误判连接中断。",
  CODEXMANAGER_UPSTREAM_CONNECT_TIMEOUT_SECS:
    "控制连接上游服务器时的超时时间，单位秒；主要影响握手和网络建立阶段。",
  CODEXMANAGER_UPSTREAM_BASE_URL:
    "控制默认上游地址；修改后，网关会把请求转发到新的目标地址。",
};

const THEMES = [
  { id: "tech", name: "企业蓝", color: "#2563eb" },
  { id: "dark", name: "极夜黑", color: "#09090b" },
  { id: "dark-one", name: "深邃黑", color: "#282c34" },
  { id: "business", name: "事务金", color: "#c28100" },
  { id: "mint", name: "薄荷绿", color: "#059669" },
  { id: "sunset", name: "晚霞橙", color: "#ea580c" },
  { id: "grape", name: "葡萄灰紫", color: "#7c3aed" },
  { id: "ocean", name: "海湾青", color: "#0284c7" },
  { id: "forest", name: "松林绿", color: "#166534" },
  { id: "rose", name: "玫瑰粉", color: "#db2777" },
  { id: "slate", name: "石板灰", color: "#475569" },
  { id: "aurora", name: "极光青", color: "#0d9488" },
];

const ROUTE_STRATEGY_LABELS: Record<string, string> = {
  ordered: "顺序优先 (Ordered)",
  balanced: "均衡轮询 (Balanced)",
  weighted: "加权轮询 (Weighted)",
  "least-latency": "最低延迟优先 (Least Latency)",
  "cost-first": "成本优先 (Cost First)",
};

const RETRY_BACKOFF_LABELS: Record<string, string> = {
  immediate: "立即重试",
  fixed: "固定间隔",
  exponential: "指数退避",
};

const RESIDENCY_REQUIREMENT_LABELS: Record<string, string> = {
  "": "不限制",
  us: "仅美国 (us)",
};
const EMPTY_RESIDENCY_OPTION = "__none__";
const FREEPROXY_PROTOCOL_OPTIONS = [
  { value: "socks5", label: "Socks5" },
  { value: "https", label: "HTTPS" },
  { value: "http", label: "HTTP" },
  { value: "auto", label: "自动优选" },
] as const;
const FREEPROXY_ANONYMITY_OPTIONS = [
  { value: "elite", label: "仅高匿" },
  { value: "anonymous_or_elite", label: "匿名 + 高匿" },
  { value: "all", label: "全部" },
] as const;

const DEFAULT_FREE_ACCOUNT_MAX_MODEL_OPTIONS = [
  "auto",
  "gpt-5",
  "gpt-5-codex",
  "gpt-5-codex-mini",
  "gpt-5.1",
  "gpt-5.1-codex",
  "gpt-5.1-codex-max",
  "gpt-5.1-codex-mini",
  "gpt-5.2",
  "gpt-5.2-codex",
  "gpt-5.3-codex",
  "gpt-5.4",
] as const;

const ALERT_RULE_TYPE_LABELS: Record<string, string> = {
  token_refresh_fail: "Token 刷新连续失败",
  usage_threshold: "额度使用率阈值",
  error_rate: "网关错误率阈值",
  all_unavailable: "所有账号不可用",
};

const ALERT_CHANNEL_TYPE_LABELS: Record<string, string> = {
  webhook: "Webhook",
  bark: "Bark",
  telegram: "Telegram Bot",
  wecom: "企业微信机器人",
};

const PLUGIN_RUNTIME_LABELS: Record<string, string> = {
  lua: "Lua",
};

const PLUGIN_HOOK_POINT_LABELS: Record<string, string> = {
  pre_route: "pre_route",
  post_route: "post_route",
  post_response: "post_response",
};

const PLUGIN_TEMPLATES = [
  {
    id: "quota_guard",
    name: "额度守卫",
    description: "在上游路由前拦截低额度账号请求",
    runtime: "lua",
    hookPoints: ["pre_route"],
    timeoutMs: 100,
    scriptContent: `function handle(ctx)
  local account = ctx and ctx.account or nil
  local remain = account and account.primary_remain_percent or nil
  if remain ~= nil and remain < 5 then
    return {
      action = "reject",
      status = 429,
      body = {
        error = {
          message = "account quota too low",
          type = "quota_limit"
        }
      }
    }
  end
  return { action = "continue" }
end`,
  },
  {
    id: "response_audit",
    name: "响应审计",
    description: "在响应阶段为高风险结果补充标记",
    runtime: "lua",
    hookPoints: ["post_response"],
    timeoutMs: 100,
    scriptContent: `function handle(ctx)
  local response = ctx and ctx.response or nil
  local status = response and response.status or 0
  if status >= 500 then
    return {
      action = "continue",
      annotations = {
        severity = "error",
        tag = "upstream_5xx"
      }
    }
  end
  return { action = "continue" }
end`,
  },
] as const;

function formatFreeAccountModelLabel(value: string | null | undefined): string {
  const normalized = String(value || "").trim();
  if (!normalized || normalized === "auto") {
    return "跟随请求";
  }
  return normalized;
}

const SETTINGS_TABS = [
  "general",
  "appearance",
  "gateway",
  "alerts",
  "plugins",
  "tasks",
  "env",
] as const;
type SettingsTab = (typeof SETTINGS_TABS)[number];
const SETTINGS_ACTIVE_TAB_KEY = "codexmanager.settings.active-tab";
const NEW_ALERT_DRAFT_ID = "__new__";
const NEW_PLUGIN_DRAFT_ID = "__new_plugin__";

type AlertRuleDraft = {
  id: string | null;
  name: string;
  type: string;
  enabled: boolean;
  configText: string;
};

type AlertChannelDraft = {
  id: string | null;
  name: string;
  type: string;
  enabled: boolean;
  configText: string;
};

type PluginDraft = {
  id: string | null;
  name: string;
  description: string;
  runtime: string;
  hookPoints: string[];
  scriptContent: string;
  enabled: boolean;
  timeoutMs: string;
};

function readInitialSettingsTab(): SettingsTab {
  if (typeof window === "undefined") return "general";
  const savedTab = window.sessionStorage.getItem(SETTINGS_ACTIVE_TAB_KEY);
  if (savedTab && SETTINGS_TABS.includes(savedTab as SettingsTab)) {
    return savedTab as SettingsTab;
  }
  return "general";
}

function stringifyNumber(value: number | null | undefined): string {
  return value == null ? "" : String(value);
}

function parseIntegerInput(value: string, minimum = 0): number | null {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return null;
  const rounded = Math.trunc(numeric);
  if (rounded < minimum) return null;
  return rounded;
}

function parseStatusCodeListInput(value: string): number[] | null {
  const normalized = value.trim();
  if (!normalized) return [];
  const parts = normalized
    .split(/[\s,，]+/)
    .map((item) => item.trim())
    .filter(Boolean);
  if (!parts.length) return [];
  const values = parts.map((item) => Number(item));
  if (
    values.some(
      (item) => !Number.isInteger(item) || item < 100 || item > 599
    )
  ) {
    return null;
  }
  return Array.from(new Set(values)).sort((a, b) => a - b);
}

function formatCpaSyncTimestamp(value: number | null | undefined): string {
  if (typeof value !== "number" || value <= 0) return "--";
  return new Date(value * 1000).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function describeCpaSyncStatus(status: AccountCpaSyncStatusResult | null | undefined): string {
  switch (status?.status) {
    case "running":
      return "后台正在执行 CPA 拉取和导入。";
    case "misconfigured":
      return "定时同步已开启，但 API URL 或 Management Key 还不完整。";
    case "error":
      return "最近一次同步失败，调度器会按固定间隔继续重试。";
    case "idle":
      return "调度器已待命，会按固定分钟间隔自动执行。";
    case "disabled":
    default:
      return "当前未启用定时同步。";
  }
}

function getCpaSyncStatusBadgeClass(status: AccountCpaSyncStatusResult | null | undefined): string {
  switch (status?.status) {
    case "running":
      return "border-sky-500/40 bg-sky-500/10 text-sky-600";
    case "misconfigured":
      return "border-amber-500/40 bg-amber-500/10 text-amber-600";
    case "error":
      return "border-rose-500/40 bg-rose-500/10 text-rose-600";
    case "idle":
      return "border-emerald-500/40 bg-emerald-500/10 text-emerald-600";
    case "disabled":
    default:
      return "border-border/60 bg-background/60 text-muted-foreground";
  }
}

function formatStatusCodeListInput(value: number[] | null | undefined): string {
  return Array.isArray(value) ? value.join(", ") : "";
}

function formatJsonPretty(value: Record<string, unknown>): string {
  return JSON.stringify(value, null, 2);
}

function buildAlertRuleConfigPreset(type: string): Record<string, unknown> {
  switch (type) {
    case "token_refresh_fail":
      return { threshold: 3, channelIds: [], cooldownSecs: 1800 };
    case "error_rate":
      return {
        thresholdPercent: 20,
        windowMinutes: 5,
        minRequests: 20,
        channelIds: [],
        cooldownSecs: 1800,
      };
    case "all_unavailable":
      return { channelIds: [], cooldownSecs: 600 };
    case "usage_threshold":
    default:
      return { thresholdPercent: 90, channelIds: [], cooldownSecs: 1800 };
  }
}

function buildAlertChannelConfigPreset(type: string): Record<string, unknown> {
  switch (type) {
    case "bark":
      return { url: "https://api.day.app/your-device-key" };
    case "telegram":
      return { botToken: "", chatId: "" };
    case "wecom":
      return { webhookUrl: "" };
    case "webhook":
    default:
      return { url: "http://127.0.0.1:8080/alerts" };
  }
}

function createAlertRuleDraft(rule?: AlertRule | null): AlertRuleDraft {
  if (!rule) {
    return {
      id: null,
      name: "",
      type: "usage_threshold",
      enabled: true,
      configText: formatJsonPretty(buildAlertRuleConfigPreset("usage_threshold")),
    };
  }
  return {
    id: rule.id,
    name: rule.name,
    type: rule.ruleType,
    enabled: rule.enabled,
    configText: formatJsonPretty(rule.config),
  };
}

function createAlertChannelDraft(channel?: AlertChannel | null): AlertChannelDraft {
  if (!channel) {
    return {
      id: null,
      name: "",
      type: "webhook",
      enabled: true,
      configText: formatJsonPretty(buildAlertChannelConfigPreset("webhook")),
    };
  }
  return {
    id: channel.id,
    name: channel.name,
    type: channel.channelType,
    enabled: channel.enabled,
    configText: formatJsonPretty(channel.config),
  };
}

function createPluginDraft(plugin?: PluginItem | null): PluginDraft {
  if (!plugin) {
    return {
      id: null,
      name: "",
      description: "",
      runtime: "lua",
      hookPoints: ["pre_route"],
      scriptContent: PLUGIN_TEMPLATES[0]?.scriptContent ?? "",
      enabled: true,
      timeoutMs: "100",
    };
  }
  return {
    id: plugin.id,
    name: plugin.name,
    description: plugin.description ?? "",
    runtime: plugin.runtime,
    hookPoints: plugin.hookPoints.map((item) => String(item)),
    scriptContent: plugin.scriptContent,
    enabled: plugin.enabled,
    timeoutMs: String(plugin.timeoutMs),
  };
}

function statusBadgeVariant(
  status: string
): "default" | "secondary" | "destructive" | "outline" {
  if (status.includes("success") || status.includes("sent")) return "default";
  if (status.includes("failure") || status.includes("failed")) return "destructive";
  return "secondary";
}

function formatStorageBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

function countProxyPoolEntries(value: string | null | undefined): number {
  const normalized = String(value || "").trim();
  if (!normalized) return 0;
  return normalized
    .split(/[\n\r,;]+/)
    .map((item) => item.trim())
    .filter(Boolean).length;
}

function formatHealthcheckSuccessRate(result: HealthcheckRunResult | null | undefined): string {
  if (!result || result.sampledAccounts <= 0) {
    return "--";
  }
  return `${Math.round((result.successCount / result.sampledAccounts) * 100)}%`;
}

type UpdateCheckSummary = {
  repo: string;
  mode: string;
  isPortable: boolean;
  hasUpdate: boolean;
  canPrepare: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseTag: string;
  releaseName: string;
  reason: string;
};

type UpdatePrepareSummary = {
  prepared: boolean;
  mode: string;
  isPortable: boolean;
  releaseTag: string;
  latestVersion: string;
  assetName: string;
  assetPath: string;
  downloaded: boolean;
};

type CheckUpdateRequest = {
  silent?: boolean;
};

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function readStringField(source: Record<string, unknown>, key: string): string {
  const value = source[key];
  return typeof value === "string" ? value : "";
}

function readBooleanField(source: Record<string, unknown>, key: string): boolean {
  return source[key] === true;
}

function normalizeUpdateCheckSummary(payload: unknown): UpdateCheckSummary {
  const source = asRecord(payload) ?? {};
  return {
    repo: readStringField(source, "repo"),
    mode: readStringField(source, "mode"),
    isPortable: readBooleanField(source, "isPortable"),
    hasUpdate: readBooleanField(source, "hasUpdate"),
    canPrepare: readBooleanField(source, "canPrepare"),
    currentVersion: readStringField(source, "currentVersion"),
    latestVersion: readStringField(source, "latestVersion"),
    releaseTag: readStringField(source, "releaseTag"),
    releaseName: readStringField(source, "releaseName"),
    reason: readStringField(source, "reason"),
  };
}

function normalizeUpdatePrepareSummary(payload: unknown): UpdatePrepareSummary {
  const source = asRecord(payload) ?? {};
  return {
    prepared: readBooleanField(source, "prepared"),
    mode: readStringField(source, "mode"),
    isPortable: readBooleanField(source, "isPortable"),
    releaseTag: readStringField(source, "releaseTag"),
    latestVersion: readStringField(source, "latestVersion"),
    assetName: readStringField(source, "assetName"),
    assetPath: readStringField(source, "assetPath"),
    downloaded: readBooleanField(source, "downloaded"),
  };
}

function buildReleaseUrl(summary: UpdateCheckSummary | null): string {
  if (!summary?.repo) {
    return "https://github.com/qxcnm/Codex-Manager/releases";
  }
  const normalizedTag = summary.releaseTag || (summary.latestVersion ? `v${summary.latestVersion}` : "");
  if (!normalizedTag) {
    return `https://github.com/${summary.repo}/releases`;
  }
  return `https://github.com/${summary.repo}/releases/tag/${normalizedTag}`;
}

export default function SettingsPage() {
  const { setAppSettings: setStoreSettings } = useAppStore();
  const { theme, setTheme } = useTheme();
  const queryClient = useQueryClient();
  const isDesktopRuntime = isTauriRuntime();
  const lastSyncedSnapshotThemeRef = useRef<string | null>(null);
  const lastSyncedAppearancePresetRef = useRef<string | null>(null);
  const autoUpdateCheckedRef = useRef(false);
  const [activeTab, setActiveTab] = useState<SettingsTab>(readInitialSettingsTab);
  const [envSearch, setEnvSearch] = useState("");
  const [selectedEnvKey, setSelectedEnvKey] = useState<string | null>(null);
  const [envDrafts, setEnvDrafts] = useState<Record<string, string>>({});
  const [upstreamProxyDraft, setUpstreamProxyDraft] = useState<string | null>(null);
  const [mcpPortDraft, setMcpPortDraft] = useState<string | null>(null);
  const [gatewayOriginatorDraft, setGatewayOriginatorDraft] = useState<string | null>(null);
  const [newAccountProtectionDaysDraft, setNewAccountProtectionDaysDraft] = useState<string | null>(null);
  const [quotaProtectionThresholdDraft, setQuotaProtectionThresholdDraft] = useState<string | null>(null);
  const [retryPolicyMaxRetriesDraft, setRetryPolicyMaxRetriesDraft] = useState<string | null>(null);
  const [retryableStatusCodesDraft, setRetryableStatusCodesDraft] = useState<string | null>(null);
  const [responseCacheTtlDraft, setResponseCacheTtlDraft] = useState<string | null>(null);
  const [responseCacheMaxEntriesDraft, setResponseCacheMaxEntriesDraft] = useState<string | null>(null);
  const [freeProxyProtocol, setFreeProxyProtocol] = useState("socks5");
  const [freeProxyAnonymity, setFreeProxyAnonymity] = useState("elite");
  const [freeProxyCountry, setFreeProxyCountry] = useState("");
  const [freeProxyLimit, setFreeProxyLimit] = useState("20");
  const [freeProxyClearSingleProxy, setFreeProxyClearSingleProxy] = useState(true);
  const [freeProxySyncRegisterPool, setFreeProxySyncRegisterPool] = useState(true);
  const [freeProxySyncResult, setFreeProxySyncResult] = useState<FreeProxySyncResult | null>(null);
  const [freeProxyClearConfirmOpen, setFreeProxyClearConfirmOpen] = useState(false);
  const [lastUpdateCheck, setLastUpdateCheck] = useState<UpdateCheckSummary | null>(null);
  const [updateDialogCheck, setUpdateDialogCheck] = useState<UpdateCheckSummary | null>(null);
  const [preparedUpdate, setPreparedUpdate] = useState<UpdatePrepareSummary | null>(null);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);
  const [transportDraft, setTransportDraft] = useState<
    Partial<Record<"sseKeepaliveIntervalMs" | "upstreamStreamTimeoutMs", string>>
  >({});
  const [backgroundTaskDraft, setBackgroundTaskDraft] = useState<Record<string, string>>({});
  const [cpaSyncApiUrlDraft, setCpaSyncApiUrlDraft] = useState<string | null>(null);
  const [cpaSyncScheduleIntervalDraft, setCpaSyncScheduleIntervalDraft] = useState<string | null>(
    null
  );
  const [cpaSyncManagementKeyDraft, setCpaSyncManagementKeyDraft] = useState("");
  const [cpaSyncResult, setCpaSyncResult] = useState<AccountCpaSyncResult | null>(null);
  const [teamManagerApiUrlDraft, setTeamManagerApiUrlDraft] = useState<string | null>(null);
  const [teamManagerApiKeyDraft, setTeamManagerApiKeyDraft] = useState("");
  const [remoteManagementSecretDraft, setRemoteManagementSecretDraft] = useState("");
  const [payloadRewriteRulesDraft, setPayloadRewriteRulesDraft] = useState<string | null>(null);
  const [modelAliasPoolsDraft, setModelAliasPoolsDraft] = useState<string | null>(null);
  const [selectedAlertRuleId, setSelectedAlertRuleId] = useState<string | null>(null);
  const [selectedAlertChannelId, setSelectedAlertChannelId] = useState<string | null>(null);
  const [selectedPluginId, setSelectedPluginId] = useState<string | null>(null);
  const [alertRuleDraft, setAlertRuleDraft] = useState<AlertRuleDraft>(() =>
    createAlertRuleDraft()
  );
  const [alertChannelDraft, setAlertChannelDraft] = useState<AlertChannelDraft>(() =>
    createAlertChannelDraft()
  );
  const [pluginDraft, setPluginDraft] = useState<PluginDraft>(() => createPluginDraft());

  const { data: snapshot, isLoading } = useQuery({
    queryKey: ["app-settings-snapshot"],
    queryFn: () => appClient.getSettings(),
  });
  const { data: cpaSyncStatus } = useQuery({
    queryKey: ["cpa-sync-status"],
    queryFn: () => accountClient.getCpaSyncStatus(),
    refetchInterval: (query) =>
      shouldPollCpaSyncStatus(query.state.data as AccountCpaSyncStatusResult | undefined)
        ? 15_000
        : 60_000,
    refetchIntervalInBackground: true,
  });
  const { data: responseCacheStats } = useQuery({
    queryKey: ["gateway-cache-stats"],
    queryFn: () => serviceClient.getGatewayCacheStats(),
    refetchInterval: 30_000,
  });
  const { data: healthcheckConfig } = useQuery({
    queryKey: ["healthcheck-config"],
    queryFn: () => serviceClient.getHealthcheckConfig(),
    refetchInterval: 30_000,
    refetchIntervalInBackground: true,
  });
  const { data: alertRules = [] } = useQuery({
    queryKey: ["alert-rules"],
    queryFn: () => serviceClient.listAlertRules(),
  });
  const { data: alertChannels = [] } = useQuery({
    queryKey: ["alert-channels"],
    queryFn: () => serviceClient.listAlertChannels(),
  });
  const { data: alertHistory = [] } = useQuery({
    queryKey: ["alert-history"],
    queryFn: () => serviceClient.listAlertHistory(50),
    refetchInterval: 30_000,
    refetchIntervalInBackground: true,
  });
  const { data: plugins = [] } = useQuery({
    queryKey: ["plugins"],
    queryFn: () => serviceClient.listPlugins(),
  });
  const visibleMenuItems = useMemo(
    () => normalizeVisibleMenuItems(snapshot?.visibleMenuItems),
    [snapshot?.visibleMenuItems]
  );

  const updateSettings = useMutation({
    mutationFn: (patch: Partial<AppSettings> & { _silent?: boolean }) => {
      const actualPatch = { ...patch };
      delete actualPatch._silent;
      return appClient.setSettings(actualPatch);
    },
    onSuccess: (nextSnapshot, variables) => {
      queryClient.setQueryData(["app-settings-snapshot"], nextSnapshot);
      setStoreSettings(nextSnapshot);
      if (nextSnapshot.lowTransparency) {
        document.body.classList.add("low-transparency");
      } else {
        document.body.classList.remove("low-transparency");
      }
      applyAppearancePreset(nextSnapshot.appearancePreset);
      if (
        "responseCacheEnabled" in variables ||
        "responseCacheTtlSecs" in variables ||
        "responseCacheMaxEntries" in variables
      ) {
        void queryClient.invalidateQueries({ queryKey: ["gateway-cache-stats"] });
      }
      if ("backgroundTasks" in variables) {
        void queryClient.invalidateQueries({ queryKey: ["healthcheck-config"] });
      }
      if (
        "cpaSyncEnabled" in variables ||
        "cpaSyncScheduleEnabled" in variables ||
        "cpaSyncScheduleIntervalMinutes" in variables
      ) {
        void queryClient.invalidateQueries({ queryKey: ["cpa-sync-status"] });
      }
      if (!variables._silent) {
        toast.success("设置已更新");
      }
    },
    onError: (error: unknown) => {
      toast.error(`更新失败: ${getAppErrorMessage(error)}`);
    },
  });

  const testTeamManager = useMutation({
    mutationFn: (payload: { apiUrl?: string | null; apiKey?: string | null }) =>
      accountClient.testTeamManager(payload.apiUrl, payload.apiKey),
    onSuccess: (result) => {
      if (result?.success) {
        toast.success(result.message || "Team Manager 连接测试成功");
      } else {
        toast.error(result?.message || "Team Manager 连接测试失败");
      }
    },
    onError: (error: unknown) => {
      toast.error(`测试 Team Manager 失败: ${getAppErrorMessage(error)}`);
    },
  });

  const testCpaSync = useMutation({
    mutationFn: (payload: { apiUrl?: string | null; managementKey?: string | null }) =>
      accountClient.testCpaSync(payload.apiUrl, payload.managementKey),
    onSuccess: (result) => {
      if (result?.success) {
        toast.success(result.message || "CPA 连接测试成功");
      } else {
        toast.error(result?.message || "CPA 连接测试失败");
      }
    },
    onError: (error: unknown) => {
      toast.error(`测试 CPA 失败: ${getAppErrorMessage(error)}`);
    },
  });

  const syncCpaAccounts = useMutation({
    mutationFn: (payload: { apiUrl?: string | null; managementKey?: string | null }) =>
      accountClient.syncCpaAccounts(payload.apiUrl, payload.managementKey),
    onSuccess: async (result) => {
      setCpaSyncResult(result);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["accounts"] }),
        queryClient.invalidateQueries({ queryKey: ["usage"] }),
        queryClient.invalidateQueries({ queryKey: ["usage-aggregate"] }),
        queryClient.invalidateQueries({ queryKey: ["today-summary"] }),
        queryClient.invalidateQueries({ queryKey: ["cpa-sync-status"] }),
      ]);
      toast.success(
        `CPA 同步完成：新增 ${result.created}，更新 ${result.updated}，失败 ${result.failed}`
      );
    },
    onError: (error: unknown) => {
      void queryClient.invalidateQueries({ queryKey: ["cpa-sync-status"] });
      toast.error(`同步 CPA 账号失败: ${getAppErrorMessage(error)}`);
    },
  });

  const syncFreeProxyPool = useMutation({
    mutationFn: async () => {
      const limit = parseIntegerInput(freeProxyLimit, 1);
      if (limit == null || limit > 100) {
        throw new Error("代理数量请输入 1 到 100 之间的整数");
      }
      return serviceClient.syncFreeProxyPool({
        protocol: freeProxyProtocol,
        anonymity: freeProxyAnonymity,
        country: freeProxyCountry.trim(),
        limit,
        clearUpstreamProxyUrl: freeProxyClearSingleProxy,
        syncRegisterProxyPool: freeProxySyncRegisterPool,
      });
    },
    onSuccess: async (result) => {
      setFreeProxySyncResult(result);
      const nextSnapshot = await appClient.getSettings();
      queryClient.setQueryData(["app-settings-snapshot"], nextSnapshot);
      setStoreSettings(nextSnapshot);
      toast.success(
        freeProxySyncRegisterPool
          ? `已同步 ${result.appliedCount} 个代理到网关，并写入注册代理池`
          : `已同步 ${result.appliedCount} 个 freeproxy 代理到代理池`
      );
    },
    onError: (error: unknown) => {
      toast.error(`同步 freeproxy 失败: ${getAppErrorMessage(error)}`);
    },
  });
  const clearFreeProxyPool = useMutation({
    mutationFn: () => serviceClient.clearFreeProxyPool(),
    onSuccess: async (result) => {
      setFreeProxySyncResult(null);
      const nextSnapshot = await appClient.getSettings();
      queryClient.setQueryData(["app-settings-snapshot"], nextSnapshot);
      setStoreSettings(nextSnapshot);
      toast.success(describeFreeProxyClearResult(result));
      if (result.failedRegisterProxyCount > 0) {
        toast.warning(`注册代理池仍有 ${result.remainingRegisterProxyCount} 个代理未删除`);
      }
    },
    onError: (error: unknown) => {
      toast.error(`清空代理池失败: ${getAppErrorMessage(error)}`);
    },
  });
  const clearGatewayCache = useMutation({
    mutationFn: () => serviceClient.clearGatewayCache(),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["gateway-cache-stats"] });
      toast.success("响应缓存已清空");
    },
    onError: (error: unknown) => {
      toast.error(`清空缓存失败: ${getAppErrorMessage(error)}`);
    },
  });
  const runHealthcheck = useMutation({
    mutationFn: () => serviceClient.runHealthcheck(),
    onSuccess: (result) => {
      queryClient.setQueryData(
        ["healthcheck-config"],
        (current: HealthcheckConfig | undefined) => ({
          enabled:
            current?.enabled ?? snapshot?.backgroundTasks.sessionProbePollingEnabled ?? false,
          intervalSecs:
            current?.intervalSecs ?? snapshot?.backgroundTasks.sessionProbeIntervalSecs ?? 1800,
          sampleSize:
            current?.sampleSize ?? snapshot?.backgroundTasks.sessionProbeSampleSize ?? 2,
          recentRun: result,
        })
      );
      void queryClient.invalidateQueries({ queryKey: ["dashboard-health"] });
      toast.success(
        result.failureCount > 0
          ? `巡检完成：成功 ${result.successCount}，失败 ${result.failureCount}`
          : `巡检完成：抽检 ${result.sampledAccounts} 个账号，全部通过`
      );
    },
    onError: (error: unknown) => {
      toast.error(`执行巡检失败: ${getAppErrorMessage(error)}`);
    },
  });

  const checkUpdate = useMutation({
    mutationFn: (request?: CheckUpdateRequest) => {
      void request;
      return appClient.checkUpdate();
    },
    onSuccess: (result, request) => {
      const summary = normalizeUpdateCheckSummary(result);
      setLastUpdateCheck(summary);
      if (summary.hasUpdate) {
        setUpdateDialogCheck(summary);
        setPreparedUpdate((current) =>
          current && current.latestVersion === summary.latestVersion ? current : null
        );
        setUpdateDialogOpen(true);
        if (!request?.silent) {
          toast.success(
            `发现新版本 ${summary.latestVersion || summary.releaseTag || "可用"}`
          );
        }
        return;
      }
      setPreparedUpdate(null);
      if (!request?.silent) {
        toast.success(
          summary.reason
            ? `已检查更新：${summary.reason}`
            : `当前已是最新版本 ${summary.currentVersion || ""}`.trim()
        );
      }
    },
    onError: (error: unknown) => {
      toast.error(`检查更新失败: ${getAppErrorMessage(error)}`);
    },
  });

  const handleVisibleMenuToggle = (menuId: AppNavItemId, checked: boolean) => {
    if (!snapshot) return;
    const current = new Set(visibleMenuItems);
    if (checked) {
      current.add(menuId);
    } else {
      current.delete(menuId);
    }
    updateSettings.mutate({
      visibleMenuItems: sanitizeVisibleMenuItems(Array.from(current)),
    });
  };

  const prepareUpdate = useMutation({
    mutationFn: () => appClient.prepareUpdate(),
    onSuccess: (result) => {
      const summary = normalizeUpdatePrepareSummary(result);
      setPreparedUpdate(summary);
      setUpdateDialogOpen(true);
      toast.success(
        summary.isPortable
          ? `更新已下载，重启应用后即可更新到 ${summary.latestVersion || "新版本"}`
          : `安装包已下载完成，可立即安装 ${summary.latestVersion || "新版本"}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`下载更新失败: ${getAppErrorMessage(error)}`);
    },
  });

  const applyPreparedUpdate = useMutation({
    mutationFn: (payload: { isPortable: boolean }) =>
      payload.isPortable ? appClient.applyUpdatePortable() : appClient.launchInstaller(),
    onSuccess: (_result, payload) => {
      setUpdateDialogOpen(false);
      toast.success(payload.isPortable ? "即将重启并应用更新" : "安装程序已启动");
    },
    onError: (error: unknown, payload) => {
      toast.error(
        `${payload.isPortable ? "应用更新" : "启动安装程序"}失败: ${getAppErrorMessage(error)}`
      );
    },
  });

  const saveAlertRule = useMutation({
    mutationFn: (payload: AlertRuleDraft & { config: Record<string, unknown> }) =>
      serviceClient.upsertAlertRule({
        id: payload.id,
        name: payload.name,
        type: payload.type,
        config: payload.config,
        enabled: payload.enabled,
      }),
    onSuccess: async (item) => {
      await queryClient.invalidateQueries({ queryKey: ["alert-rules"] });
      setSelectedAlertRuleId(item.id);
      setAlertRuleDraft(createAlertRuleDraft(item));
      toast.success("告警规则已保存");
    },
    onError: (error: unknown) => {
      toast.error(`保存告警规则失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteAlertRule = useMutation({
    mutationFn: (id: string) => serviceClient.deleteAlertRule(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["alert-rules"] });
      setSelectedAlertRuleId(null);
      setAlertRuleDraft(createAlertRuleDraft());
      toast.success("告警规则已删除");
    },
    onError: (error: unknown) => {
      toast.error(`删除告警规则失败: ${getAppErrorMessage(error)}`);
    },
  });

  const saveAlertChannel = useMutation({
    mutationFn: (payload: AlertChannelDraft & { config: Record<string, unknown> }) =>
      serviceClient.upsertAlertChannel({
        id: payload.id,
        name: payload.name,
        type: payload.type,
        config: payload.config,
        enabled: payload.enabled,
      }),
    onSuccess: async (item) => {
      await queryClient.invalidateQueries({ queryKey: ["alert-channels"] });
      setSelectedAlertChannelId(item.id);
      setAlertChannelDraft(createAlertChannelDraft(item));
      toast.success("告警渠道已保存");
    },
    onError: (error: unknown) => {
      toast.error(`保存告警渠道失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteAlertChannel = useMutation({
    mutationFn: (id: string) => serviceClient.deleteAlertChannel(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["alert-channels"] });
      await queryClient.invalidateQueries({ queryKey: ["alert-history"] });
      setSelectedAlertChannelId(null);
      setAlertChannelDraft(createAlertChannelDraft());
      toast.success("告警渠道已删除");
    },
    onError: (error: unknown) => {
      toast.error(`删除告警渠道失败: ${getAppErrorMessage(error)}`);
    },
  });

  const testAlertChannel = useMutation({
    mutationFn: (id: string) => serviceClient.testAlertChannel(id),
    onSuccess: async (result) => {
      await queryClient.invalidateQueries({ queryKey: ["alert-history"] });
      toast.success(
        result.sentAt
          ? `测试通知已发送（${formatTsFromSeconds(result.sentAt, "刚刚")}）`
          : "测试通知已发送"
      );
    },
    onError: (error: unknown) => {
      toast.error(`测试告警渠道失败: ${getAppErrorMessage(error)}`);
    },
  });

  const savePlugin = useMutation({
    mutationFn: (payload: PluginDraft & { timeoutMsValue: number }) =>
      serviceClient.upsertPlugin({
        id: payload.id,
        name: payload.name.trim(),
        description: payload.description.trim() || null,
        runtime: payload.runtime,
        hookPoints: payload.hookPoints,
        scriptContent: payload.scriptContent,
        enabled: payload.enabled,
        timeoutMs: payload.timeoutMsValue,
      }),
    onSuccess: async (item) => {
      await queryClient.invalidateQueries({ queryKey: ["plugins"] });
      setSelectedPluginId(item.id);
      setPluginDraft(createPluginDraft(item));
      toast.success("插件配置已保存");
    },
    onError: (error: unknown) => {
      toast.error(`保存插件失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deletePlugin = useMutation({
    mutationFn: (id: string) => serviceClient.deletePlugin(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["plugins"] });
      setSelectedPluginId(null);
      setPluginDraft(createPluginDraft());
      toast.success("插件已删除");
    },
    onError: (error: unknown) => {
      toast.error(`删除插件失败: ${getAppErrorMessage(error)}`);
    },
  });

  useEffect(() => {
    if (!snapshot?.theme) return;
    if (lastSyncedSnapshotThemeRef.current === snapshot.theme) return;

    lastSyncedSnapshotThemeRef.current = snapshot.theme;
    const currentAppliedTheme =
      typeof document !== "undefined"
        ? document.documentElement.getAttribute("data-theme")
        : null;

    if (snapshot.theme !== currentAppliedTheme) {
      setTheme(snapshot.theme);
    }
  }, [setTheme, snapshot?.theme]);

  useEffect(() => {
    if (!snapshot) return;
    const nextPreset = normalizeAppearancePreset(snapshot.appearancePreset);
    if (lastSyncedAppearancePresetRef.current === nextPreset) return;

    lastSyncedAppearancePresetRef.current = nextPreset;
    applyAppearancePreset(nextPreset);
  }, [snapshot]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.sessionStorage.setItem(SETTINGS_ACTIVE_TAB_KEY, activeTab);
  }, [activeTab]);

  useEffect(() => {
    if (!isDesktopRuntime || !snapshot?.updateAutoCheck || autoUpdateCheckedRef.current) {
      return;
    }
    autoUpdateCheckedRef.current = true;
    checkUpdate.mutate({ silent: true });
  }, [checkUpdate, isDesktopRuntime, snapshot?.updateAutoCheck]);

  const handleOpenReleasePage = () => {
    void appClient
      .openInBrowser(buildReleaseUrl(updateDialogCheck))
      .catch((error) => {
        toast.error(`打开发布页失败: ${getAppErrorMessage(error)}`);
      });
  };

  const filteredEnvCatalog = useMemo(() => {
    const catalog = snapshot?.envOverrideCatalog || [];
    if (!envSearch) return catalog;
    const keyword = envSearch.toLowerCase();
    return catalog.filter(
      (item) =>
        item.key.toLowerCase().includes(keyword) ||
        item.label.toLowerCase().includes(keyword)
    );
  }, [envSearch, snapshot?.envOverrideCatalog]);

  const selectedEnvItem = useMemo(
    () => snapshot?.envOverrideCatalog.find((item) => item.key === selectedEnvKey),
    [selectedEnvKey, snapshot?.envOverrideCatalog]
  );

  const upstreamProxyInput = upstreamProxyDraft ?? (snapshot?.upstreamProxyUrl || "");
  const gatewayOriginatorInput =
    gatewayOriginatorDraft ?? (snapshot?.gatewayOriginator || "codex_cli_rs");
  const quotaProtectionThresholdInput =
    quotaProtectionThresholdDraft ??
    stringifyNumber(snapshot?.quotaProtectionThresholdPercent);
  const newAccountProtectionDaysInput =
    newAccountProtectionDaysDraft ??
    stringifyNumber(snapshot?.newAccountProtectionDays);
  const retryPolicyMaxRetriesInput =
    retryPolicyMaxRetriesDraft ?? stringifyNumber(snapshot?.retryPolicyMaxRetries);
  const retryableStatusCodesInput =
    retryableStatusCodesDraft ??
    formatStatusCodeListInput(snapshot?.retryPolicyRetryableStatusCodes);
  const mcpPortInput = mcpPortDraft ?? stringifyNumber(snapshot?.mcpPort);
  const responseCacheTtlInput =
    responseCacheTtlDraft ?? stringifyNumber(snapshot?.responseCacheTtlSecs);
  const responseCacheMaxEntriesInput =
    responseCacheMaxEntriesDraft ?? stringifyNumber(snapshot?.responseCacheMaxEntries);
  const transportInputValues = {
    sseKeepaliveIntervalMs:
      transportDraft.sseKeepaliveIntervalMs ??
      stringifyNumber(snapshot?.sseKeepaliveIntervalMs),
    upstreamStreamTimeoutMs:
      transportDraft.upstreamStreamTimeoutMs ??
      stringifyNumber(snapshot?.upstreamStreamTimeoutMs),
  };
  const selectedEnvValue = selectedEnvKey
    ? envDrafts[selectedEnvKey] ??
      snapshot?.envOverrides[selectedEnvKey] ??
      selectedEnvItem?.defaultValue ??
      ""
    : "";
  const proxyPoolValue = snapshot?.envOverrides.CODEXMANAGER_PROXY_LIST || "";
  const proxyPoolCount = countProxyPoolEntries(proxyPoolValue);
  const teamManagerApiUrlInput = teamManagerApiUrlDraft ?? snapshot?.teamManagerApiUrl ?? "";
  const cpaSyncApiUrlInput = cpaSyncApiUrlDraft ?? snapshot?.cpaSyncApiUrl ?? "";
  const cpaSyncScheduleIntervalInput =
    cpaSyncScheduleIntervalDraft ?? stringifyNumber(snapshot?.cpaSyncScheduleIntervalMinutes ?? 30);
  const remoteManagementSecretInput = remoteManagementSecretDraft.trim();
  const payloadRewriteRulesInput =
    payloadRewriteRulesDraft ?? snapshot?.payloadRewriteRulesJson ?? "[]";
  const modelAliasPoolsInput = modelAliasPoolsDraft ?? snapshot?.modelAliasPoolsJson ?? "[]";
  const cacheStats: GatewayResponseCacheStats = responseCacheStats ?? {
    enabled: snapshot?.responseCacheEnabled ?? false,
    ttlSecs: snapshot?.responseCacheTtlSecs ?? 3600,
    maxEntries: snapshot?.responseCacheMaxEntries ?? 256,
    entryCount: 0,
    estimatedBytes: 0,
    hitCount: 0,
    missCount: 0,
    hitRatePercent: 0,
  };
  const latestHealthcheck = healthcheckConfig?.recentRun ?? null;

  const lastIntentThemeRef = useRef<string | null>(null);
  const lastIntentAppearancePresetRef = useRef<string | null>(null);

  const handleThemeChange = (nextTheme: string) => {
    if (!snapshot || nextTheme === snapshot.theme) return;
    const previousSnapshot = snapshot;
    const previousTheme = snapshot.theme || "tech";

    // 1. Immediately update local UI and intent lock
    lastIntentThemeRef.current = nextTheme;
    lastSyncedSnapshotThemeRef.current = nextTheme;
    
    setActiveTab("appearance");
    if (typeof window !== "undefined") {
      window.sessionStorage.setItem(SETTINGS_ACTIVE_TAB_KEY, "appearance");
    }
    
    setTheme(nextTheme);

    // 2. Optimistic local update
    queryClient.setQueryData(["app-settings-snapshot"], {
      ...snapshot,
      theme: nextTheme,
    });
    setStoreSettings({ ...snapshot, theme: nextTheme });

    // 3. Immediate persist to backend (No debounce)
    updateSettings.mutate(
      { theme: nextTheme, _silent: true },
      {
        onSuccess: (updatedSnapshot) => {
          // Double check if this is still our intent
          if (lastIntentThemeRef.current === nextTheme) {
            queryClient.setQueryData(["app-settings-snapshot"], updatedSnapshot);
            setStoreSettings(updatedSnapshot);
          }
        },
        onError: () => {
          // Only revert if no newer intent has been made
          if (lastIntentThemeRef.current === nextTheme) {
            queryClient.setQueryData(["app-settings-snapshot"], previousSnapshot);
            setStoreSettings(previousSnapshot);
            setTheme(previousTheme);
          }
        },
      }
    );
  };

  const handleAppearancePresetChange = (nextPreset: string) => {
    if (!snapshot) return;

    const normalizedPreset = normalizeAppearancePreset(nextPreset);
    const previousSnapshot = snapshot;
    const previousPreset = normalizeAppearancePreset(snapshot.appearancePreset);
    if (normalizedPreset === previousPreset) return;

    lastIntentAppearancePresetRef.current = normalizedPreset;
    lastSyncedAppearancePresetRef.current = normalizedPreset;
    applyAppearancePreset(normalizedPreset);

    queryClient.setQueryData(["app-settings-snapshot"], {
      ...snapshot,
      appearancePreset: normalizedPreset,
    });
    setStoreSettings({ ...snapshot, appearancePreset: normalizedPreset });

    updateSettings.mutate(
      { appearancePreset: normalizedPreset, _silent: true },
      {
        onSuccess: (updatedSnapshot) => {
          if (lastIntentAppearancePresetRef.current === normalizedPreset) {
            queryClient.setQueryData(["app-settings-snapshot"], updatedSnapshot);
            setStoreSettings(updatedSnapshot);
          }
        },
        onError: () => {
          if (lastIntentAppearancePresetRef.current === normalizedPreset) {
            queryClient.setQueryData(["app-settings-snapshot"], previousSnapshot);
            setStoreSettings(previousSnapshot);
            applyAppearancePreset(previousPreset);
          }
        },
      }
    );
  };

  const updateBackgroundTasks = (patch: Partial<BackgroundTaskSettings>) => {
    if (!snapshot) return;
    updateSettings.mutate({
      backgroundTasks: {
        ...snapshot.backgroundTasks,
        ...patch,
      },
    });
  };

  const saveTransportField = (
    key: "sseKeepaliveIntervalMs" | "upstreamStreamTimeoutMs",
    minimum: number
  ) => {
    const nextValue = parseIntegerInput(transportInputValues[key], minimum);
    if (nextValue == null) {
      toast.error("请输入合法的数值");
      setTransportDraft((current) => {
        const nextDraft = { ...current };
        delete nextDraft[key];
        return nextDraft;
      });
      return;
    }
    void updateSettings
      .mutateAsync({ [key]: nextValue } as Partial<AppSettings>)
      .then(() => {
        setTransportDraft((current) => {
          const nextDraft = { ...current };
          delete nextDraft[key];
          return nextDraft;
        });
      })
      .catch(() => undefined);
  };

  const saveBackgroundTaskField = (
    key: keyof BackgroundTaskSettings,
    minimum = 1,
    maximum?: number
  ) => {
    if (!snapshot) return;
    const draftKey = String(key);
    const sourceValue =
      backgroundTaskDraft[draftKey] ?? stringifyNumber(snapshot.backgroundTasks[key] as number);
    const nextValue = parseIntegerInput(sourceValue, minimum);
    if (nextValue == null || (maximum != null && nextValue > maximum)) {
      toast.error(
        maximum != null ? `请输入 ${minimum} 到 ${maximum} 之间的数值` : "请输入合法的数值"
      );
      setBackgroundTaskDraft((current) => {
        const nextDraft = { ...current };
        delete nextDraft[draftKey];
        return nextDraft;
      });
      return;
    }
    void updateSettings
      .mutateAsync({
        backgroundTasks: {
          ...snapshot.backgroundTasks,
          [key]: nextValue,
        },
      })
      .then(() => {
        setBackgroundTaskDraft((current) => {
          const nextDraft = { ...current };
          delete nextDraft[draftKey];
          return nextDraft;
        });
      })
      .catch(() => undefined);
  };

  const saveQuotaProtectionThreshold = () => {
    if (!snapshot) return;
    const nextValue = parseIntegerInput(quotaProtectionThresholdInput, 0);
    if (nextValue == null || nextValue > 100) {
      toast.error("请输入 0 到 100 之间的百分比");
      setQuotaProtectionThresholdDraft(null);
      return;
    }
    if (nextValue === snapshot.quotaProtectionThresholdPercent) {
      setQuotaProtectionThresholdDraft(null);
      return;
    }
    void updateSettings
      .mutateAsync({ quotaProtectionThresholdPercent: nextValue })
      .then(() => setQuotaProtectionThresholdDraft(null))
      .catch(() => undefined);
  };

  const saveNewAccountProtectionDays = () => {
    if (!snapshot) return;
    const nextValue = parseIntegerInput(newAccountProtectionDaysInput, 0);
    if (nextValue == null || nextValue > 30) {
      toast.error("新号保护天数请输入 0 到 30 之间的整数");
      setNewAccountProtectionDaysDraft(null);
      return;
    }
    if (nextValue === snapshot.newAccountProtectionDays) {
      setNewAccountProtectionDaysDraft(null);
      return;
    }
    void updateSettings
      .mutateAsync({ newAccountProtectionDays: nextValue })
      .then(() => setNewAccountProtectionDaysDraft(null))
      .catch(() => undefined);
  };

  const saveRetryPolicyMaxRetries = () => {
    if (!snapshot) return;
    const nextValue = parseIntegerInput(retryPolicyMaxRetriesInput, 0);
    if (nextValue == null || nextValue > 10) {
      toast.error("最大重试次数请输入 0 到 10 之间的整数");
      setRetryPolicyMaxRetriesDraft(null);
      return;
    }
    if (nextValue === snapshot.retryPolicyMaxRetries) {
      setRetryPolicyMaxRetriesDraft(null);
      return;
    }
    void updateSettings
      .mutateAsync({ retryPolicyMaxRetries: nextValue })
      .then(() => setRetryPolicyMaxRetriesDraft(null))
      .catch(() => undefined);
  };

  const saveRetryableStatusCodes = () => {
    if (!snapshot) return;
    const nextValue = parseStatusCodeListInput(retryableStatusCodesInput);
    if (nextValue == null) {
      toast.error("请输入合法的 HTTP 状态码列表，例如 429, 502, 503");
      setRetryableStatusCodesDraft(null);
      return;
    }
    if (
      JSON.stringify(nextValue) ===
      JSON.stringify(snapshot.retryPolicyRetryableStatusCodes)
    ) {
      setRetryableStatusCodesDraft(null);
      return;
    }
    void updateSettings
      .mutateAsync({ retryPolicyRetryableStatusCodes: nextValue })
      .then(() => setRetryableStatusCodesDraft(null))
      .catch(() => undefined);
  };

  const saveResponseCacheTtl = () => {
    if (!snapshot) return;
    const nextValue = parseIntegerInput(responseCacheTtlInput, 1);
    if (nextValue == null) {
      toast.error("缓存 TTL 请输入大于等于 1 的秒数");
      setResponseCacheTtlDraft(null);
      return;
    }
    if (nextValue === snapshot.responseCacheTtlSecs) {
      setResponseCacheTtlDraft(null);
      return;
    }
    void updateSettings
      .mutateAsync({ responseCacheTtlSecs: nextValue })
      .then(() => setResponseCacheTtlDraft(null))
      .catch(() => undefined);
  };

  const saveResponseCacheMaxEntries = () => {
    if (!snapshot) return;
    const nextValue = parseIntegerInput(responseCacheMaxEntriesInput, 1);
    if (nextValue == null) {
      toast.error("最大缓存条目数请输入大于等于 1 的整数");
      setResponseCacheMaxEntriesDraft(null);
      return;
    }
    if (nextValue === snapshot.responseCacheMaxEntries) {
      setResponseCacheMaxEntriesDraft(null);
      return;
    }
    void updateSettings
      .mutateAsync({ responseCacheMaxEntries: nextValue })
      .then(() => setResponseCacheMaxEntriesDraft(null))
      .catch(() => undefined);
  };

  const saveMcpPort = () => {
    if (!snapshot) return;
    const nextValue = parseIntegerInput(mcpPortInput, 1);
    if (nextValue == null || nextValue > 65535) {
      toast.error("MCP 端口请输入 1 到 65535 之间的整数");
      setMcpPortDraft(null);
      return;
    }
    if (nextValue === snapshot.mcpPort) {
      setMcpPortDraft(null);
      return;
    }
    void updateSettings
      .mutateAsync({ mcpPort: nextValue })
      .then(() => setMcpPortDraft(null))
      .catch(() => undefined);
  };

  const handleSaveEnv = () => {
    if (!selectedEnvKey || !snapshot) return;
    void updateSettings
      .mutateAsync({
        envOverrides: {
          ...snapshot.envOverrides,
          [selectedEnvKey]: selectedEnvValue,
        },
      })
      .then(() => {
        setEnvDrafts((current) => {
          const nextDraft = { ...current };
          delete nextDraft[selectedEnvKey];
          return nextDraft;
        });
      })
      .catch(() => undefined);
  };

  const handleResetEnv = () => {
    if (!selectedEnvKey || !snapshot) return;
    const nextOverrides = { ...snapshot.envOverrides };
    delete nextOverrides[selectedEnvKey];
    void updateSettings
      .mutateAsync({ envOverrides: nextOverrides })
      .then(() => {
        setEnvDrafts((current) => {
          const nextDraft = { ...current };
          delete nextDraft[selectedEnvKey];
          return nextDraft;
        });
      })
      .catch(() => undefined);
  };

  const handleSaveTeamManager = () => {
    if (!snapshot) return;
    const nextApiUrl = teamManagerApiUrlInput.trim();
    void updateSettings
      .mutateAsync({
        teamManagerEnabled: snapshot.teamManagerEnabled,
        teamManagerApiUrl: nextApiUrl,
        ...(teamManagerApiKeyDraft.trim()
          ? { teamManagerApiKey: teamManagerApiKeyDraft.trim() }
          : {}),
      })
      .then(() => {
        setTeamManagerApiUrlDraft(null);
        setTeamManagerApiKeyDraft("");
      })
      .catch(() => undefined);
  };

  const handleSaveCpaSync = () => {
    if (!snapshot) return;
    const nextApiUrl = cpaSyncApiUrlInput.trim();
    const intervalMinutes = parseCpaSyncScheduleInterval(cpaSyncScheduleIntervalInput.trim());
    if (intervalMinutes == null) {
      toast.error("定时同步间隔请输入大于等于 1 的整数分钟");
      return;
    }
    void updateSettings
      .mutateAsync({
        cpaSyncEnabled: snapshot.cpaSyncEnabled,
        cpaSyncApiUrl: nextApiUrl,
        cpaSyncScheduleEnabled: snapshot.cpaSyncScheduleEnabled,
        cpaSyncScheduleIntervalMinutes: intervalMinutes,
        ...(cpaSyncManagementKeyDraft.trim()
          ? { cpaSyncManagementKey: cpaSyncManagementKeyDraft.trim() }
          : {}),
      })
      .then(() => {
        setCpaSyncApiUrlDraft(null);
        setCpaSyncScheduleIntervalDraft(null);
        setCpaSyncManagementKeyDraft("");
      })
      .catch(() => undefined);
  };

  const handleSaveRemoteManagement = () => {
    if (!snapshot) return;
    if (
      snapshot.remoteManagementEnabled &&
      !snapshot.remoteManagementSecretConfigured &&
      !remoteManagementSecretInput
    ) {
      toast.error("启用远程管理 API 前请先设置访问密钥");
      return;
    }
    void updateSettings
      .mutateAsync({
        remoteManagementEnabled: snapshot.remoteManagementEnabled,
        ...(remoteManagementSecretInput
          ? { remoteManagementSecret: remoteManagementSecretInput }
          : {}),
      })
      .then(() => {
        setRemoteManagementSecretDraft("");
      })
      .catch(() => undefined);
  };

  const handleSavePayloadRewriteRules = () => {
    const raw = payloadRewriteRulesInput.trim();
    const candidate = raw || "[]";
    try {
      const parsed = JSON.parse(candidate);
      if (!Array.isArray(parsed)) {
        toast.error("Payload Rewrite 规则必须是 JSON 数组");
        return;
      }
    } catch {
      toast.error("Payload Rewrite 规则不是合法 JSON");
      return;
    }
    void updateSettings
      .mutateAsync({
        payloadRewriteRulesJson: candidate,
      })
      .then(() => {
        setPayloadRewriteRulesDraft(null);
      })
      .catch(() => undefined);
  };

  const handleSaveModelAliasPools = () => {
    const raw = modelAliasPoolsInput.trim();
    const candidate = raw || "[]";
    try {
      const parsed = JSON.parse(candidate);
      if (!Array.isArray(parsed)) {
        toast.error("模型别名池配置必须是 JSON 数组");
        return;
      }
    } catch {
      toast.error("模型别名池配置不是合法 JSON");
      return;
    }
    void updateSettings
      .mutateAsync({
        modelAliasPoolsJson: candidate,
      })
      .then(() => {
        setModelAliasPoolsDraft(null);
      })
      .catch(() => undefined);
  };

  const handleTestTeamManager = () => {
    if (!snapshot) return;
    void testTeamManager.mutate({
      apiUrl: teamManagerApiUrlInput.trim() || null,
      apiKey:
        teamManagerApiKeyDraft.trim() ||
        (snapshot.teamManagerHasApiKey ? "use_saved_key" : null),
      });
  };

  const handleTestCpaSync = () => {
    if (!snapshot) return;
    void testCpaSync.mutate({
      apiUrl: cpaSyncApiUrlInput.trim() || null,
      managementKey:
        cpaSyncManagementKeyDraft.trim() ||
        (snapshot.cpaSyncHasManagementKey ? "use_saved_key" : null),
      });
  };

  const handleRunCpaSync = () => {
    if (!snapshot) return;
    void syncCpaAccounts.mutate({
      apiUrl: cpaSyncApiUrlInput.trim() || null,
      managementKey:
        cpaSyncManagementKeyDraft.trim() ||
        (snapshot.cpaSyncHasManagementKey ? "use_saved_key" : null),
      });
  };

  const resolvedSelectedAlertRuleId = useMemo(() => {
    if (selectedAlertRuleId === NEW_ALERT_DRAFT_ID) {
      return NEW_ALERT_DRAFT_ID;
    }
    if (!alertRules.length) {
      return null;
    }
    if (selectedAlertRuleId && alertRules.some((item) => item.id === selectedAlertRuleId)) {
      return selectedAlertRuleId;
    }
    return alertRules[0]?.id ?? null;
  }, [alertRules, selectedAlertRuleId]);

  const resolvedSelectedAlertChannelId = useMemo(() => {
    if (selectedAlertChannelId === NEW_ALERT_DRAFT_ID) {
      return NEW_ALERT_DRAFT_ID;
    }
    if (!alertChannels.length) {
      return null;
    }
    if (
      selectedAlertChannelId &&
      alertChannels.some((item) => item.id === selectedAlertChannelId)
    ) {
      return selectedAlertChannelId;
    }
    return alertChannels[0]?.id ?? null;
  }, [alertChannels, selectedAlertChannelId]);

  const selectedAlertRule = useMemo(
    () =>
      resolvedSelectedAlertRuleId && resolvedSelectedAlertRuleId !== NEW_ALERT_DRAFT_ID
        ? alertRules.find((item) => item.id === resolvedSelectedAlertRuleId) ?? null
        : null,
    [alertRules, resolvedSelectedAlertRuleId]
  );
  const selectedAlertChannel = useMemo(
    () =>
      resolvedSelectedAlertChannelId && resolvedSelectedAlertChannelId !== NEW_ALERT_DRAFT_ID
        ? alertChannels.find((item) => item.id === resolvedSelectedAlertChannelId) ?? null
        : null,
    [alertChannels, resolvedSelectedAlertChannelId]
  );
  const resolvedSelectedPluginId = useMemo(() => {
    if (selectedPluginId === NEW_PLUGIN_DRAFT_ID) {
      return NEW_PLUGIN_DRAFT_ID;
    }
    if (!plugins.length) {
      return null;
    }
    if (selectedPluginId && plugins.some((item) => item.id === selectedPluginId)) {
      return selectedPluginId;
    }
    return plugins[0]?.id ?? null;
  }, [plugins, selectedPluginId]);
  const selectedPlugin = useMemo(
    () =>
      resolvedSelectedPluginId && resolvedSelectedPluginId !== NEW_PLUGIN_DRAFT_ID
        ? plugins.find((item) => item.id === resolvedSelectedPluginId) ?? null
        : null,
    [plugins, resolvedSelectedPluginId]
  );

  const activeAlertRuleDraft = useMemo(() => {
    if (resolvedSelectedAlertRuleId === NEW_ALERT_DRAFT_ID) {
      return alertRuleDraft;
    }
    if (selectedAlertRule && alertRuleDraft.id === selectedAlertRule.id) {
      return alertRuleDraft;
    }
    return selectedAlertRule ? createAlertRuleDraft(selectedAlertRule) : createAlertRuleDraft();
  }, [alertRuleDraft, resolvedSelectedAlertRuleId, selectedAlertRule]);

  const activeAlertChannelDraft = useMemo(() => {
    if (resolvedSelectedAlertChannelId === NEW_ALERT_DRAFT_ID) {
      return alertChannelDraft;
    }
    if (selectedAlertChannel && alertChannelDraft.id === selectedAlertChannel.id) {
      return alertChannelDraft;
    }
    return selectedAlertChannel
      ? createAlertChannelDraft(selectedAlertChannel)
      : createAlertChannelDraft();
  }, [alertChannelDraft, resolvedSelectedAlertChannelId, selectedAlertChannel]);
  const activePluginDraft = useMemo(() => {
    if (resolvedSelectedPluginId === NEW_PLUGIN_DRAFT_ID) {
      return pluginDraft;
    }
    if (selectedPlugin && pluginDraft.id === selectedPlugin.id) {
      return pluginDraft;
    }
    return selectedPlugin ? createPluginDraft(selectedPlugin) : createPluginDraft();
  }, [pluginDraft, resolvedSelectedPluginId, selectedPlugin]);

  const updateAlertRuleDraft = (updater: (current: AlertRuleDraft) => AlertRuleDraft) => {
    setAlertRuleDraft((current) =>
      updater(current.id === activeAlertRuleDraft.id ? current : activeAlertRuleDraft)
    );
  };

  const updateAlertChannelDraft = (
    updater: (current: AlertChannelDraft) => AlertChannelDraft
  ) => {
    setAlertChannelDraft((current) =>
      updater(current.id === activeAlertChannelDraft.id ? current : activeAlertChannelDraft)
    );
  };
  const updatePluginDraft = (updater: (current: PluginDraft) => PluginDraft) => {
    setPluginDraft((current) =>
      updater(current.id === activePluginDraft.id ? current : activePluginDraft)
    );
  };
  const applyPluginTemplate = (templateId: string) => {
    const template = PLUGIN_TEMPLATES.find((item) => item.id === templateId);
    if (!template) return;
    setSelectedPluginId(NEW_PLUGIN_DRAFT_ID);
    setPluginDraft({
      id: null,
      name: template.name,
      description: template.description,
      runtime: template.runtime,
      hookPoints: [...template.hookPoints],
      scriptContent: template.scriptContent,
      enabled: true,
      timeoutMs: String(template.timeoutMs),
    });
  };

  const handleSaveAlertRule = () => {
    try {
      const parsed = JSON.parse(activeAlertRuleDraft.configText || "{}") as Record<string, unknown>;
      void saveAlertRule.mutate({ ...activeAlertRuleDraft, config: parsed });
    } catch (error) {
      toast.error(`规则配置 JSON 无法解析: ${getAppErrorMessage(error)}`);
    }
  };

  const handleSaveAlertChannel = () => {
    try {
      const parsed = JSON.parse(activeAlertChannelDraft.configText || "{}") as Record<string, unknown>;
      void saveAlertChannel.mutate({ ...activeAlertChannelDraft, config: parsed });
    } catch (error) {
      toast.error(`渠道配置 JSON 无法解析: ${getAppErrorMessage(error)}`);
    }
  };
  const handleSavePlugin = () => {
    const normalizedName = activePluginDraft.name.trim();
    if (!normalizedName) {
      toast.error("请输入插件名称");
      return;
    }
    if (!activePluginDraft.hookPoints.length) {
      toast.error("至少选择一个 Hook 点");
      return;
    }
    if (!activePluginDraft.scriptContent.trim()) {
      toast.error("请输入插件脚本");
      return;
    }
    const timeoutMsValue = parseIntegerInput(activePluginDraft.timeoutMs, 1);
    if (timeoutMsValue == null || timeoutMsValue > 60_000) {
      toast.error("超时时间请输入 1 到 60000 之间的整数");
      return;
    }
    void savePlugin.mutate({
      ...activePluginDraft,
      name: normalizedName,
      timeoutMsValue,
    });
  };

  if (isLoading || !snapshot) {
    return <div className="flex h-64 items-center justify-center text-muted-foreground">加载配置中...</div>;
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-bold tracking-tight">系统设置</h2>
        <p className="mt-1 text-sm text-muted-foreground">管理应用行为、网关策略及后台任务</p>
      </div>

      <Tabs
        value={activeTab}
        onValueChange={(value) => {
          if (value && SETTINGS_TABS.includes(value as SettingsTab)) {
            setActiveTab(value as SettingsTab);
          }
        }}
        className="w-full"
      >
        <TabsList className="glass-card mb-6 flex h-11 w-full justify-start overflow-x-auto rounded-xl border-none p-1 no-scrollbar lg:w-fit">
          <TabsTrigger value="general" className="gap-2 px-5 shrink-0">
            <SettingsIcon className="h-4 w-4" /> 通用
          </TabsTrigger>
          <TabsTrigger value="appearance" className="gap-2 px-5 shrink-0">
            <Palette className="h-4 w-4" /> 外观
          </TabsTrigger>
          <TabsTrigger value="gateway" className="gap-2 px-5 shrink-0">
            <Globe className="h-4 w-4" /> 网关
          </TabsTrigger>
          <TabsTrigger value="alerts" className="gap-2 px-5 shrink-0">
            <BellRing className="h-4 w-4" /> 告警
          </TabsTrigger>
          <TabsTrigger value="plugins" className="gap-2 px-5 shrink-0">
            <PlugZap className="h-4 w-4" /> 插件
          </TabsTrigger>
          <TabsTrigger value="tasks" className="gap-2 px-5 shrink-0">
            <Cpu className="h-4 w-4" /> 任务
          </TabsTrigger>
          <TabsTrigger value="env" className="gap-2 px-5 shrink-0">
            <Variable className="h-4 w-4" /> 环境
          </TabsTrigger>
        </TabsList>

        <TabsContent value="general" className="space-y-6">
          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <AppWindow className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">基础设置</CardTitle>
              </div>
              <CardDescription>控制应用启动和窗口行为</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>自动检查更新</Label>
                  <p className="text-xs text-muted-foreground">启动时自动检测新版本</p>
                </div>
                <Switch
                  checked={snapshot.updateAutoCheck}
                  onCheckedChange={(value) => updateSettings.mutate({ updateAutoCheck: value })}
                />
              </div>
              <div className="flex flex-col gap-3 rounded-2xl border border-border/50 bg-background/45 p-4 md:flex-row md:items-center md:justify-between">
                <div className="space-y-1">
                  <Label>检查更新</Label>
                  <p className="text-xs text-muted-foreground">
                    {isDesktopRuntime
                      ? "立即检查 GitHub Releases 是否有新版本可用"
                      : "Web / Docker 版不提供桌面应用更新检查"}
                  </p>
                  {lastUpdateCheck ? (
                    <p className="text-xs text-muted-foreground">
                      {lastUpdateCheck.hasUpdate
                        ? `发现新版本 ${lastUpdateCheck.latestVersion || lastUpdateCheck.releaseTag || "可用"}`
                        : lastUpdateCheck.reason ||
                          `当前版本 ${lastUpdateCheck.currentVersion || "未知"} 已是最新`}
                    </p>
                  ) : null}
                </div>
                <Button
                  variant="outline"
                  className="gap-2 self-start md:self-auto"
                  disabled={
                    !isDesktopRuntime ||
                    checkUpdate.isPending ||
                    prepareUpdate.isPending ||
                    applyPreparedUpdate.isPending
                  }
                  onClick={() => checkUpdate.mutate({ silent: false })}
                >
                  <RefreshCw className={cn("h-4 w-4", checkUpdate.isPending && "animate-spin")} />
                  检查更新
                </Button>
              </div>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>关闭时最小化到托盘</Label>
                  <p className="text-xs text-muted-foreground">点击关闭按钮不会直接退出程序</p>
                </div>
                <Switch
                  checked={snapshot.closeToTrayOnClose}
                  disabled={!snapshot.closeToTraySupported}
                  onCheckedChange={(value) =>
                    updateSettings.mutate({ closeToTrayOnClose: value })
                  }
                />
              </div>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>视觉性能模式</Label>
                  <p className="text-xs text-muted-foreground">关闭毛玻璃等特效以提升低配电脑性能</p>
                </div>
                <Switch
                  checked={snapshot.lowTransparency}
                  onCheckedChange={(value) => updateSettings.mutate({ lowTransparency: value })}
                />
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <Download className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">CLIProxyAPI / CPA</CardTitle>
              </div>
              <CardDescription>
                配置 CPA Management API，把已登录的 Codex auth 文件单向同步到当前账号池。
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>启用同步源</Label>
                  <p className="text-xs text-muted-foreground">
                    开启后保留这组 CPA 配置，便于后续手动测试和立即同步。
                  </p>
                </div>
                <Switch
                  checked={snapshot.cpaSyncEnabled}
                  onCheckedChange={(value) => updateSettings.mutate({ cpaSyncEnabled: value })}
                />
              </div>

              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>定时同步</Label>
                  <p className="text-xs text-muted-foreground">
                    由服务端后台定时触发，适合 NAS / Docker 常驻部署。
                  </p>
                </div>
                <Switch
                  checked={snapshot.cpaSyncScheduleEnabled}
                  onCheckedChange={(value) =>
                    updateSettings.mutate({ cpaSyncScheduleEnabled: value })
                  }
                />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="cpa-sync-schedule-interval">同步间隔（分钟）</Label>
                <Input
                  id="cpa-sync-schedule-interval"
                  inputMode="numeric"
                  placeholder="30"
                  value={cpaSyncScheduleIntervalInput}
                  onChange={(event) => setCpaSyncScheduleIntervalDraft(event.target.value)}
                />
                <p className="text-[11px] text-muted-foreground">
                  使用固定分钟数。保存后立即热更新，无需重启容器。
                </p>
              </div>

              <div className="grid gap-2">
                <Label htmlFor="cpa-sync-api-url">CPA API URL</Label>
                <Input
                  id="cpa-sync-api-url"
                  placeholder="https://your-cpa.example.com"
                  value={cpaSyncApiUrlInput}
                  onChange={(event) => setCpaSyncApiUrlDraft(event.target.value)}
                />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="cpa-sync-management-key">Management Key</Label>
                <Input
                  id="cpa-sync-management-key"
                  type="password"
                  placeholder={
                    snapshot.cpaSyncHasManagementKey
                      ? "留空则保留当前已保存 Management Key"
                      : "输入 CLIProxyAPI Management Key"
                  }
                  value={cpaSyncManagementKeyDraft}
                  onChange={(event) => setCpaSyncManagementKeyDraft(event.target.value)}
                />
                <p className="text-[11px] text-muted-foreground">
                  {snapshot.cpaSyncHasManagementKey
                    ? "当前已保存 Management Key，重新输入后会覆盖。这里不是网页登录密码。"
                    : "当前还未保存 Management Key。这里填写的是 Management Key，不是网页登录密码。"}
                </p>
              </div>

              <div className="rounded-xl border border-border/60 bg-background/40 p-3 text-xs text-muted-foreground">
                当前同步为单向导入：会读取 CPA 中已登录的 Codex/OpenAI/ChatGPT auth 文件并导入本地号池，不会删除 CPA 侧账号。
              </div>

              <div className="space-y-3 rounded-xl border border-border/60 bg-background/40 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <Badge className={cn("border", getCpaSyncStatusBadgeClass(cpaSyncStatus))}>
                        {formatCpaSyncStatusLabel(cpaSyncStatus)}
                      </Badge>
                      <span className="text-xs text-muted-foreground">
                        {describeCpaSyncStatus(cpaSyncStatus)}
                      </span>
                    </div>
                    <p className="text-[11px] text-muted-foreground">
                      最近触发：{cpaSyncStatus?.lastTrigger || "--"}
                    </p>
                  </div>
                  <div className="text-right text-xs text-muted-foreground">
                    <p>下次执行</p>
                    <p className="text-sm font-medium text-foreground">
                      {formatCpaSyncTimestamp(cpaSyncStatus?.nextRunAt)}
                    </p>
                  </div>
                </div>

                <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
                  <div>
                    <p className="text-[11px] text-muted-foreground">固定间隔</p>
                    <p className="text-sm font-medium">
                      {cpaSyncStatus?.intervalMinutes ?? snapshot.cpaSyncScheduleIntervalMinutes} 分钟
                    </p>
                  </div>
                  <div>
                    <p className="text-[11px] text-muted-foreground">上次开始</p>
                    <p className="text-sm font-medium">
                      {formatCpaSyncTimestamp(cpaSyncStatus?.lastStartedAt)}
                    </p>
                  </div>
                  <div>
                    <p className="text-[11px] text-muted-foreground">上次完成</p>
                    <p className="text-sm font-medium">
                      {formatCpaSyncTimestamp(cpaSyncStatus?.lastFinishedAt)}
                    </p>
                  </div>
                  <div>
                    <p className="text-[11px] text-muted-foreground">上次成功</p>
                    <p className="text-sm font-medium">
                      {formatCpaSyncTimestamp(cpaSyncStatus?.lastSuccessAt)}
                    </p>
                  </div>
                </div>

                {cpaSyncStatus?.lastSummary ? (
                  <div className="rounded-lg border border-border/50 bg-background/60 p-3 text-xs text-muted-foreground">
                    {cpaSyncStatus.lastSummary}
                  </div>
                ) : null}

                {cpaSyncStatus?.lastError ? (
                  <div className="rounded-lg border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-600">
                    {cpaSyncStatus.lastError}
                  </div>
                ) : null}
              </div>

              <div className="flex flex-wrap gap-3">
                <Button
                  className="gap-2"
                  disabled={updateSettings.isPending}
                  onClick={handleSaveCpaSync}
                >
                  <Save className="h-4 w-4" />
                  保存 CPA 设置
                </Button>
                <Button
                  variant="outline"
                  className="gap-2"
                  disabled={testCpaSync.isPending}
                  onClick={handleTestCpaSync}
                >
                  <ExternalLink className="h-4 w-4" />
                  {testCpaSync.isPending ? "测试中..." : "测试连接"}
                </Button>
                <Button
                  variant="outline"
                  className="gap-2"
                  disabled={syncCpaAccounts.isPending || cpaSyncStatus?.isRunning}
                  onClick={handleRunCpaSync}
                >
                  <RefreshCw
                    className={cn(
                      "h-4 w-4",
                      (syncCpaAccounts.isPending || cpaSyncStatus?.isRunning) && "animate-spin"
                    )}
                  />
                  {syncCpaAccounts.isPending || cpaSyncStatus?.isRunning
                    ? "同步中..."
                    : "立即同步"}
                </Button>
              </div>

              {cpaSyncResult ? (
                <div className="space-y-3 rounded-xl border border-border/60 bg-background/40 p-4">
                  <div className="grid gap-3 sm:grid-cols-3 lg:grid-cols-7">
                    <div>
                      <p className="text-[11px] text-muted-foreground">总文件数</p>
                      <p className="text-base font-semibold">{cpaSyncResult.totalFiles}</p>
                    </div>
                    <div>
                      <p className="text-[11px] text-muted-foreground">可导入文件</p>
                      <p className="text-base font-semibold">{cpaSyncResult.eligibleFiles}</p>
                    </div>
                    <div>
                      <p className="text-[11px] text-muted-foreground">已下载</p>
                      <p className="text-base font-semibold">{cpaSyncResult.downloadedFiles}</p>
                    </div>
                    <div>
                      <p className="text-[11px] text-muted-foreground">成功导入</p>
                      <p className="text-base font-semibold">
                        {cpaSyncResult.created + cpaSyncResult.updated}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] text-muted-foreground">新增账号</p>
                      <p className="text-base font-semibold text-emerald-500">
                        {cpaSyncResult.created}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] text-muted-foreground">更新账号</p>
                      <p className="text-base font-semibold text-sky-500">
                        {cpaSyncResult.updated}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] text-muted-foreground">失败</p>
                      <p className="text-base font-semibold text-rose-500">
                        {cpaSyncResult.failed}
                      </p>
                    </div>
                  </div>

                  <div className="grid gap-3 lg:grid-cols-2">
                    <div className="space-y-2">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        本次导入账号 ID
                      </p>
                      <div className="max-h-28 overflow-auto whitespace-pre-wrap rounded-lg border border-border/50 bg-background/60 p-2 text-xs">
                        {cpaSyncResult.importedAccountIds.length ? (
                          cpaSyncResult.importedAccountIds.join("\n")
                        ) : (
                          <span className="text-muted-foreground">本次没有产生可识别的账号 ID</span>
                        )}
                      </div>
                    </div>
                    <div className="space-y-2">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        错误摘要
                      </p>
                      <div className="max-h-28 overflow-auto whitespace-pre-wrap rounded-lg border border-border/50 bg-background/60 p-2 text-xs">
                        {cpaSyncResult.errors.length ? (
                          cpaSyncResult.errors.slice(0, 8).join("\n")
                        ) : (
                          <span className="text-muted-foreground">没有错误</span>
                        )}
                      </div>
                    </div>
                  </div>
                </div>
              ) : null}
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <Globe className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">Team Manager</CardTitle>
              </div>
              <CardDescription>配置支付后的一键上传目标，用于把可用账号同步到 Team Manager</CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>启用上传</Label>
                  <p className="text-xs text-muted-foreground">
                    开启后，可在支付页和账号页直接上传到 Team Manager
                  </p>
                </div>
                <Switch
                  checked={snapshot.teamManagerEnabled}
                  onCheckedChange={(value) =>
                    updateSettings.mutate({ teamManagerEnabled: value })
                  }
                />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="team-manager-api-url">API URL</Label>
                <Input
                  id="team-manager-api-url"
                  placeholder="https://your-team-manager.example.com"
                  value={teamManagerApiUrlInput}
                  onChange={(event) => setTeamManagerApiUrlDraft(event.target.value)}
                />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="team-manager-api-key">API Key</Label>
                <Input
                  id="team-manager-api-key"
                  type="password"
                  placeholder={
                    snapshot.teamManagerHasApiKey ? "留空则保留当前已保存 Key" : "输入 Team Manager API Key"
                  }
                  value={teamManagerApiKeyDraft}
                  onChange={(event) => setTeamManagerApiKeyDraft(event.target.value)}
                />
                <p className="text-[11px] text-muted-foreground">
                  {snapshot.teamManagerHasApiKey
                    ? "当前已保存 API Key，重新输入后会覆盖。"
                    : "当前还未保存 API Key。"}
                </p>
              </div>

              <div className="flex flex-wrap gap-3">
                <Button
                  className="gap-2"
                  disabled={updateSettings.isPending}
                  onClick={handleSaveTeamManager}
                >
                  <Save className="h-4 w-4" />
                  保存 Team Manager 设置
                </Button>
                <Button
                  variant="outline"
                  className="gap-2"
                  disabled={testTeamManager.isPending}
                  onClick={handleTestTeamManager}
                >
                  <ExternalLink className="h-4 w-4" />
                  {testTeamManager.isPending ? "测试中..." : "测试连接"}
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <SettingsIcon className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">远程管理 API</CardTitle>
              </div>
              <CardDescription>
                为外部脚本或运维面板开放一个受密钥保护的管理 RPC 入口。
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>启用远程管理</Label>
                  <p className="text-xs text-muted-foreground">
                    开启后，可通过 <code>/api/management/rpc</code> + 访问密钥远程调用管理接口。
                  </p>
                </div>
                <Switch
                  checked={snapshot.remoteManagementEnabled}
                  onCheckedChange={(value) =>
                    updateSettings.mutate({
                      remoteManagementEnabled: value,
                      _silent: !value,
                    })
                  }
                />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="remote-management-secret">访问密钥</Label>
                <Input
                  id="remote-management-secret"
                  type="password"
                  placeholder={
                    snapshot.remoteManagementSecretConfigured
                      ? "留空则保留当前已保存密钥"
                      : "输入远程管理访问密钥"
                  }
                  value={remoteManagementSecretDraft}
                  onChange={(event) => setRemoteManagementSecretDraft(event.target.value)}
                />
                <p className="text-[11px] text-muted-foreground">
                  {snapshot.remoteManagementSecretConfigured
                    ? "当前已保存访问密钥，重新输入后会覆盖。调用时请使用请求头 x-codexmanager-management-secret。"
                    : "当前未保存访问密钥。建议仅在 Web 地址对外暴露时启用。"}
                </p>
              </div>

              <div className="rounded-2xl border border-border/50 bg-background/45 p-4 text-xs text-muted-foreground">
                <p>示例：</p>
                <code className="mt-2 block break-all">
                  {`curl -X POST http://127.0.0.1:48761/api/management/rpc -H "content-type: application/json" -H "x-codexmanager-management-secret: <secret>" -d '{"id":1,"method":"appSettings/get","params":null}'`}
                </code>
              </div>

              <div className="flex flex-wrap gap-3">
                <Button
                  className="gap-2"
                  disabled={updateSettings.isPending}
                  onClick={handleSaveRemoteManagement}
                >
                  <Save className="h-4 w-4" />
                  保存远程管理设置
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="appearance" className="space-y-6">
          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <Palette className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">样式版本</CardTitle>
              </div>
              <CardDescription>在渐变版本和默认版本之间切换</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid gap-3 md:grid-cols-2">
                {APPEARANCE_PRESETS.map((item) => {
                  const currentPreset = normalizeAppearancePreset(snapshot.appearancePreset);
                  const isActive = currentPreset === item.id;
                  return (
                    <button
                      key={item.id}
                      onClick={() => handleAppearancePresetChange(item.id)}
                      className={cn(
                        "group relative rounded-2xl border p-4 text-left transition-all duration-300 hover:-translate-y-0.5",
                        isActive
                          ? "border-primary bg-primary/10 shadow-lg ring-1 ring-primary"
                          : "border-border/60 bg-background/50 hover:bg-accent/30"
                      )}
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div className="space-y-1.5">
                          <div className="text-sm font-semibold">{item.name}</div>
                          <p className="text-xs leading-5 text-muted-foreground">
                            {item.description}
                          </p>
                        </div>
                        {isActive ? (
                          <div className="rounded-full bg-primary p-1 text-primary-foreground shadow-sm">
                            <Check className="h-3 w-3" />
                          </div>
                        ) : null}
                      </div>
                      <div className="mt-3 flex items-end gap-2.5">
                        <div
                          className={cn(
                            "h-14 flex-1 rounded-xl border",
                            item.id === "modern"
                              ? "border-primary/20 bg-[linear-gradient(160deg,rgba(255,255,255,0.88),rgba(37,99,235,0.1)),linear-gradient(180deg,rgba(191,219,254,0.6),rgba(255,255,255,0.85))]"
                              : "border-slate-300/70 bg-[radial-gradient(at_0%_0%,#bfdbfe_0px,transparent_50%),radial-gradient(at_100%_0%,#cffafe_0px,transparent_50%),radial-gradient(at_50%_100%,#ffffff_0px,transparent_50%),rgba(255,255,255,0.86)]"
                          )}
                        />
                        <div className="flex w-16 flex-col gap-1.5">
                          <div
                            className={cn(
                              "h-4 rounded-lg border",
                              item.id === "modern"
                                ? "border-primary/15 bg-white/80 shadow-sm"
                                : "border-slate-300/70 bg-white/70"
                            )}
                          />
                          <div
                            className={cn(
                              "h-4 rounded-lg border",
                              item.id === "modern"
                                ? "border-primary/15 bg-white/70 shadow-sm"
                                : "border-slate-300/70 bg-white/60"
                            )}
                          />
                        </div>
                      </div>
                    </button>
                  );
                })}
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <Palette className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">界面主题</CardTitle>
              </div>
              <CardDescription>选择您喜爱的配色方案，适配不同工作心情</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-12">
                {THEMES.map((item) => (
                  <button
                    key={item.id}
                    onClick={() => handleThemeChange(item.id)}
                    className={cn(
                      "group relative flex flex-col items-center gap-2.5 rounded-2xl border p-4 transition-all duration-300 hover:scale-105",
                      theme === item.id
                        ? "border-primary bg-primary/10 shadow-lg ring-1 ring-primary"
                        : "border-transparent bg-muted/20 hover:bg-accent/40"
                    )}
                  >
                    <div
                      className="h-10 w-10 rounded-full border-2 border-white/20 shadow-md"
                      style={{ backgroundColor: item.color }}
                    />
                    <span
                      className={cn(
                        "whitespace-nowrap text-[10px] font-semibold transition-colors",
                        theme === item.id
                          ? "text-primary"
                          : "text-muted-foreground group-hover:text-foreground"
                      )}
                    >
                      {item.name}
                    </span>
                    {theme === item.id ? (
                      <div className="absolute right-2 top-2 rounded-full bg-primary p-0.5 text-primary-foreground shadow-sm">
                        <Check className="h-2.5 w-2.5" />
                      </div>
                    ) : null}
                  </button>
                ))}
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <AppWindow className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">菜单显示</CardTitle>
              </div>
              <CardDescription>
                选择侧边栏需要展示的菜单；设置入口始终保留，避免误隐藏。
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid gap-3 md:grid-cols-2">
                {APP_NAV_ITEMS.map((item) => {
                  const locked = APP_NAV_ALWAYS_VISIBLE_IDS.includes(item.id);
                  const checked = visibleMenuItems.includes(item.id);
                  return (
                    <div
                      key={item.id}
                      className="flex items-center justify-between rounded-2xl border border-border/60 bg-background/50 px-4 py-3"
                    >
                      <div className="min-w-0">
                        <p className="text-sm font-medium">{item.name}</p>
                        <p className="text-[11px] text-muted-foreground">{item.href}</p>
                      </div>
                      <div className="flex items-center gap-3">
                        {locked ? (
                          <Badge variant="secondary" className="bg-primary/10 text-primary">
                            固定
                          </Badge>
                        ) : null}
                        <Switch
                          checked={checked}
                          disabled={locked || updateSettings.isPending}
                          onCheckedChange={(value) =>
                            handleVisibleMenuToggle(item.id, value)
                          }
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="gateway" className="space-y-4">
          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <CardTitle className="text-base">网关策略</CardTitle>
              <CardDescription>配置账号选路和请求头处理方式</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid gap-2">
                <Label>账号选路策略</Label>
                <Select
                  value={snapshot.routeStrategy || "ordered"}
                  onValueChange={(value) =>
                    updateSettings.mutate({ routeStrategy: value || "ordered" })
                  }
                >
                  <SelectTrigger className="w-full md:w-[300px]">
                    <SelectValue placeholder="选择策略">
                      {(value) => {
                        const nextValue = String(value || "").trim();
                        if (!nextValue) return "选择策略";
                        return ROUTE_STRATEGY_LABELS[nextValue] || nextValue;
                      }}
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ordered">顺序优先 (Ordered)</SelectItem>
                    <SelectItem value="balanced">均衡轮询 (Balanced)</SelectItem>
                    <SelectItem value="weighted">加权轮询 (Weighted)</SelectItem>
                    <SelectItem value="least-latency">
                      最低延迟优先 (Least Latency)
                    </SelectItem>
                    <SelectItem value="cost-first">成本优先 (Cost First)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-[10px] text-muted-foreground">
                  顺序优先：按账号候选顺序优先尝试，默认只会在头部小窗口内按健康度做轻微换头；
                  均衡轮询：按“平台密钥 + 模型”维度严格轮询可用账号，默认不做健康度换头；
                  加权轮询：剩余额度越高，命中概率越高；最低延迟优先：优先最近响应更快的账号；
                  成本优先：优先 free，再到 plus / team。
                </p>
              </div>

              <div className="grid gap-4 rounded-2xl border border-border/50 bg-background/35 p-4">
                <div className="flex items-center justify-between gap-4">
                  <div className="space-y-0.5">
                    <div className="flex items-center gap-2">
                      <PlugZap className="h-4 w-4 text-primary" />
                      <Label>MCP Server</Label>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      控制实验性 MCP 入口。当前已接通 stdio 与 HTTP SSE，两种传输共用同一开关。
                    </p>
                  </div>
                  <Switch
                    checked={snapshot.mcpEnabled}
                    onCheckedChange={(value) =>
                      updateSettings.mutate({ mcpEnabled: value })
                    }
                  />
                </div>

                <div className="grid gap-2 md:max-w-[240px]">
                  <Label>HTTP SSE 端口</Label>
                  <Input
                    type="number"
                    min={1}
                    max={65535}
                    value={mcpPortInput}
                    onChange={(event) => setMcpPortDraft(event.target.value)}
                    onBlur={saveMcpPort}
                  />
                  <p className="text-[10px] text-muted-foreground">
                    `codexmanager-mcp --http-sse` 默认监听此端口，默认 <code>48762</code>。关闭开关后，stdio 与 HTTP SSE 都会拒绝连接。
                  </p>
                </div>
              </div>

              <div className="grid gap-2">
                <Label>Free 账号使用模型</Label>
                <Select
                  value={snapshot.freeAccountMaxModel || "auto"}
                  onValueChange={(value) =>
                    updateSettings.mutate({ freeAccountMaxModel: value || "auto" })
                  }
                >
                  <SelectTrigger className="w-full md:w-[300px]">
                    <SelectValue placeholder="选择 free 账号使用模型">
                      {(value) => formatFreeAccountModelLabel(String(value || ""))}
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    {(snapshot.freeAccountMaxModelOptions?.length
                      ? snapshot.freeAccountMaxModelOptions
                      : DEFAULT_FREE_ACCOUNT_MAX_MODEL_OPTIONS
                    ).map((model) => (
                      <SelectItem key={model} value={model}>
                        {formatFreeAccountModelLabel(model)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-[10px] text-muted-foreground">
                  设为“跟随请求”时，不会额外改写 free / 7天单窗口账号的模型；
                  只有你选了具体模型后，命中这些账号时才会统一改写为该模型。
                </p>
              </div>

              <div className="grid gap-2 md:max-w-[240px]">
                <Label>新号保护天数</Label>
                <Input
                  type="number"
                  min={0}
                  max={30}
                  value={newAccountProtectionDaysInput}
                  onChange={(event) => setNewAccountProtectionDaysDraft(event.target.value)}
                  onBlur={saveNewAccountProtectionDays}
                />
                <p className="text-[10px] text-muted-foreground">
                  默认 <code>3</code> 天。保护期内的新账号仍可参与路由，但会自动排在成熟账号之后；填
                  <code>0</code> 可关闭。
                </p>
              </div>

              <div className="grid gap-4 rounded-2xl border border-border/50 bg-background/35 p-4">
                <div className="flex items-center justify-between gap-4">
                  <div className="space-y-0.5">
                    <Label>额度保护</Label>
                    <p className="text-xs text-muted-foreground">
                      开启后，当账号剩余额度小于等于阈值时，网关会自动跳过该账号。
                    </p>
                  </div>
                  <Switch
                    checked={snapshot.quotaProtectionEnabled}
                    onCheckedChange={(value) =>
                      updateSettings.mutate({ quotaProtectionEnabled: value })
                    }
                  />
                </div>
                <div className="grid gap-2 md:max-w-[240px]">
                  <Label>剩余额度阈值 (%)</Label>
                  <Input
                    type="number"
                    min={0}
                    max={100}
                    value={quotaProtectionThresholdInput}
                    onChange={(event) =>
                      setQuotaProtectionThresholdDraft(event.target.value)
                    }
                    onBlur={saveQuotaProtectionThreshold}
                  />
                  <p className="text-[10px] text-muted-foreground">
                    例如填 <code>10</code> 表示剩余额度小于等于 10% 时不再参与路由。
                  </p>
                </div>
              </div>

              <div className="flex items-center justify-between border-t pt-6">
                <div className="space-y-0.5">
                  <Label>请求体压缩</Label>
                  <p className="text-xs text-muted-foreground">
                    对齐官方 Codex：流式 <code>/responses</code> 请求发往 ChatGPT Codex backend 时，默认使用
                    <code>zstd</code> 压缩请求体。
                  </p>
                </div>
                <Switch
                  checked={snapshot.requestCompressionEnabled}
                  onCheckedChange={(value) =>
                    updateSettings.mutate({ requestCompressionEnabled: value })
                  }
                />
              </div>

              <div className="grid gap-4 rounded-2xl border border-border/50 bg-background/35 p-4">
                <div className="flex items-center justify-between gap-4">
                  <div className="space-y-0.5">
                    <div className="flex items-center gap-2">
                      <Database className="h-4 w-4 text-primary" />
                      <Label>响应缓存</Label>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      对非流式重复请求复用最近一次响应，命中时会返回
                      <code> X-CodexManager-Cache: HIT</code>。
                    </p>
                  </div>
                  <Switch
                    checked={snapshot.responseCacheEnabled}
                    onCheckedChange={(value) =>
                      updateSettings.mutate({ responseCacheEnabled: value })
                    }
                  />
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <div className="grid gap-2">
                    <Label>缓存 TTL（秒）</Label>
                    <Input
                      type="number"
                      min={1}
                      value={responseCacheTtlInput}
                      onChange={(event) => setResponseCacheTtlDraft(event.target.value)}
                      onBlur={saveResponseCacheTtl}
                    />
                    <p className="text-[10px] text-muted-foreground">
                      相同请求在 TTL 过期前可直接复用缓存结果。
                    </p>
                  </div>
                  <div className="grid gap-2">
                    <Label>最大缓存条目数</Label>
                    <Input
                      type="number"
                      min={1}
                      value={responseCacheMaxEntriesInput}
                      onChange={(event) => setResponseCacheMaxEntriesDraft(event.target.value)}
                      onBlur={saveResponseCacheMaxEntries}
                    />
                    <p className="text-[10px] text-muted-foreground">
                      达到上限后会优先淘汰最旧的缓存条目。
                    </p>
                  </div>
                </div>

                <div className="grid gap-3 md:grid-cols-4">
                  <div className="rounded-2xl border border-border/50 bg-background/40 px-4 py-3">
                    <p className="text-[10px] text-muted-foreground">当前条目</p>
                    <p className="mt-1 text-lg font-semibold">{cacheStats.entryCount}</p>
                  </div>
                  <div className="rounded-2xl border border-border/50 bg-background/40 px-4 py-3">
                    <p className="text-[10px] text-muted-foreground">命中率</p>
                    <p className="mt-1 text-lg font-semibold">
                      {cacheStats.hitRatePercent.toFixed(cacheStats.hitRatePercent >= 10 ? 1 : 2)}%
                    </p>
                  </div>
                  <div className="rounded-2xl border border-border/50 bg-background/40 px-4 py-3">
                    <p className="text-[10px] text-muted-foreground">Hit / Miss</p>
                    <p className="mt-1 text-lg font-semibold">
                      {cacheStats.hitCount} / {cacheStats.missCount}
                    </p>
                  </div>
                  <div className="rounded-2xl border border-border/50 bg-background/40 px-4 py-3">
                    <p className="text-[10px] text-muted-foreground">估算占用</p>
                    <p className="mt-1 text-lg font-semibold">
                      {formatStorageBytes(cacheStats.estimatedBytes)}
                    </p>
                  </div>
                </div>

                <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border/50 pt-4">
                  <p className="text-[10px] text-muted-foreground">
                    当前配置：TTL {cacheStats.ttlSecs}s，容量 {cacheStats.maxEntries} 条。
                  </p>
                  <Button
                    variant="outline"
                    onClick={() => clearGatewayCache.mutate()}
                    disabled={clearGatewayCache.isPending || cacheStats.entryCount <= 0}
                  >
                    <RotateCcw className="mr-2 h-4 w-4" />
                    清空缓存
                  </Button>
                </div>
              </div>

              <div className="grid gap-4 rounded-2xl border border-border/50 bg-background/35 p-4">
                <div className="space-y-1">
                  <Label>失败重试策略</Label>
                  <p className="text-xs text-muted-foreground">
                    控制主链路在上游失败时最多切换多少次候选账号，以及每次切换前的等待策略。
                  </p>
                </div>

                <div className="grid gap-4 md:grid-cols-3">
                  <div className="grid gap-2">
                    <Label>最大重试次数</Label>
                    <Input
                      type="number"
                      min={0}
                      max={10}
                      value={retryPolicyMaxRetriesInput}
                      onChange={(event) =>
                        setRetryPolicyMaxRetriesDraft(event.target.value)
                      }
                      onBlur={saveRetryPolicyMaxRetries}
                    />
                    <p className="text-[10px] text-muted-foreground">
                      填 <code>0</code> 表示失败后不再切换候选账号。
                    </p>
                  </div>

                  <div className="grid gap-2">
                    <Label>退避策略</Label>
                    <Select
                      value={snapshot.retryPolicyBackoffStrategy || "exponential"}
                      onValueChange={(value) =>
                        updateSettings.mutate({
                          retryPolicyBackoffStrategy: value || "exponential",
                        })
                      }
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="选择退避策略">
                          {(value) =>
                            RETRY_BACKOFF_LABELS[String(value || "")] ||
                            String(value || "")
                          }
                        </SelectValue>
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="immediate">立即重试</SelectItem>
                        <SelectItem value="fixed">固定间隔</SelectItem>
                        <SelectItem value="exponential">指数退避</SelectItem>
                      </SelectContent>
                    </Select>
                    <p className="text-[10px] text-muted-foreground">
                      指数退避会随着失败次数增加逐步拉长等待时间。
                    </p>
                  </div>

                  <div className="grid gap-2">
                    <Label>可重试状态码</Label>
                    <Input
                      value={retryableStatusCodesInput}
                      onChange={(event) =>
                        setRetryableStatusCodesDraft(event.target.value)
                      }
                      onBlur={saveRetryableStatusCodes}
                      placeholder="429, 500, 502, 503"
                    />
                    <p className="text-[10px] text-muted-foreground">
                      仅这些状态码会触发候选账号切换，多个值用逗号分隔。
                    </p>
                  </div>
                </div>
              </div>

              <div className="grid gap-2 border-t pt-6">
                <Label>Originator</Label>
                <Input
                  className="h-10 max-w-md font-mono"
                  value={gatewayOriginatorInput}
                  onChange={(event) => setGatewayOriginatorDraft(event.target.value)}
                  onBlur={() => {
                    if (gatewayOriginatorDraft == null) return;
                    if (gatewayOriginatorInput === (snapshot.gatewayOriginator || "codex_cli_rs")) {
                      setGatewayOriginatorDraft(null);
                      return;
                    }
                    void updateSettings
                      .mutateAsync({ gatewayOriginator: gatewayOriginatorInput })
                      .then(() => setGatewayOriginatorDraft(null))
                      .catch(() => undefined);
                  }}
                />
                <p className="text-[10px] text-muted-foreground">
                  对齐官方 Codex 的上游 Originator。默认值为 <code>codex_cli_rs</code>，会同步影响登录和网关上游请求头。
                </p>
              </div>

              <div className="grid gap-2">
                <Label>Residency Requirement</Label>
                <Select
                  value={
                    (snapshot.gatewayResidencyRequirement ?? "") || EMPTY_RESIDENCY_OPTION
                  }
                  onValueChange={(value) =>
                    updateSettings.mutate({
                      gatewayResidencyRequirement:
                        value === EMPTY_RESIDENCY_OPTION ? "" : (value ?? ""),
                    })
                  }
                >
                  <SelectTrigger className="w-full md:w-[300px]">
                    <SelectValue placeholder="选择地域约束">
                      {(value) => {
                        const nextValue =
                          String(value || "") === EMPTY_RESIDENCY_OPTION
                            ? ""
                            : String(value || "");
                        return RESIDENCY_REQUIREMENT_LABELS[nextValue] || nextValue;
                      }}
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    {(snapshot.gatewayResidencyRequirementOptions?.length
                      ? snapshot.gatewayResidencyRequirementOptions
                      : ["", "us"]
                    ).map((value) => (
                      <SelectItem
                        key={value || EMPTY_RESIDENCY_OPTION}
                        value={value || EMPTY_RESIDENCY_OPTION}
                      >
                        {RESIDENCY_REQUIREMENT_LABELS[value] || value}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-[10px] text-muted-foreground">
                  对齐官方 Codex 的 <code>x-openai-internal-codex-residency</code> 头。
                  当前只支持留空或 <code>us</code>。
                </p>
              </div>

              <div className="flex items-center justify-between border-t pt-6">
                <div className="space-y-0.5">
                  <Label>请求头收敛策略</Label>
                  <p className="text-xs text-muted-foreground">移除高风险会话头，降低 Cloudflare 验证命中率</p>
                </div>
                <Switch
                  checked={snapshot.cpaNoCookieHeaderModeEnabled}
                  onCheckedChange={(value) =>
                    updateSettings.mutate({ cpaNoCookieHeaderModeEnabled: value })
                  }
                />
              </div>

              <div className="grid gap-2 pt-2">
                <Label>上游代理 (Proxy)</Label>
                <Input
                  placeholder="http://127.0.0.1:7890"
                  className="h-10 max-w-md font-mono"
                  value={upstreamProxyInput}
                  onChange={(event) => setUpstreamProxyDraft(event.target.value)}
                  onBlur={() => {
                    if (upstreamProxyDraft == null) return;
                    if (upstreamProxyInput === (snapshot.upstreamProxyUrl || "")) {
                      setUpstreamProxyDraft(null);
                      return;
                    }
                    void updateSettings
                      .mutateAsync({ upstreamProxyUrl: upstreamProxyInput })
                      .then(() => setUpstreamProxyDraft(null))
                      .catch(() => undefined);
                  }}
                />
                <p className="text-[10px] text-muted-foreground">支持 http/https/socks5，留空表示直连。</p>
              </div>

              <div className="grid gap-4 rounded-2xl border border-border/50 bg-background/35 p-4">
                <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                  <div className="space-y-1">
                    <Label>freeproxy 快速同步</Label>
                    <p className="text-xs text-muted-foreground">
                      从 freeproxy 公共代理池抓取代理，并写入 <code>CODEXMANAGER_PROXY_LIST</code>。
                      当前代理池共 {proxyPoolCount} 个代理。
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      variant="outline"
                      className="gap-2 self-start"
                      disabled={syncFreeProxyPool.isPending || clearFreeProxyPool.isPending}
                      onClick={() => syncFreeProxyPool.mutate()}
                    >
                      <Download className={cn("h-4 w-4", syncFreeProxyPool.isPending && "animate-pulse")} />
                      同步代理池
                    </Button>
                    <Button
                      variant="destructive"
                      className="gap-2 self-start"
                      disabled={syncFreeProxyPool.isPending || clearFreeProxyPool.isPending}
                      onClick={() => setFreeProxyClearConfirmOpen(true)}
                    >
                      <Trash2 className={cn("h-4 w-4", clearFreeProxyPool.isPending && "animate-pulse")} />
                      清空代理池
                    </Button>
                  </div>
                </div>

                <div className="grid gap-4 md:grid-cols-4">
                  <div className="grid gap-2">
                    <Label>协议</Label>
                    <Select
                      value={freeProxyProtocol}
                      onValueChange={(value) => setFreeProxyProtocol(value || "socks5")}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="选择协议" />
                      </SelectTrigger>
                      <SelectContent>
                        {FREEPROXY_PROTOCOL_OPTIONS.map((item) => (
                          <SelectItem key={item.value} value={item.value}>
                            {item.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="grid gap-2">
                    <Label>匿名度</Label>
                    <Select
                      value={freeProxyAnonymity}
                      onValueChange={(value) => setFreeProxyAnonymity(value || "elite")}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="选择匿名度" />
                      </SelectTrigger>
                      <SelectContent>
                        {FREEPROXY_ANONYMITY_OPTIONS.map((item) => (
                          <SelectItem key={item.value} value={item.value}>
                            {item.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="grid gap-2">
                    <Label>国家过滤</Label>
                    <Input
                      placeholder="US,DE,JP"
                      value={freeProxyCountry}
                      onChange={(event) => setFreeProxyCountry(event.target.value)}
                    />
                  </div>

                  <div className="grid gap-2">
                    <Label>同步数量</Label>
                    <Input
                      type="number"
                      min={1}
                      max={100}
                      value={freeProxyLimit}
                      onChange={(event) => setFreeProxyLimit(event.target.value)}
                    />
                  </div>
                </div>

                <div className="flex items-center justify-between gap-4 rounded-xl border border-border/40 bg-background/40 px-3 py-2">
                  <div className="space-y-0.5">
                    <Label>同步后清空单代理配置</Label>
                    <p className="text-[10px] text-muted-foreground">
                      开启后会自动清空上面的单个代理 URL，避免代理池被单代理覆盖。
                    </p>
                  </div>
                  <Switch
                    checked={freeProxyClearSingleProxy}
                    onCheckedChange={setFreeProxyClearSingleProxy}
                  />
                </div>

                <div className="flex items-center justify-between gap-4 rounded-xl border border-border/40 bg-background/40 px-3 py-2">
                  <div className="space-y-0.5">
                    <Label>同步到注册代理池</Label>
                    <p className="text-[10px] text-muted-foreground">
                      开启后会把同一批代理同步到注册服务的代理列表，注册账号时也会自动使用这些代理。
                    </p>
                  </div>
                  <Switch
                    checked={freeProxySyncRegisterPool}
                    onCheckedChange={setFreeProxySyncRegisterPool}
                  />
                </div>

                <p className="text-[10px] text-muted-foreground">
                  默认更适合选择 <code>Socks5 + 仅高匿</code>。国家支持多值，逗号分隔；留空表示不过滤。
                </p>

                {freeProxySyncResult ? (
                  <div className="grid gap-2 rounded-xl border border-border/40 bg-background/45 p-3">
                    <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                      <span>源更新时间：{freeProxySyncResult.sourceUpdatedAt || "未知"}</span>
                      <span>抓取总数：{freeProxySyncResult.fetchedCount}</span>
                      <span>命中：{freeProxySyncResult.matchedCount}</span>
                      <span>已写入：{freeProxySyncResult.appliedCount}</span>
                      {freeProxySyncResult.registerProxySyncEnabled ? (
                        <span>
                          注册池：总计 {freeProxySyncResult.registerProxyTotalCount}，新增{" "}
                          {freeProxySyncResult.registerProxyCreatedCount}，更新{" "}
                          {freeProxySyncResult.registerProxyUpdatedCount}
                        </span>
                      ) : null}
                    </div>
                    {freeProxySyncResult.previousUpstreamProxyUrl ? (
                      <p className="text-[10px] text-muted-foreground">
                        原单代理：
                        <code>{freeProxySyncResult.previousUpstreamProxyUrl}</code>
                        {freeProxySyncResult.clearedUpstreamProxyUrl ? "，已自动清空。" : "，目前仍保留。"}
                      </p>
                    ) : null}
                    <div className="rounded-lg bg-black/5 p-2 font-mono text-[11px] leading-5 text-muted-foreground">
                      {freeProxySyncResult.proxies.slice(0, 8).join("\n")}
                      {freeProxySyncResult.proxies.length > 8
                        ? `\n... 其余 ${freeProxySyncResult.proxies.length - 8} 个已省略`
                        : ""}
                    </div>
                  </div>
                ) : null}
                <ConfirmDialog
                  open={freeProxyClearConfirmOpen}
                  onOpenChange={setFreeProxyClearConfirmOpen}
                  title="清空代理池"
                  description="该操作会同时清空网关代理池 CODEXMANAGER_PROXY_LIST 和注册代理池，且不会自动恢复。是否继续？"
                  confirmText="确认清空"
                  confirmVariant="destructive"
                  onConfirm={() => void clearFreeProxyPool.mutateAsync()}
                />
              </div>

              <div className="grid grid-cols-2 gap-4 border-t pt-6">
                <div className="grid gap-2">
                  <Label>SSE 保活间隔 (ms)</Label>
                  <Input
                    type="number"
                    value={transportInputValues.sseKeepaliveIntervalMs}
                    onChange={(event) =>
                      setTransportDraft((current) => ({
                        ...current,
                        sseKeepaliveIntervalMs: event.target.value,
                      }))
                    }
                    onBlur={() => saveTransportField("sseKeepaliveIntervalMs", 1)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label>上游流式超时 (ms)</Label>
                  <Input
                    type="number"
                    value={transportInputValues.upstreamStreamTimeoutMs}
                    onChange={(event) =>
                      setTransportDraft((current) => ({
                        ...current,
                        upstreamStreamTimeoutMs: event.target.value,
                      }))
                    }
                    onBlur={() => saveTransportField("upstreamStreamTimeoutMs", 0)}
                  />
                </div>
              </div>

              <div className="grid gap-3 border-t pt-6">
                <div className="space-y-1">
                  <Label htmlFor="payload-rewrite-rules">声明式 Payload Rewrite 规则</Label>
                  <p className="text-xs text-muted-foreground">
                    用 JSON 数组定义网关请求体顶层字段改写规则。当前仅支持
                    <code className="mx-1">set</code>
                    和
                    <code className="mx-1">set_if_missing</code>
                    ，路径支持精确匹配或
                    <code className="mx-1">*</code>
                    ，并且禁止改写
                    <code className="mx-1">model</code>
                    。
                  </p>
                </div>
                <Textarea
                  id="payload-rewrite-rules"
                  rows={8}
                  className="font-mono text-xs leading-5"
                  placeholder={`[
  {
    "path": "/v1/responses",
    "field": "service_tier",
    "mode": "set_if_missing",
    "value": "flex"
  }
]`}
                  value={payloadRewriteRulesInput}
                  onChange={(event) => setPayloadRewriteRulesDraft(event.target.value)}
                />
                <div className="rounded-xl border border-border/40 bg-background/40 p-3 text-[11px] text-muted-foreground">
                  保存后会同步写入
                  <code className="mx-1">appSettings</code>
                  持久化配置；如需环境变量覆盖，可使用
                  <code className="mx-1">CODEXMANAGER_PAYLOAD_REWRITE_RULES</code>
                  。
                </div>
                <div className="flex flex-wrap gap-3">
                  <Button
                    className="gap-2"
                    disabled={updateSettings.isPending}
                    onClick={handleSavePayloadRewriteRules}
                  >
                    <Save className="h-4 w-4" />
                    保存 Rewrite 规则
                  </Button>
                  <Button
                    variant="outline"
                    disabled={updateSettings.isPending}
                    onClick={() => setPayloadRewriteRulesDraft(snapshot?.payloadRewriteRulesJson ?? "[]")}
                  >
                    还原当前配置
                  </Button>
                </div>
              </div>

              <div className="grid gap-3 border-t pt-6">
                <div className="space-y-1">
                  <Label htmlFor="model-alias-pools">模型别名 / 模型池</Label>
                  <p className="text-xs text-muted-foreground">
                    用 JSON 数组定义客户端可见模型名到真实模型池的映射。当前支持
                    <code className="mx-1">ordered</code>
                    和
                    <code className="mx-1">weighted</code>
                    两种策略，命中后会先选出真实模型，再继续走现有的 API Key 模型降级链。
                  </p>
                </div>
                <Textarea
                  id="model-alias-pools"
                  rows={10}
                  className="font-mono text-xs leading-5"
                  placeholder={`[
  {
    "alias": "o3-auto",
    "strategy": "weighted",
    "targets": [
      { "model": "o3", "weight": 8 },
      { "model": "o4-mini", "weight": 2 }
    ]
  }
]`}
                  value={modelAliasPoolsInput}
                  onChange={(event) => setModelAliasPoolsDraft(event.target.value)}
                />
                <div className="rounded-xl border border-border/40 bg-background/40 p-3 text-[11px] text-muted-foreground">
                  保存后会同步写入
                  <code className="mx-1">appSettings</code>
                  持久化配置；如需环境变量覆盖，可使用
                  <code className="mx-1">CODEXMANAGER_MODEL_ALIAS_POOLS</code>
                  。请求日志中的
                  <code className="mx-1">requestedModel</code>
                  会保留别名，真实上游模型不同步时会返回
                  <code className="mx-1">X-CodexManager-Actual-Model</code>
                  。
                </div>
                <div className="flex flex-wrap gap-3">
                  <Button
                    className="gap-2"
                    disabled={updateSettings.isPending}
                    onClick={handleSaveModelAliasPools}
                  >
                    <Save className="h-4 w-4" />
                    保存模型池配置
                  </Button>
                  <Button
                    variant="outline"
                    disabled={updateSettings.isPending}
                    onClick={() => setModelAliasPoolsDraft(snapshot?.modelAliasPoolsJson ?? "[]")}
                  >
                    还原当前配置
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="alerts" className="space-y-4">
          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <BellRing className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">告警通知</CardTitle>
              </div>
              <CardDescription>
                先配置通知渠道，再保存规则。当前支持 Webhook、Bark、Telegram Bot、企业微信机器人。
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid gap-6 xl:grid-cols-2">
                <Card className="border-border/50 bg-background/35 shadow-none">
                  <CardHeader className="pb-4">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <CardTitle className="text-sm">通知渠道</CardTitle>
                        <CardDescription>保存后可直接发送测试通知</CardDescription>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setSelectedAlertChannelId(NEW_ALERT_DRAFT_ID);
                          setAlertChannelDraft(createAlertChannelDraft());
                        }}
                      >
                        <PlusCircle className="mr-2 h-4 w-4" />
                        新建
                      </Button>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="flex flex-wrap gap-2">
                      {alertChannels.length ? (
                        alertChannels.map((item) => (
                          <button
                            key={item.id}
                            type="button"
                            onClick={() => {
                              setSelectedAlertChannelId(item.id);
                              setAlertChannelDraft(createAlertChannelDraft(item));
                            }}
                            className={cn(
                              "rounded-xl border px-3 py-2 text-left text-sm transition-colors",
                              selectedAlertChannel?.id === item.id
                                ? "border-primary bg-primary/10 text-foreground"
                                : "border-border/50 bg-background/40 text-muted-foreground hover:text-foreground"
                            )}
                          >
                            <div className="font-medium">{item.name}</div>
                            <div className="text-[11px]">
                              {ALERT_CHANNEL_TYPE_LABELS[item.channelType] || item.channelType}
                            </div>
                          </button>
                        ))
                      ) : (
                        <p className="text-sm text-muted-foreground">还没有保存任何告警渠道。</p>
                      )}
                    </div>

                    <div className="grid gap-4">
                      <div className="grid gap-2">
                        <Label>渠道名称</Label>
                        <Input
                          value={activeAlertChannelDraft.name}
                          onChange={(event) =>
                            updateAlertChannelDraft((current) => ({
                              ...current,
                              name: event.target.value,
                            }))
                          }
                          placeholder="例如：本地 Webhook / 运维 Telegram"
                        />
                      </div>

                      <div className="grid gap-2 md:grid-cols-[minmax(0,220px)_auto] md:items-end">
                        <div className="grid gap-2">
                          <Label>渠道类型</Label>
                          <Select
                            value={activeAlertChannelDraft.type}
                            onValueChange={(value) =>
                              updateAlertChannelDraft((current) => ({
                                ...current,
                                type: value || "webhook",
                                configText: formatJsonPretty(
                                  buildAlertChannelConfigPreset(value || "webhook")
                                ),
                              }))
                            }
                          >
                            <SelectTrigger>
                              <SelectValue placeholder="选择渠道类型" />
                            </SelectTrigger>
                            <SelectContent>
                              {Object.entries(ALERT_CHANNEL_TYPE_LABELS).map(([value, label]) => (
                                <SelectItem key={value} value={value}>
                                  {label}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </div>
                        <div className="flex items-center justify-between gap-3 rounded-xl border border-border/50 bg-background/40 px-3 py-2">
                          <Label>启用</Label>
                          <Switch
                            checked={activeAlertChannelDraft.enabled}
                            onCheckedChange={(value) =>
                              updateAlertChannelDraft((current) => ({
                                ...current,
                                enabled: value,
                              }))
                            }
                          />
                        </div>
                      </div>

                      <div className="grid gap-2">
                        <Label>配置 JSON</Label>
                        <Textarea
                          className="min-h-40 font-mono text-xs"
                          value={activeAlertChannelDraft.configText}
                          onChange={(event) =>
                            updateAlertChannelDraft((current) => ({
                              ...current,
                              configText: event.target.value,
                            }))
                          }
                        />
                        <p className="text-[10px] text-muted-foreground">
                          Webhook / Bark 使用 <code>{`{"url":"..."}`}</code>；
                          Telegram 使用 <code>{`{"botToken":"","chatId":""}`}</code>；
                          企业微信使用 <code>{`{"webhookUrl":"..."}`}</code>。
                        </p>
                      </div>

                      <div className="flex flex-wrap gap-3">
                        <Button onClick={handleSaveAlertChannel} disabled={saveAlertChannel.isPending}>
                          <Save className="mr-2 h-4 w-4" />
                          保存渠道
                        </Button>
                        <Button
                          variant="outline"
                          onClick={() =>
                            activeAlertChannelDraft.id &&
                            testAlertChannel.mutate(activeAlertChannelDraft.id)
                          }
                          disabled={!activeAlertChannelDraft.id || testAlertChannel.isPending}
                        >
                          <Play className="mr-2 h-4 w-4" />
                          测试发送
                        </Button>
                        <Button
                          variant="outline"
                          onClick={() =>
                            activeAlertChannelDraft.id &&
                            deleteAlertChannel.mutate(activeAlertChannelDraft.id)
                          }
                          disabled={!activeAlertChannelDraft.id || deleteAlertChannel.isPending}
                        >
                          <Trash2 className="mr-2 h-4 w-4" />
                          删除
                        </Button>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card className="border-border/50 bg-background/35 shadow-none">
                  <CardHeader className="pb-4">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <CardTitle className="text-sm">告警规则</CardTitle>
                        <CardDescription>配置触发条件、渠道绑定和静默期</CardDescription>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setSelectedAlertRuleId(NEW_ALERT_DRAFT_ID);
                          setAlertRuleDraft(createAlertRuleDraft());
                        }}
                      >
                        <PlusCircle className="mr-2 h-4 w-4" />
                        新建
                      </Button>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="flex flex-wrap gap-2">
                      {alertRules.length ? (
                        alertRules.map((item) => (
                          <button
                            key={item.id}
                            type="button"
                            onClick={() => {
                              setSelectedAlertRuleId(item.id);
                              setAlertRuleDraft(createAlertRuleDraft(item));
                            }}
                            className={cn(
                              "rounded-xl border px-3 py-2 text-left text-sm transition-colors",
                              selectedAlertRule?.id === item.id
                                ? "border-primary bg-primary/10 text-foreground"
                                : "border-border/50 bg-background/40 text-muted-foreground hover:text-foreground"
                            )}
                          >
                            <div className="font-medium">{item.name}</div>
                            <div className="text-[11px]">
                              {ALERT_RULE_TYPE_LABELS[item.ruleType] || item.ruleType}
                            </div>
                          </button>
                        ))
                      ) : (
                        <p className="text-sm text-muted-foreground">还没有保存任何告警规则。</p>
                      )}
                    </div>

                    <div className="grid gap-4">
                      <div className="grid gap-2">
                        <Label>规则名称</Label>
                        <Input
                          value={activeAlertRuleDraft.name}
                          onChange={(event) =>
                            updateAlertRuleDraft((current) => ({
                              ...current,
                              name: event.target.value,
                            }))
                          }
                          placeholder="例如：额度超过 90% / 全部账号不可用"
                        />
                      </div>

                      <div className="grid gap-2 md:grid-cols-[minmax(0,240px)_auto] md:items-end">
                        <div className="grid gap-2">
                          <Label>规则类型</Label>
                          <Select
                            value={activeAlertRuleDraft.type}
                            onValueChange={(value) =>
                              updateAlertRuleDraft((current) => ({
                                ...current,
                                type: value || "usage_threshold",
                                configText: formatJsonPretty(
                                  buildAlertRuleConfigPreset(value || "usage_threshold")
                                ),
                              }))
                            }
                          >
                            <SelectTrigger>
                              <SelectValue placeholder="选择规则类型" />
                            </SelectTrigger>
                            <SelectContent>
                              {Object.entries(ALERT_RULE_TYPE_LABELS).map(([value, label]) => (
                                <SelectItem key={value} value={value}>
                                  {label}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </div>
                        <div className="flex items-center justify-between gap-3 rounded-xl border border-border/50 bg-background/40 px-3 py-2">
                          <Label>启用</Label>
                          <Switch
                            checked={activeAlertRuleDraft.enabled}
                            onCheckedChange={(value) =>
                              updateAlertRuleDraft((current) => ({
                                ...current,
                                enabled: value,
                              }))
                            }
                          />
                        </div>
                      </div>

                      <div className="grid gap-2">
                        <Label>配置 JSON</Label>
                        <Textarea
                          className="min-h-40 font-mono text-xs"
                          value={activeAlertRuleDraft.configText}
                          onChange={(event) =>
                            updateAlertRuleDraft((current) => ({
                              ...current,
                              configText: event.target.value,
                            }))
                          }
                        />
                        <p className="text-[10px] text-muted-foreground">
                          常见字段建议：<code>thresholdPercent</code>、<code>windowMinutes</code>、
                          <code>threshold</code>、<code>channelIds</code>、<code>cooldownSecs</code>。
                        </p>
                      </div>

                      <div className="flex flex-wrap gap-3">
                        <Button onClick={handleSaveAlertRule} disabled={saveAlertRule.isPending}>
                          <Save className="mr-2 h-4 w-4" />
                          保存规则
                        </Button>
                        <Button
                          variant="outline"
                          onClick={() =>
                            activeAlertRuleDraft.id &&
                            deleteAlertRule.mutate(activeAlertRuleDraft.id)
                          }
                          disabled={!activeAlertRuleDraft.id || deleteAlertRule.isPending}
                        >
                          <Trash2 className="mr-2 h-4 w-4" />
                          删除
                        </Button>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              </div>

              <Card className="border-border/50 bg-background/35 shadow-none">
                <CardHeader>
                  <div className="flex items-center gap-2">
                    <History className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">最近告警历史</CardTitle>
                  </div>
                  <CardDescription>展示最近 50 条测试与触发记录</CardDescription>
                </CardHeader>
                <CardContent>
                  {alertHistory.length ? (
                    <Table>
                      <TableHeader>
                        <TableRow className="border-border/50">
                          <TableHead>时间</TableHead>
                          <TableHead>状态</TableHead>
                          <TableHead>规则</TableHead>
                          <TableHead>渠道</TableHead>
                          <TableHead className="w-full">说明</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {alertHistory.map((item) => (
                          <TableRow key={item.id} className="border-border/30">
                            <TableCell>
                              {item.createdAt ? formatTsFromSeconds(item.createdAt, "--") : "--"}
                            </TableCell>
                            <TableCell>
                              <Badge variant={statusBadgeVariant(item.status)}>{item.status}</Badge>
                            </TableCell>
                            <TableCell>{item.ruleName || "--"}</TableCell>
                            <TableCell>{item.channelName || "--"}</TableCell>
                            <TableCell className="whitespace-normal">{item.message}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      暂无告警历史，保存渠道后可先发送一条测试通知验证链路。
                    </p>
                  )}
                </CardContent>
              </Card>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="plugins" className="space-y-4">
          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <PlugZap className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">插件 / Hook 管理</CardTitle>
              </div>
              <CardDescription>
                管理插件元数据、Hook 点声明与脚本内容。当前先接通插件配置 CRUD，Lua 执行沙箱仍待后端收口。
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid gap-6 xl:grid-cols-[320px_minmax(0,1fr)]">
                <Card className="border-border/50 bg-background/35 shadow-none">
                  <CardHeader className="pb-4">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <CardTitle className="text-sm">已保存插件</CardTitle>
                        <CardDescription>可直接切换编辑，状态随保存生效</CardDescription>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setSelectedPluginId(NEW_PLUGIN_DRAFT_ID);
                          setPluginDraft(createPluginDraft());
                        }}
                      >
                        <PlusCircle className="mr-2 h-4 w-4" />
                        新建
                      </Button>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {plugins.length ? (
                      plugins.map((item) => (
                        <button
                          key={item.id}
                          type="button"
                          onClick={() => {
                            setSelectedPluginId(item.id);
                            setPluginDraft(createPluginDraft(item));
                          }}
                          className={cn(
                            "w-full rounded-2xl border px-4 py-3 text-left transition-colors",
                            selectedPlugin?.id === item.id
                              ? "border-primary bg-primary/10"
                              : "border-border/50 bg-background/40 hover:bg-accent/30"
                          )}
                        >
                          <div className="flex items-start justify-between gap-3">
                            <div className="space-y-1">
                              <div className="font-medium">{item.name}</div>
                              <p className="text-xs text-muted-foreground">
                                {item.description || "未填写描述"}
                              </p>
                            </div>
                            <Badge variant={item.enabled ? "default" : "secondary"}>
                              {item.enabled ? "启用" : "停用"}
                            </Badge>
                          </div>
                          <div className="mt-3 flex flex-wrap gap-2 text-[11px] text-muted-foreground">
                            <span>{PLUGIN_RUNTIME_LABELS[item.runtime] || item.runtime}</span>
                            <span>·</span>
                            <span>{item.hookPoints.join(", ") || "--"}</span>
                          </div>
                        </button>
                      ))
                    ) : (
                      <div className="rounded-2xl border border-dashed border-border/60 bg-background/20 p-4 text-sm text-muted-foreground">
                        还没有插件配置。可直接使用右侧模板创建一个初始脚本。
                      </div>
                    )}
                  </CardContent>
                </Card>

                <Card className="border-border/50 bg-background/35 shadow-none">
                  <CardHeader className="pb-4">
                    <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                      <div>
                        <CardTitle className="text-sm">插件编辑器</CardTitle>
                        <CardDescription>
                          选择 Hook 点、脚本超时和 Lua 脚本内容，保存后会持久化到插件注册表。
                        </CardDescription>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        {PLUGIN_TEMPLATES.map((template) => (
                          <Button
                            key={template.id}
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => applyPluginTemplate(template.id)}
                          >
                            {template.name}
                          </Button>
                        ))}
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-5">
                    <div className="grid gap-4 md:grid-cols-2">
                      <div className="grid gap-2">
                        <Label>插件名称</Label>
                        <Input
                          value={activePluginDraft.name}
                          onChange={(event) =>
                            updatePluginDraft((current) => ({
                              ...current,
                              name: event.target.value,
                            }))
                          }
                          placeholder="例如：额度守卫 / 审计增强"
                        />
                      </div>
                      <div className="grid gap-2">
                        <Label>运行时</Label>
                        <Select
                          value={activePluginDraft.runtime}
                          onValueChange={(value) =>
                            updatePluginDraft((current) => ({
                              ...current,
                              runtime: value || "lua",
                            }))
                          }
                        >
                          <SelectTrigger>
                            <SelectValue placeholder="选择运行时" />
                          </SelectTrigger>
                          <SelectContent>
                            {Object.entries(PLUGIN_RUNTIME_LABELS).map(([value, label]) => (
                              <SelectItem key={value} value={value}>
                                {label}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                    </div>

                    <div className="grid gap-2">
                      <Label>说明</Label>
                      <Input
                        value={activePluginDraft.description}
                        onChange={(event) =>
                          updatePluginDraft((current) => ({
                            ...current,
                            description: event.target.value,
                          }))
                        }
                        placeholder="描述插件目的、风险边界或适用场景"
                      />
                    </div>

                    <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_180px]">
                      <div className="grid gap-2">
                        <Label>Hook 点</Label>
                        <div className="flex flex-wrap gap-2">
                          {Object.entries(PLUGIN_HOOK_POINT_LABELS).map(([value, label]) => {
                            const active = activePluginDraft.hookPoints.includes(value);
                            return (
                              <button
                                key={value}
                                type="button"
                                onClick={() =>
                                  updatePluginDraft((current) => ({
                                    ...current,
                                    hookPoints: active
                                      ? current.hookPoints.filter((item) => item !== value)
                                      : [...current.hookPoints, value],
                                  }))
                                }
                                className={cn(
                                  "rounded-xl border px-3 py-2 text-sm transition-colors",
                                  active
                                    ? "border-primary bg-primary/10 text-foreground"
                                    : "border-border/50 bg-background/40 text-muted-foreground hover:text-foreground"
                                )}
                              >
                                {label}
                              </button>
                            );
                          })}
                        </div>
                      </div>
                      <div className="grid gap-2">
                        <Label>超时 (ms)</Label>
                        <Input
                          type="number"
                          min={1}
                          max={60000}
                          value={activePluginDraft.timeoutMs}
                          onChange={(event) =>
                            updatePluginDraft((current) => ({
                              ...current,
                              timeoutMs: event.target.value,
                            }))
                          }
                        />
                      </div>
                    </div>

                    <div className="flex items-center justify-between gap-3 rounded-xl border border-border/50 bg-background/40 px-4 py-3">
                      <div className="space-y-0.5">
                        <Label>启用插件</Label>
                        <p className="text-xs text-muted-foreground">
                          关闭后仍会保留脚本与 Hook 配置，仅停止在注册表中生效。
                        </p>
                      </div>
                      <Switch
                        checked={activePluginDraft.enabled}
                        onCheckedChange={(value) =>
                          updatePluginDraft((current) => ({
                            ...current,
                            enabled: value,
                          }))
                        }
                      />
                    </div>

                    <div className="grid gap-2">
                      <Label>Lua 脚本</Label>
                      <Textarea
                        className="min-h-80 font-mono text-xs"
                        value={activePluginDraft.scriptContent}
                        onChange={(event) =>
                          updatePluginDraft((current) => ({
                            ...current,
                            scriptContent: event.target.value,
                          }))
                        }
                        placeholder="function handle(ctx) ... end"
                      />
                      <p className="text-[10px] text-muted-foreground">
                        模板仅提供脚手架示例；后端 Lua 执行沙箱与超时保护仍在继续实现，本页先负责真实配置持久化。
                      </p>
                    </div>

                    <div className="flex flex-wrap gap-3">
                      <Button onClick={handleSavePlugin} disabled={savePlugin.isPending}>
                        <Save className="mr-2 h-4 w-4" />
                        保存插件
                      </Button>
                      <Button
                        variant="outline"
                        onClick={() => {
                          setSelectedPluginId(NEW_PLUGIN_DRAFT_ID);
                          setPluginDraft(createPluginDraft());
                        }}
                      >
                        <RotateCcw className="mr-2 h-4 w-4" />
                        重置草稿
                      </Button>
                      <Button
                        variant="outline"
                        onClick={() =>
                          activePluginDraft.id && deletePlugin.mutate(activePluginDraft.id)
                        }
                        disabled={!activePluginDraft.id || deletePlugin.isPending}
                      >
                        <Trash2 className="mr-2 h-4 w-4" />
                        删除
                      </Button>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="tasks" className="space-y-4">
          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <CardTitle className="text-base">后台任务线程</CardTitle>
              <CardDescription>管理自动轮询和保活任务；用量轮询会跳过手动禁用账号</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {[
                {
                  label: "用量轮询线程",
                  enabledKey: "usagePollingEnabled",
                  intervalKey: "usagePollIntervalSecs",
                },
                {
                  label: "网关保活线程",
                  enabledKey: "gatewayKeepaliveEnabled",
                  intervalKey: "gatewayKeepaliveIntervalSecs",
                },
                {
                  label: "令牌刷新轮询",
                  enabledKey: "tokenRefreshPollingEnabled",
                  intervalKey: "tokenRefreshPollIntervalSecs",
                },
                {
                  label: "登录态巡检线程",
                  enabledKey: "sessionProbePollingEnabled",
                  intervalKey: "sessionProbeIntervalSecs",
                },
              ].map((task) => (
                <div
                  key={task.enabledKey}
                  className="flex items-center justify-between gap-4 rounded-lg bg-accent/20 p-3"
                >
                  <div className="flex items-center gap-3">
                    <Switch
                      checked={snapshot.backgroundTasks[task.enabledKey as keyof BackgroundTaskSettings] as boolean}
                      onCheckedChange={(value) =>
                        updateBackgroundTasks({
                          [task.enabledKey]: value,
                        } as Partial<BackgroundTaskSettings>)
                      }
                    />
                    <Label>{task.label}</Label>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground">间隔(秒)</span>
                    <Input
                      className="h-8 w-20"
                      type="number"
                      value={
                        backgroundTaskDraft[task.intervalKey] ||
                        stringifyNumber(
                          snapshot.backgroundTasks[
                            task.intervalKey as keyof BackgroundTaskSettings
                          ] as number
                        )
                      }
                      onChange={(event) =>
                        setBackgroundTaskDraft((current) => ({
                          ...current,
                          [task.intervalKey]: event.target.value,
                        }))
                      }
                      onBlur={() =>
                        saveBackgroundTaskField(
                          task.intervalKey as keyof BackgroundTaskSettings,
                          1
                        )
                      }
                    />
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <CardTitle className="text-base">登录态有效性巡检</CardTitle>
                <CardDescription>
                  周期性抽检少量账号，提前发现 401/403、停用或代理异常，不等真实请求打过去才暴露
                </CardDescription>
              </div>
              <Button
                className="sm:self-start"
                variant="outline"
                onClick={() => runHealthcheck.mutate()}
                disabled={runHealthcheck.isPending}
              >
                <RefreshCw
                  className={cn("mr-2 h-4 w-4", runHealthcheck.isPending && "animate-spin")}
                />
                {runHealthcheck.isPending ? "巡检中..." : "立即巡检"}
              </Button>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="grid gap-2 rounded-2xl border border-border/50 bg-background/30 p-4">
                  <Label>单轮抽检账号数</Label>
                  <Input
                    type="number"
                    min={1}
                    value={
                      backgroundTaskDraft.sessionProbeSampleSize ||
                      stringifyNumber(snapshot.backgroundTasks.sessionProbeSampleSize)
                    }
                    onChange={(event) =>
                      setBackgroundTaskDraft((current) => ({
                        ...current,
                        sessionProbeSampleSize: event.target.value,
                      }))
                    }
                    onBlur={() => saveBackgroundTaskField("sessionProbeSampleSize", 1)}
                  />
                  <p className="text-[10px] text-muted-foreground">
                    每轮只抽样少量账号，尽量用轻量巡检换取提前预警，避免对号池造成额外压力。
                  </p>
                </div>

                <div className="rounded-2xl border border-dashed border-border/60 bg-background/20 p-4 text-xs leading-6 text-muted-foreground">
                  巡检会复用模型列表探针判断登录态是否还能走核心鉴权链路。
                  失败结果会进入首页失败看板，也会参与现有自动治理判断。
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-3">
                <div className="rounded-2xl border border-border/50 bg-background/30 p-4">
                  <p className="text-[11px] font-medium text-muted-foreground">最近巡检时间</p>
                  <p className="mt-2 text-sm font-semibold">
                    {formatTsFromSeconds(latestHealthcheck?.finishedAt, "尚未执行")}
                  </p>
                  <p className="mt-2 text-[11px] text-muted-foreground">
                    开始于 {formatTsFromSeconds(latestHealthcheck?.startedAt, "--")}
                  </p>
                </div>
                <div className="rounded-2xl border border-border/50 bg-background/30 p-4">
                  <p className="text-[11px] font-medium text-muted-foreground">最近成功率</p>
                  <p className="mt-2 text-2xl font-bold">
                    {formatHealthcheckSuccessRate(latestHealthcheck)}
                  </p>
                  <p className="mt-2 text-[11px] text-muted-foreground">
                    成功 {latestHealthcheck?.successCount ?? 0} / 抽检{" "}
                    {latestHealthcheck?.sampledAccounts ?? 0}
                  </p>
                </div>
                <div className="rounded-2xl border border-border/50 bg-background/30 p-4">
                  <p className="text-[11px] font-medium text-muted-foreground">最近失败账号</p>
                  <p className="mt-2 text-2xl font-bold">
                    {latestHealthcheck?.failureCount ?? 0}
                  </p>
                  <p className="mt-2 text-[11px] text-muted-foreground">
                    总候选 {latestHealthcheck?.totalAccounts ?? 0} 个
                  </p>
                </div>
              </div>

              <div className="rounded-2xl border border-border/50 bg-background/25 p-4">
                <p className="text-[11px] font-medium text-muted-foreground">失败账号摘要</p>
                {latestHealthcheck?.failedAccounts?.length ? (
                  <div className="mt-3 flex flex-wrap gap-2">
                    {latestHealthcheck.failedAccounts.slice(0, 6).map((item) => (
                      <span
                        key={`${item.accountId}-${item.reason}`}
                        className="rounded-full border border-amber-500/30 bg-amber-500/10 px-3 py-1 text-[11px] text-amber-700 dark:text-amber-300"
                        title={item.reason}
                      >
                        {(item.label || item.accountId).trim()}
                      </span>
                    ))}
                  </div>
                ) : (
                  <p className="mt-3 text-xs text-muted-foreground">
                    最近一次巡检没有发现失败账号。
                  </p>
                )}
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <CardTitle className="text-base">账号池自动补号</CardTitle>
              <CardDescription>
                当高于额度门槛的可用账号数量过少时，后台会自动调用注册服务补充账号池
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/50 bg-background/35 p-4">
                <div className="space-y-0.5">
                  <Label>启用自动补号</Label>
                  <p className="text-xs text-muted-foreground">
                    触发后会自动注册并导入新账号；若已有注册任务在跑，则本轮跳过避免冲突。
                  </p>
                </div>
                <Switch
                  checked={snapshot.backgroundTasks.autoRegisterPoolEnabled}
                  onCheckedChange={(value) =>
                    updateBackgroundTasks({ autoRegisterPoolEnabled: value })
                  }
                />
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="grid gap-2 rounded-2xl border border-border/50 bg-background/30 p-4">
                  <Label>触发保底账号数</Label>
                  <Input
                    type="number"
                    min={1}
                    value={
                      backgroundTaskDraft.autoRegisterReadyAccountCount ||
                      stringifyNumber(snapshot.backgroundTasks.autoRegisterReadyAccountCount)
                    }
                    onChange={(event) =>
                      setBackgroundTaskDraft((current) => ({
                        ...current,
                        autoRegisterReadyAccountCount: event.target.value,
                      }))
                    }
                    onBlur={() =>
                      saveBackgroundTaskField("autoRegisterReadyAccountCount", 1)
                    }
                  />
                  <p className="text-[10px] text-muted-foreground">
                    当“满足额度门槛的可用账号数”小于等于这个值时触发补号。
                  </p>
                </div>

                <div className="grid gap-2 rounded-2xl border border-border/50 bg-background/30 p-4">
                  <Label>可用额度门槛 (%)</Label>
                  <Input
                    type="number"
                    min={0}
                    max={100}
                    value={
                      backgroundTaskDraft.autoRegisterReadyRemainPercent ||
                      stringifyNumber(snapshot.backgroundTasks.autoRegisterReadyRemainPercent)
                    }
                    onChange={(event) =>
                      setBackgroundTaskDraft((current) => ({
                        ...current,
                        autoRegisterReadyRemainPercent: event.target.value,
                      }))
                    }
                    onBlur={() =>
                      saveBackgroundTaskField("autoRegisterReadyRemainPercent", 0, 100)
                    }
                  />
                  <p className="text-[10px] text-muted-foreground">
                    只有剩余额度大于等于该百分比的账号，才会计入可用账号数。
                  </p>
                </div>
              </div>

              <div className="rounded-2xl border border-dashed border-border/60 bg-background/20 p-4 text-xs leading-6 text-muted-foreground">
                例如：保底账号数设为 <code>3</code>，额度门槛设为 <code>20%</code>，
                当系统检测到仅剩 3 个或更少账号仍有至少 20% 剩余额度时，就会自动注册新号补充池子。
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <CardTitle className="text-base">风险账号自动治理</CardTitle>
              <CardDescription>
                后台轮询会扫描近期高风险失败事件，只对高置信度异常账号自动停用或标记停用
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/50 bg-background/35 p-4">
                <div className="space-y-0.5">
                  <Label>启用自动治理</Label>
                  <p className="text-xs text-muted-foreground">
                    目前只处理三类高风险情况：账号已停用、Refresh 连续失效、低健康分下连续 401/403。
                  </p>
                </div>
                <Switch
                  checked={snapshot.backgroundTasks.autoDisableRiskyAccountsEnabled}
                  onCheckedChange={(value) =>
                    updateBackgroundTasks({ autoDisableRiskyAccountsEnabled: value })
                  }
                />
              </div>

              <div className="grid gap-4 md:grid-cols-3">
                <div className="grid gap-2 rounded-2xl border border-border/50 bg-background/30 p-4">
                  <Label>失败次数阈值</Label>
                  <Input
                    type="number"
                    min={1}
                    value={
                      backgroundTaskDraft.autoDisableRiskyAccountsFailureThreshold ||
                      stringifyNumber(
                        snapshot.backgroundTasks.autoDisableRiskyAccountsFailureThreshold
                      )
                    }
                    onChange={(event) =>
                      setBackgroundTaskDraft((current) => ({
                        ...current,
                        autoDisableRiskyAccountsFailureThreshold: event.target.value,
                      }))
                    }
                    onBlur={() =>
                      saveBackgroundTaskField(
                        "autoDisableRiskyAccountsFailureThreshold",
                        1
                      )
                    }
                  />
                  <p className="text-[10px] text-muted-foreground">
                    Refresh 连续失效，或 401/403 连续出现达到这个次数后才会进入治理判断。
                  </p>
                </div>

                <div className="grid gap-2 rounded-2xl border border-border/50 bg-background/30 p-4">
                  <Label>健康分阈值</Label>
                  <Input
                    type="number"
                    min={1}
                    max={200}
                    value={
                      backgroundTaskDraft.autoDisableRiskyAccountsHealthScoreThreshold ||
                      stringifyNumber(
                        snapshot.backgroundTasks.autoDisableRiskyAccountsHealthScoreThreshold
                      )
                    }
                    onChange={(event) =>
                      setBackgroundTaskDraft((current) => ({
                        ...current,
                        autoDisableRiskyAccountsHealthScoreThreshold: event.target.value,
                      }))
                    }
                    onBlur={() =>
                      saveBackgroundTaskField(
                        "autoDisableRiskyAccountsHealthScoreThreshold",
                        1,
                        200
                      )
                    }
                  />
                  <p className="text-[10px] text-muted-foreground">
                    只有当账号健康分低于等于该值时，连续 401/403 才会被自动停用。
                  </p>
                </div>

                <div className="grid gap-2 rounded-2xl border border-border/50 bg-background/30 p-4">
                  <Label>回看窗口 (分钟)</Label>
                  <Input
                    type="number"
                    min={1}
                    value={
                      backgroundTaskDraft.autoDisableRiskyAccountsLookbackMins ||
                      stringifyNumber(
                        snapshot.backgroundTasks.autoDisableRiskyAccountsLookbackMins
                      )
                    }
                    onChange={(event) =>
                      setBackgroundTaskDraft((current) => ({
                        ...current,
                        autoDisableRiskyAccountsLookbackMins: event.target.value,
                      }))
                    }
                    onBlur={() =>
                      saveBackgroundTaskField("autoDisableRiskyAccountsLookbackMins", 1)
                    }
                  />
                  <p className="text-[10px] text-muted-foreground">
                    统计最近这段时间内的失败事件，避免用很久以前的异常误伤当前账号。
                  </p>
                </div>
              </div>

              <div className="rounded-2xl border border-dashed border-border/60 bg-background/20 p-4 text-xs leading-6 text-muted-foreground">
                自动治理是保守策略：
                检测到“账号已停用”会直接标记为已停用；
                Refresh 连续失效会自动禁用；
                401/403 只有在连续触发且健康分已经很低时才会禁用。
                429 限流不会自动禁用，避免把短时抖动账号误杀。
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <CardTitle className="text-base">账号冷却期</CardTitle>
              <CardDescription>
                当账号触发认证失败、限流、低配额或停用时，先冷却一段时间再重新参与路由，避免坏账号被短时间反复命中
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                {[
                  {
                    key: "accountCooldownAuthSecs",
                    label: "认证异常冷却 (秒)",
                    description: "401/403 或 challenge 场景使用，建议设成几分钟。",
                  },
                  {
                    key: "accountCooldownRateLimitedSecs",
                    label: "429 限流首档 (秒)",
                    description: "首次 429 的冷却时长；连续触发仍会自动叠加到更长梯度。",
                  },
                  {
                    key: "accountCooldownServerErrorSecs",
                    label: "5xx 冷却 (秒)",
                    description: "上游服务端异常时的基础冷却时间。",
                  },
                  {
                    key: "accountCooldownNetworkSecs",
                    label: "网络异常冷却 (秒)",
                    description: "连接超时、代理抖动、流中断等网络问题使用。",
                  },
                  {
                    key: "accountCooldownLowQuotaSecs",
                    label: "低配额冷却 (秒)",
                    description: "用量刷新判定低配额或额度耗尽后使用，避免继续打到快见底账号。",
                  },
                  {
                    key: "accountCooldownDeactivatedSecs",
                    label: "停用账号冷却 (秒)",
                    description: "命中 deactivated 后使用，建议设成较长时间。",
                  },
                ].map((item) => (
                  <div
                    key={item.key}
                    className="grid gap-2 rounded-2xl border border-border/50 bg-background/30 p-4"
                  >
                    <Label>{item.label}</Label>
                    <Input
                      type="number"
                      min={0}
                      value={
                        backgroundTaskDraft[item.key] ||
                        stringifyNumber(
                          snapshot.backgroundTasks[
                            item.key as keyof BackgroundTaskSettings
                          ] as number
                        )
                      }
                      onChange={(event) =>
                        setBackgroundTaskDraft((current) => ({
                          ...current,
                          [item.key]: event.target.value,
                        }))
                      }
                      onBlur={() =>
                        saveBackgroundTaskField(
                          item.key as keyof BackgroundTaskSettings,
                          0
                        )
                      }
                    />
                    <p className="text-[10px] leading-5 text-muted-foreground">
                      {item.description}
                    </p>
                  </div>
                ))}
              </div>

              <div className="rounded-2xl border border-dashed border-border/60 bg-background/20 p-4 text-xs leading-6 text-muted-foreground">
                低配额与停用状态现在会显式进入冷却队列。
                如果你想更激进地绕开异常账号，可以把认证异常和低配额冷却设长一点；
                设为 <code>0</code> 则表示该类失败不额外施加冷却秒数。
              </div>
            </CardContent>
          </Card>

          <Card className="glass-card border-none shadow-md">
            <CardHeader>
              <CardTitle className="text-base">Worker 并发参数</CardTitle>
              <CardDescription>调整执行单元并发规模；用量刷新并发会直接影响手动刷新和后台轮询</CardDescription>
            </CardHeader>
            <CardContent className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {[
                { label: "用量刷新并发", key: "usageRefreshWorkers" },
                { label: "HTTP 因子", key: "httpWorkerFactor" },
                { label: "HTTP 最小并发", key: "httpWorkerMin" },
                { label: "流式因子", key: "httpStreamWorkerFactor" },
                { label: "流式最小并发", key: "httpStreamWorkerMin" },
              ].map((worker) => (
                <div key={worker.key} className="grid gap-1.5">
                  <Label className="text-xs">{worker.label}</Label>
                  <Input
                    type="number"
                    className="h-9"
                    value={
                      backgroundTaskDraft[worker.key] ||
                      stringifyNumber(
                        snapshot.backgroundTasks[
                          worker.key as keyof BackgroundTaskSettings
                        ] as number
                      )
                    }
                    onChange={(event) =>
                      setBackgroundTaskDraft((current) => ({
                        ...current,
                        [worker.key]: event.target.value,
                      }))
                    }
                    onBlur={() =>
                      saveBackgroundTaskField(worker.key as keyof BackgroundTaskSettings, 1)
                    }
                  />
                </div>
              ))}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="env" className="space-y-4">
          <div className="grid gap-6 md:grid-cols-[300px_1fr]">
            <Card className="glass-card flex h-[500px] flex-col border-none shadow-md">
              <CardHeader className="pb-3">
                <div className="relative">
                  <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                  <Input
                    placeholder="搜索变量..."
                    className="h-9 pl-9"
                    value={envSearch}
                    onChange={(event) => setEnvSearch(event.target.value)}
                  />
                </div>
              </CardHeader>
              <CardContent className="flex-1 overflow-y-auto p-2">
                <div className="space-y-1">
                  {filteredEnvCatalog.map((item) => (
                    <button
                      key={item.key}
                      onClick={() => setSelectedEnvKey(item.key)}
                      className={cn(
                        "w-full rounded-md px-3 py-2 text-left text-sm transition-colors",
                        selectedEnvKey === item.key
                          ? "bg-primary text-primary-foreground"
                          : "hover:bg-accent"
                      )}
                    >
                      <div className="truncate font-medium">{item.label}</div>
                      <code className="block truncate text-[10px] opacity-70">{item.key}</code>
                    </button>
                  ))}
                </div>
              </CardContent>
            </Card>

            <Card className="glass-card min-h-[500px] border-none shadow-md">
              {selectedEnvKey ? (
                <>
                  <CardHeader>
                    <div className="flex flex-col gap-1">
                      <CardTitle className="text-lg">{selectedEnvItem?.label}</CardTitle>
                      <code className="w-fit rounded bg-primary/10 px-2 py-0.5 text-xs text-primary">
                        {selectedEnvKey}
                      </code>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-6">
                    <div className="rounded-lg border bg-accent/30 p-4 text-sm leading-relaxed text-muted-foreground">
                      <Info className="mr-2 inline-block h-4 w-4 text-primary" />
                      {ENV_DESCRIPTION_MAP[selectedEnvKey] ||
                        `${selectedEnvItem?.label} 对应环境变量，修改后会应用到相关模块。`}
                    </div>

                    <div className="space-y-2">
                      <Label>当前值</Label>
                      <Input
                        value={selectedEnvValue}
                        onChange={(event) => {
                          if (!selectedEnvKey) return;
                          setEnvDrafts((current) => ({
                            ...current,
                            [selectedEnvKey]: event.target.value,
                          }));
                        }}
                        className="h-11 font-mono"
                        placeholder="输入变量值"
                      />
                      <p className="text-[10px] text-muted-foreground">
                        默认值:{" "}
                        <span className="font-mono italic">
                          {selectedEnvItem?.defaultValue || "空"}
                        </span>
                      </p>
                    </div>

                    <div className="flex gap-3 border-t pt-4">
                      <Button onClick={handleSaveEnv} className="gap-2">
                        <Save className="h-4 w-4" /> 保存修改
                      </Button>
                      <Button variant="outline" onClick={handleResetEnv} className="gap-2">
                        <RotateCcw className="h-4 w-4" /> 恢复默认
                      </Button>
                    </div>
                  </CardContent>
                </>
              ) : (
                <CardContent className="flex h-full flex-col items-center justify-center gap-4 text-muted-foreground">
                  <div className="rounded-full bg-accent/30 p-4">
                    <Variable className="h-12 w-12 opacity-20" />
                  </div>
                  <p>请从左侧列表选择一个环境变量进行配置</p>
                </CardContent>
              )}
            </Card>
          </div>
        </TabsContent>
      </Tabs>

      <Dialog
        open={updateDialogOpen}
        onOpenChange={(open) => {
          if (prepareUpdate.isPending || applyPreparedUpdate.isPending) {
            return;
          }
          setUpdateDialogOpen(open);
        }}
      >
        <DialogContent
          showCloseButton={false}
          className="glass-card border-none p-6 sm:max-w-[480px]"
        >
          <DialogHeader>
            <DialogTitle>{preparedUpdate ? "更新已准备完成" : "发现新版本"}</DialogTitle>
            <DialogDescription>
              {preparedUpdate
                ? preparedUpdate.isPortable
                  ? "更新包已下载完成。确认后将重启应用并替换当前程序。"
                  : "安装包已下载完成。确认后会启动系统安装程序。"
                : `当前版本 ${updateDialogCheck?.currentVersion || "未知"}，发现新版本 ${
                    updateDialogCheck?.latestVersion ||
                    updateDialogCheck?.releaseTag ||
                    "可用"
                  }。`}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3 text-sm">
            <div className="rounded-2xl border border-border/50 bg-background/45 p-4">
              <div className="flex items-center justify-between gap-4">
                <span className="text-muted-foreground">当前版本</span>
                <span className="font-medium">
                  {updateDialogCheck?.currentVersion || "未知"}
                </span>
              </div>
              <div className="mt-2 flex items-center justify-between gap-4">
                <span className="text-muted-foreground">目标版本</span>
                <span className="font-medium">
                  {preparedUpdate?.latestVersion ||
                    updateDialogCheck?.latestVersion ||
                    updateDialogCheck?.releaseTag ||
                    "未知"}
                </span>
              </div>
              <div className="mt-2 flex items-center justify-between gap-4">
                <span className="text-muted-foreground">更新模式</span>
                <span className="font-medium">
                  {(preparedUpdate?.isPortable ?? updateDialogCheck?.isPortable)
                    ? "便携包更新"
                    : "安装包更新"}
                </span>
              </div>
              {preparedUpdate?.assetName ? (
                <div className="mt-2 flex items-center justify-between gap-4">
                  <span className="text-muted-foreground">更新文件</span>
                  <span className="max-w-[240px] truncate font-mono text-xs">
                    {preparedUpdate.assetName}
                  </span>
                </div>
              ) : null}
            </div>

            {preparedUpdate ? null : updateDialogCheck?.reason ? (
              <div className="rounded-2xl border border-border/50 bg-muted/40 p-4 text-xs leading-5 text-muted-foreground">
                {updateDialogCheck.reason}
              </div>
            ) : (
              <div className="rounded-2xl border border-border/50 bg-muted/40 p-4 text-xs leading-5 text-muted-foreground">
                建议先下载更新包，下载完成后再执行安装或重启更新。
              </div>
            )}
          </div>

          <DialogFooter className="gap-2 sm:gap-2">
            <Button
              variant="outline"
              disabled={prepareUpdate.isPending || applyPreparedUpdate.isPending}
              onClick={() => setUpdateDialogOpen(false)}
            >
              稍后
            </Button>
            {preparedUpdate ? (
              <Button
                className="gap-2"
                disabled={applyPreparedUpdate.isPending}
                onClick={() =>
                  applyPreparedUpdate.mutate({ isPortable: preparedUpdate.isPortable })
                }
              >
                <Download className="h-4 w-4" />
                {applyPreparedUpdate.isPending
                  ? preparedUpdate.isPortable
                    ? "正在应用更新..."
                    : "正在启动安装程序..."
                  : preparedUpdate.isPortable
                    ? "重启并更新"
                    : "立即安装"}
              </Button>
            ) : updateDialogCheck?.canPrepare ? (
              <Button
                className="gap-2"
                disabled={prepareUpdate.isPending}
                onClick={() => prepareUpdate.mutate()}
              >
                <Download className="h-4 w-4" />
                {prepareUpdate.isPending ? "正在下载更新..." : "下载更新"}
              </Button>
            ) : (
              <Button className="gap-2" onClick={handleOpenReleasePage}>
                <ExternalLink className="h-4 w-4" />
                打开发布页
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
