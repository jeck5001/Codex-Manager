"use client";

import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Copy,
  Clock3,
  Eye,
  EyeOff,
  MoreVertical,
  Plus,
  RefreshCw,
  Settings2,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { ApiKeyModal } from "@/components/modals/api-key-modal";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
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
import { useApiKeys } from "@/hooks/useApiKeys";
import { accountClient } from "@/lib/api/account-client";
import { useAppStore } from "@/lib/store/useAppStore";
import { formatCompactNumber, formatTsFromSeconds } from "@/lib/utils/usage";

export default function ApiKeysPage() {
  const { serviceStatus } = useAppStore();
  const {
    apiKeys,
    isLoading,
    deleteApiKey,
    toggleApiKeyStatus,
    refreshModels,
    readApiKeySecret,
    renewApiKey,
    isToggling,
    isRefreshingModels,
  } = useApiKeys();
  const [revealedSecrets, setRevealedSecrets] = useState<Record<string, string>>({});
  const [loadingSecretId, setLoadingSecretId] = useState<string | null>(null);
  const [apiKeyModalOpen, setApiKeyModalOpen] = useState(false);
  const [editingKeyId, setEditingKeyId] = useState<string | null>(null);
  const [deleteKeyId, setDeleteKeyId] = useState<string | null>(null);
  const [renewKeyId, setRenewKeyId] = useState<string | null>(null);
  const [renewExpiresAtInput, setRenewExpiresAtInput] = useState("");
  const [nowTs, setNowTs] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    const timer = window.setInterval(() => {
      setNowTs(Math.floor(Date.now() / 1000));
    }, 60_000);
    return () => window.clearInterval(timer);
  }, []);

  const editingApiKey = useMemo(
    () => apiKeys.find((item) => item.id === editingKeyId) || null,
    [apiKeys, editingKeyId]
  );
  const renewApiKeyTarget = useMemo(
    () => apiKeys.find((item) => item.id === renewKeyId) || null,
    [apiKeys, renewKeyId]
  );
  const { data: usageByKey = {} } = useQuery({
    queryKey: ["apikey-usage-stats"],
    queryFn: async () => {
      const stats = await accountClient.listApiKeyUsageStats();
      return stats.reduce<Record<string, number>>((result, item) => {
        const keyId = String(item.keyId || "").trim();
        if (!keyId) return result;
        result[keyId] = Math.max(0, item.totalTokens || 0);
        return result;
      }, {});
    },
    enabled: serviceStatus.connected,
    refetchInterval: 5000,
    retry: 1,
  });

  const openCreateModal = () => {
    setEditingKeyId(null);
    setApiKeyModalOpen(true);
  };

  const openEditModal = (id: string) => {
    setEditingKeyId(id);
    setApiKeyModalOpen(true);
  };

  const ensureSecretLoaded = async (id: string) => {
    if (revealedSecrets[id]) {
      return revealedSecrets[id];
    }
    setLoadingSecretId(id);
    try {
      const secret = await readApiKeySecret(id);
      if (!secret) {
        throw new Error("后端未返回密钥明文");
      }
      setRevealedSecrets((current) => ({ ...current, [id]: secret }));
      return secret;
    } finally {
      setLoadingSecretId(null);
    }
  };

  const toggleSecret = async (id: string) => {
    if (revealedSecrets[id]) {
      setRevealedSecrets((current) => {
        const nextState = { ...current };
        delete nextState[id];
        return nextState;
      });
      return;
    }

    try {
      await ensureSecretLoaded(id);
    } catch (error: unknown) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  };

  const copyToClipboard = async (id: string) => {
    try {
      const secret = await ensureSecretLoaded(id);
      await navigator.clipboard.writeText(secret);
      toast.success("已复制到剪贴板");
    } catch (error: unknown) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  };

  const handleDelete = (id: string) => {
    setDeleteKeyId(id);
  };

  const openRenewDialog = (id: string) => {
    const target = apiKeys.find((item) => item.id === id) || null;
    const fallbackExpiresAt = Math.max(
      Math.floor(Date.now() / 1000) + 3600,
      (target?.expiresAt ?? 0) + 86400
    );
    setRenewKeyId(id);
    setRenewExpiresAtInput(toDateTimeLocalValue(fallbackExpiresAt));
  };

  const submitRenew = async () => {
    if (!renewKeyId) return;
    const expiresAt = parseDateTimeLocalValue(renewExpiresAtInput);
    if (!expiresAt) {
      toast.error("请选择新的过期时间");
      return;
    }
    await renewApiKey({ id: renewKeyId, expiresAt });
    setRenewKeyId(null);
  };

  return (
    <div className="space-y-6 animate-in fade-in duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold tracking-tight">平台密钥</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            创建和管理网关调用所需的访问令牌
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            className="glass-card h-10 gap-2"
            onClick={() => refreshModels(true)}
            disabled={isRefreshingModels}
          >
            <RefreshCw className={isRefreshingModels ? "h-4 w-4 animate-spin" : "h-4 w-4"} />
            刷新模型
          </Button>
          <Button className="h-10 gap-2 shadow-lg shadow-primary/20" onClick={openCreateModal}>
            <Plus className="h-4 w-4" /> 创建密钥
          </Button>
        </div>
      </div>

      <Card className="glass-card overflow-hidden border-none py-0 shadow-xl backdrop-blur-md">
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>密钥 / ID</TableHead>
                <TableHead>名称</TableHead>
                <TableHead>协议</TableHead>
                <TableHead>绑定模型</TableHead>
                <TableHead>总使用 Token</TableHead>
                <TableHead>过期时间</TableHead>
                <TableHead>状态</TableHead>
                <TableHead className="text-center">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                Array.from({ length: 3 }).map((_, index) => (
                  <TableRow key={index}>
                    <TableCell><Skeleton className="h-4 w-32" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-24" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-20" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-28" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-20" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-28" /></TableCell>
                    <TableCell><Skeleton className="h-6 w-16 rounded-full" /></TableCell>
                    <TableCell className="text-center"><Skeleton className="mx-auto h-8 w-8" /></TableCell>
                  </TableRow>
                ))
              ) : apiKeys.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={8} className="h-48 text-center">
                    <div className="flex flex-col items-center justify-center gap-2 text-muted-foreground">
                      <Plus className="h-8 w-8 opacity-20" />
                      <p>暂无平台密钥，点击右上角创建</p>
                    </div>
                  </TableCell>
                </TableRow>
              ) : (
                apiKeys.map((key) => {
                  const revealed = revealedSecrets[key.id];
                  const normalizedStatus = String(key.status || "").toLowerCase();
                  const isEnabled = normalizedStatus === "active";
                  const countdown = formatExpirationCountdown(key.expiresAt, nowTs);
                  const statusLabel = apiKeyStatusLabel(normalizedStatus);

                  return (
                    <TableRow key={key.id} className="group">
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <code className="rounded border border-primary/5 bg-muted/50 px-2 py-1 font-mono text-[10px] text-primary">
                            {revealed
                              ? revealed
                              : loadingSecretId === key.id
                                ? "读取中..."
                                : `${key.id.slice(0, 8)}...`}
                          </code>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7 text-muted-foreground hover:text-primary"
                            onClick={() => void toggleSecret(key.id)}
                          >
                            {revealed ? (
                              <EyeOff className="h-3.5 w-3.5" />
                            ) : (
                              <Eye className="h-3.5 w-3.5" />
                            )}
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7 text-muted-foreground hover:text-primary"
                            onClick={() => void copyToClipboard(key.id)}
                          >
                            <Copy className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      </TableCell>
                      <TableCell className="text-sm font-semibold">{key.name || "未命名"}</TableCell>
                      <TableCell>
                        <Badge variant="outline" className="bg-accent/20 text-[10px] font-normal capitalize">
                          {key.protocol.replace(/_/g, " ")}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-xs font-medium text-muted-foreground">
                        {key.model ? (
                          key.model
                        ) : (
                          <span title="跟随请求表示使用请求体里的实际 model；请求日志展示的是最终生效模型。">
                            跟随请求
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {formatCompactNumber(usageByKey[key.id] ?? 0, "0")}
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground">
                        <div className="space-y-1">
                          <div>{key.expiresAt ? formatTsFromSeconds(key.expiresAt, "永不过期") : "永不过期"}</div>
                          {key.expiresAt ? (
                            <div className="flex items-center gap-1 text-[11px]">
                              <Clock3 className="h-3 w-3" />
                              <span>{countdown}</span>
                            </div>
                          ) : null}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Switch
                            className="scale-75"
                            checked={isEnabled}
                            disabled={isToggling || normalizedStatus === "expired"}
                            onCheckedChange={(enabled) =>
                              toggleApiKeyStatus({ id: key.id, enabled })
                            }
                          />
                          <span className="text-[10px] font-medium text-muted-foreground">
                            {statusLabel}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="table-action-cell gap-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-muted-foreground transition-colors hover:text-primary"
                            onClick={() => openEditModal(key.id)}
                            title="编辑配置"
                          >
                            <Settings2 className="h-4 w-4" />
                          </Button>
                          <DropdownMenu>
                            <DropdownMenuTrigger>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-8 w-8"
                                render={<span />}
                                nativeButton={false}
                              >
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuItem className="gap-2" onClick={() => openEditModal(key.id)}>
                                设置模型与推理
                              </DropdownMenuItem>
                              <DropdownMenuItem className="gap-2" onClick={() => openRenewDialog(key.id)}>
                                <Clock3 className="h-4 w-4" /> 续期
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                className="gap-2 text-red-500"
                                onClick={() => handleDelete(key.id)}
                              >
                                <Trash2 className="h-4 w-4" /> 删除密钥
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <ApiKeyModal
        open={apiKeyModalOpen}
        onOpenChange={setApiKeyModalOpen}
        apiKey={editingApiKey}
      />
      <ConfirmDialog
        open={Boolean(deleteKeyId)}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteKeyId(null);
          }
        }}
        title="删除平台密钥"
        description={`确定删除平台密钥 ${apiKeys.find((item) => item.id === deleteKeyId)?.name || ""} 吗？删除后不可恢复。`}
        confirmText="删除"
        confirmVariant="destructive"
        onConfirm={() => {
          if (!deleteKeyId) return;
          deleteApiKey(deleteKeyId);
        }}
      />
      <Dialog
        open={Boolean(renewKeyId)}
        onOpenChange={(open) => {
          if (!open) {
            setRenewKeyId(null);
            setRenewExpiresAtInput("");
          }
        }}
      >
        <DialogContent className="glass-card border-none sm:max-w-md">
          <DialogHeader>
            <DialogTitle>续期平台密钥</DialogTitle>
            <DialogDescription>
              为 {renewApiKeyTarget?.name || renewApiKeyTarget?.id || "当前密钥"} 设置新的过期时间。
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-2 py-2">
            <Label htmlFor="renew-expires-at">新的过期时间</Label>
            <Input
              id="renew-expires-at"
              type="datetime-local"
              value={renewExpiresAtInput}
              onChange={(event) => setRenewExpiresAtInput(event.target.value)}
            />
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setRenewKeyId(null)}>
              取消
            </Button>
            <Button onClick={() => void submitRenew()}>
              保存续期
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function parseDateTimeLocalValue(value: string): number | null {
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) {
    return null;
  }
  return Math.floor(timestamp / 1000);
}

function toDateTimeLocalValue(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return "";
  }
  const offset = date.getTimezoneOffset();
  const localDate = new Date(date.getTime() - offset * 60_000);
  return localDate.toISOString().slice(0, 16);
}

function formatExpirationCountdown(expiresAt: number | null, nowTs: number): string {
  if (!expiresAt) {
    return "永不过期";
  }
  const diff = expiresAt - nowTs;
  if (diff <= 0) {
    return "已过期";
  }
  const days = Math.floor(diff / 86_400);
  const hours = Math.floor((diff % 86_400) / 3_600);
  const minutes = Math.floor((diff % 3_600) / 60);
  if (days > 0) {
    return `${days} 天 ${hours} 小时后过期`;
  }
  if (hours > 0) {
    return `${hours} 小时 ${minutes} 分钟后过期`;
  }
  return `${Math.max(1, minutes)} 分钟后过期`;
}

function apiKeyStatusLabel(status: string): string {
  switch (status) {
    case "active":
      return "启用";
    case "expired":
      return "已过期";
    case "disabled":
      return "禁用";
    default:
      return status || "未知";
  }
}
