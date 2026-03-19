"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { 
  Dialog, 
  DialogContent, 
  DialogDescription, 
  DialogHeader, 
  DialogTitle
} from "@/components/ui/dialog";
import { 
  Tabs, 
  TabsContent, 
  TabsList, 
  TabsTrigger 
} from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { 
  Select, 
  SelectContent, 
  SelectItem, 
  SelectTrigger, 
  SelectValue 
} from "@/components/ui/select";
import { accountClient } from "@/lib/api/account-client";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import { FileUp, Info, LogIn, Clipboard, ExternalLink, Hash, Sparkles, RefreshCw } from "lucide-react";
import type {
  RegisterAvailableServicesResult,
  RegisterServiceGroup,
  RegisterTaskSnapshot,
} from "@/types";

interface AddAccountModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

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

export function AddAccountModal({ open, onOpenChange }: AddAccountModalProps) {
  const [activeTab, setActiveTab] = useState("login");
  const [isLoading, setIsLoading] = useState(false);
  const [isPollingLogin, setIsPollingLogin] = useState(false);
  const [loginHint, setLoginHint] = useState("");
  const queryClient = useQueryClient();
  const loginPollTokenRef = useRef(0);
  const registerPollTokenRef = useRef(0);

  // Login Form
  const [tags, setTags] = useState("");
  const [note, setNote] = useState("");
  const [group, setGroup] = useState("");
  const [loginUrl, setLoginUrl] = useState("");
  const [manualCallback, setManualCallback] = useState("");

  // Bulk Import
  const [bulkContent, setBulkContent] = useState("");

  // Register Integration
  const [registerServices, setRegisterServices] =
    useState<RegisterAvailableServicesResult | null>(null);
  const [isRegisterLoading, setIsRegisterLoading] = useState(false);
  const [isRegisterSubmitting, setIsRegisterSubmitting] = useState(false);
  const [isRegisterImporting, setIsRegisterImporting] = useState(false);
  const [registerError, setRegisterError] = useState("");
  const [registerHint, setRegisterHint] = useState("");
  const [registerServiceType, setRegisterServiceType] = useState("tempmail");
  const [registerServiceId, setRegisterServiceId] = useState("");
  const [registerProxy, setRegisterProxy] = useState("");
  const [registerTask, setRegisterTask] = useState<RegisterTaskSnapshot | null>(null);

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
    setRegisterServices(null);
    setIsRegisterLoading(false);
    setIsRegisterSubmitting(false);
    setIsRegisterImporting(false);
    setRegisterError("");
    setRegisterHint("");
    setRegisterServiceType("tempmail");
    setRegisterServiceId("");
    setRegisterProxy("");
    setRegisterTask(null);
  }, []);

  const invalidateLoginQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["accounts"] }),
      queryClient.invalidateQueries({ queryKey: ["usage"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  };

  const handleDialogOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) {
      resetModalState();
    }
    onOpenChange(nextOpen);
  };

  const completeLoginSuccess = async (message: string) => {
    await invalidateLoginQueries();
    toast.success(message);
    resetModalState();
    onOpenChange(false);
  };

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

  const syncRegisterSelection = useCallback(
    (catalog: RegisterAvailableServicesResult) => {
      const nextOptions = [
        { value: "tempmail", group: catalog.tempmail },
        { value: "outlook", group: catalog.outlook },
        { value: "custom_domain", group: catalog.customDomain },
        { value: "temp_mail", group: catalog.tempMail },
      ].filter((item) => item.group.available);
      const nextType =
        nextOptions.find((item) => item.value === registerServiceType)?.value ||
        nextOptions[0]?.value ||
        "tempmail";
      const nextGroup = nextOptions.find((item) => item.value === nextType)?.group || null;
      const nextServiceId =
        nextGroup?.services.find((item) => item.id != null)?.id != null
          ? String(nextGroup.services.find((item) => item.id != null)?.id)
          : "";
      setRegisterServiceType(nextType);
      setRegisterServiceId(nextServiceId);
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

  useEffect(() => {
    if (!open || activeTab !== "register" || registerServices || isRegisterLoading) {
      return;
    }
    void loadRegisterServices();
  }, [activeTab, isRegisterLoading, loadRegisterServices, open, registerServices]);

  const completeRegisterSuccess = async (message: string) => {
    await invalidateLoginQueries();
    toast.success(message);
    resetModalState();
    onOpenChange(false);
  };

  const importCompletedRegisterTask = async (taskUuid: string) => {
    setIsRegisterImporting(true);
    setRegisterHint("注册成功，正在自动导入账号...");
    try {
      const imported = await accountClient.importRegisterTask(taskUuid);
      await completeRegisterSuccess(`账号已注册并导入：${imported.email}`);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setRegisterError(`注册成功，但自动导入失败：${message}`);
      setRegisterHint("");
      toast.error(`自动导入失败: ${message}`);
    } finally {
      setIsRegisterImporting(false);
      setIsRegisterSubmitting(false);
    }
  };

  const waitForRegisterTask = async (taskUuid: string) => {
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
            await importCompletedRegisterTask(snapshot.taskUuid);
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
  };

  const handleStartLogin = async () => {
    setIsLoading(true);
    setLoginHint("");
    try {
      const result = await accountClient.startLogin({
        tags: tags.split(",").map(t => t.trim()).filter(Boolean),
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
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["accounts"] }),
        queryClient.invalidateQueries({ queryKey: ["usage"] }),
        queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
      ]);
      resetModalState();
      onOpenChange(false);
    } catch (err: unknown) {
      toast.error(`导入失败: ${getBulkImportErrorMessage(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleStartRegister = async () => {
    if (!selectedRegisterGroup?.available) {
      toast.error("当前没有可用的注册邮箱服务");
      return;
    }
    setRegisterError("");
    setRegisterHint("");
    setIsRegisterSubmitting(true);
    try {
      const task = await accountClient.startRegisterTask({
        emailServiceType: registerServiceType,
        emailServiceId: registerServiceId ? Number(registerServiceId) : null,
        proxy: registerProxy || null,
      });
      setRegisterTask(task);
      toast.success("注册任务已启动");
      void waitForRegisterTask(task.taskUuid);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setRegisterError(message);
      setIsRegisterSubmitting(false);
      toast.error(`启动注册失败: ${message}`);
    }
  };

  const copyUrl = () => {
    if (!loginUrl) return;
    navigator.clipboard.writeText(loginUrl);
    toast.success("链接已复制");
  };

  return (
    <Dialog open={open} onOpenChange={handleDialogOpenChange}>
      <DialogContent className="glass-card max-h-[85vh] overflow-hidden border-none p-0 sm:max-w-[640px]">
        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <div className="shrink-0 bg-muted/20 px-6 pt-6">
            <DialogHeader className="mb-4">
              <DialogTitle className="flex items-center gap-2">
                <LogIn className="h-5 w-5 text-primary" />
                新增账号
              </DialogTitle>
              <DialogDescription>
                通过登录授权或批量导入文本内容来添加账号。
              </DialogDescription>
            </DialogHeader>
            <TabsList className="grid w-full grid-cols-3 h-10 mb-0">
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

          <div className="max-h-[calc(85vh-154px)] overflow-y-auto p-6">
            <TabsContent value="login" className="mt-0 space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>标签 (逗号分隔)</Label>
                  <Input placeholder="例如：高频, 团队A" value={tags} onChange={e => setTags(e.target.value)} />
                </div>
                <div className="space-y-2">
                  <Label>分组</Label>
                  <Select value={group} onValueChange={(val) => val && setGroup(val)}>
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
                <Input placeholder="例如：主号 / 测试号" value={note} onChange={e => setNote(e.target.value)} />
              </div>

              <div className="pt-2">
                <Button onClick={handleStartLogin} disabled={isLoading || isPollingLogin} className="w-full gap-2">
                  <ExternalLink className="h-4 w-4" /> 登录授权
                </Button>
                {loginUrl && (
                  <div className="mt-3 p-2 rounded-lg bg-primary/5 border border-primary/10 flex items-center gap-2 animate-in fade-in zoom-in duration-300">
                    <Input value={loginUrl} readOnly className="font-mono text-[10px] h-8 border-none bg-transparent" />
                    <Button variant="ghost" size="sm" onClick={copyUrl} className="h-8 w-8 p-0">
                      <Clipboard className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                )}
                {loginHint ? (
                  <p className="mt-2 text-xs text-muted-foreground">{loginHint}</p>
                ) : null}
              </div>

              <div className="space-y-3 pt-4 border-t">
                <div className="space-y-2">
                  <Label className="text-xs flex items-center gap-1.5 text-muted-foreground">
                    <Hash className="h-3 w-3" /> 手动解析回调 (当本地 48760 端口占用时)
                  </Label>
                  <div className="flex gap-2">
                    <Input 
                      placeholder="粘贴浏览器跳转后的完整回调 URL (包含 state 和 code)" 
                      value={manualCallback}
                      onChange={e => setManualCallback(e.target.value)}
                      className="font-mono text-[10px] h-9" 
                    />
                    <Button 
                      variant="secondary" 
                      onClick={handleManualCallback} 
                      disabled={isLoading} 
                      className="h-9 px-4 shrink-0"
                    >
                      解析
                    </Button>
                  </div>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="register" className="mt-0 space-y-4">
              <div className="rounded-lg border border-primary/15 bg-primary/5 p-3 text-xs text-muted-foreground">
                通过内置的 `codex-register` 注册服务自动创建账号。注册完成后会直接导入到当前账号列表。
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
                  onClick={() => void loadRegisterServices()}
                  disabled={isRegisterLoading || isRegisterSubmitting || isRegisterImporting}
                  className="gap-2"
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                  刷新
                </Button>
              </div>

              {registerError ? (
                <div className="rounded-lg border border-red-500/20 bg-red-500/5 p-3 text-xs text-red-600 dark:text-red-400">
                  {registerError}
                </div>
              ) : null}

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>邮箱服务类型</Label>
                  <Select
                    value={registerServiceType}
                    onValueChange={(value) => {
                      if (!value) return;
                      setRegisterServiceType(value);
                      const nextGroup = registerTypeOptions.find((item) => item.value === value)?.group;
                      const nextId =
                        nextGroup?.services.find((item) => item.id != null)?.id != null
                          ? String(nextGroup.services.find((item) => item.id != null)?.id)
                          : "";
                      setRegisterServiceId(nextId);
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

              {selectedRegisterServiceHasChoices ? (
                <div className="space-y-2">
                  <Label>具体服务</Label>
                  <Select
                    value={registerServiceId}
                    onValueChange={(value) => {
                      if (!value) return;
                      setRegisterServiceId(value);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="选择具体服务" />
                    </SelectTrigger>
                    <SelectContent>
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

              {selectedRegisterGroup?.services?.[0]?.description ? (
                <p className="text-xs text-muted-foreground">
                  {selectedRegisterGroup.services[0].description}
                </p>
              ) : null}

              <Button
                onClick={handleStartRegister}
                disabled={
                  isRegisterLoading ||
                  isRegisterSubmitting ||
                  isRegisterImporting ||
                  !selectedRegisterGroup?.available
                }
                className="w-full gap-2"
              >
                <Sparkles className="h-4 w-4" />
                {isRegisterSubmitting || isRegisterImporting ? "处理中..." : "开始注册并导入"}
              </Button>

              {registerHint ? (
                <p className="text-xs text-muted-foreground">{registerHint}</p>
              ) : null}

              {registerTask ? (
                <div className="space-y-3 rounded-lg border border-border/60 bg-muted/20 p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium">任务状态</p>
                      <p className="font-mono text-[11px] text-muted-foreground">
                        {registerTask.taskUuid}
                      </p>
                    </div>
                    <div className="text-sm">{registerTask.status || "--"}</div>
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
                      className="min-h-[220px] resize-none overflow-auto whitespace-pre-wrap break-all [overflow-wrap:anywhere] font-mono text-[10px] leading-4"
                    />
                  </div>
                </div>
              ) : null}
            </TabsContent>

            <TabsContent value="bulk" className="mt-0 space-y-4">
              <div className="space-y-2">
                <Label>账号数据 (Token 可每行一个，JSON 可整段粘贴)</Label>
                <Textarea 
                  placeholder="粘贴账号数据。普通 Token 可每行一个；完整 JSON / JSON 数组请整段粘贴。"
                  className="min-h-[250px] resize-none overflow-auto whitespace-pre-wrap break-all [overflow-wrap:anywhere] font-mono text-[10px] leading-4"
                  value={bulkContent}
                  onChange={(e) => setBulkContent(e.target.value)}
                />
              </div>
              <div className="rounded-lg bg-blue-500/5 border border-blue-500/20 p-3 text-[10px] text-blue-600 dark:text-blue-400 leading-relaxed">
                <Info className="h-3.5 w-3.5 inline-block mr-1.5 -mt-0.5" />
                支持格式：ChatGPT 账号（Refresh Token）、 Claude Session 等。系统将自动识别格式并导入。
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
