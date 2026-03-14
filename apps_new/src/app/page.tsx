"use client";

import { useCallback, useEffect, useId, useMemo, useRef, useState, type CSSProperties } from "react";
import {
  Activity,
  Banknote,
  BadgeCheck,
  Copy,
  Download,
  FolderOpen,
  Gauge,
  Import,
  KeyRound,
  LoaderCircle,
  Orbit,
  Palette,
  Plus,
  RefreshCw,
  Search,
  ServerCog,
  Shield,
  ShieldCheck,
  Sparkles,
  Star,
  Trash2,
  Upload,
} from "lucide-react";
import { toast } from "sonner";

import { pollLoginUntilSettled, useCodexApp } from "@/app/codex-app-provider";
import { navItems, themeOptions } from "@/lib/codex/ui-config";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
} from "@/components/ui/breadcrumb";
import {
  buildRequestRouteMeta,
  copyText,
  getApiKeyStatusMeta,
  getProtocolLabel,
  getRequestStatusTone,
  parseLoginCallbackUrl,
  REASONING_OPTIONS,
  resolveRequestDisplayPath,
  resolveRequestLogAccountLabel,
} from "@/lib/codex/app-helpers";
import { buildAccountUsageRefreshText } from "@/lib/codex/account-usage-refresh";
import {
  buildEnvOverrideDescription,
  buildEnvOverrideHint,
  buildEnvOverrideOptionLabel,
  filterEnvOverrideCatalog,
  formatEnvOverrideApplyModeLabel,
  formatEnvOverrideDisplayValue,
  formatEnvOverrideScopeLabel,
} from "@/lib/codex/env-overrides";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarRail,
  SidebarSeparator,
} from "@/components/ui/sidebar";
import { Switch } from "@/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import {
  appUpdateApplyPortable,
  appUpdateCheck,
  appUpdateLaunchInstaller,
  appUpdatePrepare,
  appUpdateStatus,
  appWindowUnsavedDraftSectionsSet,
  openInBrowser,
} from "@/lib/codex/client";
import { toPositiveInt } from "@/lib/codex/settings";
import {
  calculateAvailability,
  computeAggregateRemainingStats,
  computeUsageStats,
  formatCompactNumber,
  formatLimitLabel,
  formatResetLabel,
  formatTimestamp,
  parseCredits,
  remainingPercent,
} from "@/lib/codex/format";
import {
  buildRequestLogIdentity,
  resolveNextRequestLogRenderedCount,
  resolveRequestLogVirtualWindow,
} from "@/lib/codex/request-log-rendering";
import type {
  AccountRecord,
  AvailabilityLevel,
  PageId,
  RequestLogRecord,
  UpdateCheckResponse,
  UpdateStatusResponse,
  UsageSnapshot,
} from "@/lib/codex/types";

function availabilityToneClass(level: "ok" | "warn" | "bad" | "unknown") {
  if (level === "ok") {
    return "bg-emerald-500/15 text-emerald-700";
  }
  if (level === "warn") {
    return "bg-amber-500/15 text-amber-700";
  }
  if (level === "bad") {
    return "bg-rose-500/15 text-rose-700";
  }
  return "bg-slate-500/15 text-slate-700";
}

function requestToneClass(tone: "ok" | "warn" | "bad" | "unknown") {
  if (tone === "ok") {
    return "bg-emerald-500/15 text-emerald-700";
  }
  if (tone === "warn") {
    return "bg-amber-500/15 text-amber-700";
  }
  if (tone === "bad") {
    return "bg-rose-500/15 text-rose-700";
  }
  return "bg-slate-500/15 text-slate-700";
}

function canParticipateInRouting(level: AvailabilityLevel) {
  return level !== "warn" && level !== "bad";
}

function findRecursiveField(value: unknown, keys: string[]): unknown {
  if (!value || typeof value !== "object") {
    return undefined;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      const nested = findRecursiveField(item, keys);
      if (nested !== undefined) {
        return nested;
      }
    }
    return undefined;
  }

  const record = value as Record<string, unknown>;
  for (const key of keys) {
    if (key in record) {
      return record[key];
    }
  }
  for (const candidate of Object.values(record)) {
    const nested = findRecursiveField(candidate, keys);
    if (nested !== undefined) {
      return nested;
    }
  }
  return undefined;
}

function extractCreditsSummary(raw: string | null | undefined) {
  const parsed = parseCredits(raw);
  const balanceRaw = findRecursiveField(parsed, ["balance"]);
  const unlimitedRaw = findRecursiveField(parsed, ["unlimited", "isUnlimited"]);
  const balance =
    typeof balanceRaw === "number"
      ? balanceRaw
      : typeof balanceRaw === "string"
        ? Number.parseFloat(balanceRaw)
        : null;
  const unlimited = typeof unlimitedRaw === "boolean" ? unlimitedRaw : null;

  if ((balance == null || !Number.isFinite(balance)) && unlimited == null) {
    return null;
  }

  return {
    balance: balance != null && Number.isFinite(balance) ? balance : null,
    unlimited,
  };
}

function findLatestRequestAccountId(requestLogs: RequestLogRecord[]) {
  let latestHit: RequestLogRecord | null = null;

  for (const item of requestLogs) {
    const accountId = String(item?.accountId || "").trim();
    if (!accountId) {
      continue;
    }
    if (!latestHit || Number(item?.createdAt || 0) > Number(latestHit?.createdAt || 0)) {
      latestHit = item;
    }
  }

  return String(latestHit?.accountId || "").trim() || null;
}

function pickDashboardCurrentAccount(
  accounts: AccountRecord[],
  usageById: Map<string, UsageSnapshot>,
  requestLogs: RequestLogRecord[],
  manualPreferredAccountId: string,
) {
  if (!accounts.length) {
    return null;
  }

  const preferredId = String(manualPreferredAccountId || "").trim();
  if (preferredId) {
    const preferred = accounts.find((item) => item.id === preferredId);
    if (preferred && canParticipateInRouting(calculateAvailability(usageById.get(preferred.id), preferred).level)) {
      return preferred;
    }
  }

  const latestRequestAccountId = findLatestRequestAccountId(requestLogs);
  if (latestRequestAccountId) {
    const latestRequestAccount = accounts.find((item) => item.id === latestRequestAccountId);
    if (
      latestRequestAccount
      && canParticipateInRouting(calculateAvailability(usageById.get(latestRequestAccount.id), latestRequestAccount).level)
    ) {
      return latestRequestAccount;
    }
  }

  const firstParticipating = accounts.find((item) =>
    canParticipateInRouting(calculateAvailability(usageById.get(item.id), item).level),
  );
  if (firstParticipating) {
    return firstParticipating;
  }

  if (preferredId) {
    const preferred = accounts.find((item) => item.id === preferredId);
    if (preferred) {
      return preferred;
    }
  }

  if (latestRequestAccountId) {
    const latestRequestAccount = accounts.find((item) => item.id === latestRequestAccountId);
    if (latestRequestAccount) {
      return latestRequestAccount;
    }
  }

  return accounts[0];
}

function pickBestRecommendation(
  accounts: AccountRecord[],
  usageById: Map<string, UsageSnapshot>,
  windowKey: "primary" | "secondary",
) {
  let best: { account: AccountRecord; remain: number } | null = null;

  for (const account of accounts) {
    const usage = usageById.get(account.id);
    if (!canParticipateInRouting(calculateAvailability(usage, account).level)) {
      continue;
    }

    const remain =
      windowKey === "primary"
        ? remainingPercent(usage?.usedPercent)
        : remainingPercent(usage?.secondaryUsedPercent);

    if (remain == null) {
      continue;
    }

    if (!best || remain > best.remain) {
      best = { account, remain };
    }
  }

  return best;
}

const settingsSectionLabelMap: Record<string, string> = {
  "background-tasks": "后台任务与并发",
  "env-overrides": "高级环境变量覆盖",
  "security-settings": "安全与传输",
  "service-settings": "服务与网关策略",
};

const REQUEST_LOG_INITIAL_BATCH = 120;
const REQUEST_LOG_APPEND_BATCH = 80;
const REQUEST_LOG_SCROLL_BUFFER = 320;
const REQUEST_LOG_COLUMN_COUNT = 10;
const REQUEST_LOG_VIRTUAL_ROW_HEIGHT = 54;
const REQUEST_LOG_VIRTUAL_OVERSCAN_ROWS = 12;

function formatVersionLabel(version: string | null | undefined) {
  const clean = String(version || "").trim();
  if (!clean) {
    return "--";
  }
  return clean.startsWith("v") ? clean : `v${clean}`;
}

function buildDesktopUpdateStatusText(
  status: UpdateStatusResponse | null,
  busyState: "idle" | "checking" | "preparing" | "applying",
) {
  if (busyState === "checking") {
    return "正在检查更新...";
  }
  if (busyState === "preparing") {
    return "正在下载并准备更新...";
  }
  if (busyState === "applying") {
    return "正在启动更新流程...";
  }
  if (!status) {
    return "尚未读取更新器状态";
  }
  if (status.pending) {
    const versionLabel = formatVersionLabel(status.pending.latestVersion);
    return status.pending.isPortable
      ? `新版本 ${versionLabel} 已下载完成，可重启应用完成替换`
      : `新版本 ${versionLabel} 已准备完成，可启动安装程序`;
  }
  if (status.lastError) {
    return `上次检查失败：${status.lastError}`;
  }
  if (status.lastCheck?.hasUpdate) {
    const versionLabel = formatVersionLabel(status.lastCheck.latestVersion);
    if (status.lastCheck.canPrepare) {
      return `发现新版本 ${versionLabel}，可立即下载`;
    }
    return status.lastCheck.reason?.trim() || `发现新版本 ${versionLabel}`;
  }
  if (status.lastCheck) {
    return "当前已是最新版本";
  }
  return "尚未检查更新";
}

function resolveDesktopUpdateActionLabel(
  status: UpdateStatusResponse | null,
  busyState: "idle" | "checking" | "preparing" | "applying",
) {
  if (busyState === "checking") {
    return "检查中...";
  }
  if (busyState === "preparing") {
    return "下载中...";
  }
  if (busyState === "applying") {
    return "处理中...";
  }
  if (status?.pending) {
    return status.pending.isPortable ? "重启更新" : "启动安装";
  }
  if (status?.lastCheck?.hasUpdate && status.lastCheck.canPrepare) {
    return "下载更新";
  }
  return "检查更新";
}

export default function Home() {
  const app = useCodexApp();
  const setSettingsDraftState = app.setSettingsDraftState;
  const updateAutoCheckId = useId();
  const closeToTrayId = useId();
  const lightweightModeId = useId();
  const lowTransparencyId = useId();
  const headerModeId = useId();
  const usagePollingId = useId();
  const gatewayKeepaliveId = useId();
  const tokenRefreshId = useId();
  const autoUpdateCheckedRef = useRef(false);

  const [serviceAddrDraft, setServiceAddrDraft] = useState(app.settings.serviceAddr);
  const [serviceListenMode, setServiceListenMode] = useState(app.settings.serviceListenMode);
  const [routeStrategy, setRouteStrategy] = useState(app.settings.routeStrategy);

  const [proxyDraft, setProxyDraft] = useState(app.settings.upstreamProxyUrl);
  const [transportForm, setTransportForm] = useState({
    sseKeepaliveIntervalMs: String(app.settings.sseKeepaliveIntervalMs),
    upstreamStreamTimeoutMs: String(app.settings.upstreamStreamTimeoutMs),
  });
  const [webPassword, setWebPassword] = useState("");
  const [webPasswordConfirm, setWebPasswordConfirm] = useState("");

  const [backgroundForm, setBackgroundForm] = useState({
    usagePollingEnabled: app.settings.backgroundTasks.usagePollingEnabled,
    usagePollIntervalSecs: String(app.settings.backgroundTasks.usagePollIntervalSecs),
    gatewayKeepaliveEnabled: app.settings.backgroundTasks.gatewayKeepaliveEnabled,
    gatewayKeepaliveIntervalSecs: String(app.settings.backgroundTasks.gatewayKeepaliveIntervalSecs),
    tokenRefreshPollingEnabled: app.settings.backgroundTasks.tokenRefreshPollingEnabled,
    tokenRefreshPollIntervalSecs: String(app.settings.backgroundTasks.tokenRefreshPollIntervalSecs),
    usageRefreshWorkers: String(app.settings.backgroundTasks.usageRefreshWorkers),
    httpWorkerFactor: String(app.settings.backgroundTasks.httpWorkerFactor),
    httpWorkerMin: String(app.settings.backgroundTasks.httpWorkerMin),
    httpStreamWorkerFactor: String(app.settings.backgroundTasks.httpStreamWorkerFactor),
    httpStreamWorkerMin: String(app.settings.backgroundTasks.httpStreamWorkerMin),
  });

  const [envSearch, setEnvSearch] = useState("");
  const [selectedEnvKey, setSelectedEnvKey] = useState(app.settings.envOverrideCatalog[0]?.key ?? "");
  const [draftOverrides, setDraftOverrides] = useState(app.settings.envOverrides);
  const [desktopUpdateStatus, setDesktopUpdateStatus] = useState<UpdateStatusResponse | null>(null);
  const [desktopUpdateBusy, setDesktopUpdateBusy] = useState<"idle" | "checking" | "preparing" | "applying">("idle");

  const runtimeModeResolved = app.runtimeModeResolved;
  const browserMode = !app.canManageService;
  const closeToTraySupported = app.settings.closeToTraySupported;
  const lightweightModeAvailable = closeToTraySupported && app.settings.closeToTrayOnClose;
  const normalizedEnvSearch = envSearch.trim().toLowerCase();
  const hasEnvCatalog = app.settings.envOverrideCatalog.length > 0;

  useEffect(() => {
    setServiceAddrDraft(app.settings.serviceAddr);
    setServiceListenMode(app.settings.serviceListenMode);
    setRouteStrategy(app.settings.routeStrategy);
    setProxyDraft(app.settings.upstreamProxyUrl);
    setTransportForm({
      sseKeepaliveIntervalMs: String(app.settings.sseKeepaliveIntervalMs),
      upstreamStreamTimeoutMs: String(app.settings.upstreamStreamTimeoutMs),
    });
    setWebPassword("");
    setWebPasswordConfirm("");
    setBackgroundForm({
      usagePollingEnabled: app.settings.backgroundTasks.usagePollingEnabled,
      usagePollIntervalSecs: String(app.settings.backgroundTasks.usagePollIntervalSecs),
      gatewayKeepaliveEnabled: app.settings.backgroundTasks.gatewayKeepaliveEnabled,
      gatewayKeepaliveIntervalSecs: String(app.settings.backgroundTasks.gatewayKeepaliveIntervalSecs),
      tokenRefreshPollingEnabled: app.settings.backgroundTasks.tokenRefreshPollingEnabled,
      tokenRefreshPollIntervalSecs: String(app.settings.backgroundTasks.tokenRefreshPollIntervalSecs),
      usageRefreshWorkers: String(app.settings.backgroundTasks.usageRefreshWorkers),
      httpWorkerFactor: String(app.settings.backgroundTasks.httpWorkerFactor),
      httpWorkerMin: String(app.settings.backgroundTasks.httpWorkerMin),
      httpStreamWorkerFactor: String(app.settings.backgroundTasks.httpStreamWorkerFactor),
      httpStreamWorkerMin: String(app.settings.backgroundTasks.httpStreamWorkerMin),
    });
    setEnvSearch("");
    setSelectedEnvKey(app.settings.envOverrideCatalog[0]?.key ?? "");
    setDraftOverrides(app.settings.envOverrides);
  }, [app.settings, app.settingsSyncRevision]);

  function applyServiceDraft() {
    setServiceAddrDraft(app.settings.serviceAddr);
    setServiceListenMode(app.settings.serviceListenMode);
    setRouteStrategy(app.settings.routeStrategy);
  }

  function applySecurityDraft(settings: typeof app.settings) {
    setProxyDraft(settings.upstreamProxyUrl);
    setTransportForm({
      sseKeepaliveIntervalMs: String(settings.sseKeepaliveIntervalMs),
      upstreamStreamTimeoutMs: String(settings.upstreamStreamTimeoutMs),
    });
  }

  function applyBackgroundForm(settings: typeof app.settings) {
    setBackgroundForm({
      usagePollingEnabled: settings.backgroundTasks.usagePollingEnabled,
      usagePollIntervalSecs: String(settings.backgroundTasks.usagePollIntervalSecs),
      gatewayKeepaliveEnabled: settings.backgroundTasks.gatewayKeepaliveEnabled,
      gatewayKeepaliveIntervalSecs: String(settings.backgroundTasks.gatewayKeepaliveIntervalSecs),
      tokenRefreshPollingEnabled: settings.backgroundTasks.tokenRefreshPollingEnabled,
      tokenRefreshPollIntervalSecs: String(settings.backgroundTasks.tokenRefreshPollIntervalSecs),
      usageRefreshWorkers: String(settings.backgroundTasks.usageRefreshWorkers),
      httpWorkerFactor: String(settings.backgroundTasks.httpWorkerFactor),
      httpWorkerMin: String(settings.backgroundTasks.httpWorkerMin),
      httpStreamWorkerFactor: String(settings.backgroundTasks.httpStreamWorkerFactor),
      httpStreamWorkerMin: String(settings.backgroundTasks.httpStreamWorkerMin),
    });
  }

  function applySavedEnvValue(settings: typeof app.settings, key: string) {
    setDraftOverrides((current) => ({
      ...current,
      [key]: settings.envOverrides[key] ?? "",
    }));
  }

  const serviceAddrDirty = !browserMode && serviceAddrDraft !== app.settings.serviceAddr;
  const serviceListenModeDirty = serviceListenMode !== app.settings.serviceListenMode;
  const routeStrategyDirty = routeStrategy !== app.settings.routeStrategy;
  const hasUnsavedServiceDrafts = serviceAddrDirty || serviceListenModeDirty || routeStrategyDirty;

  useEffect(() => {
    setSettingsDraftState("service-settings", hasUnsavedServiceDrafts);
    return () => {
      setSettingsDraftState("service-settings", false);
    };
  }, [hasUnsavedServiceDrafts, setSettingsDraftState]);

  const proxyDirty = proxyDraft !== app.settings.upstreamProxyUrl;
  const transportDirty =
    transportForm.sseKeepaliveIntervalMs !== String(app.settings.sseKeepaliveIntervalMs)
    || transportForm.upstreamStreamTimeoutMs !== String(app.settings.upstreamStreamTimeoutMs);
  const passwordDraftDirty = webPassword.length > 0 || webPasswordConfirm.length > 0;
  const hasUnsavedSecurityDrafts = proxyDirty || transportDirty || passwordDraftDirty;

  useEffect(() => {
    setSettingsDraftState("security-settings", hasUnsavedSecurityDrafts);
    return () => {
      setSettingsDraftState("security-settings", false);
    };
  }, [hasUnsavedSecurityDrafts, setSettingsDraftState]);

  async function savePassword() {
    if (!webPassword) {
      toast.error("请输入新的 Web 访问密码；如需移除保护请使用“清除密码”");
      return;
    }
    if (webPassword !== webPasswordConfirm) {
      toast.error("两次输入的密码不一致");
      return;
    }
    const saved = await app.saveSettingsPatch({ webAccessPassword: webPassword }, "Web 访问密码已保存");
    if (saved) {
      setWebPassword("");
      setWebPasswordConfirm("");
    }
  }

  const dirtyFieldCount = useMemo(() => {
    const saved = app.settings.backgroundTasks;
    return [
      backgroundForm.usagePollingEnabled !== saved.usagePollingEnabled,
      backgroundForm.usagePollIntervalSecs !== String(saved.usagePollIntervalSecs),
      backgroundForm.gatewayKeepaliveEnabled !== saved.gatewayKeepaliveEnabled,
      backgroundForm.gatewayKeepaliveIntervalSecs !== String(saved.gatewayKeepaliveIntervalSecs),
      backgroundForm.tokenRefreshPollingEnabled !== saved.tokenRefreshPollingEnabled,
      backgroundForm.tokenRefreshPollIntervalSecs !== String(saved.tokenRefreshPollIntervalSecs),
      backgroundForm.usageRefreshWorkers !== String(saved.usageRefreshWorkers),
      backgroundForm.httpWorkerFactor !== String(saved.httpWorkerFactor),
      backgroundForm.httpWorkerMin !== String(saved.httpWorkerMin),
      backgroundForm.httpStreamWorkerFactor !== String(saved.httpStreamWorkerFactor),
      backgroundForm.httpStreamWorkerMin !== String(saved.httpStreamWorkerMin),
    ].filter(Boolean).length;
  }, [app.settings.backgroundTasks, backgroundForm]);

  const hasUnsavedBackgroundDrafts = dirtyFieldCount > 0;

  useEffect(() => {
    setSettingsDraftState("background-tasks", hasUnsavedBackgroundDrafts);
    return () => {
      setSettingsDraftState("background-tasks", false);
    };
  }, [hasUnsavedBackgroundDrafts, setSettingsDraftState]);

  const numericFields = [
    {
      id: "usage-poll-interval-secs",
      name: "usagePollIntervalSecs",
      label: "用量轮询间隔（秒）",
      value: backgroundForm.usagePollIntervalSecs,
      update: (value: string) => setBackgroundForm((current) => ({ ...current, usagePollIntervalSecs: value })),
    },
    {
      id: "gateway-keepalive-interval-secs",
      name: "gatewayKeepaliveIntervalSecs",
      label: "保活间隔（秒）",
      value: backgroundForm.gatewayKeepaliveIntervalSecs,
      update: (value: string) =>
        setBackgroundForm((current) => ({ ...current, gatewayKeepaliveIntervalSecs: value })),
    },
    {
      id: "token-refresh-poll-interval-secs",
      name: "tokenRefreshPollIntervalSecs",
      label: "令牌刷新间隔（秒）",
      value: backgroundForm.tokenRefreshPollIntervalSecs,
      update: (value: string) =>
        setBackgroundForm((current) => ({ ...current, tokenRefreshPollIntervalSecs: value })),
    },
    {
      id: "usage-refresh-workers",
      name: "usageRefreshWorkers",
      label: "用量刷新线程数",
      value: backgroundForm.usageRefreshWorkers,
      update: (value: string) => setBackgroundForm((current) => ({ ...current, usageRefreshWorkers: value })),
    },
    {
      id: "http-worker-factor",
      name: "httpWorkerFactor",
      label: "普通请求并发因子",
      value: backgroundForm.httpWorkerFactor,
      update: (value: string) => setBackgroundForm((current) => ({ ...current, httpWorkerFactor: value })),
    },
    {
      id: "http-worker-min",
      name: "httpWorkerMin",
      label: "普通请求最小并发",
      value: backgroundForm.httpWorkerMin,
      update: (value: string) => setBackgroundForm((current) => ({ ...current, httpWorkerMin: value })),
    },
    {
      id: "http-stream-worker-factor",
      name: "httpStreamWorkerFactor",
      label: "流式请求并发因子",
      value: backgroundForm.httpStreamWorkerFactor,
      update: (value: string) => setBackgroundForm((current) => ({ ...current, httpStreamWorkerFactor: value })),
    },
    {
      id: "http-stream-worker-min",
      name: "httpStreamWorkerMin",
      label: "流式请求最小并发",
      value: backgroundForm.httpStreamWorkerMin,
      update: (value: string) => setBackgroundForm((current) => ({ ...current, httpStreamWorkerMin: value })),
    },
  ] as const;

  const savedEnvValues = app.settings.envOverrides;
  const dirtyKeys = useMemo(
    () =>
      app.settings.envOverrideCatalog
        .map((item) => item.key)
        .filter((key) => (draftOverrides[key] ?? "") !== (savedEnvValues[key] ?? "")),
    [app.settings.envOverrideCatalog, draftOverrides, savedEnvValues],
  );
  const filteredEnvItems = useMemo(
    () => filterEnvOverrideCatalog(app.settings.envOverrideCatalog, normalizedEnvSearch),
    [app.settings.envOverrideCatalog, normalizedEnvSearch],
  );
  const activeEnvKey =
    filteredEnvItems.find((item) => item.key === selectedEnvKey)?.key ?? filteredEnvItems[0]?.key ?? "";
  const activeEnvItem = filteredEnvItems.find((item) => item.key === activeEnvKey) || null;
  const envValue = activeEnvKey ? draftOverrides[activeEnvKey] ?? "" : "";
  const savedEnvValue = activeEnvKey ? savedEnvValues[activeEnvKey] ?? "" : "";
  const defaultEnvValue = activeEnvItem?.defaultValue ?? "";
  const activeEnvDirty = !!activeEnvKey && envValue !== savedEnvValue;
  const hasUnsavedEnvDrafts = dirtyKeys.length > 0;
  const canRestoreDefault =
    !!activeEnvKey && (activeEnvDirty || savedEnvValue !== defaultEnvValue || envValue !== defaultEnvValue);
  const activeEnvHint = buildEnvOverrideHint(
    activeEnvItem,
    envValue,
    activeEnvDirty ? "当前编辑内容仍是本地草稿，尚未写入服务端。" : "",
  );

  useEffect(() => {
    setSettingsDraftState("env-overrides", hasUnsavedEnvDrafts);
    return () => {
      setSettingsDraftState("env-overrides", false);
    };
  }, [hasUnsavedEnvDrafts, setSettingsDraftState]);

  const dashboardStats = computeUsageStats(app.accounts, app.usageSnapshots);
  const dashboardAggregate = app.usageSnapshots.length
    ? app.usageAggregateSummary
    : computeAggregateRemainingStats(app.accounts, app.usageSnapshots);
  const dashboardCurrentAccount = pickDashboardCurrentAccount(
    app.accounts,
    app.accountUsageById,
    app.requestLogs,
    app.manualPreferredAccountId,
  );
  const dashboardCurrentUsage = dashboardCurrentAccount
    ? app.accountUsageById.get(dashboardCurrentAccount.id)
    : undefined;
  const dashboardCurrentAvailability = calculateAvailability(dashboardCurrentUsage, dashboardCurrentAccount);
  const dashboardPrimaryRecommendation = pickBestRecommendation(app.accounts, app.accountUsageById, "primary");
  const dashboardSecondaryRecommendation = pickBestRecommendation(app.accounts, app.accountUsageById, "secondary");
  const dashboardRankedAccounts = app.accounts
    .map((account) => {
      const usage = app.accountUsageById.get(account.id);
      return {
        account,
        availability: calculateAvailability(usage, account),
        primaryRemain:
          usage?.usedPercent == null ? null : Math.max(0, 100 - Number(usage.usedPercent || 0)),
        secondaryRemain:
          usage?.secondaryUsedPercent == null ? null : Math.max(0, 100 - Number(usage.secondaryUsedPercent || 0)),
      };
    })
    .sort((left, right) => (right.primaryRemain ?? -1) - (left.primaryRemain ?? -1))
    .slice(0, 5);

  const requestSummary = app.requestLogTodaySummary;
  const [requestLogClearOpen, setRequestLogClearOpen] = useState(false);
  const [requestLogClearPending, setRequestLogClearPending] = useState(false);
  const [requestLogRenderedCount, setRequestLogRenderedCount] = useState(REQUEST_LOG_INITIAL_BATCH);
  const requestLogNearBottomRef = useRef(false);
  const previousRequestLogKeysRef = useRef<string[]>([]);
  const accountFileInputRef = useRef<HTMLInputElement | null>(null);
  const [selectedAccountIdsState, setSelectedAccountIdsState] = useState<string[]>([]);
  const [accountAddDialogOpen, setAccountAddDialogOpen] = useState(false);
  const [usageAccountId, setUsageAccountId] = useState<string | null>(null);
  const [accountConfirmDeleteIds, setAccountConfirmDeleteIds] = useState<string[]>([]);
  const [accountConfirmPending, setAccountConfirmPending] = useState(false);
  const [accountGroupName, setAccountGroupName] = useState("TEAM");
  const [accountTags, setAccountTags] = useState("");
  const [accountNote, setAccountNote] = useState("");
  const [accountLoginUrl, setAccountLoginUrl] = useState("");
  const [accountManualCallbackUrl, setAccountManualCallbackUrl] = useState("");
  const [accountLoginHint, setAccountLoginHint] = useState("授权完成后会自动刷新账号池。");
  const [accountAddPending, setAccountAddPending] = useState(false);
  const [accountLoginId, setAccountLoginId] = useState<string | null>(null);
  const [accountUsageRefreshPending, setAccountUsageRefreshPending] = useState(false);
  const [accountSortDrafts, setAccountSortDrafts] = useState<Record<string, string>>({});
  const [pendingAccountSortIds, setPendingAccountSortIds] = useState<Record<string, boolean>>({});
  const [apiKeyCreateOpen, setApiKeyCreateOpen] = useState(false);
  const [apiKeyDeleteId, setApiKeyDeleteId] = useState<string | null>(null);
  const [apiKeyDeletePending, setApiKeyDeletePending] = useState(false);
  const [pendingApiKeyIds, setPendingApiKeyIds] = useState<Record<string, boolean>>({});
  const [apiKeyModelDrafts, setApiKeyModelDrafts] = useState<Record<string, string>>({});
  const [apiKeyReasoningDrafts, setApiKeyReasoningDrafts] = useState<Record<string, string>>({});
  const [apiKeyName, setApiKeyName] = useState("");
  const [apiKeyProtocolType, setApiKeyProtocolType] = useState("openai_compat");
  const [apiKeyModelSlug, setApiKeyModelSlug] = useState("");
  const [apiKeyReasoningEffort, setApiKeyReasoningEffort] = useState("");
  const [apiKeyUpstreamBaseUrl, setApiKeyUpstreamBaseUrl] = useState("");
  const [apiKeyAzureApiKey, setApiKeyAzureApiKey] = useState("");
  const [createdApiKeySecret, setCreatedApiKeySecret] = useState("");
  const [apiKeyCreatePending, setApiKeyCreatePending] = useState(false);
  const [confirmSyncOpen, setConfirmSyncOpen] = useState(false);
  const [confirmSyncPending, setConfirmSyncPending] = useState(false);
  const [confirmLeaveOpen, setConfirmLeaveOpen] = useState(false);
  const [pendingPage, setPendingPage] = useState<PageId | null>(null);
  const [requestLogScrollTop, setRequestLogScrollTop] = useState(0);
  const [requestLogViewportHeight, setRequestLogViewportHeight] = useState(0);

  const validAccountIdSet = useMemo(() => new Set(app.accounts.map((item) => item.id)), [app.accounts]);
  const selectedAccountIds = useMemo(
    () => selectedAccountIdsState.filter((id) => validAccountIdSet.has(id)),
    [selectedAccountIdsState, validAccountIdSet],
  );
  const selectedAccountIdSet = useMemo(() => new Set(selectedAccountIds), [selectedAccountIds]);
  const visibleAccountIds = useMemo(() => app.visibleAccounts.map((item) => item.id), [app.visibleAccounts]);
  const accountTotalPages = Math.max(1, Math.ceil(app.accountTotal / app.accountPageSize));
  const accountPageStart = app.accountTotal > 0 ? (app.accountPage - 1) * app.accountPageSize + 1 : 0;
  const accountPageEnd = app.accountTotal > 0 ? accountPageStart + app.visibleAccounts.length - 1 : 0;
  const allVisibleAccountsChecked =
    visibleAccountIds.length > 0 && visibleAccountIds.every((id) => selectedAccountIdSet.has(id));
  const usageDialogAccount = useMemo(
    () => (usageAccountId ? app.accounts.find((item) => item.id === usageAccountId) || null : null),
    [app.accounts, usageAccountId],
  );
  const visibleUsageDialogAccount = useMemo(
    () => (usageAccountId ? app.visibleAccounts.find((item) => item.id === usageAccountId) || null : null),
    [app.visibleAccounts, usageAccountId],
  );
  const selectedAccountUsageSnapshot = usageAccountId ? app.accountUsageById.get(usageAccountId) : undefined;
  const selectedAccountCreditsSummary = extractCreditsSummary(selectedAccountUsageSnapshot?.creditsJson);
  const accountUsageRefreshText = useMemo(
    () => buildAccountUsageRefreshText(app.accountUsageRefreshProgress),
    [app.accountUsageRefreshProgress],
  );

  useEffect(() => {
    setAccountSortDrafts((current) => {
      const next = { ...current };
      for (const account of app.visibleAccounts) {
        if (pendingAccountSortIds[account.id]) {
          continue;
        }
        next[account.id] = String(account.sort ?? 0);
      }
      return next;
    });
  }, [app.visibleAccounts, pendingAccountSortIds]);

  const activeApiKeyCount = app.apiKeys.filter((item) => String(item.status || "").toLowerCase() === "active").length;
  const apiKeyIsAzureProtocol = apiKeyProtocolType === "azure_openai";
  const activeTheme = themeOptions.find((item) => item.id === app.theme) || themeOptions[0];
  const requestLogKeys = useMemo(
    () => app.visibleRequestLogs.map((item, index) => buildRequestLogIdentity(item, index)),
    [app.visibleRequestLogs],
  );
  const renderedRequestLogs = useMemo(
    () => app.visibleRequestLogs.slice(0, requestLogRenderedCount),
    [app.visibleRequestLogs, requestLogRenderedCount],
  );
  const requestLogVirtualWindow = useMemo(
    () =>
      resolveRequestLogVirtualWindow({
        renderedCount: renderedRequestLogs.length,
        scrollTop: requestLogScrollTop,
        viewportHeight: requestLogViewportHeight,
        estimatedRowHeight: REQUEST_LOG_VIRTUAL_ROW_HEIGHT,
        overscanRows: REQUEST_LOG_VIRTUAL_OVERSCAN_ROWS,
      }),
    [renderedRequestLogs.length, requestLogScrollTop, requestLogViewportHeight],
  );
  const virtualRenderedRequestLogs = useMemo(
    () => renderedRequestLogs.slice(requestLogVirtualWindow.startIndex, requestLogVirtualWindow.endIndex),
    [renderedRequestLogs, requestLogVirtualWindow.endIndex, requestLogVirtualWindow.startIndex],
  );
  const hasMoreRequestLogs = renderedRequestLogs.length < app.visibleRequestLogs.length;
  const remainingRequestLogCount = Math.max(0, app.visibleRequestLogs.length - renderedRequestLogs.length);
  const latestDesktopUpdateCheck: UpdateCheckResponse | null = desktopUpdateStatus?.lastCheck || null;
  const desktopUpdateStatusText = buildDesktopUpdateStatusText(desktopUpdateStatus, desktopUpdateBusy);
  const desktopUpdateActionLabel = resolveDesktopUpdateActionLabel(desktopUpdateStatus, desktopUpdateBusy);
  const desktopUpdaterBusy = desktopUpdateBusy !== "idle";
  const settingsDraftSectionCount = app.settingsDraftSections.length;
  const shouldConfirmSettingsSync = app.currentPage === "settings" && settingsDraftSectionCount > 0;
  const dirtySectionLabels = app.settingsDraftSections.map(
    (sectionId) => settingsSectionLabelMap[sectionId] || sectionId,
  );
  const dirtySectionNames = dirtySectionLabels.join("、");

  function toggleAccountSelected(accountId: string, checked: boolean) {
    setSelectedAccountIdsState((current) =>
      checked ? Array.from(new Set([...current, accountId])) : current.filter((id) => id !== accountId),
    );
  }

  function resetAccountSelection() {
    setSelectedAccountIdsState([]);
  }

  useEffect(() => {
    if (!accountAddDialogOpen || !accountLoginId) {
      return;
    }

    const controller = new AbortController();
    void pollLoginUntilSettled(
      (options) => app.getAccountLoginStatus(accountLoginId, options),
      controller.signal,
    )
      .then(async (result) => {
        if (result.status === "success") {
          setAccountLoginHint("授权完成，正在同步账号池...");
          await app.refreshAll({ silent: true });
          toast.success("账号已写入账号池");
          setAccountAddDialogOpen(false);
          return;
        }
        if (result.error) {
          setAccountLoginHint(result.error);
        }
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        setAccountLoginHint(String(error || "授权状态检查失败"));
      });

    return () => controller.abort();
  }, [accountAddDialogOpen, accountLoginId, app]);

  useEffect(() => {
    if (app.desktopMode || !shouldConfirmSettingsSync) {
      return;
    }

    function handleBeforeUnload(event: BeforeUnloadEvent) {
      event.preventDefault();
      event.returnValue = "";
    }

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", handleBeforeUnload);
    };
  }, [app.desktopMode, shouldConfirmSettingsSync]);

  useEffect(() => {
    if (!app.desktopMode) {
      return;
    }

    const sections = shouldConfirmSettingsSync
      ? app.settingsDraftSections.map((sectionId) => settingsSectionLabelMap[sectionId] || sectionId)
      : [];

    void appWindowUnsavedDraftSectionsSet(sections).catch((error) => {
      console.warn("同步桌面端未保存设置区块失败", error);
    });

    return () => {
      void appWindowUnsavedDraftSectionsSet([]).catch((error) => {
        console.warn("清理桌面端未保存设置区块失败", error);
      });
    };
  }, [app.desktopMode, app.settingsDraftSections, shouldConfirmSettingsSync]);

  const refreshDesktopUpdateStatus = useCallback(async (options?: { silent?: boolean }) => {
    if (!app.desktopMode) {
      setDesktopUpdateStatus(null);
      return null;
    }

    try {
      const status = await appUpdateStatus();
      setDesktopUpdateStatus(status);
      return status;
    } catch (error) {
      if (!options?.silent) {
        toast.error(`读取更新器状态失败：${error instanceof Error ? error.message : String(error)}`);
      }
      return null;
    }
  }, [app.desktopMode]);

  useEffect(() => {
    if (!app.desktopMode) {
      autoUpdateCheckedRef.current = false;
      setDesktopUpdateStatus(null);
      setDesktopUpdateBusy("idle");
      return;
    }

    void refreshDesktopUpdateStatus({ silent: true });
  }, [app.desktopMode, refreshDesktopUpdateStatus]);

  useEffect(() => {
    if (!app.desktopMode || !app.settings.updateAutoCheck || autoUpdateCheckedRef.current) {
      return;
    }

    autoUpdateCheckedRef.current = true;
    const timer = window.setTimeout(() => {
      setDesktopUpdateBusy("checking");
      void appUpdateCheck()
        .then((result) => {
          const versionLabel = formatVersionLabel(result.latestVersion);
          if (result.hasUpdate) {
            toast(result.canPrepare ? `发现新版本 ${versionLabel}` : result.reason?.trim() || `发现新版本 ${versionLabel}`);
          }
          return refreshDesktopUpdateStatus({ silent: true });
        })
        .catch((error) => {
          console.warn("自动检查更新失败", error);
        })
        .finally(() => {
          setDesktopUpdateBusy("idle");
        });
    }, 1200);

    return () => window.clearTimeout(timer);
  }, [app.desktopMode, app.settings.updateAutoCheck, refreshDesktopUpdateStatus]);

  useEffect(() => {
    const previousKeys = previousRequestLogKeysRef.current;
    previousRequestLogKeysRef.current = requestLogKeys;

    const nextRenderedCount = resolveNextRequestLogRenderedCount({
      previousKeys,
      nextKeys: requestLogKeys,
      currentRenderedCount: requestLogRenderedCount,
      initialBatch: REQUEST_LOG_INITIAL_BATCH,
      appendBatch: REQUEST_LOG_APPEND_BATCH,
      wasNearBottom: requestLogNearBottomRef.current,
    });

    if (nextRenderedCount <= REQUEST_LOG_INITIAL_BATCH || !requestLogKeys.length) {
      requestLogNearBottomRef.current = false;
    }

    setRequestLogRenderedCount(nextRenderedCount);
  }, [requestLogKeys, requestLogRenderedCount]);

  const appendRequestLogBatch = useCallback((batchSize = REQUEST_LOG_APPEND_BATCH) => {
    setRequestLogRenderedCount((current) => {
      if (current >= app.visibleRequestLogs.length) {
        return current;
      }
      return Math.min(app.visibleRequestLogs.length, current + batchSize);
    });
  }, [app.visibleRequestLogs.length]);

  const handleRequestLogScrollCapture = useCallback((event: React.UIEvent<HTMLElement>) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) {
      return;
    }

    const scrollTop = Number(target.scrollTop);
    const clientHeight = Number(target.clientHeight);
    const scrollHeight = Number(target.scrollHeight);
    if (!Number.isFinite(scrollTop) || !Number.isFinite(clientHeight) || !Number.isFinite(scrollHeight)) {
      return;
    }

    setRequestLogScrollTop(scrollTop);
    setRequestLogViewportHeight(clientHeight);
    requestLogNearBottomRef.current = scrollTop + clientHeight >= scrollHeight - REQUEST_LOG_SCROLL_BUFFER;
    if (requestLogNearBottomRef.current) {
      appendRequestLogBatch();
    }
  }, [appendRequestLogBatch]);

  async function handleStartAccountLogin() {
    setAccountAddPending(true);
    setAccountLoginHint("正在生成授权链接...");
    const result = await app.startAccountLogin({
      groupName: accountGroupName,
      tags: accountTags,
      note: accountNote,
    });
    if (!result) {
      setAccountAddPending(false);
      setAccountLoginHint("启动授权失败，请检查服务状态。");
      return;
    }
    setAccountLoginUrl(result.authUrl || "");
    setAccountLoginId(result.loginId || null);
    setAccountLoginHint(result.warning || "浏览器已打开，请完成授权。");
    setAccountAddPending(false);
  }

  async function handleAccountSortCommit(account: AccountRecord) {
    const accountId = String(account.id || "").trim();
    if (!accountId) {
      return;
    }

    const draftValue = String(accountSortDrafts[accountId] ?? account.sort ?? 0).trim();
    const parsedSort = Number.parseInt(draftValue || "0", 10);
    const nextSort = Number.isFinite(parsedSort) ? parsedSort : 0;
    const previousSort = Number(account.sort ?? 0);

    setAccountSortDrafts((current) => ({
      ...current,
      [accountId]: String(nextSort),
    }));

    if (nextSort === previousSort) {
      return;
    }

    setPendingAccountSortIds((current) => ({
      ...current,
      [accountId]: true,
    }));

    const ok = await app.updateAccountSort(accountId, nextSort, previousSort);

    setPendingAccountSortIds((current) => {
      const next = { ...current };
      delete next[accountId];
      return next;
    });

    if (!ok) {
      setAccountSortDrafts((current) => ({
        ...current,
        [accountId]: String(previousSort),
      }));
    }
  }

  async function handleDesktopUpdateAction() {
    if (!app.desktopMode || desktopUpdaterBusy) {
      return;
    }

    try {
      if (desktopUpdateStatus?.pending) {
        setDesktopUpdateBusy("applying");
        if (desktopUpdateStatus.pending.isPortable) {
          const result = await appUpdateApplyPortable();
          toast.success(result.message || "便携更新已开始应用");
        } else {
          const result = await appUpdateLaunchInstaller();
          toast.success(result.message || "安装程序已启动");
        }
        await refreshDesktopUpdateStatus({ silent: true });
        return;
      }

      if (desktopUpdateStatus?.lastCheck?.hasUpdate && desktopUpdateStatus.lastCheck.canPrepare) {
        setDesktopUpdateBusy("preparing");
        const result = await appUpdatePrepare();
        toast.success(
          result.isPortable
            ? `更新已下载：${formatVersionLabel(result.latestVersion)}，可点击“重启更新”完成替换`
            : `更新已准备：${formatVersionLabel(result.latestVersion)}，可点击“启动安装”继续`,
        );
        await refreshDesktopUpdateStatus({ silent: true });
        return;
      }

      setDesktopUpdateBusy("checking");
      const result = await appUpdateCheck();
      const versionLabel = formatVersionLabel(result.latestVersion);
      if (result.hasUpdate) {
        toast.success(result.canPrepare ? `发现新版本 ${versionLabel}，再次点击即可下载` : result.reason?.trim() || `发现新版本 ${versionLabel}`);
      } else {
        toast("当前已是最新版本");
      }
      await refreshDesktopUpdateStatus({ silent: true });
    } catch (error) {
      toast.error(`更新失败：${error instanceof Error ? error.message : String(error)}`);
      await refreshDesktopUpdateStatus({ silent: true });
    } finally {
      setDesktopUpdateBusy("idle");
    }
  }

  async function handleManualAccountCallback() {
    const parsed = parseLoginCallbackUrl(accountManualCallbackUrl);
    if ("error" in parsed) {
      setAccountLoginHint(parsed.error || "回调链接格式不正确");
      return;
    }
    setAccountAddPending(true);
    const success = await app.completeAccountLogin(parsed.payload);
    setAccountAddPending(false);
    if (success) {
      setAccountAddDialogOpen(false);
    }
  }

  function setApiKeyRowPending(apiKeyId: string, pending: boolean) {
    setPendingApiKeyIds((current) => {
      const next = { ...current };
      if (pending) {
        next[apiKeyId] = true;
      } else {
        delete next[apiKeyId];
      }
      return next;
    });
  }

  function clearCreatedApiKeySecret() {
    if (createdApiKeySecret) {
      setCreatedApiKeySecret("");
    }
  }

  function requestPageChange(nextPage: PageId) {
    if (nextPage === app.currentPage) {
      return;
    }

    if (shouldConfirmSettingsSync && nextPage !== "settings") {
      setPendingPage(nextPage);
      setConfirmLeaveOpen(true);
      return;
    }

    app.setCurrentPage(nextPage);
  }

  return (
    <main className="min-h-screen">
      <SidebarProvider
        style={
          {
            "--sidebar-width": "19rem",
          } as CSSProperties
        }
      >
        <Sidebar collapsible="icon" variant="inset">
          <SidebarHeader className="gap-3 p-3">
            <div className="rounded-[1.35rem] border border-sidebar-border/70 bg-sidebar-accent/55 p-3">
              <div className="flex items-center gap-3">
                <div className="flex size-11 items-center justify-center rounded-2xl bg-sidebar-primary text-sidebar-primary-foreground shadow-sm">
                  CM
                </div>
                <div className="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
                  <div className="truncate text-sm font-semibold tracking-tight">CodexManager</div>
                  <div className="truncate text-xs text-sidebar-foreground/70">React 控制台 / 本地网关 / 账号池</div>
                </div>
                <Badge variant="secondary" className="rounded-full group-data-[collapsible=icon]:hidden">
                  <Sparkles className="size-3.5" />
                  重构态
                </Badge>
              </div>
            </div>
          </SidebarHeader>

          <SidebarContent className="gap-2 px-2 pb-2">
            <SidebarGroup className="pt-0">
              <SidebarGroupLabel>导航</SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {navItems.map((item) => {
                    const Icon = item.icon;
                    return (
                      <SidebarMenuItem key={item.page}>
                        <SidebarMenuButton
                          className="h-auto items-start gap-3 rounded-2xl px-3 py-3"
                          isActive={item.page === app.currentPage}
                          size="lg"
                          tooltip={item.title}
                          onClick={() => requestPageChange(item.page)}
                        >
                          <Icon className="mt-0.5 size-4 shrink-0" />
                          <span className="grid min-w-0 gap-1 text-left group-data-[collapsible=icon]:hidden">
                            <span className="truncate text-sm font-medium">{item.title}</span>
                            <span className="line-clamp-2 text-xs text-sidebar-foreground/70">{item.description}</span>
                          </span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    );
                  })}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>

            <SidebarGroup>
              <SidebarGroupLabel>控制</SidebarGroupLabel>
              <SidebarGroupContent className="space-y-2">
                <Card className="border-sidebar-border/70 bg-sidebar-accent/35 py-0 shadow-none">
                  <CardHeader className="px-3 pt-3 pb-0">
                    <CardTitle className="flex items-center gap-2 text-sm">
                      <ServerCog className="size-4 text-primary" />
                      本地服务
                    </CardTitle>
                    <CardDescription className="text-xs">
                      {!runtimeModeResolved
                        ? "正在检测当前运行环境与本地服务能力。"
                        : app.canManageService
                          ? "桌面端可直接启停服务，并切换本地监听地址。"
                          : "浏览器模式固定走服务端预配置的 RPC 入口。"}
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3 px-3 pb-3">
                    <Input
                      aria-label={
                        !runtimeModeResolved ? "服务入口地址" : app.canManageService ? "本地服务地址" : "当前 RPC 入口"
                      }
                      className="bg-background/70 shadow-none"
                      disabled={!runtimeModeResolved || !app.canManageService}
                      name="serviceAddress"
                      placeholder={
                        !runtimeModeResolved
                          ? "正在检测运行环境"
                          : app.canManageService
                            ? "localhost:48760"
                            : "当前 RPC 入口由服务端预配置"
                      }
                      value={app.serviceAddress}
                      onChange={(event) => app.setServiceAddress(event.target.value)}
                    />
                    <p className="text-xs leading-relaxed text-muted-foreground">
                      {!runtimeModeResolved
                        ? "运行环境检测完成后会显示可用的服务控制方式。"
                        : app.canManageService
                          ? "桌面端可直接修改本地连接地址；停止或重启服务后会按该地址重新连接。"
                          : "当前 Web 控制台通过 Next 服务端代理转发到此入口，不能在这里临时切换目标地址。"}
                    </p>
                    <div className="rounded-2xl border border-border/70 bg-background/70 px-3 py-3">
                      <div className="flex items-center justify-between gap-3">
                        <div className="grid gap-0.5">
                          <span className="text-sm font-medium">服务连接</span>
                          <span className="text-xs text-muted-foreground">
                            {app.serviceBusy ? "处理中..." : app.serviceConnected ? "已连接" : "未连接"}
                          </span>
                        </div>
                        <Switch
                          checked={app.serviceConnected}
                          disabled={!runtimeModeResolved || !app.canManageService || app.serviceBusy}
                          name="service-connection"
                          aria-label={app.serviceConnected ? "服务已连接" : "服务未连接"}
                          onCheckedChange={() => {
                            void app.toggleServiceConnection();
                          }}
                        />
                      </div>
                      <div className="mt-3 rounded-xl border border-dashed border-border/70 bg-background/60 px-3 py-2 text-xs text-muted-foreground">
                        {app.serviceHint || (app.serviceConnected ? "服务运行正常" : "等待连接")}
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card className="border-sidebar-border/70 bg-sidebar-accent/35 py-0 shadow-none">
                  <CardHeader className="px-3 pt-3 pb-0">
                    <CardTitle className="flex items-center gap-2 text-sm">
                      <Palette className="size-4 text-primary" />
                      主题
                    </CardTitle>
                    <CardDescription className="text-xs">当前主题：{activeTheme.label}</CardDescription>
                  </CardHeader>
                  <CardContent className="px-3 pb-3">
                    <DropdownMenu>
                      <DropdownMenuTrigger render={<Button className="w-full justify-between" variant="outline" />}>
                        {activeTheme.label}
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end" className="w-64">
                        <DropdownMenuRadioGroup
                          value={activeTheme.id}
                          onValueChange={(value) => {
                            void app.setTheme(value as (typeof themeOptions)[number]["id"]);
                          }}
                        >
                          {themeOptions.map((option) => (
                            <DropdownMenuRadioItem
                              key={option.id}
                              value={option.id}
                              className="flex items-start justify-between gap-3 py-2"
                            >
                              <div className="grid gap-1">
                                <span>{option.label}</span>
                                <span className="text-xs text-muted-foreground">{option.description}</span>
                              </div>
                              <span
                                aria-hidden="true"
                                className="mt-1 size-2.5 rounded-full"
                                style={{ backgroundColor: option.accent }}
                              />
                            </DropdownMenuRadioItem>
                          ))}
                        </DropdownMenuRadioGroup>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </CardContent>
                </Card>
              </SidebarGroupContent>
            </SidebarGroup>
          </SidebarContent>

          <SidebarSeparator />

          <SidebarFooter className="p-3">
            <div className="grid gap-2 rounded-[1.25rem] border border-sidebar-border/70 bg-sidebar-accent/35 px-3 py-3 text-xs text-sidebar-foreground/70">
              <div className="flex items-center gap-2 text-sidebar-foreground">
                <BadgeCheck className="size-4 text-primary" />
                {app.refreshing ? "同步中" : app.pageTitle}
              </div>
              <p className="leading-relaxed">当前页面由 React 状态驱动，已脱离旧版 DOM runtime。</p>
            </div>
          </SidebarFooter>

          <SidebarRail />
        </Sidebar>

        <SidebarInset className="cm-shell">
          <div className="grid gap-5">
            <header className="sticky top-4 z-20 rounded-[1.75rem] border border-border/70 bg-card/86 px-5 py-5 shadow-[0_18px_48px_rgba(15,23,42,0.08)] backdrop-blur">
              <div className="flex flex-col gap-5 xl:flex-row xl:items-end xl:justify-between">
                <div className="flex min-w-0 items-start gap-3">
                  <div className="min-w-0 space-y-3">
                    <Breadcrumb>
                      <BreadcrumbList>
                        <BreadcrumbItem>
                          <BreadcrumbPage>{app.pageKicker}</BreadcrumbPage>
                        </BreadcrumbItem>
                      </BreadcrumbList>
                    </Breadcrumb>
                    <div className="space-y-2">
                      <h1 className="truncate text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
                        {app.pageTitle}
                      </h1>
                      <p className="max-w-3xl text-sm leading-6 text-muted-foreground">{app.pageDescription}</p>
                    </div>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => {
                      if (shouldConfirmSettingsSync) {
                        setConfirmSyncOpen(true);
                        return;
                      }
                      void app.refreshAll();
                    }}
                  >
                    <RefreshCw className="size-4" />
                    全局同步
                  </Button>
                  {shouldConfirmSettingsSync ? (
                    <Badge variant="secondary" className="rounded-full">
                      {settingsDraftSectionCount} 个未保存区块
                    </Badge>
                  ) : null}
                  <Button type="button" variant="secondary" onClick={() => requestPageChange("settings")}>
                    <ServerCog className="size-4" />
                    打开设置
                  </Button>
                </div>
              </div>

              <div className="mt-5 grid gap-3 md:grid-cols-3">
                <div className="rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">账号池</div>
                  <div className="mt-2 flex items-center gap-2 text-2xl font-semibold">{app.accounts.length}</div>
                </div>
                <div className="rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">平台密钥</div>
                  <div className="mt-2 flex items-center gap-2 text-2xl font-semibold">
                    <KeyRound className="size-4 text-primary" />
                    {app.apiKeys.length}
                  </div>
                </div>
                <div className="rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">今日令牌</div>
                  <div className="mt-2 flex items-center gap-2 text-2xl font-semibold">
                    <Activity className="size-4 text-primary" />
                    {formatCompactNumber(app.requestLogTodaySummary.todayTokens, "0")}
                  </div>
                </div>
              </div>
            </header>

            <div className="grid gap-5">
        {app.currentPage === "dashboard" ? (
          <section className="grid gap-5">
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="flex items-start justify-between gap-3 p-5">
                  <div className="space-y-2">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">总账号数</div>
                    <div className="text-3xl font-semibold tracking-tight">{dashboardStats.total}</div>
                    <div className="text-sm text-muted-foreground">账号池全量规模</div>
                  </div>
                  <div className="flex size-11 items-center justify-center rounded-2xl bg-secondary text-secondary-foreground">
                    <Orbit className="size-5" />
                  </div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="flex items-start justify-between gap-3 p-5">
                  <div className="space-y-2">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">可用账号</div>
                    <div className="text-3xl font-semibold tracking-tight">{dashboardStats.okCount}</div>
                    <div className="text-sm text-muted-foreground">当前可参与调度</div>
                  </div>
                  <div className="flex size-11 items-center justify-center rounded-2xl bg-secondary text-secondary-foreground">
                    <ShieldCheck className="size-5" />
                  </div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="flex items-start justify-between gap-3 p-5">
                  <div className="space-y-2">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">不可用账号</div>
                    <div className="text-3xl font-semibold tracking-tight">{dashboardStats.unavailableCount}</div>
                    <div className="text-sm text-muted-foreground">当前不参与调度</div>
                  </div>
                  <div className="flex size-11 items-center justify-center rounded-2xl bg-secondary text-secondary-foreground">
                    <Shield className="size-5" />
                  </div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="flex items-start justify-between gap-3 p-5">
                  <div className="space-y-2">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">低配额</div>
                    <div className="text-3xl font-semibold tracking-tight">{dashboardStats.lowCount}</div>
                    <div className="text-sm text-muted-foreground">剩余额度接近阈值</div>
                  </div>
                  <div className="flex size-11 items-center justify-center rounded-2xl bg-secondary text-secondary-foreground">
                    <Gauge className="size-5" />
                  </div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="flex items-start justify-between gap-3 p-5">
                  <div className="space-y-2">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">今日令牌</div>
                    <div className="text-3xl font-semibold tracking-tight">
                      {formatCompactNumber(app.requestLogTodaySummary.todayTokens, "0")}
                    </div>
                    <div className="text-sm text-muted-foreground">非缓存输入 + 输出</div>
                  </div>
                  <div className="flex size-11 items-center justify-center rounded-2xl bg-secondary text-secondary-foreground">
                    <Activity className="size-5" />
                  </div>
                </CardContent>
              </Card>
            </div>

            <div className="grid gap-5 xl:grid-cols-[1.2fr_0.8fr]">
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>账号池剩余额度</CardTitle>
                  <CardDescription>主窗口与次窗口剩余率，直接反映当前池子健康度。</CardDescription>
                </CardHeader>
                <CardContent className="grid gap-5 pt-5 lg:grid-cols-2">
                  <div className="rounded-[1.4rem] border border-border/70 bg-background/75 p-5">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">主窗口 / 5 小时</div>
                    <div className="mt-3 text-4xl font-semibold tracking-tight">
                      {dashboardAggregate.primaryRemainPercent == null ? "--" : `${dashboardAggregate.primaryRemainPercent}%`}
                    </div>
                    <div className="mt-4">
                      <div className="mb-2 flex items-center gap-3 text-sm">
                        <span className="font-medium">已统计账号</span>
                        <span className="ml-auto text-muted-foreground tabular-nums">
                          {dashboardAggregate.primaryKnownCount}/{dashboardAggregate.primaryBucketCount}
                        </span>
                      </div>
                      <Progress
                        aria-label={`主窗口剩余额度，已统计 ${dashboardAggregate.primaryKnownCount}/${dashboardAggregate.primaryBucketCount}，剩余 ${dashboardAggregate.primaryRemainPercent == null ? "--" : `${dashboardAggregate.primaryRemainPercent}%`}`}
                        value={dashboardAggregate.primaryRemainPercent ?? 0}
                      />
                    </div>
                  </div>

                  <div className="rounded-[1.4rem] border border-border/70 bg-background/75 p-5">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">次窗口 / 7 天</div>
                    <div className="mt-3 text-4xl font-semibold tracking-tight">
                      {dashboardAggregate.secondaryRemainPercent == null ? "--" : `${dashboardAggregate.secondaryRemainPercent}%`}
                    </div>
                    <div className="mt-4">
                      <div className="mb-2 flex items-center gap-3 text-sm">
                        <span className="font-medium">已统计账号</span>
                        <span className="ml-auto text-muted-foreground tabular-nums">
                          {dashboardAggregate.secondaryKnownCount}/{dashboardAggregate.secondaryBucketCount}
                        </span>
                      </div>
                      <Progress
                        aria-label={`次窗口剩余额度，已统计 ${dashboardAggregate.secondaryKnownCount}/${dashboardAggregate.secondaryBucketCount}，剩余 ${dashboardAggregate.secondaryRemainPercent == null ? "--" : `${dashboardAggregate.secondaryRemainPercent}%`}`}
                        value={dashboardAggregate.secondaryRemainPercent ?? 0}
                      />
                    </div>
                  </div>
                </CardContent>
              </Card>

              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>当前调度焦点</CardTitle>
                  <CardDescription>优先账号、最近请求命中、配额窗口和最近采样时间。</CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 pt-5">
                  {dashboardCurrentAccount ? (
                    <>
                      <div className="rounded-[1.35rem] border border-border/70 bg-background/75 p-4">
                        <div className="flex items-start justify-between gap-3">
                          <div className="space-y-1">
                            <div className="text-lg font-semibold">{dashboardCurrentAccount.label || dashboardCurrentAccount.id}</div>
                            <div className="text-sm text-muted-foreground">{dashboardCurrentAccount.id}</div>
                          </div>
                          <Badge className={`rounded-full ${availabilityToneClass(dashboardCurrentAvailability.level)}`}>
                            {dashboardCurrentAvailability.text}
                          </Badge>
                        </div>
                      </div>

                      <div className="rounded-[1.35rem] border border-border/70 bg-background/75 p-4">
                        <div className="grid gap-3">
                          <div className="grid gap-2">
                            <div className="flex items-center gap-3 text-sm">
                              <span className="font-medium">{formatLimitLabel(dashboardCurrentUsage?.windowMinutes, "5小时用量")}</span>
                              <span className="ml-auto text-muted-foreground tabular-nums">
                                {dashboardCurrentUsage?.usedPercent == null ? "--" : `${100 - Number(dashboardCurrentUsage.usedPercent || 0)}%`}
                              </span>
                            </div>
                            <Progress
                              aria-label={`${formatLimitLabel(dashboardCurrentUsage?.windowMinutes, "5小时用量")}，剩余 ${dashboardCurrentUsage?.usedPercent == null ? "--" : `${100 - Number(dashboardCurrentUsage.usedPercent || 0)}%`}`}
                              value={dashboardCurrentUsage?.usedPercent == null ? 0 : 100 - Number(dashboardCurrentUsage.usedPercent || 0)}
                            />
                            <div className="text-xs text-muted-foreground">
                              {formatResetLabel(dashboardCurrentUsage?.resetsAt)}
                            </div>
                          </div>
                          <div className="grid gap-2">
                            <div className="flex items-center gap-3 text-sm">
                              <span className="font-medium">7天用量</span>
                              <span className="ml-auto text-muted-foreground tabular-nums">
                                {dashboardCurrentUsage?.secondaryUsedPercent == null ? "--" : `${100 - Number(dashboardCurrentUsage.secondaryUsedPercent || 0)}%`}
                              </span>
                            </div>
                            <Progress
                              aria-label={`7天用量，剩余 ${dashboardCurrentUsage?.secondaryUsedPercent == null ? "--" : `${100 - Number(dashboardCurrentUsage.secondaryUsedPercent || 0)}%`}`}
                              value={dashboardCurrentUsage?.secondaryUsedPercent == null ? 0 : 100 - Number(dashboardCurrentUsage.secondaryUsedPercent || 0)}
                            />
                            <div className="text-xs text-muted-foreground">
                              {formatResetLabel(dashboardCurrentUsage?.secondaryResetsAt)}
                            </div>
                          </div>
                        </div>
                      </div>

                      <div className="rounded-[1.35rem] border border-dashed border-border/70 bg-background/60 px-4 py-3 text-sm text-muted-foreground">
                        最近刷新：{formatTimestamp(dashboardCurrentUsage?.capturedAt, "暂无刷新记录")}
                      </div>

                      <div className="grid gap-3 md:grid-cols-2">
                        <div className="rounded-[1.35rem] border border-border/70 bg-background/75 p-4">
                          <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">最佳账号 / 5 小时</div>
                          {dashboardPrimaryRecommendation ? (
                            <div className="mt-3 grid gap-1">
                              <div className="font-medium">
                                {dashboardPrimaryRecommendation.account.label || dashboardPrimaryRecommendation.account.id}
                              </div>
                              <div className="text-xs text-muted-foreground">{dashboardPrimaryRecommendation.account.id}</div>
                              <div className="text-sm text-muted-foreground">{dashboardPrimaryRecommendation.remain}% 剩余</div>
                            </div>
                          ) : (
                            <div className="mt-3 text-sm text-muted-foreground">暂无账号</div>
                          )}
                        </div>
                        <div className="rounded-[1.35rem] border border-border/70 bg-background/75 p-4">
                          <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">最佳账号 / 7 天</div>
                          {dashboardSecondaryRecommendation ? (
                            <div className="mt-3 grid gap-1">
                              <div className="font-medium">
                                {dashboardSecondaryRecommendation.account.label || dashboardSecondaryRecommendation.account.id}
                              </div>
                              <div className="text-xs text-muted-foreground">{dashboardSecondaryRecommendation.account.id}</div>
                              <div className="text-sm text-muted-foreground">{dashboardSecondaryRecommendation.remain}% 剩余</div>
                            </div>
                          ) : (
                            <div className="mt-3 text-sm text-muted-foreground">暂无账号</div>
                          )}
                        </div>
                      </div>
                    </>
                  ) : (
                    <div className="grid min-h-72 place-items-center rounded-[1.4rem] border border-dashed border-border/70 bg-background/60 text-sm text-muted-foreground">
                      等待账号池数据
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>

            <div className="grid gap-5 xl:grid-cols-[1.15fr_0.85fr]">
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>建议优先账号</CardTitle>
                  <CardDescription>按主窗口剩余率排序，优先观察更稳定的账号。</CardDescription>
                </CardHeader>
                <CardContent className="pt-5">
                  <div className="overflow-hidden rounded-[1.35rem] border border-border/70 bg-background/75">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>账号</TableHead>
                          <TableHead>分组</TableHead>
                          <TableHead>主窗口</TableHead>
                          <TableHead>次窗口</TableHead>
                          <TableHead>状态</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {dashboardRankedAccounts.length > 0 ? (
                          dashboardRankedAccounts.map(({ account, primaryRemain, secondaryRemain, availability }) => (
                            <TableRow key={account.id}>
                              <TableCell>
                                <div className="grid gap-1">
                                  <span className="font-medium">{account.label || account.id}</span>
                                  <span className="text-xs text-muted-foreground">{account.id}</span>
                                </div>
                              </TableCell>
                              <TableCell>{account.groupName || "-"}</TableCell>
                              <TableCell>{primaryRemain == null ? "--" : `${primaryRemain}%`}</TableCell>
                              <TableCell>{secondaryRemain == null ? "--" : `${secondaryRemain}%`}</TableCell>
                              <TableCell>
                                <Badge className={`rounded-full ${availabilityToneClass(availability.level)}`}>{availability.text}</Badge>
                              </TableCell>
                            </TableRow>
                          ))
                        ) : (
                          <TableRow>
                            <TableCell colSpan={5} className="h-24 text-center text-muted-foreground">
                              暂无可排序账号
                            </TableCell>
                          </TableRow>
                        )}
                      </TableBody>
                    </Table>
                  </div>
                </CardContent>
              </Card>

              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>今日请求概览</CardTitle>
                  <CardDescription>令牌结构和成本趋势的轻量视图。</CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 pt-5">
                  <div className="rounded-[1.35rem] border border-border/70 bg-background/75 p-4">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">缓存输入</div>
                    <div className="mt-2 text-3xl font-semibold">{formatCompactNumber(app.requestLogTodaySummary.cachedInputTokens, "0")}</div>
                  </div>
                  <div className="rounded-[1.35rem] border border-border/70 bg-background/75 p-4">
                    <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">推理输出</div>
                    <div className="mt-2 text-3xl font-semibold">{formatCompactNumber(app.requestLogTodaySummary.reasoningOutputTokens, "0")}</div>
                  </div>
                  <div className="rounded-[1.35rem] border border-border/70 bg-background/75 p-4">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">预计费用</div>
                        <div className="mt-2 text-3xl font-semibold">${app.requestLogTodaySummary.estimatedCost.toFixed(2)}</div>
                      </div>
                      <div className="flex size-12 items-center justify-center rounded-2xl bg-secondary text-secondary-foreground">
                        <Banknote className="size-5" />
                      </div>
                    </div>
                  </div>
                  <div className="rounded-[1.35rem] border border-dashed border-border/70 bg-background/60 px-4 py-3 text-sm text-muted-foreground">
                    <div>服务状态：{app.serviceConnected ? "已连接" : "未连接"}</div>
                    <div className="mt-1">{app.serviceHint || "当前服务稳定，可直接继续操作。"}</div>
                  </div>
                </CardContent>
              </Card>
            </div>
          </section>
        ) : null}
        {app.currentPage === "accounts" ? (
          <section className="grid gap-5">
            <Card className="border-border/70 bg-card/85 shadow-none">
              <CardHeader className="border-b border-border/60">
                <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
                  <div>
                    <CardTitle>账号管理</CardTitle>
                    <CardDescription>授权登录、导入、批量删除、优先账号控制与用量查看统一收敛到 React 工作台。</CardDescription>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button type="button" variant="outline" onClick={() => void app.refreshAll()}>
                      <RefreshCw className="size-4" />
                      全量同步
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      disabled={app.accountUsageRefreshProgress.active}
                      onClick={() => void app.refreshAllAccountUsage()}
                    >
                      {app.accountUsageRefreshProgress.active ? (
                        <LoaderCircle className="size-4 animate-spin" />
                      ) : (
                        <RefreshCw className="size-4" />
                      )}
                      刷新用量
                    </Button>
                    <Button type="button" variant="outline" onClick={() => accountFileInputRef.current?.click()}>
                      <Import className="size-4" />
                      导入 JSON
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      disabled={!app.desktopMode}
                      title={app.desktopMode ? "从目录批量导入账号 JSON" : "浏览器模式暂不支持目录导入"}
                      onClick={() => void app.importAccountsFromDirectory()}
                    >
                      <FolderOpen className="size-4" />
                      导入目录
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      title={app.desktopMode ? "按账号文件导出到目录" : "下载为单个 JSON 导出文件"}
                      onClick={() => void app.exportAccountsByFile()}
                    >
                      <Download className="size-4" />
                      导出账号
                    </Button>
                    <Button type="button" variant="secondary" onClick={() => void app.deleteUnavailableFreeAccounts()}>
                      <Shield className="size-4" />
                      清理不可用免费
                    </Button>
                    {app.manualPreferredAccountId ? (
                      <Button type="button" variant="secondary" onClick={() => void app.setManualPreferredAccountId("")}>
                        <Star className="size-4" />
                        恢复自动调度
                      </Button>
                    ) : null}
                    <Button type="button" onClick={() => setAccountAddDialogOpen(true)}>
                      <Plus className="size-4" />
                      添加账号
                    </Button>
                  </div>
                </div>
              </CardHeader>

              <CardContent className="grid gap-5 pt-5">
                {accountUsageRefreshText ? (
                  <div
                    aria-live="polite"
                    className="rounded-2xl border border-border/70 bg-background/75 px-4 py-3 text-sm text-muted-foreground"
                  >
                    {accountUsageRefreshText}
                  </div>
                ) : null}

                <div className="grid gap-3 xl:grid-cols-[1.1fr_0.55fr_0.55fr]">
                  <Input
                    name="account-search"
                    aria-label="搜索账号"
                    placeholder="搜索账号名 / 编号 / 备注..."
                    value={app.accountSearch}
                    onChange={(event) => {
                      resetAccountSelection();
                      app.setAccountSearch(event.target.value);
                    }}
                  />
                  <NativeSelect
                    name="account-group-filter"
                    aria-label="按账号分组筛选"
                    value={app.accountGroupFilter}
                    onChange={(event) => {
                      resetAccountSelection();
                      app.setAccountGroupFilter(event.target.value);
                    }}
                  >
                    <NativeSelectOption value="all">全部分组</NativeSelectOption>
                    {app.accountGroups.map((group) => (
                      <NativeSelectOption key={group} value={group}>
                        {group}
                      </NativeSelectOption>
                    ))}
                  </NativeSelect>
                  <NativeSelect
                    name="account-status-filter"
                    aria-label="按账号状态筛选"
                    value={app.accountFilter}
                    onChange={(event) => {
                      resetAccountSelection();
                      app.setAccountFilter(event.target.value);
                    }}
                  >
                    <NativeSelectOption value="all">全部状态</NativeSelectOption>
                    <NativeSelectOption value="active">仅可用</NativeSelectOption>
                    <NativeSelectOption value="low">低配额</NativeSelectOption>
                  </NativeSelect>
                </div>

                {selectedAccountIds.length > 0 ? (
                  <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="text-sm text-muted-foreground">已选择 {selectedAccountIds.length} 个账号</div>
                    <Button type="button" variant="destructive" onClick={() => setAccountConfirmDeleteIds(selectedAccountIds)}>
                      <Trash2 className="size-4" />
                      删除所选账号
                    </Button>
                  </div>
                ) : null}

                <div className="overflow-hidden rounded-[1.4rem] border border-border/70 bg-background/75">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead className="w-14">
                          <Checkbox
                            checked={allVisibleAccountsChecked}
                            name="account-select-all"
                            aria-label="选择当前页全部账号"
                            onCheckedChange={(checked) => {
                              setSelectedAccountIdsState((current) => {
                                const next = new Set(current.filter((id) => validAccountIdSet.has(id)));
                                if (checked) {
                                  visibleAccountIds.forEach((id) => next.add(id));
                                } else {
                                  visibleAccountIds.forEach((id) => next.delete(id));
                                }
                                return Array.from(next);
                              });
                            }}
                          />
                        </TableHead>
                        <TableHead>账号</TableHead>
                        <TableHead>分组</TableHead>
                        <TableHead className="w-28">排序</TableHead>
                        <TableHead>额度窗口</TableHead>
                        <TableHead>状态</TableHead>
                        <TableHead>最近刷新</TableHead>
                        <TableHead>操作</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {app.visibleAccounts.length > 0 ? (
                        app.visibleAccounts.map((account) => {
                          const isPreferred = app.manualPreferredAccountId === account.id;
                          return (
                            <TableRow key={account.id}>
                              <TableCell>
                                <Checkbox
                                  checked={selectedAccountIdSet.has(account.id)}
                                  name={`account-select-${account.id}`}
                                  aria-label={`选择账号 ${account.label || account.id}`}
                                  onCheckedChange={(checked) => toggleAccountSelected(account.id, Boolean(checked))}
                                />
                              </TableCell>
                              <TableCell>
                                <div className="grid gap-1">
                                  <span className="font-medium">{account.label || account.id}</span>
                                  <span className="text-xs text-muted-foreground">{account.id}</span>
                                </div>
                              </TableCell>
                              <TableCell>{account.groupName || "-"}</TableCell>
                              <TableCell>
                                <Input
                                  type="number"
                                  inputMode="numeric"
                                  className="h-8 w-20"
                                  aria-label={`设置账号 ${account.label || account.id} 的排序`}
                                  value={accountSortDrafts[account.id] ?? String(account.sort ?? 0)}
                                  disabled={Boolean(pendingAccountSortIds[account.id])}
                                  onChange={(event) => {
                                    setAccountSortDrafts((current) => ({
                                      ...current,
                                      [account.id]: event.target.value,
                                    }));
                                  }}
                                  onBlur={() => void handleAccountSortCommit(account)}
                                  onKeyDown={(event) => {
                                    if (event.key === "Enter") {
                                      event.preventDefault();
                                      void handleAccountSortCommit(account);
                                    }
                                  }}
                                />
                              </TableCell>
                              <TableCell>
                                <div className="grid gap-3">
                                  <div className="grid min-w-44 gap-2">
                                    <div className="flex items-center gap-3 text-sm">
                                      <span className="font-medium">主窗口</span>
                                      <span className="ml-auto text-muted-foreground tabular-nums">
                                        {account.primaryRemain == null ? "--" : `${account.primaryRemain}%`}
                                      </span>
                                    </div>
                                    <Progress
                                      aria-label={`主窗口，剩余 ${account.primaryRemain == null ? "--" : `${account.primaryRemain}%`}`}
                                      value={account.primaryRemain ?? 0}
                                    />
                                  </div>
                                  <div className="grid min-w-44 gap-2">
                                    <div className="flex items-center gap-3 text-sm">
                                      <span className="font-medium">次窗口</span>
                                      <span className="ml-auto text-muted-foreground tabular-nums">
                                        {account.secondaryRemain == null ? "--" : `${account.secondaryRemain}%`}
                                      </span>
                                    </div>
                                    <Progress
                                      aria-label={`次窗口，剩余 ${account.secondaryRemain == null ? "--" : `${account.secondaryRemain}%`}`}
                                      value={account.secondaryRemain ?? 0}
                                    />
                                  </div>
                                </div>
                              </TableCell>
                              <TableCell>
                                <Badge className={`rounded-full ${availabilityToneClass(account.availability.level)}`}>
                                  {account.availability.text}
                                </Badge>
                              </TableCell>
                              <TableCell>{formatTimestamp(account.usage?.capturedAt, "暂无刷新记录")}</TableCell>
                              <TableCell>
                                <div className="flex flex-wrap gap-2">
                                  <Button type="button" size="sm" variant="outline" onClick={() => setUsageAccountId(account.id)}>
                                    查看用量
                                  </Button>
                                  <Button
                                    type="button"
                                    size="sm"
                                    variant={isPreferred ? "secondary" : "outline"}
                                    disabled={account.availability.level === "warn" || account.availability.level === "bad"}
                                    title={
                                      account.availability.level === "warn" || account.availability.level === "bad"
                                        ? `账号当前不可用（${account.availability.text}），不参与网关选路`
                                        : "锁定为当前账号（异常前持续优先使用）"
                                    }
                                    onClick={() => {
                                      void app.setManualPreferredAccountId(isPreferred ? "" : account.id);
                                    }}
                                  >
                                    <Star className="size-3.5" />
                                    {isPreferred ? "取消优先" : "设为优先"}
                                  </Button>
                                  <Button
                                    type="button"
                                    size="sm"
                                    variant="ghost"
                                    onClick={() => setAccountConfirmDeleteIds([account.id])}
                                  >
                                    删除
                                  </Button>
                                </div>
                              </TableCell>
                            </TableRow>
                          );
                        })
                      ) : (
                        <TableRow>
                          <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                            暂无账号数据
                          </TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </div>

                <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                  <div className="text-sm text-muted-foreground">
                    {app.accountTotal > 0
                      ? `共 ${app.accountTotal} 个账号，当前显示 ${accountPageStart}-${accountPageEnd}`
                      : "共 0 个账号"}
                    {app.accountPageLoading ? " · 分页中..." : ""}
                  </div>
                  <div className="flex flex-wrap items-center gap-3">
                    <label className="flex items-center gap-2 text-sm text-muted-foreground">
                      <span>每页</span>
                      <NativeSelect
                        size="sm"
                        name="account-page-size"
                        aria-label="每页显示账号数量"
                        disabled={app.accountPageLoading}
                        value={String(app.accountPageSize)}
                        onChange={(event) => app.setAccountPageSize(Number(event.target.value))}
                      >
                        <NativeSelectOption value="5">5</NativeSelectOption>
                        <NativeSelectOption value="10">10</NativeSelectOption>
                        <NativeSelectOption value="20">20</NativeSelectOption>
                        <NativeSelectOption value="50">50</NativeSelectOption>
                        <NativeSelectOption value="80">80</NativeSelectOption>
                        <NativeSelectOption value="120">120</NativeSelectOption>
                        <NativeSelectOption value="500">500</NativeSelectOption>
                      </NativeSelect>
                    </label>
                    <Button
                      type="button"
                      variant="outline"
                      disabled={app.accountPageLoading || app.accountPage <= 1}
                      onClick={() => app.setAccountPage(app.accountPage - 1)}
                    >
                      上一页
                    </Button>
                    <div className="text-sm text-muted-foreground">
                      第 {app.accountPage} / {accountTotalPages} 页
                    </div>
                    <Button
                      type="button"
                      variant="outline"
                      disabled={app.accountPageLoading || app.accountPage >= accountTotalPages}
                      onClick={() => app.setAccountPage(app.accountPage + 1)}
                    >
                      下一页
                    </Button>
                  </div>
                </div>

                <input
                  ref={accountFileInputRef}
                  hidden
                  multiple
                  name="account-import-files"
                  accept=".json,application/json"
                  type="file"
                  onChange={(event) => {
                    const { files } = event.target;
                    if (files?.length) {
                      void app.importAccountsFromFiles(files);
                    }
                    event.target.value = "";
                  }}
                />
              </CardContent>
            </Card>

            <Dialog open={accountAddDialogOpen} onOpenChange={setAccountAddDialogOpen}>
              {accountAddDialogOpen ? (
                <DialogContent className="sm:max-w-2xl">
                  <DialogHeader>
                    <DialogTitle>添加账号</DialogTitle>
                    <DialogDescription>通过浏览器授权创建账号，支持粘贴回调链接手动完成。</DialogDescription>
                  </DialogHeader>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="grid gap-2">
                      <Label htmlFor="account-group">账号分组</Label>
                      <Input
                        id="account-group"
                        value={accountGroupName}
                        onChange={(event) => setAccountGroupName(event.target.value)}
                      />
                    </div>
                    <div className="grid gap-2">
                      <Label htmlFor="account-tags">标签</Label>
                      <Input
                        id="account-tags"
                        value={accountTags}
                        onChange={(event) => setAccountTags(event.target.value)}
                        placeholder="例如 生产,稳定"
                      />
                    </div>
                    <div className="grid gap-2 md:col-span-2">
                      <Label htmlFor="account-note">备注</Label>
                      <Textarea
                        id="account-note"
                        rows={4}
                        value={accountNote}
                        onChange={(event) => setAccountNote(event.target.value)}
                      />
                    </div>
                  </div>

                  <div className="rounded-2xl border border-border/70 bg-background/75 p-4">
                    <div className="text-sm text-muted-foreground">{accountLoginHint}</div>
                    <div className="mt-4 grid gap-3">
                      <div className="flex flex-col gap-2 sm:flex-row">
                        <Input readOnly value={accountLoginUrl} placeholder="授权链接将在这里显示" />
                        <Button
                          type="button"
                          variant="outline"
                          disabled={!accountLoginUrl}
                          onClick={() => {
                            void copyText(accountLoginUrl).then((copied) => {
                              if (copied) {
                                toast.success("授权链接已复制");
                              }
                            });
                          }}
                        >
                          <Copy className="size-4" />
                          复制链接
                        </Button>
                      </div>
                      <div className="flex flex-col gap-2 sm:flex-row">
                        <Input
                          value={accountManualCallbackUrl}
                          onChange={(event) => setAccountManualCallbackUrl(event.target.value)}
                          placeholder="粘贴完整回调地址"
                        />
                        <Button
                          type="button"
                          variant="secondary"
                          disabled={accountAddPending}
                          onClick={() => void handleManualAccountCallback()}
                        >
                          {accountAddPending ? <LoaderCircle className="size-4 animate-spin" /> : null}
                          解析回调
                        </Button>
                      </div>
                    </div>
                  </div>

                  <DialogFooter>
                    <Button type="button" variant="ghost" onClick={() => setAccountAddDialogOpen(false)}>
                      取消
                    </Button>
                    <Button type="button" disabled={accountAddPending} onClick={() => void handleStartAccountLogin()}>
                      {accountAddPending ? <LoaderCircle className="size-4 animate-spin" /> : <Upload className="size-4" />}
                      打开授权页
                    </Button>
                  </DialogFooter>
                </DialogContent>
              ) : null}
            </Dialog>

            <Dialog open={Boolean(usageAccountId)} onOpenChange={(nextOpen) => !nextOpen && setUsageAccountId(null)}>
              {usageAccountId ? (
                <DialogContent className="sm:max-w-2xl">
                  <DialogHeader>
                    <DialogTitle>账号用量</DialogTitle>
                    <DialogDescription>查看当前账号的窗口剩余和最近采样时间。</DialogDescription>
                  </DialogHeader>

                  {usageDialogAccount ? (
                    <div className="grid gap-4">
                      <div className="rounded-2xl border border-border/70 bg-background/75 p-4">
                        <div className="flex items-start justify-between gap-3">
                          <div>
                            <div className="text-lg font-semibold">{usageDialogAccount.label || usageDialogAccount.id}</div>
                            <div className="text-sm text-muted-foreground">{usageDialogAccount.id}</div>
                          </div>
                          <Badge
                            className={`rounded-full ${availabilityToneClass(visibleUsageDialogAccount?.availability.level || "unknown")}`}
                          >
                            {visibleUsageDialogAccount?.availability.text || "未知"}
                          </Badge>
                        </div>
                      </div>

                      <div className="grid gap-4 md:grid-cols-2">
                        <div className="rounded-2xl border border-border/70 bg-background/75 p-4">
                          <div className="grid min-w-44 gap-2">
                            <div className="flex items-center gap-3 text-sm">
                              <span className="font-medium">主窗口剩余</span>
                              <span className="ml-auto text-muted-foreground tabular-nums">
                                {visibleUsageDialogAccount?.primaryRemain == null ? "--" : `${visibleUsageDialogAccount.primaryRemain}%`}
                              </span>
                            </div>
                            <Progress
                              aria-label={`主窗口剩余，剩余 ${visibleUsageDialogAccount?.primaryRemain == null ? "--" : `${visibleUsageDialogAccount.primaryRemain}%`}`}
                              value={visibleUsageDialogAccount?.primaryRemain ?? 0}
                            />
                            <div className="text-xs text-muted-foreground">
                              {formatResetLabel(selectedAccountUsageSnapshot?.resetsAt)}
                            </div>
                          </div>
                        </div>
                        <div className="rounded-2xl border border-border/70 bg-background/75 p-4">
                          <div className="grid min-w-44 gap-2">
                            <div className="flex items-center gap-3 text-sm">
                              <span className="font-medium">次窗口剩余</span>
                              <span className="ml-auto text-muted-foreground tabular-nums">
                                {visibleUsageDialogAccount?.secondaryRemain == null ? "--" : `${visibleUsageDialogAccount.secondaryRemain}%`}
                              </span>
                            </div>
                            <Progress
                              aria-label={`次窗口剩余，剩余 ${visibleUsageDialogAccount?.secondaryRemain == null ? "--" : `${visibleUsageDialogAccount.secondaryRemain}%`}`}
                              value={visibleUsageDialogAccount?.secondaryRemain ?? 0}
                            />
                            <div className="text-xs text-muted-foreground">
                              {formatResetLabel(selectedAccountUsageSnapshot?.secondaryResetsAt)}
                            </div>
                          </div>
                        </div>
                      </div>

                      <div className="rounded-2xl border border-dashed border-border/70 bg-background/60 px-4 py-3 text-sm text-muted-foreground">
                        最近刷新：{formatTimestamp(selectedAccountUsageSnapshot?.capturedAt, "暂无刷新记录")}
                      </div>

                      {selectedAccountCreditsSummary ? (
                        <div className="rounded-2xl border border-border/70 bg-background/75 px-4 py-3 text-sm text-muted-foreground">
                          额度：{selectedAccountCreditsSummary.balance == null ? "--" : selectedAccountCreditsSummary.balance}
                          {selectedAccountCreditsSummary.unlimited == null
                            ? ""
                            : `（${selectedAccountCreditsSummary.unlimited ? "无限制" : "有限制"}）`}
                        </div>
                      ) : null}

                      {selectedAccountUsageSnapshot?.creditsJson ? (
                        <div className="rounded-2xl border border-border/70 bg-background/75">
                          <div className="border-b border-border/60 px-4 py-3 text-sm font-medium">Credits 原始信息</div>
                          <ScrollArea className="h-44">
                            <pre className="p-4 text-xs leading-6 text-muted-foreground">
                              {selectedAccountUsageSnapshot.creditsJson}
                            </pre>
                          </ScrollArea>
                        </div>
                      ) : null}
                    </div>
                  ) : (
                    <div className="rounded-2xl border border-dashed border-border/70 bg-background/60 px-4 py-8 text-center text-sm text-muted-foreground">
                      未找到账号信息
                    </div>
                  )}

                  <DialogFooter>
                    <Button type="button" variant="ghost" onClick={() => setUsageAccountId(null)}>
                      关闭
                    </Button>
                    <Button
                      type="button"
                      variant="secondary"
                      disabled={accountUsageRefreshPending || app.accountUsageRefreshProgress.active || !usageAccountId}
                      onClick={() => {
                        if (!usageAccountId) {
                          return;
                        }
                        setAccountUsageRefreshPending(true);
                        void app.refreshAccountUsage(usageAccountId).finally(() => setAccountUsageRefreshPending(false));
                      }}
                    >
                      {accountUsageRefreshPending ? (
                        <LoaderCircle className="size-4 animate-spin" />
                      ) : (
                        <RefreshCw className="size-4" />
                      )}
                      刷新当前账号
                    </Button>
                  </DialogFooter>
                </DialogContent>
              ) : null}
            </Dialog>

            <AlertDialog open={accountConfirmDeleteIds.length > 0} onOpenChange={(open) => !open && setAccountConfirmDeleteIds([])}>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>{accountConfirmDeleteIds.length > 1 ? "批量删除账号" : "删除账号"}</AlertDialogTitle>
                  <AlertDialogDescription>
                    {accountConfirmDeleteIds.length > 1
                      ? `确定删除选中的 ${accountConfirmDeleteIds.length} 个账号吗？此操作不可恢复。`
                      : "确定删除当前账号吗？此操作不可恢复。"}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel onClick={() => setAccountConfirmDeleteIds([])}>取消</AlertDialogCancel>
                  <AlertDialogAction
                    className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    disabled={accountConfirmPending}
                    onClick={(event) => {
                      event.preventDefault();
                      void (async () => {
                        setAccountConfirmPending(true);
                        await app.deleteAccounts(accountConfirmDeleteIds);
                        setAccountConfirmPending(false);
                        setSelectedAccountIdsState((current) =>
                          current.filter((id) => !accountConfirmDeleteIds.includes(id)),
                        );
                        setAccountConfirmDeleteIds([]);
                      })();
                    }}
                  >
                    {accountConfirmPending ? <LoaderCircle className="size-4 animate-spin" /> : null}
                    确认删除
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </section>
        ) : null}
        {app.currentPage === "apikeys" ? (
          <section className="grid gap-5">
            <div className="grid gap-4 md:grid-cols-3">
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="p-5">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">平台密钥总数</div>
                  <div className="mt-2 flex items-center gap-2 text-3xl font-semibold">
                    <KeyRound className="size-5 text-primary" />
                    {app.apiKeys.length}
                  </div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="p-5">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">已启用</div>
                  <div className="mt-2 flex items-center gap-2 text-3xl font-semibold">
                    <ShieldCheck className="size-5 text-primary" />
                    {activeApiKeyCount}
                  </div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="flex h-full items-center justify-end gap-2 p-5">
                  <Button type="button" variant="outline" onClick={() => void app.refreshApiModels(true)}>
                    <RefreshCw className="size-4" />
                    刷新模型
                  </Button>
                  <Button type="button" onClick={() => setApiKeyCreateOpen(true)}>
                    <Plus className="size-4" />
                    创建密钥
                  </Button>
                </CardContent>
              </Card>
            </div>

            <Card className="border-border/70 bg-card/85 shadow-none">
              <CardHeader className="border-b border-border/60">
                <CardTitle>平台密钥</CardTitle>
                <CardDescription>密钥状态、协议类型、模型覆盖和推理等级全部走 shadcn 表格组件管理。</CardDescription>
              </CardHeader>
              <CardContent className="pt-5">
                <div className="overflow-hidden rounded-[1.4rem] border border-border/70 bg-background/75">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>密钥</TableHead>
                        <TableHead>协议</TableHead>
                        <TableHead>模型覆盖</TableHead>
                        <TableHead>推理等级</TableHead>
                        <TableHead>状态</TableHead>
                        <TableHead>操作</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {app.apiKeys.length > 0 ? (
                        app.apiKeys.map((item) => {
                          const status = getApiKeyStatusMeta(item.status);
                          const modelSlug = apiKeyModelDrafts[item.id] ?? item.modelSlug ?? "";
                          const reasoningEffort = apiKeyReasoningDrafts[item.id] ?? item.reasoningEffort ?? "";
                          const pending = Boolean(pendingApiKeyIds[item.id]);

                          return (
                            <TableRow key={`${item.id}:${item.modelSlug || ""}:${item.reasoningEffort || ""}:${item.status || ""}`}>
                              <TableCell>
                                <div className="grid gap-1">
                                  <span className="font-medium">{item.name || "未命名密钥"}</span>
                                  <span className="text-xs text-muted-foreground">{item.id}</span>
                                </div>
                              </TableCell>
                              <TableCell>{getProtocolLabel(item.protocolType)}</TableCell>
                              <TableCell className="min-w-44">
                                <NativeSelect
                                  name={`api-key-model-${item.id}`}
                                  aria-label={`为 ${item.name || item.id} 选择模型覆盖`}
                                  value={modelSlug}
                                  onChange={(event) =>
                                    setApiKeyModelDrafts((current) => ({ ...current, [item.id]: event.target.value }))
                                  }
                                >
                                  <NativeSelectOption value="">跟随请求模型</NativeSelectOption>
                                  {app.apiModels.map((model) => (
                                    <NativeSelectOption key={model.slug} value={model.slug}>
                                      {model.displayName || model.slug}
                                    </NativeSelectOption>
                                  ))}
                                </NativeSelect>
                              </TableCell>
                              <TableCell className="min-w-40">
                                <NativeSelect
                                  name={`api-key-reasoning-${item.id}`}
                                  aria-label={`为 ${item.name || item.id} 选择推理等级`}
                                  value={reasoningEffort}
                                  onChange={(event) =>
                                    setApiKeyReasoningDrafts((current) => ({ ...current, [item.id]: event.target.value }))
                                  }
                                >
                                  {REASONING_OPTIONS.map((option) => (
                                    <NativeSelectOption key={option.value || "default"} value={option.value}>
                                      {option.label}
                                    </NativeSelectOption>
                                  ))}
                                </NativeSelect>
                              </TableCell>
                              <TableCell>
                                <Badge className={`rounded-full ${requestToneClass(status.tone)}`}>{status.label}</Badge>
                              </TableCell>
                              <TableCell>
                                <div className="flex flex-wrap gap-2">
                                  <Button
                                    type="button"
                                    size="sm"
                                    variant="outline"
                                    disabled={pending}
                                    onClick={() => {
                                      setApiKeyRowPending(item.id, true);
                                      void app
                                        .updateApiKeyModel(item, modelSlug || null, reasoningEffort || null)
                                        .finally(() => setApiKeyRowPending(item.id, false));
                                    }}
                                  >
                                    保存配置
                                  </Button>
                                  <Button
                                    type="button"
                                    size="sm"
                                    variant="outline"
                                    disabled={pending}
                                    onClick={() => {
                                      setApiKeyRowPending(item.id, true);
                                      void app
                                        .readApiKeySecret(item)
                                        .then((secret) => copyText(secret))
                                        .then((copied) => {
                                          if (copied) {
                                            toast.success("完整密钥已复制");
                                          }
                                        })
                                        .finally(() => setApiKeyRowPending(item.id, false));
                                    }}
                                  >
                                    复制
                                  </Button>
                                  <Button
                                    type="button"
                                    size="sm"
                                    variant={status.disabled ? "secondary" : "ghost"}
                                    disabled={pending}
                                    onClick={() => {
                                      setApiKeyRowPending(item.id, true);
                                      void app.toggleApiKeyStatus(item).finally(() => setApiKeyRowPending(item.id, false));
                                    }}
                                  >
                                    {status.disabled ? "启用" : "禁用"}
                                  </Button>
                                  <Button
                                    type="button"
                                    size="sm"
                                    variant="ghost"
                                    aria-label={`删除密钥 ${item.name || item.id}`}
                                    onClick={() => setApiKeyDeleteId(item.id)}
                                  >
                                    <Trash2 className="size-4" />
                                  </Button>
                                </div>
                              </TableCell>
                            </TableRow>
                          );
                        })
                      ) : (
                        <TableRow>
                          <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
                            暂无平台密钥
                          </TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </div>
              </CardContent>
            </Card>

            <Dialog open={apiKeyCreateOpen} onOpenChange={setApiKeyCreateOpen}>
              {apiKeyCreateOpen ? (
                <DialogContent className="sm:max-w-2xl">
                  <DialogHeader>
                    <DialogTitle>创建平台密钥</DialogTitle>
                    <DialogDescription>统一配置协议、模型覆盖与推理等级，支持 Azure OpenAI 兼容模式。</DialogDescription>
                  </DialogHeader>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="grid gap-2">
                      <Label htmlFor="api-key-name">名称</Label>
                      <Input
                        id="api-key-name"
                        value={apiKeyName}
                        onChange={(event) => {
                          clearCreatedApiKeySecret();
                          setApiKeyName(event.target.value);
                        }}
                        placeholder="例如 生产主 Key"
                      />
                    </div>
                    <div className="grid gap-2">
                      <Label htmlFor="api-key-protocol">协议类型</Label>
                      <NativeSelect
                        id="api-key-protocol"
                        value={apiKeyProtocolType}
                        onChange={(event) => {
                          const nextProtocolType = event.target.value;
                          clearCreatedApiKeySecret();
                          if (nextProtocolType !== "azure_openai") {
                            setApiKeyUpstreamBaseUrl("");
                            setApiKeyAzureApiKey("");
                          }
                          setApiKeyProtocolType(nextProtocolType);
                        }}
                      >
                        <NativeSelectOption value="openai_compat">OpenAI 兼容</NativeSelectOption>
                        <NativeSelectOption value="anthropic_native">Claude Code 兼容</NativeSelectOption>
                        <NativeSelectOption value="azure_openai">Azure OpenAI 兼容</NativeSelectOption>
                      </NativeSelect>
                    </div>
                    {apiKeyIsAzureProtocol ? (
                      <>
                        <div className="grid gap-2">
                          <Label htmlFor="api-key-endpoint">Azure Endpoint</Label>
                          <Input
                            id="api-key-endpoint"
                            value={apiKeyUpstreamBaseUrl}
                            onChange={(event) => {
                              clearCreatedApiKeySecret();
                              setApiKeyUpstreamBaseUrl(event.target.value);
                            }}
                            placeholder="https://xxx.openai.azure.com/"
                          />
                        </div>
                        <div className="grid gap-2">
                          <Label htmlFor="api-key-azure">Azure API Key</Label>
                          <Input
                            id="api-key-azure"
                            value={apiKeyAzureApiKey}
                            onChange={(event) => {
                              clearCreatedApiKeySecret();
                              setApiKeyAzureApiKey(event.target.value);
                            }}
                            placeholder="粘贴 Azure API Key"
                          />
                        </div>
                      </>
                    ) : null}
                    <div className="grid gap-2">
                      <Label htmlFor="api-key-model">覆盖模型</Label>
                      <NativeSelect
                        id="api-key-model"
                        value={apiKeyModelSlug}
                        onChange={(event) => {
                          clearCreatedApiKeySecret();
                          setApiKeyModelSlug(event.target.value);
                        }}
                      >
                        <NativeSelectOption value="">跟随请求模型</NativeSelectOption>
                        {app.apiModels.map((item) => (
                          <NativeSelectOption key={item.slug} value={item.slug}>
                            {item.displayName || item.slug}
                          </NativeSelectOption>
                        ))}
                      </NativeSelect>
                    </div>
                    <div className="grid gap-2">
                      <Label htmlFor="api-key-reasoning">推理等级</Label>
                      <NativeSelect
                        id="api-key-reasoning"
                        value={apiKeyReasoningEffort}
                        onChange={(event) => {
                          clearCreatedApiKeySecret();
                          setApiKeyReasoningEffort(event.target.value);
                        }}
                      >
                        {REASONING_OPTIONS.map((item) => (
                          <NativeSelectOption key={item.value || "default"} value={item.value}>
                            {item.label}
                          </NativeSelectOption>
                        ))}
                      </NativeSelect>
                    </div>
                    <div className="grid gap-2 md:col-span-2">
                      <Label htmlFor="api-key-secret">创建后明文</Label>
                      <div className="flex flex-col gap-2 sm:flex-row">
                        <Input id="api-key-secret" readOnly value={createdApiKeySecret} placeholder="创建成功后显示完整密钥" />
                        <Button
                          type="button"
                          variant="outline"
                          disabled={!createdApiKeySecret}
                          onClick={() => {
                            void copyText(createdApiKeySecret).then((copied) => {
                              if (copied) {
                                toast.success("完整密钥已复制");
                              }
                            });
                          }}
                        >
                          <Copy className="size-4" />
                          复制
                        </Button>
                      </div>
                    </div>
                  </div>

                  <DialogFooter>
                    <Button type="button" variant="ghost" onClick={() => setApiKeyCreateOpen(false)}>
                      关闭
                    </Button>
                    <Button
                      type="button"
                      disabled={apiKeyCreatePending}
                      onClick={() => {
                        if (apiKeyIsAzureProtocol && !apiKeyUpstreamBaseUrl.trim()) {
                          toast.error("Azure OpenAI 兼容模式必须填写 Endpoint");
                          return;
                        }
                        if (apiKeyIsAzureProtocol && !apiKeyAzureApiKey.trim()) {
                          toast.error("Azure OpenAI 兼容模式必须填写 API Key");
                          return;
                        }

                        setCreatedApiKeySecret("");
                        setApiKeyCreatePending(true);
                        void app
                          .createApiKey({
                            name: apiKeyName.trim() || null,
                            modelSlug: apiKeyModelSlug || null,
                            reasoningEffort: apiKeyReasoningEffort || null,
                            protocolType: apiKeyProtocolType,
                            upstreamBaseUrl: apiKeyIsAzureProtocol ? apiKeyUpstreamBaseUrl.trim() || null : null,
                            staticHeadersJson:
                              apiKeyIsAzureProtocol && apiKeyAzureApiKey.trim()
                                ? JSON.stringify({ "api-key": apiKeyAzureApiKey.trim() })
                                : null,
                          })
                          .then((result) => {
                            if (result?.key) {
                              setCreatedApiKeySecret(result.key);
                            }
                          })
                          .finally(() => setApiKeyCreatePending(false));
                      }}
                    >
                      {apiKeyCreatePending ? <LoaderCircle className="size-4 animate-spin" /> : <Plus className="size-4" />}
                      创建密钥
                    </Button>
                  </DialogFooter>
                </DialogContent>
              ) : null}
            </Dialog>

            <AlertDialog open={Boolean(apiKeyDeleteId)} onOpenChange={(open) => !open && setApiKeyDeleteId(null)}>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>删除平台密钥</AlertDialogTitle>
                  <AlertDialogDescription>删除后无法恢复，确定继续吗？</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel onClick={() => setApiKeyDeleteId(null)}>取消</AlertDialogCancel>
                  <AlertDialogAction
                    className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    disabled={apiKeyDeletePending}
                    onClick={(event) => {
                      event.preventDefault();
                      void (async () => {
                        if (!apiKeyDeleteId) {
                          return;
                        }
                        const target = app.apiKeys.find((item) => item.id === apiKeyDeleteId);
                        if (!target) {
                          setApiKeyDeleteId(null);
                          return;
                        }
                        setApiKeyDeletePending(true);
                        await app.deleteApiKey(target);
                        setApiKeyDeletePending(false);
                        setApiKeyDeleteId(null);
                      })();
                    }}
                  >
                    {apiKeyDeletePending ? <LoaderCircle className="size-4 animate-spin" /> : null}
                    删除
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </section>
        ) : null}
        {app.currentPage === "requestlogs" ? (
          <section className="grid gap-5">
            <div className="grid gap-4 md:grid-cols-4">
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="p-5">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">今日令牌</div>
                  <div className="mt-2 text-3xl font-semibold">{formatCompactNumber(requestSummary.todayTokens, "0")}</div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="p-5">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">缓存输入</div>
                  <div className="mt-2 text-3xl font-semibold">{formatCompactNumber(requestSummary.cachedInputTokens, "0")}</div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="p-5">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">推理输出</div>
                  <div className="mt-2 text-3xl font-semibold">{formatCompactNumber(requestSummary.reasoningOutputTokens, "0")}</div>
                </CardContent>
              </Card>
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardContent className="p-5">
                  <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">预计费用</div>
                  <div className="mt-2 text-3xl font-semibold">${requestSummary.estimatedCost.toFixed(2)}</div>
                </CardContent>
              </Card>
            </div>

            <Card className="border-border/70 bg-card/85 shadow-none">
              <CardHeader className="border-b border-border/60">
                <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
                  <div>
                    <CardTitle>请求日志</CardTitle>
                    <CardDescription>按状态、路径、模型和账号检索网关流量，支持刷新和清空。</CardDescription>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button type="button" variant="outline" onClick={() => void app.refreshRequestLogs()}>
                      <RefreshCw className="size-4" />
                      刷新日志
                    </Button>
                    <Button type="button" variant="destructive" onClick={() => setRequestLogClearOpen(true)}>
                      <Trash2 className="size-4" />
                      清空日志
                    </Button>
                  </div>
                </div>
              </CardHeader>

              <CardContent className="grid gap-5 pt-5">
                <div className="grid gap-3 xl:grid-cols-[1fr_14rem]">
                  <div className="relative">
                    <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      className="pl-9"
                      name="request-log-query"
                      aria-label="搜索请求日志"
                      placeholder="搜索 方法 / 路径 / 模型 / traceId / 错误..."
                      value={app.requestLogQuery}
                      onChange={(event) => app.setRequestLogQuery(event.target.value)}
                    />
                  </div>
                  <NativeSelect
                    name="request-log-status-filter"
                    aria-label="按请求状态筛选"
                    value={app.requestLogStatusFilter}
                    onChange={(event) => app.setRequestLogStatusFilter(event.target.value)}
                  >
                    <NativeSelectOption value="all">全部状态</NativeSelectOption>
                    <NativeSelectOption value="2xx">仅 2xx</NativeSelectOption>
                    <NativeSelectOption value="4xx">仅 4xx</NativeSelectOption>
                    <NativeSelectOption value="5xx">仅 5xx</NativeSelectOption>
                  </NativeSelect>
                </div>

                <div className="overflow-hidden rounded-[1.4rem] border border-border/70 bg-background/75">
                  <ScrollArea className="h-[36rem]" onScrollCapture={handleRequestLogScrollCapture}>
                    <Table className="min-w-[1080px]">
                      <TableHeader>
                        <TableRow>
                          <TableHead>时间</TableHead>
                          <TableHead>账号</TableHead>
                          <TableHead>Key</TableHead>
                          <TableHead>方法</TableHead>
                          <TableHead>路径</TableHead>
                          <TableHead>模型</TableHead>
                          <TableHead>推理等级</TableHead>
                          <TableHead>状态</TableHead>
                          <TableHead>错误</TableHead>
                          <TableHead>Trace</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {app.visibleRequestLogs.length > 0 ? (
                          <>
                            {requestLogVirtualWindow.topSpacerHeight > 0 ? (
                              <TableRow aria-hidden="true">
                                <TableCell
                                  colSpan={REQUEST_LOG_COLUMN_COUNT}
                                  className="h-0 border-0 p-0"
                                  style={{ height: requestLogVirtualWindow.topSpacerHeight }}
                                />
                              </TableRow>
                            ) : null}
                            {virtualRenderedRequestLogs.map((item, index) => {
                              const globalIndex = requestLogVirtualWindow.startIndex + index;
                              const tone = getRequestStatusTone(item.statusCode ?? null);
                              const displayPath = resolveRequestDisplayPath(item);
                              const routeMeta = buildRequestRouteMeta(item);
                              return (
                                <TableRow key={requestLogKeys[globalIndex] || item.__identity || item.id || globalIndex}>
                                  <TableCell>{formatTimestamp(item.createdAt, "--")}</TableCell>
                                  <TableCell>
                                    <div className="grid gap-1">
                                      <span className="font-medium">
                                        {resolveRequestLogAccountLabel(item, app.accountLabelsById)}
                                      </span>
                                      <span className="text-xs text-muted-foreground">{item.accountId || "-"}</span>
                                    </div>
                                  </TableCell>
                                  <TableCell>{item.keyId || "-"}</TableCell>
                                  <TableCell>{item.method || "-"}</TableCell>
                                  <TableCell className="max-w-80">
                                    <div className="grid gap-2">
                                      <div className="break-all text-sm">{displayPath}</div>
                                      {routeMeta.length > 0 ? (
                                        <div className="text-xs text-muted-foreground">{routeMeta.join(" · ")}</div>
                                      ) : null}
                                      <Button
                                        type="button"
                                        size="sm"
                                        variant="ghost"
                                        className="w-fit px-0"
                                        onClick={() => {
                                          void copyText(displayPath).then((copied) => {
                                            if (copied) {
                                              toast.success("路径已复制");
                                            }
                                          });
                                        }}
                                      >
                                        <Copy className="size-3.5" />
                                        复制路径
                                      </Button>
                                    </div>
                                  </TableCell>
                                  <TableCell>{item.model || "-"}</TableCell>
                                  <TableCell>{item.reasoningEffort || "-"}</TableCell>
                                  <TableCell>
                                    <Badge className={`rounded-full ${requestToneClass(tone)}`}>
                                      {item.statusCode || "未知"}
                                    </Badge>
                                  </TableCell>
                                  <TableCell className="max-w-60">
                                    <div className="line-clamp-3 text-sm text-muted-foreground">{item.error || "-"}</div>
                                  </TableCell>
                                  <TableCell>{item.traceId || "-"}</TableCell>
                                </TableRow>
                              );
                            })}
                            {requestLogVirtualWindow.bottomSpacerHeight > 0 ? (
                              <TableRow aria-hidden="true">
                                <TableCell
                                  colSpan={REQUEST_LOG_COLUMN_COUNT}
                                  className="h-0 border-0 p-0"
                                  style={{ height: requestLogVirtualWindow.bottomSpacerHeight }}
                                />
                              </TableRow>
                            ) : null}
                          </>
                        ) : (
                          <TableRow>
                            <TableCell colSpan={REQUEST_LOG_COLUMN_COUNT} className="h-24 text-center text-muted-foreground">
                              暂无请求日志
                            </TableCell>
                          </TableRow>
                        )}
                      </TableBody>
                    </Table>
                    {app.visibleRequestLogs.length > 0 ? (
                      <div className="flex items-center justify-between gap-3 border-t border-border/60 bg-background/85 px-4 py-3 text-xs text-muted-foreground">
                        <span>
                          当前已渲染 {renderedRequestLogs.length} / {app.visibleRequestLogs.length} 条日志
                        </span>
                        {hasMoreRequestLogs ? (
                          <Button
                            type="button"
                            size="sm"
                            variant="outline"
                            onClick={() => appendRequestLogBatch()}
                          >
                            继续加载 {Math.min(REQUEST_LOG_APPEND_BATCH, remainingRequestLogCount)} 条
                          </Button>
                        ) : (
                          <span>已显示全部结果</span>
                        )}
                      </div>
                    ) : null}
                  </ScrollArea>
                </div>
              </CardContent>
            </Card>

            <AlertDialog open={requestLogClearOpen} onOpenChange={setRequestLogClearOpen}>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>清空请求日志</AlertDialogTitle>
                  <AlertDialogDescription>确定清空全部请求日志吗？该操作不可撤销。</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel onClick={() => setRequestLogClearOpen(false)}>取消</AlertDialogCancel>
                  <AlertDialogAction
                    className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    disabled={requestLogClearPending}
                    onClick={(event) => {
                      event.preventDefault();
                      void (async () => {
                        setRequestLogClearPending(true);
                        const ok = await app.clearRequestLogs();
                        if (ok) {
                          await app.refreshRequestLogs({ silent: true });
                          setRequestLogClearOpen(false);
                        }
                        setRequestLogClearPending(false);
                      })();
                    }}
                  >
                    {requestLogClearPending ? <LoaderCircle className="size-4 animate-spin" /> : null}
                    立即清空
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </section>
        ) : null}
        {app.currentPage === "settings" ? (
          <section className="grid gap-5">
            <div className="grid gap-5 xl:grid-cols-2">
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>外观与桌面行为</CardTitle>
                  <CardDescription>主题、自动更新、托盘与透明度设置。</CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 pt-5">
                  <div className="grid gap-2">
                    <Label htmlFor="theme-select">主题</Label>
                    <NativeSelect
                      id="theme-select"
                      name="theme"
                      value={app.theme}
                      onChange={(event) => {
                        void app.setTheme(event.target.value as typeof app.theme);
                      }}
                    >
                      {themeOptions.map((option) => (
                        <NativeSelectOption key={option.id} value={option.id}>
                          {option.label}
                        </NativeSelectOption>
                      ))}
                    </NativeSelect>
                  </div>

                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${updateAutoCheckId}-title`}>自动检查更新</div>
                      <div className="text-xs text-muted-foreground" id={`${updateAutoCheckId}-description`}>
                        仅保存偏好，浏览器模式下不会执行桌面更新。
                      </div>
                    </div>
                    <Switch
                      checked={app.settings.updateAutoCheck}
                      id={updateAutoCheckId}
                      name="updateAutoCheck"
                      aria-labelledby={`${updateAutoCheckId}-title`}
                      aria-describedby={`${updateAutoCheckId}-description`}
                      onCheckedChange={(checked) => {
                        void app.saveSettingsPatch({ updateAutoCheck: checked }, "自动更新检查偏好已保存");
                      }}
                    />
                  </div>

                  <div className="grid gap-3 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="flex flex-col gap-3 xl:flex-row xl:items-start xl:justify-between">
                      <div className="grid gap-1">
                        <div className="text-sm font-medium">桌面更新器</div>
                        <div className="text-xs text-muted-foreground">
                          {app.desktopMode
                            ? `当前版本 ${formatVersionLabel(desktopUpdateStatus?.currentVersion)}，${desktopUpdateStatusText}`
                            : "仅桌面端支持检查、下载和安装更新。"}
                        </div>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <Button
                          type="button"
                          variant="outline"
                          disabled={!app.desktopMode || desktopUpdaterBusy}
                          onClick={() => {
                            void refreshDesktopUpdateStatus();
                          }}
                        >
                          <RefreshCw className={`size-4${desktopUpdaterBusy ? " animate-spin" : ""}`} />
                          刷新状态
                        </Button>
                        <Button
                          type="button"
                          disabled={!app.desktopMode || desktopUpdaterBusy}
                          onClick={() => {
                            void handleDesktopUpdateAction();
                          }}
                        >
                          {desktopUpdaterBusy ? <LoaderCircle className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
                          {desktopUpdateActionLabel}
                        </Button>
                        <Button
                          type="button"
                          variant="outline"
                          disabled={!app.desktopMode || !desktopUpdateStatus?.repo}
                          onClick={() => {
                            if (!desktopUpdateStatus?.repo) {
                              return;
                            }
                            void openInBrowser(`https://github.com/${desktopUpdateStatus.repo}/releases`);
                          }}
                        >
                          <Sparkles className="size-4" />
                          查看发布页
                        </Button>
                      </div>
                    </div>
                    <div className="grid gap-3 text-sm text-muted-foreground xl:grid-cols-3">
                      <div className="rounded-xl border border-border/60 bg-background/70 px-3 py-2">
                        <div className="text-[11px] uppercase tracking-[0.14em]">当前版本</div>
                        <div className="mt-1 font-medium text-foreground">
                          {formatVersionLabel(desktopUpdateStatus?.currentVersion)}
                        </div>
                      </div>
                      <div className="rounded-xl border border-border/60 bg-background/70 px-3 py-2">
                        <div className="text-[11px] uppercase tracking-[0.14em]">最近检查</div>
                        <div className="mt-1 font-medium text-foreground">
                          {latestDesktopUpdateCheck?.checkedAtUnixSecs
                            ? formatTimestamp(latestDesktopUpdateCheck.checkedAtUnixSecs, "尚未检查更新")
                            : "尚未检查更新"}
                        </div>
                      </div>
                      <div className="rounded-xl border border-border/60 bg-background/70 px-3 py-2">
                        <div className="text-[11px] uppercase tracking-[0.14em]">更新仓库</div>
                        <div className="mt-1 truncate font-medium text-foreground">
                          {desktopUpdateStatus?.repo || "--"}
                        </div>
                      </div>
                    </div>
                    {latestDesktopUpdateCheck?.hasUpdate ? (
                      <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-800">
                        已发现新版本 {formatVersionLabel(latestDesktopUpdateCheck.latestVersion)}
                        {latestDesktopUpdateCheck.canPrepare
                          ? "，可直接下载准备。"
                          : latestDesktopUpdateCheck.reason?.trim()
                            ? `：${latestDesktopUpdateCheck.reason.trim()}`
                            : "。"}
                      </div>
                    ) : null}
                    {desktopUpdateStatus?.lastError ? (
                      <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-sm text-rose-800">
                        最近一次更新检查失败：{desktopUpdateStatus.lastError}
                      </div>
                    ) : null}
                  </div>

                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${closeToTrayId}-title`}>关闭时最小化到托盘</div>
                      <div className="text-xs text-muted-foreground" id={`${closeToTrayId}-description`}>
                        {!app.desktopMode
                          ? "仅桌面端可用，用于保持后台运行。"
                          : closeToTraySupported
                            ? "桌面端可用，用于保持后台运行。"
                            : "当前系统托盘不可用，无法启用该行为。"}
                      </div>
                    </div>
                    <Switch
                      checked={app.settings.closeToTrayOnClose}
                      disabled={!closeToTraySupported}
                      id={closeToTrayId}
                      name="closeToTrayOnClose"
                      aria-labelledby={`${closeToTrayId}-title`}
                      aria-describedby={`${closeToTrayId}-description`}
                      onCheckedChange={(checked) => {
                        void app.saveSettingsPatch({ closeToTrayOnClose: checked }, "托盘行为已保存");
                      }}
                    />
                  </div>

                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${lightweightModeId}-title`}>托盘隐藏时释放窗口内存</div>
                      <div className="text-xs text-muted-foreground" id={`${lightweightModeId}-description`}>
                        {!app.desktopMode
                          ? "仅桌面端可用。"
                          : !closeToTraySupported
                            ? "当前系统托盘不可用，无法启用该策略。"
                            : !app.settings.closeToTrayOnClose
                              ? "需先开启“关闭时最小化到托盘”后才可调整。"
                              : "适合低配设备，减少后台占用。"}
                      </div>
                    </div>
                    <Switch
                      checked={app.settings.lightweightModeOnCloseToTray}
                      disabled={!lightweightModeAvailable}
                      id={lightweightModeId}
                      name="lightweightModeOnCloseToTray"
                      aria-labelledby={`${lightweightModeId}-title`}
                      aria-describedby={`${lightweightModeId}-description`}
                      onCheckedChange={(checked) => {
                        void app.saveSettingsPatch({ lightweightModeOnCloseToTray: checked }, "内存策略已保存");
                      }}
                    />
                  </div>

                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${lowTransparencyId}-title`}>低透明度 / 性能模式</div>
                      <div className="text-xs text-muted-foreground" id={`${lowTransparencyId}-description`}>
                        减少模糊和发光层，优先响应速度。
                      </div>
                    </div>
                    <Switch
                      checked={app.settings.lowTransparency}
                      id={lowTransparencyId}
                      name="lowTransparency"
                      aria-labelledby={`${lowTransparencyId}-title`}
                      aria-describedby={`${lowTransparencyId}-description`}
                      onCheckedChange={(checked) => {
                        void app.saveSettingsPatch({ lowTransparency: checked }, "视觉性能设置已保存");
                      }}
                    />
                  </div>
                </CardContent>
              </Card>

              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>服务与网关策略</CardTitle>
                  <CardDescription>
                    {browserMode
                      ? "浏览器模式下当前 RPC 入口只读展示，其余项保存为服务侧偏好。"
                      : "服务地址、监听模式、选路策略和头部收敛。"}
                  </CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 pt-5">
                  {browserMode ? (
                    <div className="rounded-2xl border border-dashed border-border/70 bg-background/60 px-4 py-3 text-sm text-muted-foreground">
                      当前 Web 控制台通过 Next 服务端代理访问 codexmanager-service。
                      这里不会切换浏览器当前连接目标，只会写回服务自身的持久化偏好。
                    </div>
                  ) : null}
                  <div className="grid gap-2">
                    <Label htmlFor="service-address">服务地址</Label>
                    <Input
                      aria-describedby="service-address-help"
                      disabled={browserMode}
                      id="service-address"
                      name="serviceAddress"
                      placeholder="localhost:48760"
                      value={serviceAddrDraft}
                      onChange={(event) => setServiceAddrDraft(event.target.value)}
                    />
                    <p id="service-address-help" className="text-xs leading-relaxed text-muted-foreground">
                      {browserMode
                        ? "浏览器模式下该字段只读。当前连接入口请通过 Next 运行环境中的 RPC 代理配置调整。"
                        : "桌面端保存后会同步更新前端连接目标，服务重启后按该地址重新启动。"}
                    </p>
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="listen-mode">监听模式</Label>
                    <NativeSelect
                      id="listen-mode"
                      name="serviceListenMode"
                      value={serviceListenMode}
                      onChange={(event) => setServiceListenMode(event.target.value)}
                    >
                      <NativeSelectOption value="loopback">仅本机</NativeSelectOption>
                      <NativeSelectOption value="all_interfaces">全部网卡</NativeSelectOption>
                    </NativeSelect>
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="route-strategy">网关选路</Label>
                    <NativeSelect
                      id="route-strategy"
                      name="routeStrategy"
                      value={routeStrategy}
                      onChange={(event) => setRouteStrategy(event.target.value)}
                    >
                      <NativeSelectOption value="ordered">顺序优先</NativeSelectOption>
                      <NativeSelectOption value="balanced">均衡轮询</NativeSelectOption>
                    </NativeSelect>
                  </div>
                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${headerModeId}-title`}>请求头收敛策略</div>
                      <div className="text-xs text-muted-foreground" id={`${headerModeId}-description`}>
                        移除高风险 Cookie 头，降低 WAF 误判概率。
                      </div>
                    </div>
                    <Switch
                      checked={app.settings.cpaNoCookieHeaderModeEnabled}
                      id={headerModeId}
                      name="cpaNoCookieHeaderModeEnabled"
                      aria-labelledby={`${headerModeId}-title`}
                      aria-describedby={`${headerModeId}-description`}
                      onCheckedChange={(checked) => {
                        void app.saveSettingsPatch({ cpaNoCookieHeaderModeEnabled: checked }, "请求头策略已保存");
                      }}
                    />
                  </div>
                  <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-dashed border-border/70 bg-background/60 px-4 py-3">
                    <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
                      {hasUnsavedServiceDrafts ? <Badge variant="secondary">本地草稿未保存</Badge> : null}
                      <span>
                        {app.serviceHint ||
                          (app.canManageService
                            ? "保存后立即应用到前端连接，服务监听模式重启后生效。"
                            : "浏览器模式当前连接目标由 Next 服务端环境变量决定；这里保存的是服务侧偏好配置。")}
                      </span>
                    </div>
                    <Button
                      type="button"
                      disabled={!hasUnsavedServiceDrafts}
                      onClick={() => {
                        void app
                          .saveSettingsPatch(
                            browserMode
                              ? { serviceListenMode, routeStrategy }
                              : { serviceAddr: serviceAddrDraft, serviceListenMode, routeStrategy },
                            browserMode ? "服务侧偏好已保存" : "服务与网关配置已保存",
                          )
                          .then((saved) => {
                            if (saved) {
                              applyServiceDraft();
                            }
                          });
                      }}
                    >
                      {browserMode ? "保存服务侧偏好" : "保存服务配置"}
                    </Button>
                  </div>
                </CardContent>
              </Card>
            </div>

            <div className="grid gap-5 xl:grid-cols-[0.95fr_1.05fr]">
              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>安全与传输</CardTitle>
                  <CardDescription>代理、流超时和 Web 访问密码。</CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 pt-5">
                  {hasUnsavedSecurityDrafts ? (
                    <div className="flex flex-wrap gap-2">
                      {proxyDirty ? <Badge variant="secondary">代理未保存</Badge> : null}
                      {transportDirty ? <Badge variant="secondary">传输参数未保存</Badge> : null}
                      {passwordDraftDirty ? <Badge variant="secondary">密码草稿未提交</Badge> : null}
                    </div>
                  ) : null}
                  <div className="grid gap-2">
                    <Label htmlFor="proxy-url">上游代理</Label>
                    <div className="flex flex-col gap-2 sm:flex-row">
                      <Input
                        id="proxy-url"
                        value={proxyDraft}
                        onChange={(event) => setProxyDraft(event.target.value)}
                        placeholder="http://127.0.0.1:7890"
                      />
                      <Button
                        type="button"
                        variant="outline"
                        disabled={!proxyDirty}
                        onClick={() => {
                          void app
                            .saveSettingsPatch({ upstreamProxyUrl: proxyDraft }, "代理设置已保存")
                            .then((saved) => {
                              if (saved) {
                                applySecurityDraft(saved);
                              }
                            });
                        }}
                      >
                        保存代理
                      </Button>
                    </div>
                  </div>
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="grid gap-2">
                      <Label htmlFor="sse-keepalive">SSE 保活间隔（毫秒）</Label>
                      <Input
                        id="sse-keepalive"
                        type="number"
                        value={transportForm.sseKeepaliveIntervalMs}
                        onChange={(event) =>
                          setTransportForm((current) => ({ ...current, sseKeepaliveIntervalMs: event.target.value }))
                        }
                      />
                    </div>
                    <div className="grid gap-2">
                      <Label htmlFor="stream-timeout">上游流式超时（毫秒）</Label>
                      <Input
                        id="stream-timeout"
                        type="number"
                        value={transportForm.upstreamStreamTimeoutMs}
                        onChange={(event) =>
                          setTransportForm((current) => ({ ...current, upstreamStreamTimeoutMs: event.target.value }))
                        }
                      />
                    </div>
                  </div>
                  <Button
                    type="button"
                    disabled={!transportDirty}
                    onClick={() => {
                      try {
                        const normalizedTransportForm = {
                          sseKeepaliveIntervalMs: String(
                            toPositiveInt(transportForm.sseKeepaliveIntervalMs, "SSE 保活间隔"),
                          ),
                          upstreamStreamTimeoutMs: String(
                            toPositiveInt(transportForm.upstreamStreamTimeoutMs, "上游流式超时", 0),
                          ),
                        };
                        setTransportForm(normalizedTransportForm);
                        void app
                          .saveSettingsPatch(
                            {
                              sseKeepaliveIntervalMs: Number(normalizedTransportForm.sseKeepaliveIntervalMs),
                              upstreamStreamTimeoutMs: Number(normalizedTransportForm.upstreamStreamTimeoutMs),
                            },
                            "传输参数已保存",
                          )
                          .then((saved) => {
                            if (saved) {
                              applySecurityDraft(saved);
                            }
                          });
                      } catch (error) {
                        toast.error(error instanceof Error ? error.message : String(error));
                      }
                    }}
                  >
                    保存传输设置
                  </Button>
                  <form
                    className="grid gap-3"
                    onSubmit={(event) => {
                      event.preventDefault();
                      void savePassword();
                    }}
                  >
                    <input
                      tabIndex={-1}
                      autoComplete="username"
                      className="sr-only"
                      defaultValue="codexmanager-web-access"
                      name="username"
                      type="text"
                    />
                    <div className="grid gap-2">
                      <Label htmlFor="web-password">Web 访问密码</Label>
                      <Input
                        id="web-password"
                        name="webPassword"
                        type="password"
                        autoComplete="new-password"
                        value={webPassword}
                        onChange={(event) => setWebPassword(event.target.value)}
                        placeholder="输入新密码"
                      />
                    </div>
                    <div className="grid gap-2">
                      <Label htmlFor="web-password-confirm">再次输入确认</Label>
                      <Input
                        id="web-password-confirm"
                        name="webPasswordConfirm"
                        type="password"
                        autoComplete="new-password"
                        value={webPasswordConfirm}
                        onChange={(event) => setWebPasswordConfirm(event.target.value)}
                        placeholder="再次输入确认"
                      />
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Button type="submit">保存密码</Button>
                      <Button
                        type="button"
                        variant="outline"
                        onClick={() => {
                          void app.saveSettingsPatch({ webAccessPassword: "" }, "Web 访问密码已清除").then((saved) => {
                            if (saved) {
                              setWebPassword("");
                              setWebPasswordConfirm("");
                            }
                          });
                        }}
                      >
                        清除密码
                      </Button>
                      <Badge variant="secondary" className="rounded-full">
                        {app.settings.webAccessPasswordConfigured ? "已启用保护" : "未启用保护"}
                      </Badge>
                    </div>
                  </form>
                </CardContent>
              </Card>

              <Card className="border-border/70 bg-card/85 shadow-none">
                <CardHeader className="border-b border-border/60">
                  <CardTitle>后台任务与并发</CardTitle>
                  <CardDescription>轮询开关、刷新线程和 HTTP Worker 参数。</CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 pt-5">
                  {hasUnsavedBackgroundDrafts ? (
                    <div className="flex flex-wrap gap-2">
                      <Badge variant="secondary">本地草稿 {dirtyFieldCount} 项</Badge>
                    </div>
                  ) : null}
                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${usagePollingId}-title`}>用量轮询线程</div>
                      <div className="text-xs text-muted-foreground" id={`${usagePollingId}-description`}>
                        后台定时刷新账号用量。
                      </div>
                    </div>
                    <Switch
                      checked={backgroundForm.usagePollingEnabled}
                      id={usagePollingId}
                      name="usagePollingEnabled"
                      aria-labelledby={`${usagePollingId}-title`}
                      aria-describedby={`${usagePollingId}-description`}
                      onCheckedChange={(checked) =>
                        setBackgroundForm((current) => ({ ...current, usagePollingEnabled: checked }))
                      }
                    />
                  </div>
                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${gatewayKeepaliveId}-title`}>网关保活线程</div>
                      <div className="text-xs text-muted-foreground" id={`${gatewayKeepaliveId}-description`}>
                        保持网关连接活跃。
                      </div>
                    </div>
                    <Switch
                      checked={backgroundForm.gatewayKeepaliveEnabled}
                      id={gatewayKeepaliveId}
                      name="gatewayKeepaliveEnabled"
                      aria-labelledby={`${gatewayKeepaliveId}-title`}
                      aria-describedby={`${gatewayKeepaliveId}-description`}
                      onCheckedChange={(checked) =>
                        setBackgroundForm((current) => ({ ...current, gatewayKeepaliveEnabled: checked }))
                      }
                    />
                  </div>
                  <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/70 bg-background/75 px-4 py-3">
                    <div className="grid gap-1">
                      <div className="text-sm font-medium" id={`${tokenRefreshId}-title`}>令牌刷新轮询线程</div>
                      <div className="text-xs text-muted-foreground" id={`${tokenRefreshId}-description`}>
                        后台轮询更新令牌状态。
                      </div>
                    </div>
                    <Switch
                      checked={backgroundForm.tokenRefreshPollingEnabled}
                      id={tokenRefreshId}
                      name="tokenRefreshPollingEnabled"
                      aria-labelledby={`${tokenRefreshId}-title`}
                      aria-describedby={`${tokenRefreshId}-description`}
                      onCheckedChange={(checked) =>
                        setBackgroundForm((current) => ({ ...current, tokenRefreshPollingEnabled: checked }))
                      }
                    />
                  </div>
                  <div className="grid gap-4 md:grid-cols-2">
                    {numericFields.map((field) => (
                      <div key={field.id} className="grid gap-2">
                        <Label htmlFor={field.id}>{field.label}</Label>
                        <Input
                          id={field.id}
                          name={field.name}
                          inputMode="numeric"
                          value={field.value}
                          onChange={(event) => field.update(event.target.value)}
                          placeholder={field.label}
                        />
                      </div>
                    ))}
                  </div>
                  <Button
                    type="button"
                    disabled={!hasUnsavedBackgroundDrafts}
                    onClick={() => {
                      try {
                        const normalizedBackgroundTasks = {
                          usagePollingEnabled: backgroundForm.usagePollingEnabled,
                          usagePollIntervalSecs: toPositiveInt(backgroundForm.usagePollIntervalSecs, "用量轮询间隔"),
                          gatewayKeepaliveEnabled: backgroundForm.gatewayKeepaliveEnabled,
                          gatewayKeepaliveIntervalSecs: toPositiveInt(
                            backgroundForm.gatewayKeepaliveIntervalSecs,
                            "网关保活间隔",
                          ),
                          tokenRefreshPollingEnabled: backgroundForm.tokenRefreshPollingEnabled,
                          tokenRefreshPollIntervalSecs: toPositiveInt(
                            backgroundForm.tokenRefreshPollIntervalSecs,
                            "令牌刷新间隔",
                          ),
                          usageRefreshWorkers: toPositiveInt(backgroundForm.usageRefreshWorkers, "用量刷新线程数"),
                          httpWorkerFactor: toPositiveInt(backgroundForm.httpWorkerFactor, "普通请求并发因子"),
                          httpWorkerMin: toPositiveInt(backgroundForm.httpWorkerMin, "普通请求最小并发"),
                          httpStreamWorkerFactor: toPositiveInt(backgroundForm.httpStreamWorkerFactor, "流式请求并发因子"),
                          httpStreamWorkerMin: toPositiveInt(backgroundForm.httpStreamWorkerMin, "流式请求最小并发"),
                        };
                        setBackgroundForm({
                          usagePollingEnabled: normalizedBackgroundTasks.usagePollingEnabled,
                          usagePollIntervalSecs: String(normalizedBackgroundTasks.usagePollIntervalSecs),
                          gatewayKeepaliveEnabled: normalizedBackgroundTasks.gatewayKeepaliveEnabled,
                          gatewayKeepaliveIntervalSecs: String(normalizedBackgroundTasks.gatewayKeepaliveIntervalSecs),
                          tokenRefreshPollingEnabled: normalizedBackgroundTasks.tokenRefreshPollingEnabled,
                          tokenRefreshPollIntervalSecs: String(normalizedBackgroundTasks.tokenRefreshPollIntervalSecs),
                          usageRefreshWorkers: String(normalizedBackgroundTasks.usageRefreshWorkers),
                          httpWorkerFactor: String(normalizedBackgroundTasks.httpWorkerFactor),
                          httpWorkerMin: String(normalizedBackgroundTasks.httpWorkerMin),
                          httpStreamWorkerFactor: String(normalizedBackgroundTasks.httpStreamWorkerFactor),
                          httpStreamWorkerMin: String(normalizedBackgroundTasks.httpStreamWorkerMin),
                        });
                        void app
                          .saveSettingsPatch({ backgroundTasks: normalizedBackgroundTasks }, "后台任务配置已保存")
                          .then((saved) => {
                            if (saved) {
                              applyBackgroundForm(saved);
                            }
                          });
                      } catch (error) {
                        toast.error(error instanceof Error ? error.message : String(error));
                      }
                    }}
                  >
                    保存后台任务配置
                  </Button>
                </CardContent>
              </Card>
            </div>

            <Card className="border-border/70 bg-card/85 shadow-none">
              <CardHeader className="border-b border-border/60">
                <CardTitle>高级环境变量覆盖</CardTitle>
                <CardDescription>维护 CODEXMANAGER_* 相关运行时覆盖项。</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-4 pt-5">
                {hasEnvCatalog ? (
                  <>
                    <div className="grid gap-4 xl:grid-cols-[14rem_1fr]">
                      <div className="grid gap-2">
                        <Label htmlFor="env-search">搜索变量</Label>
                        <Input
                          id="env-search"
                          value={envSearch}
                          onChange={(event) => setEnvSearch(event.target.value)}
                          placeholder="按名称或 key 搜索"
                        />
                      </div>
                      <div className="grid gap-2">
                        <Label htmlFor="env-select">变量</Label>
                        <NativeSelect
                          id="env-select"
                          disabled={!activeEnvKey}
                          value={activeEnvKey}
                          onChange={(event) => setSelectedEnvKey(event.target.value)}
                        >
                          {filteredEnvItems.length > 0 ? (
                            filteredEnvItems.map((item) => (
                              <NativeSelectOption key={item.key} value={item.key}>
                                {`${buildEnvOverrideOptionLabel(item)}${dirtyKeys.includes(item.key) ? " · 未保存" : ""}`}
                              </NativeSelectOption>
                            ))
                          ) : (
                            <NativeSelectOption value="">无匹配变量</NativeSelectOption>
                          )}
                        </NativeSelect>
                      </div>
                    </div>

                    {filteredEnvItems.length > 0 ? (
                      <div className="rounded-2xl border border-border/70 bg-background/75 p-4">
                        <div className="flex flex-wrap items-center gap-3">
                          <div className="text-base font-semibold">{buildEnvOverrideOptionLabel(activeEnvItem)}</div>
                          <Badge variant="outline">{activeEnvKey || "-"}</Badge>
                          {activeEnvItem ? (
                            <Badge variant="outline">{formatEnvOverrideScopeLabel(activeEnvItem.scope)}</Badge>
                          ) : null}
                          {activeEnvItem ? (
                            <Badge variant="outline">{formatEnvOverrideApplyModeLabel(activeEnvItem.applyMode)}</Badge>
                          ) : null}
                          {activeEnvDirty ? <Badge variant="secondary">当前变量未保存</Badge> : null}
                          {hasUnsavedEnvDrafts ? <Badge variant="secondary">本地草稿 {dirtyKeys.length}</Badge> : null}
                        </div>
                        <div className="mt-2 text-sm text-muted-foreground">
                          {buildEnvOverrideDescription(activeEnvItem)}
                        </div>
                        <div className="mt-3 grid gap-1 text-xs text-muted-foreground">
                          <div>{activeEnvHint}</div>
                          <div>已保存值：{formatEnvOverrideDisplayValue(savedEnvValue)}</div>
                          <div>默认值：{formatEnvOverrideDisplayValue(defaultEnvValue)}</div>
                        </div>
                        <div className="mt-4 grid gap-2">
                          <Label htmlFor="env-value">当前值</Label>
                          <Textarea
                            id="env-value"
                            rows={4}
                            value={envValue}
                            placeholder={activeEnvKey ? "留空并保存可恢复默认值" : "请先选择变量"}
                            onChange={(event) => {
                              if (!activeEnvKey) {
                                return;
                              }
                              setDraftOverrides((current) => ({
                                ...current,
                                [activeEnvKey]: event.target.value,
                              }));
                            }}
                          />
                        </div>
                        <div className="mt-4 flex flex-wrap gap-2">
                          <Button
                            type="button"
                            disabled={!activeEnvKey || !activeEnvDirty}
                            onClick={async () => {
                              if (!activeEnvKey) {
                                return;
                              }
                              const saved = await app.saveSettingsPatch(
                                {
                                  envOverrides: {
                                    [activeEnvKey]: draftOverrides[activeEnvKey] ?? "",
                                  },
                                },
                                "环境变量覆盖已保存",
                              );
                              if (saved) {
                                applySavedEnvValue(saved, activeEnvKey);
                              }
                            }}
                          >
                            保存变量
                          </Button>
                          <Button
                            type="button"
                            variant="outline"
                            disabled={!canRestoreDefault}
                            onClick={async () => {
                              if (!activeEnvKey) {
                                return;
                              }
                              const saved = await app.saveSettingsPatch(
                                {
                                  envOverrides: {
                                    [activeEnvKey]: "",
                                  },
                                },
                                "已恢复默认值",
                              );
                              if (saved) {
                                applySavedEnvValue(saved, activeEnvKey);
                              }
                            }}
                          >
                            恢复默认
                          </Button>
                        </div>
                      </div>
                    ) : (
                      <div className="rounded-2xl border border-dashed border-border/70 bg-background/60 px-4 py-8 text-center text-sm text-muted-foreground">
                        当前搜索条件下没有匹配的环境变量。
                      </div>
                    )}
                  </>
                ) : (
                  <div className="rounded-2xl border border-dashed border-border/70 bg-background/60 px-4 py-8 text-center text-sm text-muted-foreground">
                    当前没有可编辑的环境变量覆盖项。
                  </div>
                )}
              </CardContent>
            </Card>
          </section>
        ) : null}
            </div>
          </div>
          <AlertDialog
            open={confirmSyncOpen}
            onOpenChange={(open) => {
              if (!confirmSyncPending) {
                setConfirmSyncOpen(open);
              }
            }}
          >
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>覆盖未保存草稿？</AlertDialogTitle>
                <AlertDialogDescription>
                  {`当前设置页还有 ${settingsDraftSectionCount} 个未保存区块${dirtySectionNames ? `：${dirtySectionNames}` : ""}。继续全局同步会用服务端当前值覆盖这些本地草稿。`}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel disabled={confirmSyncPending}>先不同步</AlertDialogCancel>
                <AlertDialogAction
                  disabled={confirmSyncPending}
                  onClick={(event) => {
                    event.preventDefault();
                    void (async () => {
                      setConfirmSyncPending(true);
                      try {
                        await app.refreshAll();
                      } finally {
                        setConfirmSyncPending(false);
                        setConfirmSyncOpen(false);
                      }
                    })();
                  }}
                >
                  {confirmSyncPending ? <LoaderCircle className="size-4 animate-spin" /> : null}
                  继续同步
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
          <AlertDialog
            open={confirmLeaveOpen}
            onOpenChange={(open) => {
              setConfirmLeaveOpen(open);
              if (!open) {
                setPendingPage(null);
              }
            }}
          >
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>离开设置页？</AlertDialogTitle>
                <AlertDialogDescription>
                  {`当前设置页还有 ${settingsDraftSectionCount} 个未保存区块${dirtySectionNames ? `：${dirtySectionNames}` : ""}。离开页面会丢失这些本地草稿。`}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>留在设置</AlertDialogCancel>
                <AlertDialogAction
                  onClick={(event) => {
                    event.preventDefault();
                    if (pendingPage) {
                      app.setCurrentPage(pendingPage);
                    }
                    setPendingPage(null);
                    setConfirmLeaveOpen(false);
                  }}
                >
                  仍要离开
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </SidebarInset>
      </SidebarProvider>
    </main>
  );
}
