"use client";

import {
  useCallback,
  createContext,
  startTransition,
  useContext,
  useDeferredValue,
  useEffect,
  useEffectEvent,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useTheme } from "next-themes";
import { toast } from "sonner";

import { pageMeta } from "@/lib/codex/ui-config";
import {
  EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS,
  normalizeAccountUsageRefreshProgress,
} from "@/lib/codex/account-usage-refresh";
import { isMissingCommandError, normalizeImportContentForCompatibility } from "@/lib/codex/app-helpers";
import * as client from "@/lib/codex/client";
import {
  calculateAvailability,
  computeAggregateRemainingStats,
  EMPTY_REQUEST_LOG_TODAY_SUMMARY,
  EMPTY_USAGE_AGGREGATE_SUMMARY,
  remainingPercent,
} from "@/lib/codex/format";
import { buildRequestLogIdentity, normalizeRequestLogTodaySummary, normalizeStartupSnapshot } from "@/lib/codex/normalize";
import { createDefaultSettingsSnapshot } from "@/lib/codex/settings";
import { normalizeServiceAddress } from "@/lib/codex/service-address";
import type {
  AccountRecord,
  AccountUsageRefreshProgress,
  ActionResult,
  ApiKeyRecord,
  AppSettingsSnapshot,
  AvailabilityStatus,
  PageId,
  RequestLogRecord,
  RequestLogTodaySummary,
  ThemeId,
  UsageAggregateSummary,
  UsageSnapshot,
} from "@/lib/codex/types";

type ApiModelOption = {
  slug: string;
  displayName?: string | null;
};

type VisibleAccountRecord = AccountRecord & {
  usage?: UsageSnapshot;
  availability: AvailabilityStatus;
  primaryRemain: number | null;
  secondaryRemain: number | null;
};

type ApiKeyCreateInput = {
  name: string | null;
  modelSlug: string | null;
  reasoningEffort: string | null;
  protocolType: string | null;
  upstreamBaseUrl: string | null;
  staticHeadersJson: string | null;
};

type AccountLoginInput = {
  loginType?: string;
  groupName: string;
  tags: string;
  note: string;
};

type RefreshOptions = {
  silent?: boolean;
};

export type CodexAppContextValue = {
  currentPage: PageId;
  setCurrentPage: (page: PageId) => void;
  pageTitle: string;
  pageDescription: string;
  pageKicker: string;
  runtimeModeResolved: boolean;
  desktopMode: boolean;
  canManageService: boolean;
  theme: ThemeId;
  setTheme: (theme: ThemeId) => Promise<void>;
  settings: AppSettingsSnapshot;
  settingsSyncRevision: number;
  settingsDraftSections: string[];
  setSettingsDraftState: (sectionId: string, dirty: boolean) => void;
  serviceAddress: string;
  setServiceAddress: (value: string) => void;
  serviceConnected: boolean;
  serviceBusy: boolean;
  serviceHint: string;
  loading: boolean;
  refreshing: boolean;
  accounts: AccountRecord[];
  visibleAccounts: VisibleAccountRecord[];
  accountUsageById: Map<string, UsageSnapshot>;
  accountLabelsById: Map<string, string>;
  accountGroups: string[];
  accountSearch: string;
  setAccountSearch: (value: string) => void;
  accountFilter: string;
  setAccountFilter: (value: string) => void;
  accountGroupFilter: string;
  setAccountGroupFilter: (value: string) => void;
  accountPage: number;
  setAccountPage: (value: number) => void;
  accountPageSize: number;
  setAccountPageSize: (value: number) => void;
  accountPageLoading: boolean;
  accountTotal: number;
  apiKeys: ApiKeyRecord[];
  apiModels: ApiModelOption[];
  requestLogs: RequestLogRecord[];
  visibleRequestLogs: RequestLogRecord[];
  requestLogQuery: string;
  setRequestLogQuery: (value: string) => void;
  requestLogStatusFilter: string;
  setRequestLogStatusFilter: (value: string) => void;
  requestLogTodaySummary: RequestLogTodaySummary;
  usageAggregateSummary: UsageAggregateSummary;
  usageSnapshots: UsageSnapshot[];
  manualPreferredAccountId: string;
  accountUsageRefreshProgress: AccountUsageRefreshProgress;
  refreshAll: (options?: RefreshOptions) => Promise<boolean>;
  refreshRequestLogs: (options?: RefreshOptions) => Promise<boolean>;
  refreshAccountUsage: (accountId: string) => Promise<UsageSnapshot | null>;
  refreshAllAccountUsage: () => Promise<boolean>;
  toggleServiceConnection: () => Promise<void>;
  saveSettingsPatch: (
    patch: Partial<AppSettingsSnapshot> & Record<string, unknown>,
    successMessage?: string | null,
  ) => Promise<AppSettingsSnapshot | null>;
  createApiKey: (input: ApiKeyCreateInput) => Promise<{ id?: string; key?: string } | null>;
  refreshApiModels: (refreshRemote?: boolean) => Promise<boolean>;
  updateApiKeyModel: (
    item: ApiKeyRecord,
    modelSlug: string | null,
    reasoningEffort: string | null,
  ) => Promise<boolean>;
  deleteApiKey: (item: ApiKeyRecord) => Promise<boolean>;
  toggleApiKeyStatus: (item: ApiKeyRecord) => Promise<boolean>;
  readApiKeySecret: (item: ApiKeyRecord) => Promise<string>;
  clearRequestLogs: () => Promise<boolean>;
  deleteAccount: (accountId: string) => Promise<boolean>;
  deleteAccounts: (accountIds: string[]) => Promise<{ deleted: number; failed: number }>;
  deleteUnavailableFreeAccounts: () => Promise<boolean>;
  importAccountsFromDirectory: () => Promise<boolean>;
  exportAccountsByFile: () => Promise<boolean>;
  updateAccountSort: (accountId: string, sort: number, previousSort?: number | null) => Promise<boolean>;
  importAccountsFromFiles: (files: FileList | File[]) => Promise<boolean>;
  setManualPreferredAccountId: (accountId: string) => Promise<boolean>;
  startAccountLogin: (input: AccountLoginInput) => Promise<{ authUrl?: string; loginId?: string; warning?: string } | null>;
  getAccountLoginStatus: (
    loginId: string,
    options?: { signal?: AbortSignal; timeoutMs?: number },
  ) => Promise<{ status?: string; error?: string }>;
  completeAccountLogin: (payload: {
    state: string;
    code: string;
    redirectUri?: string;
  }) => Promise<boolean>;
};

const CodexAppContext = createContext<CodexAppContextValue | null>(null);

function formatConnectionError(error: unknown) {
  const raw = client.getErrorMessage(error).trim();
  if (!raw) {
    return "未知错误";
  }

  const lower = raw.toLowerCase();
  if (lower.includes("http 404")) {
    return "未检测到服务，请先启动 codexmanager-service";
  }
  if (lower.includes("fetch failed")) {
    return "无法连接到本地服务";
  }
  return raw;
}

function normalizeApiModels(items: unknown) {
  if (!Array.isArray(items)) {
    return [] as ApiModelOption[];
  }

  return items
    .map<ApiModelOption | null>((item) => {
      if (typeof item === "string") {
        return { slug: item, displayName: item } satisfies ApiModelOption;
      }
      if (!item || typeof item !== "object") {
        return null;
      }
      const record = item as Record<string, unknown>;
      const slug = String(record.slug || record.id || record.modelSlug || "").trim();
      if (!slug) {
        return null;
      }
      return {
        slug,
        displayName:
          typeof record.displayName === "string"
            ? record.displayName
            : typeof record.name === "string"
              ? record.name
              : slug,
      } satisfies ApiModelOption;
    })
    .filter((item): item is ApiModelOption => item !== null);
}

function matchesAccountFilter(
  account: AccountRecord,
  usage: UsageSnapshot | undefined,
  filter: string,
  query: string,
  groupFilter: string,
) {
  if (groupFilter !== "all" && String(account.groupName || "") !== groupFilter) {
    return false;
  }

  const normalizedQuery = query.trim().toLowerCase();
  if (normalizedQuery) {
    const haystack = [account.id, account.label, account.groupName, account.note]
      .map((item) => String(item || "").toLowerCase())
      .join(" ");
    if (!haystack.includes(normalizedQuery)) {
      return false;
    }
  }

  const availability = calculateAvailability(usage, account);
  if (filter === "active") {
    return availability.level === "ok";
  }
  if (filter === "low") {
    const primaryRemain = remainingPercent(usage?.usedPercent);
    const secondaryRemain = remainingPercent(usage?.secondaryUsedPercent);
    return (primaryRemain != null && primaryRemain <= 20) || (secondaryRemain != null && secondaryRemain <= 20);
  }
  return true;
}

function matchesRequestLogFilter(item: RequestLogRecord, query: string, statusFilter: string) {
  const statusCode = Number(item.statusCode || 0);
  if (statusFilter === "2xx" && (statusCode < 200 || statusCode >= 300)) {
    return false;
  }
  if (statusFilter === "4xx" && (statusCode < 400 || statusCode >= 500)) {
    return false;
  }
  if (statusFilter === "5xx" && statusCode < 500) {
    return false;
  }

  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) {
    return true;
  }

  const haystack = [
    item.method,
    item.requestPath,
    item.originalPath,
    item.path,
    item.mappedPath,
    item.adaptedPath,
    item.responseAdapter,
    item.upstreamUrl,
    item.keyId,
    item.accountId,
    item.accountLabel,
    item.model,
    item.reasoningEffort,
    item.error,
    item.statusCode,
    item.traceId,
  ]
    .map((value) => String(value || "").toLowerCase())
    .join(" ");

  return haystack.includes(normalizedQuery);
}

async function readFilesSequentially(files: File[]) {
  const contents: string[] = [];
  for (const file of files) {
    const text = String(await file.text()).trim();
    if (text) {
      contents.push(normalizeImportContentForCompatibility(text));
    }
  }
  return contents;
}

function sleep(ms: number, signal?: AbortSignal) {
  return new Promise<void>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, ms);

    function onAbort() {
      window.clearTimeout(timer);
      reject(new DOMException("Aborted", "AbortError"));
    }

    if (signal) {
      signal.addEventListener("abort", onAbort, { once: true });
    }
  });
}

export function CodexAppProvider({ children }: { children: ReactNode }) {
  const initializedRef = useRef(false);
  const accountUsageRefreshInFlightRef = useRef<Promise<boolean> | null>(null);
  const accountUsageRefreshClearTimerRef = useRef<number | null>(null);
  const { setTheme: setResolvedTheme } = useTheme();

  const [runtimeModeResolved, setRuntimeModeResolved] = useState(false);
  const [desktopMode, setDesktopModeState] = useState(false);
  const [currentPage, setCurrentPage] = useState<PageId>("dashboard");
  const [theme, setThemeState] = useState<ThemeId>("tech");
  const [settings, setSettings] = useState<AppSettingsSnapshot>(createDefaultSettingsSnapshot());
  const [settingsSyncRevision, setSettingsSyncRevision] = useState(0);
  const [settingsDraftSectionState, setSettingsDraftSectionState] = useState<Record<string, boolean>>({});
  const [serviceAddress, setServiceAddressState] = useState("localhost:48760");
  const [serviceConnected, setServiceConnected] = useState(false);
  const [serviceBusy, setServiceBusy] = useState(false);
  const [serviceHint, setServiceHint] = useState("");
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [accounts, setAccounts] = useState<AccountRecord[]>([]);
  const [usageSnapshots, setUsageSnapshots] = useState<UsageSnapshot[]>([]);
  const [apiKeys, setApiKeys] = useState<ApiKeyRecord[]>([]);
  const [apiModels, setApiModels] = useState<ApiModelOption[]>([]);
  const [requestLogs, setRequestLogs] = useState<RequestLogRecord[]>([]);
  const [requestLogTodaySummary, setRequestLogTodaySummary] = useState<RequestLogTodaySummary>(
    EMPTY_REQUEST_LOG_TODAY_SUMMARY,
  );
  const [usageAggregateSummary, setUsageAggregateSummary] = useState<UsageAggregateSummary>(
    EMPTY_USAGE_AGGREGATE_SUMMARY,
  );
  const [manualPreferredAccountId, setManualPreferredAccountIdState] = useState("");
  const [accountUsageRefreshProgress, setAccountUsageRefreshProgressState] = useState<AccountUsageRefreshProgress>(
    EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS,
  );
  const [accountSearch, setAccountSearchState] = useState("");
  const [accountFilter, setAccountFilterState] = useState("all");
  const [accountGroupFilter, setAccountGroupFilterState] = useState("all");
  const [accountPage, setAccountPageState] = useState(1);
  const [accountPageSize, setAccountPageSizeState] = useState(10);
  const [accountPageItems, setAccountPageItems] = useState<AccountRecord[]>([]);
  const [accountPageTotal, setAccountPageTotal] = useState(0);
  const [accountPageLoaded, setAccountPageLoaded] = useState(false);
  const [accountPageLoading, setAccountPageLoading] = useState(false);
  const [requestLogQuery, setRequestLogQueryState] = useState("");
  const [requestLogStatusFilter, setRequestLogStatusFilterState] = useState("all");

  const deferredAccountSearch = useDeferredValue(accountSearch);
  const deferredRequestLogQuery = useDeferredValue(requestLogQuery);

  const clearAccountUsageRefreshTimer = useCallback(() => {
    if (accountUsageRefreshClearTimerRef.current != null) {
      window.clearTimeout(accountUsageRefreshClearTimerRef.current);
      accountUsageRefreshClearTimerRef.current = null;
    }
  }, []);

  const setAccountUsageRefreshProgress = useCallback((next?: Partial<AccountUsageRefreshProgress> | null) => {
    setAccountUsageRefreshProgressState(normalizeAccountUsageRefreshProgress(next));
  }, []);

  const scheduleAccountUsageRefreshProgressClear = useCallback(() => {
    clearAccountUsageRefreshTimer();
    accountUsageRefreshClearTimerRef.current = window.setTimeout(() => {
      setAccountUsageRefreshProgressState(EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS);
      accountUsageRefreshClearTimerRef.current = null;
    }, 450);
  }, [clearAccountUsageRefreshTimer]);

  useEffect(() => clearAccountUsageRefreshTimer, [clearAccountUsageRefreshTimer]);

  useEffect(() => {
    setDesktopModeState(client.isTauriRuntime());
    setRuntimeModeResolved(true);
  }, []);

  useEffect(() => {
    setResolvedTheme(theme);
  }, [setResolvedTheme, theme]);

  const usageByAccountId = useMemo(
    () => new Map(usageSnapshots.map((item) => [item.accountId, item])),
    [usageSnapshots],
  );
  const accountLabelsById = useMemo(
    () =>
      new Map(
        accounts
          .filter((item) => item.id && item.label)
          .map((item) => [item.id, String(item.label || item.id)]),
      ),
    [accounts],
  );

  const accountGroups = useMemo(
    () =>
      Array.from(new Set(accounts.map((item) => String(item.groupName || "").trim()).filter(Boolean))).sort(),
    [accounts],
  );

  const filteredAccounts = useMemo(
    () =>
      accounts.filter((account) =>
        matchesAccountFilter(
          account,
          usageByAccountId.get(account.id),
          accountFilter,
          deferredAccountSearch,
          accountGroupFilter,
        ),
      ),
    [accountFilter, accountGroupFilter, accounts, deferredAccountSearch, usageByAccountId],
  );

  const accountTotal = currentPage === "accounts" && accountPageLoaded ? accountPageTotal : filteredAccounts.length;
  const maxAccountPage = Math.max(1, Math.ceil(accountTotal / accountPageSize));

  useEffect(() => {
    if (accountPage > maxAccountPage) {
      setAccountPageState(maxAccountPage);
    }
  }, [accountPage, maxAccountPage]);

  const visibleAccounts = useMemo(
    () =>
      (currentPage === "accounts" && accountPageLoaded
        ? accountPageItems
        : filteredAccounts.slice((accountPage - 1) * accountPageSize, accountPage * accountPageSize))
        .map((account) => {
          const usage = usageByAccountId.get(account.id);
          return {
            ...account,
            usage,
            availability: calculateAvailability(usage, account),
            primaryRemain: remainingPercent(usage?.usedPercent),
            secondaryRemain: remainingPercent(usage?.secondaryUsedPercent),
          } satisfies VisibleAccountRecord;
        }),
    [accountPage, accountPageItems, accountPageLoaded, accountPageSize, currentPage, filteredAccounts, usageByAccountId],
  );

  const visibleRequestLogs = useMemo(
    () => requestLogs.filter((item) => matchesRequestLogFilter(item, deferredRequestLogQuery, requestLogStatusFilter)),
    [deferredRequestLogQuery, requestLogStatusFilter, requestLogs],
  );

  const settingsDraftSections = useMemo(
    () =>
      Object.entries(settingsDraftSectionState)
        .filter(([, dirty]) => dirty)
        .map(([sectionId]) => sectionId)
        .sort(),
    [settingsDraftSectionState],
  );

  const setSettingsDraftState = useCallback((sectionId: string, dirty: boolean) => {
    startTransition(() => {
      setSettingsDraftSectionState((current) => {
        if (!sectionId) {
          return current;
        }

        const nextDirty = !!dirty;
        const currentDirty = !!current[sectionId];
        if (currentDirty === nextDirty) {
          return current;
        }

        const next = { ...current };
        if (nextDirty) {
          next[sectionId] = true;
        } else {
          delete next[sectionId];
        }
        return next;
      });
    });
  }, []);

  const initializeConnection = useCallback(async (nextAddress: string, options: { silent?: boolean } = {}) => {
    try {
      await client.serviceInitialize(nextAddress, { timeoutMs: 4000 });
      startTransition(() => {
        setServiceConnected(true);
        setServiceHint("");
      });
      return true;
    } catch (error) {
      const message = formatConnectionError(error);
      startTransition(() => {
        setServiceConnected(false);
        setServiceHint(message);
      });
      if (!options.silent) {
        toast.error(message);
      }
      return false;
    }
  }, []);

  const ensureConnected = useCallback(
    async (options: { silent?: boolean } = {}) => {
      const nextAddress = normalizeServiceAddress(serviceAddress);
      if (serviceConnected) {
        return nextAddress;
      }
      const connected = await initializeConnection(nextAddress, options);
      return connected ? nextAddress : null;
    },
    [initializeConnection, serviceAddress, serviceConnected],
  );

  const refreshAccountPage = useCallback(
    async (options?: { silent?: boolean; address?: string }) => {
      if (currentPage !== "accounts") {
        return true;
      }

      startTransition(() => {
        setAccountPageLoading(true);
      });

      try {
        const nextAddress = options?.address || (await ensureConnected({ silent: true }));
        if (!nextAddress) {
          startTransition(() => {
            setAccountPageLoaded(false);
            setAccountPageItems([]);
            setAccountPageTotal(0);
          });
          if (!options?.silent) {
            toast.error("连接服务失败，无法刷新账号分页");
          }
          return false;
        }

        const listResult = await client.serviceAccountList(nextAddress, {
          page: accountPage,
          pageSize: accountPageSize,
          query: deferredAccountSearch,
          filter: accountFilter,
          groupFilter: accountGroupFilter,
        });

        const nextItems = Array.isArray(listResult.items)
          ? listResult.items.filter((item) => String(item?.id || "").trim())
          : [];
        const nextTotal = Number.isFinite(Number(listResult.total))
          ? Math.max(0, Number(listResult.total))
          : nextItems.length;

        startTransition(() => {
          setAccountPageItems(nextItems);
          setAccountPageTotal(nextTotal);
          setAccountPageLoaded(true);
        });
        return true;
      } catch (error) {
        startTransition(() => {
          setAccountPageLoaded(false);
          setAccountPageItems([]);
          setAccountPageTotal(0);
        });
        if (!options?.silent) {
          toast.error(client.getErrorMessage(error));
        }
        return false;
      } finally {
        startTransition(() => {
          setAccountPageLoading(false);
        });
      }
    },
    [accountFilter, accountGroupFilter, accountPage, accountPageSize, currentPage, deferredAccountSearch, ensureConnected],
  );

  useEffect(() => {
    if (currentPage !== "accounts") {
      return;
    }
    void refreshAccountPage({ silent: true });
  }, [accountFilter, accountGroupFilter, accountPage, accountPageSize, currentPage, deferredAccountSearch, refreshAccountPage]);

  async function loadSnapshot(nextAddress: string) {
    const snapshot = normalizeStartupSnapshot(await client.serviceStartupSnapshot(nextAddress));
    const nextUsageSnapshots = snapshot.usageSnapshots || [];
    const nextAccounts = snapshot.accounts || [];
    startTransition(() => {
      setAccounts(nextAccounts);
      setUsageSnapshots(nextUsageSnapshots);
      setApiKeys(snapshot.apiKeys || []);
      setApiModels(normalizeApiModels(snapshot.apiModelOptions || []));
      setRequestLogs(snapshot.requestLogs || []);
      setRequestLogTodaySummary(
        snapshot.requestLogTodaySummary
          ? normalizeRequestLogTodaySummary(snapshot.requestLogTodaySummary)
          : EMPTY_REQUEST_LOG_TODAY_SUMMARY,
      );
      setUsageAggregateSummary(
        snapshot.usageAggregateSummary
          ? (snapshot.usageAggregateSummary as UsageAggregateSummary)
          : computeAggregateRemainingStats(nextAccounts, nextUsageSnapshots),
      );
      setManualPreferredAccountIdState(snapshot.manualPreferredAccountId || "");
    });
  }

  const bootstrap = useEffectEvent(async () => {
    setLoading(true);
    try {
      const nextSettings = await client.appSettingsGet();
      startTransition(() => {
        setSettings(nextSettings);
        setSettingsSyncRevision((current) => current + 1);
        setSettingsDraftSectionState({});
        setThemeState(nextSettings.theme);
        setServiceAddressState(nextSettings.serviceAddr);
      });

      const connected = await initializeConnection(nextSettings.serviceAddr, { silent: true });
      if (connected) {
        await loadSnapshot(nextSettings.serviceAddr);
      }
    } catch (error) {
      setServiceHint(formatConnectionError(error));
    } finally {
      setLoading(false);
    }
  });

  useEffect(() => {
    if (initializedRef.current) {
      return;
    }
    initializedRef.current = true;
    void bootstrap();
  }, []);

  async function refreshAll(options: RefreshOptions = {}) {
    setRefreshing(true);
    try {
      const nextSettings = await client.appSettingsGet();
      startTransition(() => {
        setSettings(nextSettings);
        setSettingsSyncRevision((current) => current + 1);
        setSettingsDraftSectionState({});
        setThemeState(nextSettings.theme);
        setServiceAddressState(nextSettings.serviceAddr);
      });

      const nextAddress = normalizeServiceAddress(nextSettings.serviceAddr);
      const connected = await initializeConnection(nextAddress, { silent: true });
      if (!connected) {
        if (!options.silent) {
          toast.error("连接服务失败，无法刷新数据");
        }
        return false;
      }
      await loadSnapshot(nextAddress);
      if (currentPage === "accounts") {
        await refreshAccountPage({ silent: true, address: nextAddress });
      }
      if (!options.silent) {
        toast.success("数据已同步");
      }
      return true;
    } catch (error) {
      if (!options.silent) {
        toast.error(client.getErrorMessage(error));
      }
      return false;
    } finally {
      setRefreshing(false);
    }
  }

  async function refreshRequestLogs(options: RefreshOptions = {}) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        if (!options.silent) {
          toast.error("连接服务失败，无法读取请求日志");
        }
        return false;
      }

      const [listResult, summaryResult] = await Promise.all([
        client.serviceRequestLogList(nextAddress, undefined, 300),
        client.serviceRequestLogTodaySummary(nextAddress).catch(() => EMPTY_REQUEST_LOG_TODAY_SUMMARY),
      ]);

      startTransition(() => {
        setRequestLogs(
          Array.isArray(listResult.items)
            ? listResult.items.map((item, index) => ({
                ...item,
                __identity: item.__identity || buildRequestLogIdentity(item, index),
              }))
            : [],
        );
        setRequestLogTodaySummary(normalizeRequestLogTodaySummary(summaryResult));
      });

      if (!options.silent) {
        toast.success("请求日志已刷新");
      }
      return true;
    } catch (error) {
      if (!options.silent) {
        toast.error(client.getErrorMessage(error));
      }
      return false;
    }
  }

  async function setTheme(nextTheme: ThemeId) {
    const previousTheme = theme;
    setThemeState(nextTheme);
    const saved = await saveSettingsPatch({ theme: nextTheme }, null);
    if (!saved) {
      setThemeState(previousTheme);
    }
  }

  async function saveSettingsPatch(
    patch: Partial<AppSettingsSnapshot> & Record<string, unknown>,
    successMessage: string | null = "设置已保存",
  ) {
    try {
      const payload = { ...patch } as Record<string, unknown>;
      if (Object.prototype.hasOwnProperty.call(payload, "serviceAddr")) {
        payload.serviceAddr = normalizeServiceAddress(String(payload.serviceAddr || serviceAddress));
      }

      const nextSettings = await client.appSettingsSet(payload as Partial<AppSettingsSnapshot> & Record<string, unknown>);
      startTransition(() => {
        setSettings(nextSettings);
        setThemeState(nextSettings.theme);
        setServiceAddressState(nextSettings.serviceAddr);
      });
      if (successMessage) {
        toast.success(successMessage);
      }
      return nextSettings;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return null;
    }
  }

  async function toggleServiceConnection() {
    if (!desktopMode) {
      toast.error("浏览器模式不支持启动或停止本地服务");
      return;
    }

    setServiceBusy(true);
    try {
      if (serviceConnected) {
        await client.serviceStop();
        startTransition(() => {
          setServiceConnected(false);
          setServiceHint("服务已停止");
        });
        toast.success("服务已停止");
        return;
      }

      const nextAddress = normalizeServiceAddress(serviceAddress);
      setServiceAddressState(nextAddress);
      await client.serviceStart(nextAddress);
      const connected = await initializeConnection(nextAddress, { silent: true });
      if (connected) {
        await loadSnapshot(nextAddress);
        toast.success("服务已连接");
      }
    } catch (error) {
      const message = client.getErrorMessage(error);
      setServiceHint(message);
      toast.error(message);
    } finally {
      setServiceBusy(false);
    }
  }

  async function refreshApiKeyList(silent = true) {
    const nextAddress = await ensureConnected({ silent: true });
    if (!nextAddress) {
      if (!silent) {
        toast.error("连接服务失败，无法刷新平台密钥");
      }
      return false;
    }

    const list = await client.serviceApiKeyList(nextAddress);
    startTransition(() => {
      setApiKeys(Array.isArray(list.items) ? list.items : []);
    });
    return true;
  }

  async function refreshApiModels(refreshRemote = false) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法读取模型列表");
        return false;
      }

      const result = await client.serviceApiKeyModels(nextAddress, refreshRemote);
      startTransition(() => {
        setApiModels(normalizeApiModels(result.items));
      });
      if (refreshRemote) {
        toast.success("模型列表已刷新");
      }
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function createApiKey(input: ApiKeyCreateInput) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法创建平台密钥");
        return null;
      }

      const result = await client.serviceApiKeyCreate(
        nextAddress,
        input.name,
        input.modelSlug,
        input.reasoningEffort,
        {
          protocolType: input.protocolType,
          upstreamBaseUrl: input.upstreamBaseUrl,
          staticHeadersJson: input.staticHeadersJson,
        },
      );
      await refreshApiKeyList();
      toast.success("平台密钥已创建");
      return result;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return null;
    }
  }

  async function updateApiKeyModel(item: ApiKeyRecord, modelSlug: string | null, reasoningEffort: string | null) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法更新平台密钥");
        return false;
      }

      await client.serviceApiKeyUpdateModel(nextAddress, item.id, modelSlug, reasoningEffort, {
        protocolType: item.protocolType || "openai_compat",
        upstreamBaseUrl: item.upstreamBaseUrl || null,
        staticHeadersJson: item.staticHeadersJson || null,
      });
      await refreshApiKeyList();
      toast.success("模型配置已更新");
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function deleteApiKey(item: ApiKeyRecord) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法删除平台密钥");
        return false;
      }

      await client.serviceApiKeyDelete(nextAddress, item.id);
      await refreshApiKeyList();
      toast.success("平台密钥已删除");
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function toggleApiKeyStatus(item: ApiKeyRecord) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法更新平台密钥状态");
        return false;
      }

      const disabled = String(item.status || "").toLowerCase() === "disabled";
      if (disabled) {
        await client.serviceApiKeyEnable(nextAddress, item.id);
      } else {
        await client.serviceApiKeyDisable(nextAddress, item.id);
      }
      await refreshApiKeyList();
      toast.success(disabled ? "平台密钥已启用" : "平台密钥已禁用");
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function readApiKeySecret(item: ApiKeyRecord) {
    const nextAddress = await ensureConnected({ silent: true });
    if (!nextAddress) {
      throw new Error("连接服务失败，无法读取完整密钥");
    }

    const result = await client.serviceApiKeyReadSecret(nextAddress, item.id);
    const key = String(result.key || "").trim();
    if (!key) {
      throw new Error("该密钥创建于旧版本，无法找回明文，请删除后重新创建");
    }
    return key;
  }

  async function clearRequestLogs() {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法清空请求日志");
        return false;
      }

      await client.serviceRequestLogClear(nextAddress);
      startTransition(() => {
        setRequestLogs([]);
        setRequestLogTodaySummary(EMPTY_REQUEST_LOG_TODAY_SUMMARY);
      });
      toast.success("请求日志已清空");
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function deleteAccountsSequentially(address: string, accountIds: string[]) {
    let deleted = 0;
    let failed = 0;
    for (const accountId of accountIds) {
      try {
        await client.serviceAccountDelete(address, accountId);
        deleted += 1;
      } catch {
        failed += 1;
      }
    }
    return { deleted, failed };
  }

  async function deleteAccounts(accountIds: string[]) {
    const ids = Array.from(new Set(accountIds.map((item) => String(item || "").trim()).filter(Boolean)));
    if (!ids.length) {
      return { deleted: 0, failed: 0 };
    }

    const nextAddress = await ensureConnected({ silent: true });
    if (!nextAddress) {
      toast.error("连接服务失败，无法删除账号");
      return { deleted: 0, failed: ids.length };
    }

    try {
      const result = await client.serviceAccountDeleteMany(nextAddress, ids);
      const deleted = Number((result as Record<string, unknown>).deleted ?? ids.length);
      const failed = Number((result as Record<string, unknown>).failed ?? 0);
      await refreshAll({ silent: true });
      if (failed > 0) {
        toast.warning(`已删除 ${deleted} 个账号，失败 ${failed} 个`);
      } else {
        toast.success(`已删除 ${deleted} 个账号`);
      }
      return { deleted, failed };
    } catch (error) {
      if (!isMissingCommandError(error)) {
        toast.error(client.getErrorMessage(error));
        return { deleted: 0, failed: ids.length };
      }

      const fallbackResult = await deleteAccountsSequentially(nextAddress, ids);
      await refreshAll({ silent: true });
      if (fallbackResult.failed > 0) {
        toast.warning(`已删除 ${fallbackResult.deleted} 个账号，失败 ${fallbackResult.failed} 个`);
      } else {
        toast.success(`已删除 ${fallbackResult.deleted} 个账号`);
      }
      return fallbackResult;
    }
  }

  async function deleteAccount(accountId: string) {
    const result = await deleteAccounts([accountId]);
    return result.failed === 0;
  }

  async function deleteUnavailableFreeAccounts() {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法清理免费账号");
        return false;
      }

      const result = await client.serviceAccountDeleteUnavailableFree(nextAddress);
      await refreshAll({ silent: true });
      const deleted = Number((result as Record<string, unknown>).deleted || 0);
      const scanned = Number((result as Record<string, unknown>).scanned || 0);
      toast.success(`已处理 ${scanned} 个账号，移除 ${deleted} 个不可用免费账号`);
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function importAccountsFromFiles(files: FileList | File[]) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法导入账号");
        return false;
      }

      const normalizedFiles = Array.from(files || []);
      if (!normalizedFiles.length) {
        toast.error("未选择任何文件");
        return false;
      }

      const contents = await readFilesSequentially(normalizedFiles);
      if (!contents.length) {
        toast.error("未读取到可导入内容");
        return false;
      }

      const result = (await client.serviceAccountImport(nextAddress, contents)) as ActionResult & {
        total?: number;
        created?: number;
        updated?: number;
        failed?: number;
      };
      await refreshAll({ silent: true });
      const total = Number(result.total || contents.length);
      const created = Number(result.created || 0);
      const updated = Number(result.updated || 0);
      const failed = Number(result.failed || 0);
      toast.success(`导入完成：共 ${total} 条，新增 ${created}，更新 ${updated}，失败 ${failed}`);
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function importAccountsFromDirectory() {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法导入账号");
        return false;
      }

      const result = await client.serviceAccountImportByDirectory(nextAddress);
      if (result.canceled) {
        toast("已取消导入");
        return false;
      }

      const contents = Array.isArray(result.contents) ? result.contents : [];
      if (!contents.length) {
        toast.error("所选文件夹下未找到可导入的 JSON 文件");
        return false;
      }

      const importResult = (await client.serviceAccountImport(nextAddress, contents)) as ActionResult & {
        total?: number;
        created?: number;
        updated?: number;
        failed?: number;
      };
      await refreshAll({ silent: true });
      const total = Number(importResult.total || contents.length);
      const created = Number(importResult.created || 0);
      const updated = Number(importResult.updated || 0);
      const failed = Number(importResult.failed || 0);
      const fileCount = Number(result.fileCount || contents.length);
      toast.success(`目录导入完成：${fileCount} 个文件，共 ${total} 条，新增 ${created}，更新 ${updated}，失败 ${failed}`);
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function exportAccountsByFile() {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法导出账号");
        return false;
      }

      if (!desktopMode) {
        const probeResponse = await fetch("/api/account-export?probe=1", {
          method: "GET",
          cache: "no-store",
        });
        const probeText = await probeResponse.text();
        let probePayload: Record<string, unknown> | null = null;
        try {
          probePayload = probeText ? (JSON.parse(probeText) as Record<string, unknown>) : null;
        } catch {}

        if (!probeResponse.ok || probePayload?.ok === false) {
          const message =
            typeof probePayload?.error === "string" && probePayload.error.trim()
              ? probePayload.error
              : probeText.trim() || `账号导出失败（HTTP ${probeResponse.status}）`;
          throw new Error(message);
        }

        const fileName = String(probePayload?.fileName || "").trim() || "codexmanager-accounts-export.json";
        const exported = Number(probePayload?.exported || 0);
        const skippedMissingToken = Number(probePayload?.skippedMissingToken || 0);
        window.location.assign("/api/account-export");
        toast.success(
          `已开始下载：${fileName}${exported >= 0 ? `，共 ${exported} 个账号` : ""}${skippedMissingToken > 0 ? `，跳过 ${skippedMissingToken} 个` : ""}`,
        );
        return true;
      }

      const result = await client.serviceAccountExportByAccountFiles(nextAddress);
      if (result.canceled) {
        toast("已取消导出");
        return false;
      }

      const exported = Number(result.exported || 0);
      const skippedMissingToken = Number(result.skippedMissingToken || 0);
      const outputDir = String(result.outputDir || "").trim();
      const outputHint = outputDir ? `，目录：${outputDir}` : "";
      toast.success(`导出完成：${exported} 个账号${skippedMissingToken > 0 ? `，跳过 ${skippedMissingToken} 个` : ""}${outputHint}`);
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function updateAccountSort(accountId: string, sort: number, previousSort?: number | null) {
    try {
      const normalizedId = String(accountId || "").trim();
      if (!normalizedId) {
        return false;
      }

      const nextSort = Number.isFinite(sort) ? Number(sort) : 0;
      const previous = Number.isFinite(previousSort as number) ? Number(previousSort) : null;
      if (previous != null && previous === nextSort) {
        return true;
      }

      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法更新账号排序");
        return false;
      }

      await client.serviceAccountUpdate(nextAddress, normalizedId, nextSort);
      await refreshAll({ silent: true });
      toast.success("账号排序已更新");
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function setManualPreferredAccountId(accountId: string) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法更新优先账号");
        return false;
      }

      const normalizedId = String(accountId || "").trim();
      if (!normalizedId) {
        await client.serviceGatewayManualAccountClear(nextAddress);
        setManualPreferredAccountIdState("");
        toast.success("已恢复自动调度");
        return true;
      }

      const account = accounts.find((item) => item.id === normalizedId);
      const availability = calculateAvailability(usageByAccountId.get(normalizedId), account);
      if (availability.level === "warn" || availability.level === "bad") {
        toast.error(`账号当前不可用（${availability.text}），无法锁定`);
        return false;
      }

      await client.serviceGatewayManualAccountSet(nextAddress, normalizedId);
      setManualPreferredAccountIdState(normalizedId);
      toast.success("已锁定优先账号");
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  async function refreshAccountUsage(accountId: string) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法刷新账号用量");
        return null;
      }

      await client.serviceUsageRefresh(nextAddress, accountId);
      const result = await client.serviceUsageRead(nextAddress, accountId);
      if (!result) {
        toast.error("未读取到账号用量快照");
        return null;
      }
      const snapshot = {
        ...result,
        accountId,
      } as UsageSnapshot;

      const nextSnapshots = [...usageSnapshots.filter((item) => item.accountId !== accountId), snapshot];
      startTransition(() => {
        setUsageSnapshots(nextSnapshots);
        setUsageAggregateSummary(computeAggregateRemainingStats(accounts, nextSnapshots));
      });
      toast.success("账号用量已刷新");
      return snapshot;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return null;
    }
  }

  async function refreshAllAccountUsage() {
    if (accountUsageRefreshInFlightRef.current) {
      return accountUsageRefreshInFlightRef.current;
    }

    const run = (async () => {
      clearAccountUsageRefreshTimer();
      setAccountUsageRefreshProgress({
        active: true,
        total: 1,
        completed: 0,
        remaining: 1,
        lastAccountLabel: "",
      });

      try {
        const nextAddress = await ensureConnected({ silent: true });
        if (!nextAddress) {
          setAccountUsageRefreshProgress(EMPTY_ACCOUNT_USAGE_REFRESH_PROGRESS);
          toast.error("连接服务失败，无法刷新账号用量");
          return false;
        }

        let nextAccounts = accounts.filter((item) => String(item?.id || "").trim());
        if (nextAccounts.length === 0) {
          const listResult = await client.serviceAccountList(nextAddress, { page: 1, pageSize: 5000 });
          nextAccounts = Array.isArray(listResult.items)
            ? listResult.items.filter((item) => String(item?.id || "").trim())
            : [];
        }

        if (nextAccounts.length === 0) {
          setAccountUsageRefreshProgress({
            active: true,
            total: 1,
            completed: 1,
            remaining: 0,
            lastAccountLabel: "无可刷新账号",
          });
          return true;
        }

        setAccountUsageRefreshProgress({
          active: true,
          total: nextAccounts.length,
          completed: 0,
          remaining: nextAccounts.length,
          lastAccountLabel: "",
        });

        const snapshotMap = new Map(usageSnapshots.map((item) => [item.accountId, item] as const));
        let completed = 0;
        let failed = 0;

        for (const account of nextAccounts) {
          const accountId = String(account.id || "").trim();
          const accountLabel = String(account.label || accountId || "").trim() || "未知账号";

          try {
            await client.serviceUsageRefresh(nextAddress, accountId);
            const snapshot = await client.serviceUsageRead(nextAddress, accountId);
            if (snapshot) {
              snapshotMap.set(accountId, { ...snapshot, accountId });
            } else {
              failed += 1;
            }
          } catch {
            failed += 1;
          } finally {
            completed += 1;
            setAccountUsageRefreshProgress({
              active: true,
              total: nextAccounts.length,
              completed,
              remaining: Math.max(0, nextAccounts.length - completed),
              lastAccountLabel: accountLabel,
            });
            await sleep(0);
          }
        }

        const nextSnapshots = Array.from(snapshotMap.values());
        startTransition(() => {
          setAccounts(nextAccounts);
          setUsageSnapshots(nextSnapshots);
          setUsageAggregateSummary(computeAggregateRemainingStats(nextAccounts, nextSnapshots));
        });

        if (failed > 0) {
          toast.error(`用量刷新完成，失败 ${failed}/${nextAccounts.length}`);
        } else {
          toast.success("账号用量已刷新");
        }
        return failed === 0;
      } catch (error) {
        toast.error(client.getErrorMessage(error));
        return false;
      } finally {
        scheduleAccountUsageRefreshProgressClear();
        accountUsageRefreshInFlightRef.current = null;
      }
    })();

    accountUsageRefreshInFlightRef.current = run;
    return run;
  }

  async function startAccountLogin(input: AccountLoginInput) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法启动授权流程");
        return null;
      }

      const result = await client.serviceLoginStart(nextAddress, {
        loginType: input.loginType || "chatgpt",
        openBrowser: false,
        groupName: input.groupName.trim() || null,
        tags: input.tags.trim() || null,
        note: input.note.trim() || null,
      });

      if (result.authUrl) {
        await client.openInBrowser(result.authUrl);
      }

      return result;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return null;
    }
  }

  async function getAccountLoginStatus(
    loginId: string,
    options?: { signal?: AbortSignal; timeoutMs?: number },
  ) {
    const nextAddress = await ensureConnected({ silent: true });
    if (!nextAddress) {
      return { status: "failed", error: "连接服务失败" };
    }
    return client.serviceLoginStatus(nextAddress, loginId, options);
  }

  async function completeAccountLogin(payload: { state: string; code: string; redirectUri?: string }) {
    try {
      const nextAddress = await ensureConnected({ silent: true });
      if (!nextAddress) {
        toast.error("连接服务失败，无法完成授权");
        return false;
      }

      await client.serviceLoginComplete(nextAddress, payload.state, payload.code, payload.redirectUri);
      await refreshAll({ silent: true });
      toast.success("账号授权完成");
      return true;
    } catch (error) {
      toast.error(client.getErrorMessage(error));
      return false;
    }
  }

  const page = pageMeta[currentPage];

  const contextValue: CodexAppContextValue = {
    currentPage,
    setCurrentPage,
    pageTitle: page.title,
    pageDescription: page.description,
    pageKicker: page.kicker,
    runtimeModeResolved,
    desktopMode,
    canManageService: desktopMode,
    theme,
    setTheme,
    settings,
    settingsSyncRevision,
    settingsDraftSections,
    setSettingsDraftState,
    serviceAddress,
    setServiceAddress: (value) => setServiceAddressState(value),
    serviceConnected,
    serviceBusy,
    serviceHint,
    loading,
    refreshing,
    accounts,
    visibleAccounts,
    accountUsageById: usageByAccountId,
    accountLabelsById,
    accountGroups,
    accountSearch,
    setAccountSearch: (value) => {
      startTransition(() => {
        setAccountSearchState(value);
        setAccountPageState(1);
      });
    },
    accountFilter,
    setAccountFilter: (value) => {
      startTransition(() => {
        setAccountFilterState(value);
        setAccountPageState(1);
      });
    },
    accountGroupFilter,
    setAccountGroupFilter: (value) => {
      startTransition(() => {
        setAccountGroupFilterState(value);
        setAccountPageState(1);
      });
    },
    accountPage,
    setAccountPage: (value) => setAccountPageState(Math.max(1, value)),
    accountPageSize,
    setAccountPageSize: (value) => {
      startTransition(() => {
        setAccountPageSizeState(Math.max(1, value));
        setAccountPageState(1);
      });
    },
    accountPageLoading,
    accountTotal,
    apiKeys,
    apiModels,
    requestLogs,
    visibleRequestLogs,
    requestLogQuery,
    setRequestLogQuery: (value) => setRequestLogQueryState(value),
    requestLogStatusFilter,
    setRequestLogStatusFilter: (value) => setRequestLogStatusFilterState(value),
    requestLogTodaySummary,
    usageAggregateSummary,
    usageSnapshots,
    manualPreferredAccountId,
    accountUsageRefreshProgress,
    refreshAll,
    refreshRequestLogs,
    refreshAccountUsage,
    refreshAllAccountUsage,
    toggleServiceConnection,
    saveSettingsPatch,
    createApiKey,
    refreshApiModels,
    updateApiKeyModel,
    deleteApiKey,
    toggleApiKeyStatus,
    readApiKeySecret,
    clearRequestLogs,
    deleteAccount,
    deleteAccounts,
    deleteUnavailableFreeAccounts,
    importAccountsFromDirectory,
    exportAccountsByFile,
    updateAccountSort,
    importAccountsFromFiles,
    setManualPreferredAccountId,
    startAccountLogin,
    getAccountLoginStatus,
    completeAccountLogin,
  };

  return (
    <CodexAppContext.Provider value={contextValue}>
      <div className={settings.lowTransparency ? "cm-app cm-low-transparency" : "cm-app"}>{children}</div>
    </CodexAppContext.Provider>
  );
}

export function useCodexApp() {
  const context = useContext(CodexAppContext);
  if (!context) {
    throw new Error("useCodexApp must be used within CodexAppProvider");
  }
  return context;
}

export async function pollLoginUntilSettled(
  getStatus: (options?: { signal?: AbortSignal; timeoutMs?: number }) => Promise<{ status?: string; error?: string }>,
  signal?: AbortSignal,
) {
  const deadline = Date.now() + 2 * 60 * 1000;
  while (Date.now() < deadline) {
    const result = await getStatus({ signal, timeoutMs: 6000 });
    if (result.status === "success" || result.status === "failed") {
      return result;
    }
    await sleep(1500, signal);
  }
  return { status: "failed", error: "登录超时，请重试" };
}
