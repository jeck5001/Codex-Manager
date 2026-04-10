"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Ban,
  Clipboard,
  ExternalLink,
  FileUp,
  Hash,
  Info,
  LogIn,
  Mail,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { accountClient } from "@/lib/api/account-client";
import {
  buildManualImportSummary,
  getRegisterSubmitLabel,
} from "./register-auto-import";
import {
  canShowTempMailDomainConfigPicker,
  canShowTempMailAutoCreateToggle,
  deriveTempMailAutoCreateSubmitFlag,
  getDefaultTempMailAutoCreateState,
  shouldBypassTempMailServicePickerOnSubmit,
  shouldDisableTempMailServicePicker,
} from "./register-temp-mail-auto-create";
import {
  canUseOutlookBatchRegisterMode,
  getDefaultRegisterChannel,
  getRegisterChannelLabel,
  sanitizeRegisterModeForChannel,
  type RegisterChannel,
} from "./register-mode-options";
import type {
  RegisterAvailableServicesResult,
  RegisterBatchSnapshot,
  RegisterBrowserbaseConfig,
  RegisterOutlookAccount,
  RegisterOutlookAccountsResult,
  RegisterOutlookBatchSnapshot,
  RegisterServiceGroup,
  RegisterTaskSnapshot,
} from "@/types";

interface AddAccountModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type RegisterMode = "single" | "batch" | "outlook-batch";
type RegisterExecutionMode = "pipeline" | "parallel";
const REGISTER_SERVICE_AUTO = "__auto__";
const REGISTER_TEMP_MAIL_DOMAIN_CONFIG_AUTO = "__random__";

const GROUP_LABELS: Record<string, string> = {
  TEAM: "团队 (TEAM)",
  PERSONAL: "个人 (PERSONAL)",
};

function pickImportTokenField(record: unknown, keys: string[]): string {
  const source = record && typeof record === "object" && !Array.isArray(record)
    ? (record as Record<string, unknown>)
    : null;
  if (!source) return "";
  for (const key of keys) {
    const value = source[key];
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return "";
}

function normalizeSingleImportRecord(record: unknown): unknown {
  if (!record || typeof record !== "object" || Array.isArray(record)) {
    return record;
  }
  const source = record as Record<string, unknown>;
  const tokens = source.tokens;
  if (tokens && typeof tokens === "object" && !Array.isArray(tokens)) {
    return record;
  }

  const accessToken = pickImportTokenField(record, ["access_token", "accessToken"]);
  const idToken = pickImportTokenField(record, ["id_token", "idToken"]);
  const refreshToken = pickImportTokenField(record, ["refresh_token", "refreshToken"]);
  if (!accessToken || !idToken || !refreshToken) {
    return record;
  }

  const accountIdHint = pickImportTokenField(record, [
    "account_id",
    "accountId",
    "chatgpt_account_id",
    "chatgptAccountId",
  ]);
  const normalizedTokens: Record<string, string> = {
    access_token: accessToken,
    id_token: idToken,
    refresh_token: refreshToken,
  };
  if (accountIdHint) {
    normalizedTokens.account_id = accountIdHint;
  }

  return {
    ...source,
    tokens: normalizedTokens,
  };
}

function normalizeImportContentForCompatibility(rawContent: string): string {
  const text = String(rawContent || "").trim();
  if (!text) return text;
  try {
    const parsed = JSON.parse(text);
    if (Array.isArray(parsed)) {
      return JSON.stringify(parsed.map(normalizeSingleImportRecord));
    }
    if (parsed && typeof parsed === "object") {
      return JSON.stringify(normalizeSingleImportRecord(parsed));
    }
    return text;
  } catch {
    return text;
  }
}

function buildBulkImportContents(rawContent: string): string[] {
  const text = String(rawContent || "").trim();
  if (!text) return [];

  if (text.startsWith("{") || text.startsWith("[")) {
    return [normalizeImportContentForCompatibility(text)];
  }

  return text
    .split("\n")
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => normalizeImportContentForCompatibility(item));
}

function getBulkImportErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("invalid JSON object stream")) {
    return "导入内容格式不正确。JSON 账号内容请整段粘贴；普通 Token 才按每行一个导入。";
  }
  if (message.includes("invalid JSON array")) {
    return "JSON 数组格式不正确，请检查括号和逗号后重试。";
  }
  return message;
}

function parseIntegerInput(rawValue: string, label: string, min: number): number {
  const parsed = Number(rawValue);
  if (!Number.isFinite(parsed) || !Number.isInteger(parsed) || parsed < min) {
    throw new Error(`${label}必须是大于等于 ${min} 的整数`);
  }
  return parsed;
}

function getRegisterStatusLabel(status: string) {
  const normalized = status.trim().toLowerCase();
  if (normalized === "completed") return "已完成";
  if (normalized === "running") return "运行中";
  if (normalized === "failed") return "失败";
  if (normalized === "cancelled") return "已取消";
  if (normalized === "pending") return "排队中";
  return status || "--";
}

function buildRegisterImportSummary(title: string, importedCount: number, failures: string[]) {
  const head = `${title}导入结果：成功 ${importedCount}，失败 ${failures.length}`;
  if (!failures.length) {
    return head;
  }
  return [head, "", ...failures.slice(0, 8)].join("\n");
}

function normalizeRegisterServiceIdForSubmit(rawValue: string): number | null {
  if (!rawValue || rawValue === REGISTER_SERVICE_AUTO) {
    return null;
  }
  const parsed = Number(rawValue);
  return Number.isFinite(parsed) ? parsed : null;
}

export function AddAccountModal({ open, onOpenChange }: AddAccountModalProps) {
  const [activeTab, setActiveTab] = useState("login");
  const [isLoading, setIsLoading] = useState(false);
  const [isPollingLogin, setIsPollingLogin] = useState(false);
  const [loginHint, setLoginHint] = useState("");
  const queryClient = useQueryClient();
  const loginPollTokenRef = useRef(0);
  const registerPollTokenRef = useRef(0);

  const [tags, setTags] = useState("");
  const [note, setNote] = useState("");
  const [group, setGroup] = useState("");
  const [loginUrl, setLoginUrl] = useState("");
  const [manualCallback, setManualCallback] = useState("");

  const [bulkContent, setBulkContent] = useState("");

  const [registerMode, setRegisterMode] = useState<RegisterMode>("single");
  const [registerChannel, setRegisterChannel] = useState<RegisterChannel>(getDefaultRegisterChannel);
  const [registerServices, setRegisterServices] =
    useState<RegisterAvailableServicesResult | null>(null);
  const [registerBrowserbaseConfigs, setRegisterBrowserbaseConfigs] =
    useState<RegisterBrowserbaseConfig[]>([]);
  const [registerOutlookAccounts, setRegisterOutlookAccounts] =
    useState<RegisterOutlookAccountsResult | null>(null);
  const [isRegisterLoading, setIsRegisterLoading] = useState(false);
  const [isRegisterBrowserbaseLoading, setIsRegisterBrowserbaseLoading] = useState(false);
  const [isRegisterOutlookLoading, setIsRegisterOutlookLoading] = useState(false);
  const [isRegisterSubmitting, setIsRegisterSubmitting] = useState(false);
  const [isRegisterImporting, setIsRegisterImporting] = useState(false);
  const [registerError, setRegisterError] = useState("");
  const [registerHint, setRegisterHint] = useState("");
  const [registerImportSummary, setRegisterImportSummary] = useState("");
  const [registerServiceType, setRegisterServiceType] = useState("tempmail");
  const [registerServiceId, setRegisterServiceId] = useState("");
  const [autoCreateTempMailService, setAutoCreateTempMailService] = useState(false);
  const [registerTempMailDomainConfigId, setRegisterTempMailDomainConfigId] = useState("");
  const [registerBrowserbaseConfigId, setRegisterBrowserbaseConfigId] = useState("");
  const [registerProxy, setRegisterProxy] = useState("");
  const [registerBatchCount, setRegisterBatchCount] = useState("3");
  const [registerIntervalMin, setRegisterIntervalMin] = useState("5");
  const [registerIntervalMax, setRegisterIntervalMax] = useState("30");
  const [registerConcurrency, setRegisterConcurrency] = useState("1");
  const [registerExecutionMode, setRegisterExecutionMode] =
    useState<RegisterExecutionMode>("pipeline");
  const [registerSkipRegistered, setRegisterSkipRegistered] = useState(true);
  const [registerAutoImport, setRegisterAutoImport] = useState(true);
  const [registerSelectedOutlookIds, setRegisterSelectedOutlookIds] = useState<number[]>([]);
  const [registerTask, setRegisterTask] = useState<RegisterTaskSnapshot | null>(null);
  const [registerBatch, setRegisterBatch] = useState<RegisterBatchSnapshot | null>(null);
  const [registerBatchTaskUuids, setRegisterBatchTaskUuids] = useState<string[]>([]);
  const [registerOutlookBatch, setRegisterOutlookBatch] =
    useState<RegisterOutlookBatchSnapshot | null>(null);

  const resetModalState = useCallback(() => {
    loginPollTokenRef.current += 1;
    registerPollTokenRef.current += 1;
    setActiveTab("login");
    setIsLoading(false);
    setIsPollingLogin(false);
    setLoginHint("");
    setTags("");
    setNote("");
    setGroup("");
    setLoginUrl("");
    setManualCallback("");
    setBulkContent("");
    setRegisterMode("single");
    setRegisterChannel(getDefaultRegisterChannel());
    setRegisterServices(null);
    setRegisterBrowserbaseConfigs([]);
    setRegisterOutlookAccounts(null);
    setIsRegisterLoading(false);
    setIsRegisterBrowserbaseLoading(false);
    setIsRegisterOutlookLoading(false);
    setIsRegisterSubmitting(false);
    setIsRegisterImporting(false);
    setRegisterError("");
    setRegisterHint("");
    setRegisterImportSummary("");
    setRegisterServiceType("tempmail");
    setRegisterServiceId("");
    setAutoCreateTempMailService(getDefaultTempMailAutoCreateState("tempmail"));
    setRegisterTempMailDomainConfigId("");
    setRegisterBrowserbaseConfigId("");
    setRegisterProxy("");
    setRegisterBatchCount("3");
    setRegisterIntervalMin("5");
    setRegisterIntervalMax("30");
    setRegisterConcurrency("1");
    setRegisterExecutionMode("pipeline");
    setRegisterSkipRegistered(true);
    setRegisterAutoImport(true);
    setRegisterSelectedOutlookIds([]);
    setRegisterTask(null);
    setRegisterBatch(null);
    setRegisterBatchTaskUuids([]);
    setRegisterOutlookBatch(null);
  }, []);

  const resetRegisterProgress = useCallback(() => {
    registerPollTokenRef.current += 1;
    setRegisterError("");
    setRegisterHint("");
    setRegisterImportSummary("");
    setRegisterTask(null);
    setRegisterBatch(null);
    setRegisterBatchTaskUuids([]);
    setRegisterOutlookBatch(null);
  }, []);

  const invalidateLoginQueries = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["accounts"] }),
      queryClient.invalidateQueries({ queryKey: ["usage"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  }, [queryClient]);

  const handleDialogOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) {
      resetModalState();
    }
    onOpenChange(nextOpen);
  };

  const completeLoginSuccess = useCallback(
    async (message: string) => {
      await invalidateLoginQueries();
      toast.success(message);
      resetModalState();
      onOpenChange(false);
    },
    [invalidateLoginQueries, onOpenChange, resetModalState],
  );

  const completeRegisterWithoutImport = useCallback(
    async (message: string) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["register-tasks"] }),
        queryClient.invalidateQueries({ queryKey: ["register-stats"] }),
        queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
      ]);
      toast.success(message);
      resetModalState();
      onOpenChange(false);
    },
    [onOpenChange, queryClient, resetModalState],
  );

  const waitForLogin = async (loginId: string) => {
    const pollToken = loginPollTokenRef.current + 1;
    loginPollTokenRef.current = pollToken;
    setIsPollingLogin(true);
    setLoginHint("已生成登录链接，正在等待授权完成...");

    const deadline = Date.now() + 2 * 60 * 1000;
    while (pollToken === loginPollTokenRef.current && Date.now() < deadline) {
      try {
        const result = await accountClient.getLoginStatus(loginId);
        if (pollToken !== loginPollTokenRef.current) {
          return;
        }

        const status = String(result.status || "").trim().toLowerCase();
        if (status === "success") {
          await completeLoginSuccess("登录成功");
          return;
        }
        if (status === "failed") {
          const message = result.error || "登录失败，请重试";
          setIsPollingLogin(false);
          setLoginHint(`登录失败：${message}`);
          toast.error(message);
          return;
        }
      } catch {
        if (pollToken !== loginPollTokenRef.current) {
          return;
        }
      }

      await new Promise<void>((resolve) => window.setTimeout(resolve, 1500));
    }

    if (pollToken === loginPollTokenRef.current) {
      setIsPollingLogin(false);
      setLoginHint("登录超时，请重试或使用下方手动解析回调。");
    }
  };

  const registerTypeOptions = useMemo(() => {
    if (!registerServices) return [];
    return [
      { value: "tempmail", label: "临时邮箱", group: registerServices.tempmail },
      { value: "outlook", label: "Outlook", group: registerServices.outlook },
      { value: "custom_domain", label: "自定义域名", group: registerServices.customDomain },
      { value: "mail_33_imap", label: "33mail + IMAP", group: registerServices.mail33Imap },
      { value: "temp_mail", label: "Temp Mail", group: registerServices.tempMail },
    ].filter((item) => item.group.available);
  }, [registerServices]);

  const selectedRegisterGroup = useMemo<RegisterServiceGroup | null>(() => {
    const matched = registerTypeOptions.find((item) => item.value === registerServiceType);
    return matched?.group || null;
  }, [registerServiceType, registerTypeOptions]);

  const selectedRegisterServiceHasChoices = (selectedRegisterGroup?.services || []).some(
    (item) => item.id != null,
  );

  const showTempMailAutoCreateToggle = useMemo(
    () => canShowTempMailAutoCreateToggle(registerServiceType),
    [registerServiceType],
  );
  const showTempMailDomainConfigPicker = useMemo(
    () => canShowTempMailDomainConfigPicker(registerServiceType, autoCreateTempMailService),
    [registerServiceType, autoCreateTempMailService],
  );
  const disableTempMailServicePicker = useMemo(
    () => shouldDisableTempMailServicePicker(registerServiceType, autoCreateTempMailService),
    [registerServiceType, autoCreateTempMailService],
  );
  const tempMailAutoCreateSubmitFlag = useMemo(
    () => deriveTempMailAutoCreateSubmitFlag(registerServiceType, autoCreateTempMailService),
    [registerServiceType, autoCreateTempMailService],
  );
  const bypassTempMailServicePickerOnSubmit = useMemo(
    () => shouldBypassTempMailServicePickerOnSubmit(registerServiceType, autoCreateTempMailService),
    [registerServiceType, autoCreateTempMailService],
  );
  const registerTempMailDomainConfigs = useMemo(
    () => selectedRegisterGroup?.domainConfigs || [],
    [selectedRegisterGroup],
  );

  useEffect(() => {
    if (!showTempMailAutoCreateToggle && autoCreateTempMailService) {
      setAutoCreateTempMailService(false);
    }
  }, [showTempMailAutoCreateToggle, autoCreateTempMailService]);

  useEffect(() => {
    if (!showTempMailDomainConfigPicker && registerTempMailDomainConfigId) {
      setRegisterTempMailDomainConfigId("");
      return;
    }
    if (
      registerTempMailDomainConfigId &&
      !registerTempMailDomainConfigs.some((item) => item.id === registerTempMailDomainConfigId)
    ) {
      setRegisterTempMailDomainConfigId("");
    }
  }, [
    registerTempMailDomainConfigId,
    registerTempMailDomainConfigs,
    showTempMailDomainConfigPicker,
  ]);

  useEffect(() => {
    if (tempMailAutoCreateSubmitFlag) {
      setRegisterServiceId(REGISTER_SERVICE_AUTO);
    }
  }, [tempMailAutoCreateSubmitFlag]);

  const selectedOutlookAccounts = useMemo(() => {
    if (!registerOutlookAccounts) return [];
    const selectedSet = new Set(registerSelectedOutlookIds);
    return registerOutlookAccounts.accounts.filter((account) => selectedSet.has(account.id));
  }, [registerOutlookAccounts, registerSelectedOutlookIds]);

  const selectedOutlookEmails = useMemo(() => {
    const deduped = new Set<string>();
    for (const account of selectedOutlookAccounts) {
      const email = account.email.trim();
      if (email) {
        deduped.add(email);
      }
    }
    return Array.from(deduped);
  }, [selectedOutlookAccounts]);

  const isBrowserbaseRegisterChannel = registerChannel === "browserbase_ddg";

  const enabledRegisterBrowserbaseConfigs = useMemo(
    () => registerBrowserbaseConfigs.filter((item) => item.enabled),
    [registerBrowserbaseConfigs],
  );

  const selectedRegisterBrowserbaseConfig = useMemo(
    () => enabledRegisterBrowserbaseConfigs.find((item) => String(item.id) === registerBrowserbaseConfigId) || null,
    [enabledRegisterBrowserbaseConfigs, registerBrowserbaseConfigId],
  );

  const registerBusy =
    isRegisterLoading ||
    isRegisterBrowserbaseLoading ||
    isRegisterOutlookLoading ||
    isRegisterSubmitting ||
    isRegisterImporting;

  const hasAvailableRegisterSource = isBrowserbaseRegisterChannel
    ? enabledRegisterBrowserbaseConfigs.length > 0
    : selectedRegisterGroup?.available === true;

  const syncRegisterSelection = useCallback(
    (catalog: RegisterAvailableServicesResult) => {
      const nextOptions = [
        { value: "tempmail", group: catalog.tempmail },
        { value: "outlook", group: catalog.outlook },
        { value: "custom_domain", group: catalog.customDomain },
        { value: "mail_33_imap", group: catalog.mail33Imap },
        { value: "temp_mail", group: catalog.tempMail },
      ].filter((item) => item.group.available);
      const nextType =
        nextOptions.find((item) => item.value === registerServiceType)?.value ||
        nextOptions[0]?.value ||
        "tempmail";
      const nextGroup = nextOptions.find((item) => item.value === nextType)?.group || null;
      const nextServiceId = (nextGroup?.services || []).some((item) => item.id != null)
        ? REGISTER_SERVICE_AUTO
        : "";
      setRegisterServiceType(nextType);
      setRegisterServiceId(nextServiceId);
      setAutoCreateTempMailService(getDefaultTempMailAutoCreateState(nextType));
      setRegisterTempMailDomainConfigId("");
    },
    [registerServiceType],
  );

  const loadRegisterServices = useCallback(async () => {
    setIsRegisterLoading(true);
    setRegisterError("");
    try {
      const result = await accountClient.getRegisterAvailableServices();
      setRegisterServices(result);
      syncRegisterSelection(result);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setRegisterError(message);
    } finally {
      setIsRegisterLoading(false);
    }
  }, [syncRegisterSelection]);

  const loadRegisterBrowserbaseConfigs = useCallback(async () => {
    setIsRegisterBrowserbaseLoading(true);
    setRegisterError("");
    try {
      const result = await accountClient.listRegisterBrowserbaseConfigs();
      const configs = result.configs || [];
      const enabledConfigs = configs.filter((item) => item.enabled);
      setRegisterBrowserbaseConfigs(configs);
      setRegisterBrowserbaseConfigId((current) => {
        if (enabledConfigs.some((item) => String(item.id) === current)) {
          return current;
        }
        return enabledConfigs[0] ? String(enabledConfigs[0].id) : "";
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setRegisterError(message);
    } finally {
      setIsRegisterBrowserbaseLoading(false);
    }
  }, []);

  const loadRegisterOutlookAccounts = useCallback(async () => {
    setIsRegisterOutlookLoading(true);
    setRegisterError("");
    try {
      const result = await accountClient.getRegisterOutlookAccounts();
      setRegisterOutlookAccounts(result);
      setRegisterSelectedOutlookIds((current) => {
        const validIds = current.filter((id) => result.accounts.some((account) => account.id === id));
        if (validIds.length > 0) {
          return validIds;
        }
        return result.accounts.map((account) => account.id);
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setRegisterError(message);
    } finally {
      setIsRegisterOutlookLoading(false);
    }
  }, []);

  useEffect(() => {
    setRegisterMode((current) => sanitizeRegisterModeForChannel(registerChannel, current));
  }, [registerChannel]);

  useEffect(() => {
    if (!open || activeTab !== "register" || registerServices || isRegisterLoading) {
      return;
    }
    void loadRegisterServices();
  }, [activeTab, isRegisterLoading, loadRegisterServices, open, registerServices]);

  useEffect(() => {
    if (!open || activeTab !== "register" || registerChannel !== "browserbase_ddg") {
      return;
    }
    void loadRegisterBrowserbaseConfigs();
  }, [activeTab, loadRegisterBrowserbaseConfigs, open, registerChannel]);

  useEffect(() => {
    if (
      !open ||
      activeTab !== "register" ||
      registerMode !== "outlook-batch" ||
      registerOutlookAccounts ||
      isRegisterOutlookLoading
    ) {
      return;
    }
    void loadRegisterOutlookAccounts();
  }, [
    activeTab,
    isRegisterOutlookLoading,
    loadRegisterOutlookAccounts,
    open,
    registerMode,
    registerOutlookAccounts,
  ]);

  const finalizeRegisterImport = useCallback(
    async (title: string, importedCount: number, failures: string[]) => {
      if (importedCount > 0) {
        await invalidateLoginQueries();
      }

      if (importedCount > 0 && failures.length === 0) {
        await completeLoginSuccess(`${title}完成，已导入 ${importedCount} 个账号`);
        return;
      }

      const summary = buildRegisterImportSummary(title, importedCount, failures);
      setRegisterHint("");
      setRegisterImportSummary(summary);
      setRegisterError(
        importedCount > 0
          ? `${title}已结束，但仍有 ${failures.length} 个账号导入失败`
          : `${title}未导入任何账号`
      );
      setIsRegisterSubmitting(false);
      if (importedCount > 0) {
        toast.warning(summary);
      } else {
        toast.error(summary);
      }
    },
    [completeLoginSuccess, invalidateLoginQueries],
  );

  const importRegisterAccountsByEmail = useCallback(
    async (emails: string[], title: string) => {
      const normalizedEmails = Array.from(
        new Set(emails.map((item) => item.trim()).filter(Boolean)),
      );
      if (!normalizedEmails.length) {
        setRegisterError(`${title}结束了，但没有拿到可导入的邮箱`);
        setRegisterHint("");
        setIsRegisterSubmitting(false);
        return;
      }

      setIsRegisterImporting(true);
      setRegisterHint(`${title}已结束，正在导入账号...`);
      try {
        let importedCount = 0;
        const failures: string[] = [];
        for (const email of normalizedEmails) {
          try {
            await accountClient.importRegisterAccountByEmail(email);
            importedCount += 1;
          } catch (err: unknown) {
            const message = err instanceof Error ? err.message : String(err);
            failures.push(`${email}: ${message}`);
          }
        }
        await finalizeRegisterImport(title, importedCount, failures);
      } finally {
        setIsRegisterImporting(false);
      }
    },
    [finalizeRegisterImport],
  );

  const completeRegisterEmailsWithoutImport = useCallback(
    async (emails: string[], title: string) => {
      const normalizedEmails = Array.from(
        new Set(emails.map((item) => item.trim()).filter(Boolean)),
      );
      if (!normalizedEmails.length) {
        setRegisterError(`${title}结束了，但没有拿到可手动入池的邮箱`);
        setRegisterHint("");
        setIsRegisterSubmitting(false);
        return;
      }

      await completeRegisterWithoutImport(
        buildManualImportSummary(title, normalizedEmails.length),
      );
    },
    [completeRegisterWithoutImport],
  );

  const importCompletedRegisterTask = useCallback(
    async (taskUuid: string) => {
      setIsRegisterImporting(true);
      setRegisterHint("注册成功，正在自动导入账号...");
      try {
        const imported = await accountClient.importRegisterTask(taskUuid);
        await completeLoginSuccess(`账号已注册并导入：${imported.email}`);
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        setRegisterError(`注册成功，但自动导入失败：${message}`);
        setRegisterHint("");
        setIsRegisterSubmitting(false);
        toast.error(`自动导入失败: ${message}`);
      } finally {
        setIsRegisterImporting(false);
      }
    },
    [completeLoginSuccess],
  );

  const importCompletedRegisterBatch = useCallback(
    async (taskUuids: string[], title: string) => {
      setIsRegisterImporting(true);
      setRegisterHint(`${title}已结束，正在导入成功账号...`);
      try {
        const snapshots = await Promise.all(
          taskUuids.map(async (taskUuid) => {
            try {
              return await accountClient.getRegisterTask(taskUuid);
            } catch (err: unknown) {
              const message = err instanceof Error ? err.message : String(err);
              setRegisterImportSummary((current) =>
                current
                  ? `${current}\n${taskUuid}: ${message}`
                  : `${taskUuid}: ${message}`,
              );
              return null;
            }
          }),
        );

        const importableTasks = snapshots.filter(
          (task): task is RegisterTaskSnapshot => Boolean(task?.canImport && task.taskUuid),
        );
        if (!importableTasks.length) {
          setRegisterHint("");
          setRegisterError(`${title}结束了，但没有成功注册出可导入账号`);
          setIsRegisterSubmitting(false);
          return;
        }

        let importedCount = 0;
        const failures: string[] = [];
        for (const task of importableTasks) {
          try {
            await accountClient.importRegisterTask(task.taskUuid);
            importedCount += 1;
          } catch (err: unknown) {
            const message = err instanceof Error ? err.message : String(err);
            failures.push(`${task.email || task.taskUuid}: ${message}`);
          }
        }
        await finalizeRegisterImport(title, importedCount, failures);
      } finally {
        setIsRegisterImporting(false);
      }
    },
    [finalizeRegisterImport],
  );

  const completeRegisterBatchWithoutImport = useCallback(
    async (taskUuids: string[], title: string) => {
      const snapshots = await Promise.all(
        taskUuids.map(async (taskUuid) => {
          try {
            return await accountClient.getRegisterTask(taskUuid);
          } catch (err: unknown) {
            const message = err instanceof Error ? err.message : String(err);
            setRegisterImportSummary((current) =>
              current
                ? `${current}\n${taskUuid}: ${message}`
                : `${taskUuid}: ${message}`,
            );
            return null;
          }
        }),
      );

      const importableTasks = snapshots.filter(
        (task): task is RegisterTaskSnapshot => Boolean(task?.canImport && task.taskUuid),
      );
      if (!importableTasks.length) {
        setRegisterHint("");
        setRegisterError(`${title}结束了，但没有成功注册出可手动入池的账号`);
        setIsRegisterSubmitting(false);
        return;
      }

      await completeRegisterWithoutImport(
        buildManualImportSummary(title, importableTasks.length),
      );
    },
    [completeRegisterWithoutImport],
  );

  const waitForRegisterTask = useCallback(
    async (taskUuid: string) => {
      const pollToken = registerPollTokenRef.current + 1;
      registerPollTokenRef.current = pollToken;
      setRegisterHint("注册任务已启动，正在等待完成...");

      const deadline = Date.now() + 10 * 60 * 1000;
      while (pollToken === registerPollTokenRef.current && Date.now() < deadline) {
        try {
          const snapshot = await accountClient.getRegisterTask(taskUuid);
          if (pollToken !== registerPollTokenRef.current) {
            return;
          }
          setRegisterTask(snapshot);

          const status = String(snapshot.status || "").trim().toLowerCase();
          if (status === "completed") {
            if (snapshot.canImport) {
              if (registerAutoImport) {
                await importCompletedRegisterTask(snapshot.taskUuid);
              } else {
                await completeRegisterWithoutImport(buildManualImportSummary("注册", 1));
              }
              return;
            }
            setRegisterError("注册任务已完成，但未拿到可导入的账号信息");
            setRegisterHint("");
            setIsRegisterSubmitting(false);
            return;
          }
          if (status === "failed" || status === "cancelled") {
            const message = snapshot.errorMessage || "注册失败";
            setRegisterError(message);
            setRegisterHint("");
            setIsRegisterSubmitting(false);
            toast.error(message);
            return;
          }
        } catch (err: unknown) {
          if (pollToken !== registerPollTokenRef.current) {
            return;
          }
          setRegisterError(err instanceof Error ? err.message : String(err));
        }

        await new Promise<void>((resolve) => window.setTimeout(resolve, 2000));
      }

      if (pollToken === registerPollTokenRef.current) {
        setRegisterHint("");
        setRegisterError("注册轮询超时，请稍后重试");
        setIsRegisterSubmitting(false);
      }
    },
    [completeRegisterWithoutImport, importCompletedRegisterTask, registerAutoImport],
  );

  const waitForRegisterBatch = useCallback(
    async (batchId: string, taskUuids: string[]) => {
      const pollToken = registerPollTokenRef.current + 1;
      registerPollTokenRef.current = pollToken;
      setRegisterHint("批量注册已启动，正在等待任务结束...");

      const deadline = Date.now() + 30 * 60 * 1000;
      while (pollToken === registerPollTokenRef.current && Date.now() < deadline) {
        try {
          const snapshot = await accountClient.getRegisterBatch(batchId);
          if (pollToken !== registerPollTokenRef.current) {
            return;
          }
          setRegisterBatch(snapshot);

          if (snapshot.finished || snapshot.cancelled) {
            const title = snapshot.cancelled ? "批量注册（已取消）" : "批量注册";
            if (registerAutoImport) {
              await importCompletedRegisterBatch(taskUuids, title);
            } else {
              await completeRegisterBatchWithoutImport(taskUuids, title);
            }
            return;
          }
        } catch (err: unknown) {
          if (pollToken !== registerPollTokenRef.current) {
            return;
          }
          setRegisterError(err instanceof Error ? err.message : String(err));
        }

        await new Promise<void>((resolve) => window.setTimeout(resolve, 2000));
      }

      if (pollToken === registerPollTokenRef.current) {
        setRegisterHint("");
        setRegisterError("批量注册轮询超时，请稍后重试");
        setIsRegisterSubmitting(false);
      }
    },
    [completeRegisterBatchWithoutImport, importCompletedRegisterBatch, registerAutoImport],
  );

  const waitForRegisterOutlookBatch = useCallback(
    async (batchId: string, emails: string[]) => {
      const pollToken = registerPollTokenRef.current + 1;
      registerPollTokenRef.current = pollToken;
      setRegisterHint("Outlook 批量注册已启动，正在等待任务结束...");

      const deadline = Date.now() + 30 * 60 * 1000;
      while (pollToken === registerPollTokenRef.current && Date.now() < deadline) {
        try {
          const snapshot = await accountClient.getRegisterOutlookBatch(batchId);
          if (pollToken !== registerPollTokenRef.current) {
            return;
          }
          setRegisterOutlookBatch(snapshot);

          if (snapshot.finished || snapshot.cancelled) {
            const title = snapshot.cancelled ? "Outlook 批量注册（已取消）" : "Outlook 批量注册";
            if (registerAutoImport) {
              await importRegisterAccountsByEmail(emails, title);
            } else {
              await completeRegisterEmailsWithoutImport(emails, title);
            }
            return;
          }
        } catch (err: unknown) {
          if (pollToken !== registerPollTokenRef.current) {
            return;
          }
          setRegisterError(err instanceof Error ? err.message : String(err));
        }

        await new Promise<void>((resolve) => window.setTimeout(resolve, 2000));
      }

      if (pollToken === registerPollTokenRef.current) {
        setRegisterHint("");
        setRegisterError("Outlook 批量注册轮询超时，请稍后重试");
        setIsRegisterSubmitting(false);
      }
    },
    [completeRegisterEmailsWithoutImport, importRegisterAccountsByEmail, registerAutoImport],
  );

  const handleStartLogin = async () => {
    setIsLoading(true);
    setLoginHint("");
    try {
      const result = await accountClient.startLogin({
        tags: tags.split(",").map((item) => item.trim()).filter(Boolean),
        note,
        group: group || null,
      });
      setLoginUrl(result.authUrl);
      if (result.warning) {
        toast.warning(result.warning);
      }
      toast.success("已生成登录链接，请在浏览器中完成授权");
      if (result.loginId) {
        void waitForLogin(result.loginId);
      } else {
        setLoginHint("未返回登录任务编号，请完成授权后使用手动解析。");
      }
    } catch (err: unknown) {
      toast.error(`启动登录失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleManualCallback = async () => {
    if (!manualCallback) {
      toast.error("请先粘贴回调链接");
      return;
    }
    setIsLoading(true);
    setLoginHint("正在解析回调...");
    try {
      const url = new URL(manualCallback);
      const state = url.searchParams.get("state") || "";
      const code = url.searchParams.get("code") || "";
      const redirectUri = `${url.origin}${url.pathname}`;

      await accountClient.completeLogin(state, code, redirectUri);
      await completeLoginSuccess("登录成功");
    } catch (err: unknown) {
      setLoginHint(`解析失败: ${err instanceof Error ? err.message : String(err)}`);
      toast.error(`解析失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleBulkImport = async () => {
    if (!bulkContent.trim()) return;
    setIsLoading(true);
    try {
      const contents = buildBulkImportContents(bulkContent);
      const result = await accountClient.import(contents);
      const total = Number(result?.total || 0);
      const created = Number(result?.created || 0);
      const updated = Number(result?.updated || 0);
      const failed = Number(result?.failed || 0);
      toast.success(`导入完成：共${total}，新增${created}，更新${updated}，失败${failed}`);
      await invalidateLoginQueries();
      resetModalState();
      onOpenChange(false);
    } catch (err: unknown) {
      toast.error(`导入失败: ${getBulkImportErrorMessage(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleStartRegister = async () => {
    if (registerMode !== "outlook-batch" && !hasAvailableRegisterSource) {
      toast.error(
        isBrowserbaseRegisterChannel
          ? "当前没有可用的 Browserbase-DDG 注册配置"
          : "当前没有可用的注册邮箱服务",
      );
      return;
    }

    resetRegisterProgress();
    setIsRegisterSubmitting(true);

    try {
      const resolvedRegisterMode = isBrowserbaseRegisterChannel
        ? "browserbase_ddg"
        : registerChannel === "any_auto"
          ? "any_auto"
          : "standard";
      const resolvedEmailServiceType = isBrowserbaseRegisterChannel ? "tempmail" : registerServiceType;
      const resolvedBrowserbaseConfigId = isBrowserbaseRegisterChannel
        ? normalizeRegisterServiceIdForSubmit(registerBrowserbaseConfigId)
        : null;
      const resolvedEmailServiceId =
        isBrowserbaseRegisterChannel || bypassTempMailServicePickerOnSubmit
          ? null
          : normalizeRegisterServiceIdForSubmit(registerServiceId);
      const resolvedTempMailDomainConfigId = registerTempMailDomainConfigId.trim();
      const resolvedEmailServiceConfig =
        tempMailAutoCreateSubmitFlag && registerServiceType === "temp_mail"
          ? resolvedTempMailDomainConfigId
            ? { domain_config_id: resolvedTempMailDomainConfigId }
            : {}
          : null;

      if (isBrowserbaseRegisterChannel && !resolvedBrowserbaseConfigId) {
        throw new Error("请选择一个可用的 Browserbase-DDG 配置");
      }
      if (tempMailAutoCreateSubmitFlag && registerServiceType === "temp_mail" && !registerTempMailDomainConfigs.length) {
        throw new Error("当前没有可用的 Temp-Mail 域名配置，请先到邮箱服务页配置后再试");
      }
      if (
        registerServiceType === "temp_mail" &&
        !tempMailAutoCreateSubmitFlag &&
        !selectedRegisterServiceHasChoices
      ) {
        throw new Error("当前没有可复用的 Temp-Mail 服务，请开启自动创建或先创建一条服务");
      }

      if (registerMode === "single") {
        const task = await accountClient.startRegisterTask({
          emailServiceType: resolvedEmailServiceType,
          emailServiceId: resolvedEmailServiceId,
          emailServiceConfig: resolvedEmailServiceConfig,
          registerMode: resolvedRegisterMode,
          browserbaseConfigId: resolvedBrowserbaseConfigId,
          proxy: registerProxy || null,
          autoCreateTempMailService: tempMailAutoCreateSubmitFlag,
        });
        setRegisterTask(task);
        toast.success(`${getRegisterChannelLabel(registerChannel)}任务已启动`);
        void waitForRegisterTask(task.taskUuid);
        return;
      }

      const intervalMin = parseIntegerInput(registerIntervalMin, "最小间隔", 0);
      const intervalMax = parseIntegerInput(registerIntervalMax, "最大间隔", 0);
      const concurrency = parseIntegerInput(registerConcurrency, "并发数", 1);
      if (intervalMax < intervalMin) {
        throw new Error("最大间隔不能小于最小间隔");
      }

      if (registerMode === "batch") {
        const count = parseIntegerInput(registerBatchCount, "注册数量", 1);
        const started = await accountClient.startRegisterBatch({
          emailServiceType: resolvedEmailServiceType,
          emailServiceId: resolvedEmailServiceId,
          emailServiceConfig: resolvedEmailServiceConfig,
          registerMode: resolvedRegisterMode,
          browserbaseConfigId: resolvedBrowserbaseConfigId,
          proxy: registerProxy || null,
          count,
          intervalMin,
          intervalMax,
          concurrency,
          mode: registerExecutionMode,
          autoCreateTempMailService: tempMailAutoCreateSubmitFlag,
        });
        setRegisterBatchTaskUuids(started.taskUuids);
        setRegisterBatch({
          batchId: started.batchId,
          total: started.count,
          completed: 0,
          success: 0,
          failed: 0,
          currentIndex: 0,
          cancelled: false,
          finished: false,
          progress: `0/${started.count}`,
          logs: [],
        });
        toast.success(`${getRegisterChannelLabel(registerChannel)}批量任务已启动`);
        void waitForRegisterBatch(started.batchId, started.taskUuids);
        return;
      }

      if (!registerSelectedOutlookIds.length) {
        throw new Error("请至少选择一个 Outlook 账号");
      }

      const selectedEmails = [...selectedOutlookEmails];
      const started = await accountClient.startRegisterOutlookBatch({
        serviceIds: registerSelectedOutlookIds,
        skipRegistered: registerSkipRegistered,
        proxy: registerProxy || null,
        intervalMin,
        intervalMax,
        concurrency,
        mode: registerExecutionMode,
      });

      setRegisterOutlookBatch({
        batchId: started.batchId,
        total: started.total,
        completed: 0,
        success: 0,
        failed: 0,
        skipped: started.skipped,
        currentIndex: 0,
        cancelled: false,
        finished: started.toRegister === 0,
        progress: started.toRegister === 0 ? `${started.total}/${started.total}` : `0/${started.toRegister}`,
        logs: started.toRegister === 0 ? ["[系统] 所选 Outlook 账号已全部注册，直接进入导入流程"] : [],
      });

      if (!started.batchId || started.toRegister === 0) {
        if (registerAutoImport) {
          toast.success("所选 Outlook 账号已全部注册，正在直接导入");
          await importRegisterAccountsByEmail(selectedEmails, "Outlook 批量注册");
        } else {
          await completeRegisterEmailsWithoutImport(selectedEmails, "Outlook 批量注册");
        }
        return;
      }

      toast.success("Outlook 批量注册已启动");
      void waitForRegisterOutlookBatch(started.batchId, selectedEmails);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setRegisterError(message);
      setIsRegisterSubmitting(false);
      toast.error(`启动注册失败: ${message}`);
    }
  };

  const handleCancelRegisterBatch = async () => {
    try {
      if (registerMode === "batch" && registerBatch?.batchId && !registerBatch.finished) {
        await accountClient.cancelRegisterBatch(registerBatch.batchId);
        setRegisterHint("已提交批量注册取消请求，正在等待任务收尾...");
        toast.success("已提交取消请求");
        return;
      }
      if (
        registerMode === "outlook-batch" &&
        registerOutlookBatch?.batchId &&
        !registerOutlookBatch.finished
      ) {
        await accountClient.cancelRegisterOutlookBatch(registerOutlookBatch.batchId);
        setRegisterHint("已提交 Outlook 批量注册取消请求，正在等待任务收尾...");
        toast.success("已提交取消请求");
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(`取消失败: ${message}`);
    }
  };

  const copyUrl = () => {
    if (!loginUrl) return;
    navigator.clipboard.writeText(loginUrl);
    toast.success("链接已复制");
  };

  const batchSummaryText = useMemo(() => {
    if (!registerBatch) return "";
    return [
      `批次: ${registerBatch.batchId || "--"}`,
      `进度: ${registerBatch.progress || `${registerBatch.completed}/${registerBatch.total}`}`,
      `成功: ${registerBatch.success}`,
      `失败: ${registerBatch.failed}`,
      `当前索引: ${registerBatch.currentIndex + 1}`,
      `状态: ${registerBatch.cancelled ? "已取消" : registerBatch.finished ? "已结束" : "运行中"}`,
      "",
      "任务 UUID：",
      ...registerBatchTaskUuids,
    ].join("\n");
  }, [registerBatch, registerBatchTaskUuids]);

  const outlookBatchSummaryText = useMemo(() => {
    if (!registerOutlookBatch) return "";
    return [
      `批次: ${registerOutlookBatch.batchId || "--"}`,
      `进度: ${registerOutlookBatch.progress || `${registerOutlookBatch.completed}/${registerOutlookBatch.total}`}`,
      `成功: ${registerOutlookBatch.success}`,
      `失败: ${registerOutlookBatch.failed}`,
      `跳过: ${registerOutlookBatch.skipped}`,
      "",
      ...registerOutlookBatch.logs,
    ].join("\n");
  }, [registerOutlookBatch]);

  const allOutlookSelected =
    registerOutlookAccounts != null &&
    registerOutlookAccounts.accounts.length > 0 &&
    registerOutlookAccounts.accounts.every((account) =>
      registerSelectedOutlookIds.includes(account.id),
    );

  const toggleOutlookSelection = (account: RegisterOutlookAccount) => {
    setRegisterSelectedOutlookIds((current) =>
      current.includes(account.id)
        ? current.filter((id) => id !== account.id)
        : [...current, account.id],
    );
  };

  return (
    <Dialog open={open} onOpenChange={handleDialogOpenChange}>
      <DialogContent className="glass-card border-none p-0 sm:max-w-[760px]">
        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <div className="shrink-0 bg-muted/20 px-4 pt-4 sm:px-6 sm:pt-6">
            <DialogHeader className="mb-4">
              <DialogTitle className="flex items-center gap-2">
                <LogIn className="h-5 w-5 text-primary" />
                新增账号
              </DialogTitle>
              <DialogDescription>
                通过登录授权、自动注册或批量导入来添加账号。
              </DialogDescription>
            </DialogHeader>
            <TabsList className="mb-0 grid h-10 w-full grid-cols-3">
              <TabsTrigger value="login" className="gap-2">
                <LogIn className="h-3.5 w-3.5" /> 登录授权
              </TabsTrigger>
              <TabsTrigger value="register" className="gap-2">
                <Sparkles className="h-3.5 w-3.5" /> 自动注册
              </TabsTrigger>
              <TabsTrigger value="bulk" className="gap-2">
                <FileUp className="h-3.5 w-3.5" /> 批量导入
              </TabsTrigger>
            </TabsList>
          </div>

          <div className="p-4 sm:p-6">
            <TabsContent value="login" className="mt-0 space-y-4">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label>标签 (逗号分隔)</Label>
                  <Input
                    placeholder="例如：高频, 团队A"
                    value={tags}
                    onChange={(event) => setTags(event.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label>分组</Label>
                  <Select value={group} onValueChange={(value) => value && setGroup(value)}>
                    <SelectTrigger>
                      <SelectValue placeholder="选择分组">
                        {(value) => {
                          const nextValue = String(value || "").trim();
                          if (!nextValue) return "选择分组";
                          return GROUP_LABELS[nextValue] || nextValue;
                        }}
                      </SelectValue>
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="TEAM">团队 (TEAM)</SelectItem>
                      <SelectItem value="PERSONAL">个人 (PERSONAL)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="space-y-2">
                <Label>备注/描述</Label>
                <Input
                  placeholder="例如：主号 / 测试号"
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                />
              </div>

              <div className="pt-2">
                <Button
                  onClick={handleStartLogin}
                  disabled={isLoading || isPollingLogin}
                  className="w-full gap-2"
                >
                  <ExternalLink className="h-4 w-4" /> 登录授权
                </Button>
                {loginUrl ? (
                  <div className="mt-3 flex flex-col gap-2 rounded-lg border border-primary/10 bg-primary/5 p-2 animate-in fade-in zoom-in duration-300 sm:flex-row sm:items-center">
                    <Input
                      value={loginUrl}
                      readOnly
                      className="h-8 border-none bg-transparent font-mono text-[10px]"
                    />
                    <Button variant="ghost" size="sm" onClick={copyUrl} className="h-8 w-8 p-0">
                      <Clipboard className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ) : null}
                {loginHint ? (
                  <p className="mt-2 text-xs text-muted-foreground">{loginHint}</p>
                ) : null}
              </div>

              <div className="space-y-3 border-t pt-4">
                <div className="space-y-2">
                  <Label className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Hash className="h-3 w-3" /> 手动解析回调 (当本地 48760 端口占用时)
                  </Label>
                  <div className="flex flex-col gap-2 sm:flex-row">
                    <Input
                      placeholder="粘贴浏览器跳转后的完整回调 URL (包含 state 和 code)"
                      value={manualCallback}
                      onChange={(event) => setManualCallback(event.target.value)}
                      className="h-9 font-mono text-[10px]"
                    />
                    <Button
                      variant="secondary"
                      onClick={handleManualCallback}
                      disabled={isLoading}
                      className="h-9 shrink-0 px-4"
                    >
                      解析
                    </Button>
                  </div>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="register" className="mt-0 space-y-4">
              <div className="rounded-lg border border-primary/15 bg-primary/5 p-3 text-xs text-muted-foreground">
                通过内置的 `codex-register` 服务自动创建账号。支持标准注册、Any-Auto 注册与 Browserbase-DDG 注册三种通道；单个注册会直接轮询并导入，批量注册会在任务结束后自动把成功账号导入到当前列表。
              </div>

              <div className="flex items-center justify-between rounded-lg border border-border/60 bg-muted/20 p-3">
                <div className="space-y-1">
                  <p className="text-sm font-medium">注册服务状态</p>
                  <p className="text-xs text-muted-foreground">
                    {registerServices?.serviceUrl || "未连接"}
                  </p>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    void loadRegisterServices();
                    if (registerChannel === "browserbase_ddg") {
                      void loadRegisterBrowserbaseConfigs();
                    }
                    if (registerMode === "outlook-batch") {
                      void loadRegisterOutlookAccounts();
                    }
                  }}
                  disabled={registerBusy}
                  className="gap-2"
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                  刷新
                </Button>
              </div>

              <div className="space-y-2">
                <Label>注册通道</Label>
                <div className="grid gap-2 sm:grid-cols-3">
                  <Button
                    type="button"
                    variant={registerChannel === "standard" ? "default" : "outline"}
                    disabled={registerBusy}
                    onClick={() => setRegisterChannel("standard")}
                  >
                    标准注册
                  </Button>
                  <Button
                    type="button"
                    variant={registerChannel === "any_auto" ? "default" : "outline"}
                    disabled={registerBusy}
                    onClick={() => setRegisterChannel("any_auto")}
                  >
                    Any-Auto
                  </Button>
                  <Button
                    type="button"
                    variant={registerChannel === "browserbase_ddg" ? "default" : "outline"}
                    disabled={registerBusy}
                    onClick={() => setRegisterChannel("browserbase_ddg")}
                  >
                    Browserbase-DDG
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">
                  当前通道：{getRegisterChannelLabel(registerChannel)}
                </p>
              </div>

              <div
                className={`grid gap-2 ${canUseOutlookBatchRegisterMode(registerChannel) ? "sm:grid-cols-3" : "sm:grid-cols-2"}`}
              >
                <Button
                  type="button"
                  variant={registerMode === "single" ? "default" : "outline"}
                  disabled={registerBusy}
                  onClick={() => setRegisterMode("single")}
                >
                  单个注册
                </Button>
                <Button
                  type="button"
                  variant={registerMode === "batch" ? "default" : "outline"}
                  disabled={registerBusy}
                  onClick={() => setRegisterMode("batch")}
                >
                  批量注册
                </Button>
                {canUseOutlookBatchRegisterMode(registerChannel) ? (
                  <Button
                    type="button"
                    variant={registerMode === "outlook-batch" ? "default" : "outline"}
                    disabled={registerBusy}
                    onClick={() => setRegisterMode("outlook-batch")}
                  >
                    Outlook 批量
                  </Button>
                ) : null}
              </div>

              {registerError ? (
                <div className="rounded-lg border border-red-500/20 bg-red-500/5 p-3 text-xs text-red-600 dark:text-red-400">
                  {registerError}
                </div>
              ) : null}

              {registerImportSummary ? (
                <Textarea
                  readOnly
                  value={registerImportSummary}
                  className="min-h-[110px] resize-none overflow-auto whitespace-pre-wrap break-all font-mono text-[10px] leading-4 [overflow-wrap:anywhere]"
                />
              ) : null}

              {registerMode !== "outlook-batch" ? (
                isBrowserbaseRegisterChannel ? (
                  <>
                    <div className="grid gap-4 sm:grid-cols-2">
                      <div className="space-y-2">
                        <Label>Browserbase-DDG 配置</Label>
                        <Select
                          value={registerBrowserbaseConfigId}
                          onValueChange={(value) => setRegisterBrowserbaseConfigId(value || "")}
                        >
                          <SelectTrigger>
                            <SelectValue placeholder="选择 Browserbase-DDG 配置" />
                          </SelectTrigger>
                          <SelectContent>
                            {enabledRegisterBrowserbaseConfigs.map((item) => (
                              <SelectItem key={item.id} value={String(item.id)}>
                                {item.name} (优先级 {item.priority})
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                      <div className="space-y-2">
                        <Label>代理 (可选)</Label>
                        <Input
                          placeholder="http://user:pass@host:port"
                          value={registerProxy}
                          onChange={(event) => setRegisterProxy(event.target.value)}
                        />
                      </div>
                    </div>

                    <div className="flex flex-wrap items-center gap-2">
                      <Badge variant="outline">总配置 {registerBrowserbaseConfigs.length}</Badge>
                      <Badge variant="outline">启用中 {enabledRegisterBrowserbaseConfigs.length}</Badge>
                      {selectedRegisterBrowserbaseConfig ? (
                        <Badge variant="outline">当前：{selectedRegisterBrowserbaseConfig.name}</Badge>
                      ) : null}
                    </div>

                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground">
                        该模式会使用 Browserbase 会话 + DDG alias 邮箱自动完成注册。
                      </p>
                      <p className="text-xs text-muted-foreground">
                        详细 Token、回调端口、模型等参数请到“邮箱服务”页的 Browserbase-DDG 注册配置中维护。
                      </p>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="grid gap-4 sm:grid-cols-2">
                      <div className="space-y-2">
                        <Label>邮箱服务类型</Label>
                        <Select
                          value={registerServiceType}
                          onValueChange={(value) => {
                            if (!value) return;
                            setRegisterServiceType(value);
                            const nextGroup = registerTypeOptions.find((item) => item.value === value)?.group;
                            const nextId = (nextGroup?.services || []).some((item) => item.id != null)
                              ? REGISTER_SERVICE_AUTO
                              : "";
                            setRegisterServiceId(nextId);
                            setAutoCreateTempMailService(getDefaultTempMailAutoCreateState(value));
                            setRegisterTempMailDomainConfigId("");
                          }}
                        >
                          <SelectTrigger>
                            <SelectValue placeholder="选择邮箱服务" />
                          </SelectTrigger>
                          <SelectContent>
                            {registerTypeOptions.map((item) => (
                              <SelectItem key={item.value} value={item.value}>
                                {item.label} ({item.group.count})
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                      <div className="space-y-2">
                        <Label>代理 (可选)</Label>
                    <Input
                      placeholder="http://user:pass@host:port"
                      value={registerProxy}
                      onChange={(event) => setRegisterProxy(event.target.value)}
                    />
                  </div>
                </div>

                {showTempMailAutoCreateToggle ? (
                  <div className="flex items-center justify-between rounded-lg border border-border/60 bg-muted/20 px-3 py-2">
                    <div className="space-y-1">
                      <p className="text-sm font-medium">自动创建临时 Temp-Mail 服务</p>
                      <p className="text-xs text-muted-foreground">
                        {registerMode === "batch"
                          ? "开启后，整批任务会共用 1 条临时 Temp-Mail 服务；可选域名配置，留空则随机挑一条，并在批次结束后自动删除。"
                          : "开启后，本次注册会自动创建 1 条临时 Temp-Mail 服务；可选域名配置，留空则随机挑一条，并在任务结束后自动删除。"}
                      </p>
                    </div>
                    <Switch
                      checked={autoCreateTempMailService}
                      onCheckedChange={setAutoCreateTempMailService}
                    />
                  </div>
                ) : null}

                {showTempMailDomainConfigPicker ? (
                  registerTempMailDomainConfigs.length ? (
                    <div className="space-y-2">
                      <Label>域名配置</Label>
                      <Select
                        value={registerTempMailDomainConfigId || REGISTER_TEMP_MAIL_DOMAIN_CONFIG_AUTO}
                        onValueChange={(value) => {
                          setRegisterTempMailDomainConfigId(
                            !value || value === REGISTER_TEMP_MAIL_DOMAIN_CONFIG_AUTO ? "" : value,
                          );
                        }}
                      >
                        <SelectTrigger>
                          <SelectValue placeholder="留空则随机选择一条域名配置" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value={REGISTER_TEMP_MAIL_DOMAIN_CONFIG_AUTO}>
                            留空，随机选择
                          </SelectItem>
                          {registerTempMailDomainConfigs.map((item) => (
                            <SelectItem key={item.id} value={item.id}>
                              {item.name} ({item.domainBase})
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        这里选的是 Temp-Mail 域名配置，不是已有邮箱服务。留空时后端会从现有配置里随机挑一条。
                      </p>
                    </div>
                  ) : (
                    <div className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-300">
                      当前没有可用的 Temp-Mail 域名配置。请先到“邮箱服务”页的 Cloudflare Temp-Mail 设置里新增一条域名配置。
                    </div>
                  )
                ) : null}

                {selectedRegisterServiceHasChoices ? (
                  <div className="space-y-2">
                    <Label>具体服务</Label>
                    <Select
                      value={registerServiceId}
                      disabled={disableTempMailServicePicker}
                      onValueChange={(value) => setRegisterServiceId(value || REGISTER_SERVICE_AUTO)}
                    >
                          <SelectTrigger>
                            <SelectValue placeholder="按当前类型自动轮询" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value={REGISTER_SERVICE_AUTO}>
                              按当前类型自动轮询
                            </SelectItem>
                            {(selectedRegisterGroup?.services || [])
                              .filter((item) => item.id != null)
                              .map((item) => (
                                <SelectItem key={String(item.id)} value={String(item.id)}>
                                  {item.name}
                                </SelectItem>
                              ))}
                          </SelectContent>
                        </Select>
                      </div>
                    ) : null}

                    <div className="space-y-1">
                      {selectedRegisterGroup?.services?.[0]?.description ? (
                        <p className="text-xs text-muted-foreground">
                          {selectedRegisterGroup.services[0].description}
                        </p>
                      ) : null}
                      <p className="text-xs text-muted-foreground">
                        {registerChannel === "any_auto"
                          ? "该模式会优先尝试复用已登录会话，直接从 ChatGPT Session 提取 Access Token；若失败再回退到原有 OAuth 收敛链路。"
                          : "具体服务留空时，会在当前类型的可用服务之间自动轮询；代理留空时会按代理池轮询。"}
                      </p>
                    </div>
                  </>
                )
              ) : (
                <>
                  <div className="flex items-center gap-2 rounded-lg border border-border/60 bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
                    <Mail className="h-3.5 w-3.5" />
                    当前模式会使用已导入的 Outlook 邮箱服务逐个注册，已注册邮箱可按开关选择是否跳过注册，结束后再按自动入池开关处理。
                  </div>

                  <div className="grid gap-4 sm:grid-cols-2">
                    <div className="space-y-2">
                      <Label>代理 (可选)</Label>
                      <Input
                        placeholder="http://user:pass@host:port"
                        value={registerProxy}
                        onChange={(event) => setRegisterProxy(event.target.value)}
                      />
                    </div>
                    <div className="flex items-center justify-between rounded-lg border border-border/60 bg-muted/20 px-3 py-2">
                      <div className="space-y-1">
                        <p className="text-sm font-medium">跳过已注册邮箱</p>
                        <p className="text-xs text-muted-foreground">
                          已注册邮箱不再重复注册，但结束后仍会尝试导入到当前列表
                        </p>
                      </div>
                      <Switch
                        checked={registerSkipRegistered}
                        onCheckedChange={setRegisterSkipRegistered}
                      />
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">总数 {registerOutlookAccounts?.total ?? 0}</Badge>
                    <Badge variant="outline">
                      已注册 {registerOutlookAccounts?.registeredCount ?? 0}
                    </Badge>
                    <Badge variant="outline">
                      未注册 {registerOutlookAccounts?.unregisteredCount ?? 0}
                    </Badge>
                    <Badge variant="outline">已选 {registerSelectedOutlookIds.length}</Badge>
                  </div>

                  <div className="flex items-center gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={registerBusy || !registerOutlookAccounts?.accounts.length}
                      onClick={() =>
                        setRegisterSelectedOutlookIds(
                          allOutlookSelected
                            ? []
                            : (registerOutlookAccounts?.accounts || []).map((account) => account.id),
                        )
                      }
                    >
                      {allOutlookSelected ? "取消全选" : "全选"}
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={registerBusy || !registerOutlookAccounts?.accounts.length}
                      onClick={() =>
                        setRegisterSelectedOutlookIds(
                          (registerOutlookAccounts?.accounts || [])
                            .filter((account) => !account.isRegistered)
                            .map((account) => account.id),
                        )
                      }
                    >
                      选择未注册
                    </Button>
                  </div>

                  <div className="max-h-[260px] overflow-auto rounded-lg border border-border/60 bg-muted/10">
                    {!registerOutlookAccounts?.accounts.length ? (
                      <div className="p-6 text-center text-sm text-muted-foreground">
                        暂无可用 Outlook 账号，请先到邮箱服务页面导入。
                      </div>
                    ) : (
                      <div className="divide-y divide-border/60">
                        {registerOutlookAccounts.accounts.map((account) => (
                          <label
                            key={account.id}
                            className="flex cursor-pointer items-start gap-3 px-4 py-3 hover:bg-muted/20"
                          >
                            <Checkbox
                              checked={registerSelectedOutlookIds.includes(account.id)}
                              onCheckedChange={() => toggleOutlookSelection(account)}
                            />
                            <div className="min-w-0 flex-1">
                              <div className="flex flex-wrap items-center gap-2">
                                <span className="text-sm font-medium">{account.name}</span>
                                <Badge variant="outline">{account.email}</Badge>
                                <Badge variant={account.hasOauth ? "default" : "secondary"}>
                                  {account.hasOauth ? "OAuth 完整" : "OAuth 缺失"}
                                </Badge>
                                <Badge variant={account.isRegistered ? "default" : "secondary"}>
                                  {account.isRegistered ? "已注册" : "未注册"}
                                </Badge>
                              </div>
                            </div>
                          </label>
                        ))}
                      </div>
                    )}
                  </div>
                </>
              )}

              <div className="flex items-center justify-between rounded-lg border border-border/60 bg-muted/20 px-3 py-2">
                <div className="space-y-1">
                  <p className="text-sm font-medium">注册成功后自动入池</p>
                  <p className="text-xs text-muted-foreground">
                    默认开启；关闭后只创建注册结果，可在注册中心手动加入号池。
                  </p>
                </div>
                <Switch
                  checked={registerAutoImport}
                  onCheckedChange={setRegisterAutoImport}
                />
              </div>

              {(registerMode === "batch" || registerMode === "outlook-batch") ? (
                <div className="grid gap-4 rounded-lg border border-border/60 bg-muted/10 p-4 sm:grid-cols-2">
                  {registerMode === "batch" ? (
                    <div className="space-y-2">
                      <Label>注册数量</Label>
                      <Input
                        type="number"
                        min={1}
                        value={registerBatchCount}
                        onChange={(event) => setRegisterBatchCount(event.target.value)}
                      />
                    </div>
                  ) : (
                    <div className="space-y-2">
                      <Label>执行模式</Label>
                      <Select
                        value={registerExecutionMode}
                        onValueChange={(value) =>
                          setRegisterExecutionMode(value as RegisterExecutionMode)
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="pipeline">流水线 (pipeline)</SelectItem>
                          <SelectItem value="parallel">并行 (parallel)</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  )}
                  <div className="space-y-2">
                    <Label>并发数</Label>
                    <Input
                      type="number"
                      min={1}
                      value={registerConcurrency}
                      onChange={(event) => setRegisterConcurrency(event.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>最小间隔 (秒)</Label>
                    <Input
                      type="number"
                      min={0}
                      value={registerIntervalMin}
                      onChange={(event) => setRegisterIntervalMin(event.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>最大间隔 (秒)</Label>
                    <Input
                      type="number"
                      min={0}
                      value={registerIntervalMax}
                      onChange={(event) => setRegisterIntervalMax(event.target.value)}
                    />
                  </div>
                  {registerMode === "batch" ? (
                    <div className="space-y-2 sm:col-span-2">
                      <Label>执行模式</Label>
                      <Select
                        value={registerExecutionMode}
                        onValueChange={(value) =>
                          setRegisterExecutionMode(value as RegisterExecutionMode)
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="pipeline">流水线 (pipeline)</SelectItem>
                          <SelectItem value="parallel">并行 (parallel)</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  ) : null}
                </div>
              ) : null}

              <div className="flex flex-col gap-2 sm:flex-row">
                <Button
                  onClick={handleStartRegister}
                  disabled={
                    registerBusy ||
                    (registerMode !== "outlook-batch" &&
                      (!hasAvailableRegisterSource ||
                        (isBrowserbaseRegisterChannel && !registerBrowserbaseConfigId))) ||
                    (registerMode === "outlook-batch" && registerSelectedOutlookIds.length === 0)
                  }
                  className="flex-1 gap-2"
                >
                  <Sparkles className="h-4 w-4" />
                  {isRegisterSubmitting || isRegisterImporting ? "处理中..." : getRegisterSubmitLabel(registerAutoImport)}
                </Button>
                {(registerMode === "batch" &&
                  registerBatch &&
                  !registerBatch.finished &&
                  !registerBatch.cancelled) ||
                (registerMode === "outlook-batch" &&
                  registerOutlookBatch &&
                  !registerOutlookBatch.finished &&
                  !registerOutlookBatch.cancelled) ? (
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => void handleCancelRegisterBatch()}
                    disabled={isRegisterImporting}
                    className="gap-2"
                  >
                    <Ban className="h-4 w-4" />
                    取消
                  </Button>
                ) : null}
              </div>

              {registerHint ? (
                <p className="text-xs text-muted-foreground">{registerHint}</p>
              ) : null}

              {registerTask ? (
                <div className="space-y-3 rounded-lg border border-border/60 bg-muted/20 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium">单个注册任务</p>
                      <p className="font-mono text-[11px] text-muted-foreground">
                        {registerTask.taskUuid}
                      </p>
                    </div>
                    <Badge variant="outline">
                      {getRegisterStatusLabel(registerTask.status)}
                    </Badge>
                  </div>
                  {registerTask.email ? (
                    <p className="text-xs text-muted-foreground">
                      注册邮箱：{registerTask.email}
                    </p>
                  ) : null}
                  <div className="space-y-2">
                    <Label>任务日志</Label>
                    <Textarea
                      readOnly
                      value={registerTask.logs.join("\n")}
                      className="min-h-[220px] resize-none overflow-auto whitespace-pre-wrap break-all font-mono text-[10px] leading-4 [overflow-wrap:anywhere]"
                    />
                  </div>
                </div>
              ) : null}

              {registerMode === "batch" && registerBatch ? (
                <div className="space-y-3 rounded-lg border border-border/60 bg-muted/20 p-4">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">总数 {registerBatch.total}</Badge>
                    <Badge variant="outline">完成 {registerBatch.completed}</Badge>
                    <Badge variant="outline">成功 {registerBatch.success}</Badge>
                    <Badge variant="outline">失败 {registerBatch.failed}</Badge>
                    <Badge variant="outline">{registerBatch.progress || "--"}</Badge>
                  </div>
                  <Textarea
                    readOnly
                    value={batchSummaryText}
                    className="min-h-[180px] resize-none overflow-auto whitespace-pre-wrap break-all font-mono text-[10px] leading-4 [overflow-wrap:anywhere]"
                  />
                </div>
              ) : null}

              {registerMode === "outlook-batch" && registerOutlookBatch ? (
                <div className="space-y-3 rounded-lg border border-border/60 bg-muted/20 p-4">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">总数 {registerOutlookBatch.total}</Badge>
                    <Badge variant="outline">完成 {registerOutlookBatch.completed}</Badge>
                    <Badge variant="outline">成功 {registerOutlookBatch.success}</Badge>
                    <Badge variant="outline">失败 {registerOutlookBatch.failed}</Badge>
                    <Badge variant="outline">跳过 {registerOutlookBatch.skipped}</Badge>
                    <Badge variant="outline">{registerOutlookBatch.progress || "--"}</Badge>
                  </div>
                  <Textarea
                    readOnly
                    value={outlookBatchSummaryText}
                    className="min-h-[220px] resize-none overflow-auto whitespace-pre-wrap break-all font-mono text-[10px] leading-4 [overflow-wrap:anywhere]"
                  />
                </div>
              ) : null}
            </TabsContent>

            <TabsContent value="bulk" className="mt-0 space-y-4">
              <div className="space-y-2">
                <Label>账号数据 (Token 可每行一个，JSON 可整段粘贴)</Label>
                <Textarea
                  placeholder="粘贴账号数据。普通 Token 可每行一个；完整 JSON / JSON 数组请整段粘贴。"
                  className="min-h-[250px] resize-none overflow-auto whitespace-pre-wrap break-all font-mono text-[10px] leading-4 [overflow-wrap:anywhere]"
                  value={bulkContent}
                  onChange={(event) => setBulkContent(event.target.value)}
                />
              </div>
              <div className="rounded-lg border border-blue-500/20 bg-blue-500/5 p-3 text-[10px] leading-relaxed text-blue-600 dark:text-blue-400">
                <Info className="mr-1.5 inline-block h-3.5 w-3.5 -mt-0.5" />
                支持格式：ChatGPT 账号（Refresh Token）、Claude Session 等。系统将自动识别格式并导入。
              </div>
              <Button onClick={handleBulkImport} disabled={isLoading} className="w-full">
                开始导入
              </Button>
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
