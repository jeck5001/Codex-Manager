"use client";

import { useState } from "react";
import {
  CheckCircle2,
  MoreVertical,
  Plus,
  RefreshCw,
  Trash2,
  Wrench,
  XCircle,
} from "lucide-react";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useRegisterBrowserbaseConfigs } from "@/hooks/useRegisterBrowserbaseConfigs";
import { formatApiDateTime } from "@/lib/utils/datetime";
import { cn } from "@/lib/utils";
import { toast } from "sonner";
import type { RegisterBrowserbaseConfig } from "@/types";

type BrowserbaseFormMode = "create" | "edit";

type BrowserbaseFormState = {
  mode: BrowserbaseFormMode;
  configId: number | null;
  name: string;
  enabled: boolean;
  priority: string;
  ddgToken: string;
  browserbaseApiKey: string;
  mailInboxUrl: string;
  browserbaseApiBase: string;
  browserTimezone: string;
  oauthClientId: string;
  oauthRedirectPort: string;
  agentModel: string;
  maxWaitSeconds: string;
};

const EMPTY_FORM: BrowserbaseFormState = {
  mode: "create",
  configId: null,
  name: "",
  enabled: true,
  priority: "0",
  ddgToken: "",
  browserbaseApiKey: "",
  mailInboxUrl: "",
  browserbaseApiBase: "https://gemini.browserbase.com",
  browserTimezone: "Asia/Shanghai",
  oauthClientId: "",
  oauthRedirectPort: "8787",
  agentModel: "google/gemini-2.5-computer-use-preview-10-2025",
  maxWaitSeconds: "900",
};

function formatTimestamp(value: string) {
  return formatApiDateTime(value, { emptyLabel: "未使用", withSeconds: false });
}

function readConfigString(config: Record<string, unknown>, key: string) {
  const value = config[key];
  if (typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value)) return String(value);
  return "";
}

function createFormState(
  mode: BrowserbaseFormMode,
  config?: RegisterBrowserbaseConfig,
): BrowserbaseFormState {
  const source = config?.config || {};
  return {
    mode,
    configId: config?.id ?? null,
    name: config?.name || "",
    enabled: config?.enabled ?? true,
    priority: String(config?.priority ?? 0),
    ddgToken: readConfigString(source, "ddg_token"),
    browserbaseApiKey: readConfigString(source, "browserbase_api_key"),
    mailInboxUrl: readConfigString(source, "mail_inbox_url"),
    browserbaseApiBase:
      readConfigString(source, "browserbase_api_base") || EMPTY_FORM.browserbaseApiBase,
    browserTimezone:
      readConfigString(source, "browser_timezone") || EMPTY_FORM.browserTimezone,
    oauthClientId: readConfigString(source, "oauth_client_id"),
    oauthRedirectPort:
      readConfigString(source, "oauth_redirect_port") || EMPTY_FORM.oauthRedirectPort,
    agentModel: readConfigString(source, "agent_model") || EMPTY_FORM.agentModel,
    maxWaitSeconds:
      readConfigString(source, "max_wait_seconds") || EMPTY_FORM.maxWaitSeconds,
  };
}

function buildBrowserbaseConfigPayload(form: BrowserbaseFormState): Record<string, unknown> {
  const payload: Record<string, unknown> = {
    ddg_token: form.ddgToken.trim(),
    browserbase_api_key: form.browserbaseApiKey.trim(),
    mail_inbox_url: form.mailInboxUrl.trim(),
    browserbase_api_base: form.browserbaseApiBase.trim(),
    browser_timezone: form.browserTimezone.trim(),
    oauth_client_id: form.oauthClientId.trim(),
    agent_model: form.agentModel.trim(),
  };

  const redirectPort = Number(form.oauthRedirectPort.trim());
  if (form.oauthRedirectPort.trim()) {
    payload.oauth_redirect_port = Number.isFinite(redirectPort)
      ? Math.max(1, Math.trunc(redirectPort))
      : form.oauthRedirectPort.trim();
  }

  const maxWaitSeconds = Number(form.maxWaitSeconds.trim());
  if (form.maxWaitSeconds.trim()) {
    payload.max_wait_seconds = Number.isFinite(maxWaitSeconds)
      ? Math.max(1, Math.trunc(maxWaitSeconds))
      : form.maxWaitSeconds.trim();
  }

  return payload;
}

function hasConfigFlag(config: Record<string, unknown>, key: string) {
  return config[key] === true;
}

function summarizeBrowserbaseConfig(config: RegisterBrowserbaseConfig) {
  const source = config.config || {};
  const ddgReady =
    hasConfigFlag(source, "has_ddg_token") || Boolean(readConfigString(source, "ddg_token").trim());
  const browserbaseReady =
    hasConfigFlag(source, "has_browserbase_api_key") ||
    Boolean(readConfigString(source, "browserbase_api_key").trim());
  const inboxReady =
    hasConfigFlag(source, "has_mail_inbox_url") ||
    Boolean(readConfigString(source, "mail_inbox_url").trim());
  const timezone = readConfigString(source, "browser_timezone") || "默认";
  const agentModel = readConfigString(source, "agent_model") || "默认";
  return [
    `DDG Token: ${ddgReady ? "已配置" : "未配置"}`,
    `Browserbase Key: ${browserbaseReady ? "已配置" : "未配置"}`,
    `收件箱: ${inboxReady ? "已配置" : "未配置"}`,
    `时区: ${timezone}`,
    `模型: ${agentModel}`,
  ].join(" · ");
}

export function BrowserbaseConfigCard() {
  const [formOpen, setFormOpen] = useState(false);
  const [formState, setFormState] = useState<BrowserbaseFormState>(EMPTY_FORM);
  const [isOpeningEdit, setIsOpeningEdit] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<RegisterBrowserbaseConfig | null>(null);

  const {
    configs,
    total,
    isLoading,
    refetchConfigs,
    createBrowserbaseConfig,
    updateBrowserbaseConfig,
    deleteBrowserbaseConfig,
    readBrowserbaseConfigFull,
    isCreating,
    isUpdating,
    isDeleting,
    isReadingFull,
  } = useRegisterBrowserbaseConfigs();

  const isSubmitting = isCreating || isUpdating || isReadingFull || isOpeningEdit;

  const openCreateDialog = () => {
    setFormState(EMPTY_FORM);
    setFormOpen(true);
  };

  const openEditDialog = async (configId: number) => {
    setIsOpeningEdit(true);
    try {
      const fullConfig = await readBrowserbaseConfigFull(configId);
      setFormState(createFormState("edit", fullConfig));
      setFormOpen(true);
    } catch {
      // hook 已统一 toast
    } finally {
      setIsOpeningEdit(false);
    }
  };

  const handleSubmit = async () => {
    const name = formState.name.trim();
    if (!name) {
      toast.error("请输入配置名称");
      return;
    }
    if (!formState.ddgToken.trim()) {
      toast.error("请填写 DDG Token");
      return;
    }
    if (!formState.browserbaseApiKey.trim()) {
      toast.error("请填写 Browserbase API Key");
      return;
    }
    if (!formState.mailInboxUrl.trim()) {
      toast.error("请填写收件箱地址");
      return;
    }
    if (!formState.oauthClientId.trim()) {
      toast.error("请填写 OAuth Client ID");
      return;
    }

    const parsedPriority = Number(formState.priority || 0);
    const priority = Number.isFinite(parsedPriority) ? Math.max(0, Math.trunc(parsedPriority)) : 0;
    const config = buildBrowserbaseConfigPayload(formState);

    try {
      if (formState.mode === "create") {
        await createBrowserbaseConfig({
          name,
          enabled: formState.enabled,
          priority,
          config,
        });
      } else if (formState.configId) {
        await updateBrowserbaseConfig({
          configId: formState.configId,
          name,
          enabled: formState.enabled,
          priority,
          config,
        });
      }
      setFormOpen(false);
      setFormState(EMPTY_FORM);
    } catch {
      // hook 已统一 toast
    }
  };

  const handleToggleEnabled = async (config: RegisterBrowserbaseConfig) => {
    try {
      const fullConfig = await readBrowserbaseConfigFull(config.id);
      await updateBrowserbaseConfig({
        configId: config.id,
        name: fullConfig.name,
        enabled: !config.enabled,
        priority: fullConfig.priority,
        config: fullConfig.config,
      });
    } catch {
      // hook 已统一 toast
    }
  };

  const handleDeleteConfirm = async () => {
    if (!deleteTarget) return;
    try {
      await deleteBrowserbaseConfig(deleteTarget.id);
      setDeleteTarget(null);
    } catch {
      // hook 已统一 toast
    }
  };

  return (
    <>
      <Card className="glass-card overflow-hidden border-none py-0 shadow-xl">
        <CardHeader className="border-b border-border/60">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <CardTitle>Browserbase-DDG 注册配置</CardTitle>
              <CardDescription>
                管理基于 Browserbase + DDG alias 的独立注册模式。这里的配置会直接出现在注册弹窗的“Browserbase-DDG”通道里。
              </CardDescription>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="outline">总数 {total}</Badge>
              <Badge variant="outline">启用 {configs.filter((item) => item.enabled).length}</Badge>
              <Button
                variant="outline"
                className="h-9 rounded-xl"
                onClick={() => void refetchConfigs()}
              >
                <RefreshCw className={cn("h-4 w-4", isLoading && "animate-spin")} />
                刷新
              </Button>
              <Button className="h-9 rounded-xl" onClick={openCreateDialog}>
                <Plus className="h-4 w-4" />
                新建配置
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[84px]">ID</TableHead>
                <TableHead className="min-w-[180px]">名称</TableHead>
                <TableHead className="w-[90px]">状态</TableHead>
                <TableHead className="w-[90px]">优先级</TableHead>
                <TableHead className="min-w-[300px]">配置概览</TableHead>
                <TableHead className="w-[160px]">最近使用</TableHead>
                <TableHead className="w-[72px] text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                Array.from({ length: 3 }).map((_, index) => (
                  <TableRow key={`browserbase-loading-${index}`}>
                    <TableCell colSpan={7}>
                      <Skeleton className="h-9 w-full" />
                    </TableCell>
                  </TableRow>
                ))
              ) : configs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="py-12 text-center text-muted-foreground">
                    还没有 Browserbase-DDG 配置
                  </TableCell>
                </TableRow>
              ) : (
                configs.map((config) => (
                  <TableRow key={config.id} className="border-border/60">
                    <TableCell className="font-mono text-xs text-muted-foreground">
                      #{config.id}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col gap-1">
                        <span className="font-medium">{config.name}</span>
                        <span className="text-xs text-muted-foreground">
                          更新于 {formatTimestamp(config.updatedAt)}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant={config.enabled ? "default" : "secondary"}>
                        {config.enabled ? "已启用" : "已禁用"}
                      </Badge>
                    </TableCell>
                    <TableCell>{config.priority}</TableCell>
                    <TableCell className="whitespace-normal text-xs text-muted-foreground">
                      {summarizeBrowserbaseConfig(config)}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatTimestamp(config.lastUsed)}
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            render={<span />}
                            nativeButton={false}
                          >
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-44">
                          <DropdownMenuItem onClick={() => void openEditDialog(config.id)}>
                            <Wrench className="mr-2 h-4 w-4" />
                            编辑
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => void handleToggleEnabled(config)}>
                            {config.enabled ? (
                              <XCircle className="mr-2 h-4 w-4" />
                            ) : (
                              <CheckCircle2 className="mr-2 h-4 w-4" />
                            )}
                            {config.enabled ? "禁用" : "启用"}
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => setDeleteTarget(config)}>
                            <Trash2 className="mr-2 h-4 w-4" />
                            删除
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog
        open={formOpen}
        onOpenChange={(open) => {
          setFormOpen(open);
          if (!open) {
            setFormState(EMPTY_FORM);
          }
        }}
      >
        <DialogContent className="glass-card border-none p-4 sm:max-w-[720px] sm:p-6">
          <DialogHeader>
            <DialogTitle>
              {formState.mode === "create" ? "新建 Browserbase 配置" : "编辑 Browserbase 配置"}
            </DialogTitle>
            <DialogDescription>
              DDG Token、收件箱地址、OAuth 参数与模型都会保存在这里，并在注册弹窗里按配置选择使用。
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>配置名称</Label>
              <Input
                value={formState.name}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, name: event.target.value }))
                }
                placeholder="例如：主力 Browserbase 节点"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>优先级</Label>
              <Input
                type="number"
                min="0"
                value={formState.priority}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, priority: event.target.value }))
                }
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>DDG Token</Label>
              <Input
                type="password"
                value={formState.ddgToken}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, ddgToken: event.target.value }))
                }
                placeholder="ddg_xxx"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>Browserbase API Key</Label>
              <Input
                type="password"
                value={formState.browserbaseApiKey}
                onChange={(event) =>
                  setFormState((current) => ({
                    ...current,
                    browserbaseApiKey: event.target.value,
                  }))
                }
                placeholder="bb_xxx"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>收件箱地址</Label>
              <Input
                value={formState.mailInboxUrl}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, mailInboxUrl: event.target.value }))
                }
                placeholder="https://example.com/inbox"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>Browserbase API Base</Label>
              <Input
                value={formState.browserbaseApiBase}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, browserbaseApiBase: event.target.value }))
                }
                placeholder="https://gemini.browserbase.com"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>浏览器时区</Label>
              <Input
                value={formState.browserTimezone}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, browserTimezone: event.target.value }))
                }
                placeholder="Asia/Shanghai"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>OAuth Client ID</Label>
              <Input
                value={formState.oauthClientId}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, oauthClientId: event.target.value }))
                }
                placeholder="client_xxx"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>OAuth 回调端口</Label>
              <Input
                type="number"
                min="1"
                value={formState.oauthRedirectPort}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, oauthRedirectPort: event.target.value }))
                }
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>Agent 模型</Label>
              <Input
                value={formState.agentModel}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, agentModel: event.target.value }))
                }
                placeholder="google/gemini-2.5-computer-use-preview-10-2025"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>最大等待秒数</Label>
              <Input
                type="number"
                min="1"
                value={formState.maxWaitSeconds}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, maxWaitSeconds: event.target.value }))
                }
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2 md:col-span-2">
              <Label>启用状态</Label>
              <div className="flex min-h-10 items-center justify-between gap-3 rounded-xl border border-border/60 px-3 py-2">
                <span className="text-sm text-muted-foreground">创建后立即参与 Browserbase-DDG 调度</span>
                <Switch
                  checked={formState.enabled}
                  onCheckedChange={(checked) =>
                    setFormState((current) => ({ ...current, enabled: checked }))
                  }
                />
              </div>
            </div>
          </div>

          <DialogFooter className="gap-2 sm:gap-2">
            <Button variant="outline" onClick={() => setFormOpen(false)}>
              取消
            </Button>
            <Button
              disabled={isSubmitting || !formState.name.trim()}
              onClick={() => void handleSubmit()}
            >
              {isSubmitting ? "提交中..." : formState.mode === "create" ? "创建" : "保存"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteTarget(null);
          }
        }}
        title="删除 Browserbase 配置"
        description={
          deleteTarget
            ? `确认删除“${deleteTarget.name}”吗？删除后该配置将不能再用于 Browserbase-DDG 注册。`
            : ""
        }
        confirmText={isDeleting ? "删除中..." : "确认删除"}
        confirmVariant="destructive"
        onConfirm={() => void handleDeleteConfirm()}
      />
    </>
  );
}
