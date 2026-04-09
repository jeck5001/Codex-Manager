"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  Archive,
  ExternalLink,
  Laptop,
  LoaderCircle,
  Mail,
  PlayCircle,
  RefreshCw,
  Square,
} from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { getAppErrorMessage, isTauriRuntime } from "@/lib/api/transport";
import { appClient } from "@/lib/api/app-client";
import { accountClient } from "@/lib/api/account-client";
import { hotmailLocalHelperClient } from "@/lib/api/hotmail-local-helper";
import {
  buildHotmailLocalHandoffActionState,
  buildHotmailWebLocalHandoffActionState,
  buildHotmailHandoffAccessUrl,
  buildHotmailNativeVncEndpoint,
  classifyHotmailLogLine,
  formatHotmailBatchStatus,
  getHotmailBatchProgress,
  hasHotmailPendingHandoff,
  hasHotmailPendingLocalHandoff,
  mergeHotmailBatchArtifacts,
  shouldPollHotmailBatch,
} from "./hotmail-batch-state";

const HOTMAIL_BATCH_STORAGE_KEY = "codexmanager.hotmail.activeBatchId";

function formatArtifactSize(size: number | null) {
  if (!Number.isFinite(size) || size == null || size < 0) {
    return "--";
  }
  if (size < 1024) {
    return `${size} B`;
  }
  return `${(size / 1024).toFixed(1)} KB`;
}

export default function HotmailPage() {
  const isDesktopRuntime = isTauriRuntime();
  const initialBatchId =
    typeof window === "undefined"
      ? ""
      : window.localStorage.getItem(HOTMAIL_BATCH_STORAGE_KEY) || "";
  const [count, setCount] = useState("1");
  const [concurrency, setConcurrency] = useState("1");
  const [intervalMin, setIntervalMin] = useState("2");
  const [intervalMax, setIntervalMax] = useState("5");
  const [proxy, setProxy] = useState("");
  const [batchIdInput, setBatchIdInput] = useState(initialBatchId);
  const [trackedBatchId, setTrackedBatchId] = useState(initialBatchId);
  const [showWebHelperGuide, setShowWebHelperGuide] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    if (trackedBatchId) {
      window.localStorage.setItem(HOTMAIL_BATCH_STORAGE_KEY, trackedBatchId);
      return;
    }
    window.localStorage.removeItem(HOTMAIL_BATCH_STORAGE_KEY);
  }, [trackedBatchId]);

  const batchQuery = useQuery({
    queryKey: ["hotmail-batch", trackedBatchId],
    queryFn: () => accountClient.getRegisterHotmailBatch(trackedBatchId),
    enabled: Boolean(trackedBatchId),
    retry: 1,
    refetchInterval: (query) =>
      shouldPollHotmailBatch(query.state.data ?? null) ? 3000 : false,
    refetchIntervalInBackground: true,
  });

  const artifactsQuery = useQuery({
    queryKey: ["hotmail-batch-artifacts", trackedBatchId],
    queryFn: () => accountClient.getRegisterHotmailBatchArtifacts(trackedBatchId),
    enabled: Boolean(trackedBatchId),
    retry: 1,
    refetchInterval: shouldPollHotmailBatch(batchQuery.data ?? null) ? 3000 : false,
    refetchIntervalInBackground: true,
  });

  const createMutation = useMutation({
    mutationFn: () =>
      accountClient.startRegisterHotmailBatch({
        count: Math.max(1, Number(count) || 1),
        concurrency: Math.max(1, Number(concurrency) || 1),
        intervalMin: Math.max(0, Number(intervalMin) || 0),
        intervalMax: Math.max(0, Number(intervalMax) || 0),
        proxy: proxy.trim() || null,
      }),
    onSuccess: (batch) => {
      setTrackedBatchId(batch.batchId);
      setBatchIdInput(batch.batchId);
      toast.success(`Hotmail 批次已启动: ${batch.batchId}`);
    },
    onError: (error: unknown) => {
      toast.error(`启动失败: ${getAppErrorMessage(error)}`);
    },
  });

  const cancelMutation = useMutation({
    mutationFn: () => accountClient.cancelRegisterHotmailBatch(trackedBatchId),
    onSuccess: async () => {
      toast.success("已提交取消请求");
      await Promise.all([batchQuery.refetch(), artifactsQuery.refetch()]);
    },
    onError: (error: unknown) => {
      toast.error(`取消失败: ${getAppErrorMessage(error)}`);
    },
  });

  const continueMutation = useMutation({
    mutationFn: () => accountClient.continueRegisterHotmailBatch(trackedBatchId),
    onSuccess: async (batch) => {
      toast.success(
        batch.status === "action_required" ? "会话仍在等待人工验证" : "已继续处理 Hotmail 注册",
      );
      await Promise.all([batchQuery.refetch(), artifactsQuery.refetch()]);
    },
    onError: (error: unknown) => {
      toast.error(`继续失败: ${getAppErrorMessage(error)}`);
    },
  });

  const abandonMutation = useMutation({
    mutationFn: () => accountClient.abandonRegisterHotmailBatch(trackedBatchId),
    onSuccess: async () => {
      toast.success("已放弃当前待接管的 Hotmail 尝试");
      await Promise.all([batchQuery.refetch(), artifactsQuery.refetch()]);
    },
    onError: (error: unknown) => {
      toast.error(`放弃失败: ${getAppErrorMessage(error)}`);
    },
  });

  const localHandoffMutation = useMutation({
    mutationFn: async () => {
      const payload = batchQuery.data?.localHandoff;
      if (!payload) {
        throw new Error("当前批次没有可用的本地接管数据");
      }
      return appClient.openHotmailLocalHandoff(payload);
    },
    onSuccess: () => {
      toast.success("已启动本地接管浏览器，请在本机窗口里处理微软验证");
    },
    onError: (error: unknown) => {
      toast.error(`本地接管启动失败: ${getAppErrorMessage(error)}`);
    },
  });

  const webLocalHandoffMutation = useMutation({
    mutationFn: async () => {
      const payload = batchQuery.data?.localHandoff;
      if (!payload) {
        throw new Error("当前批次没有可用的本机接管数据");
      }
      const health = await hotmailLocalHelperClient.health();
      if (!health.ok) {
        throw new Error("Hotmail helper 不可用");
      }
      if (!health.playwrightReady) {
        throw new Error("Hotmail helper 缺少 Playwright Chromium，请先执行 playwright install chromium");
      }
      return hotmailLocalHelperClient.openHandoff(payload);
    },
    onSuccess: () => {
      setShowWebHelperGuide(false);
      toast.success("已在本机启动接管浏览器，请处理微软验证后回到这里继续注册");
    },
    onError: (error: unknown) => {
      setShowWebHelperGuide(true);
      toast.error(`本机接管启动失败: ${getAppErrorMessage(error)}`);
    },
  });

  const handleTrackBatch = () => {
    const nextBatchId = batchIdInput.trim();
    if (!nextBatchId) {
      toast.error("请先输入批次 ID");
      return;
    }
    setTrackedBatchId(nextBatchId);
  };

  const handleRefresh = async () => {
    if (!trackedBatchId) {
      return;
    }
    try {
      await Promise.all([batchQuery.refetch(), artifactsQuery.refetch()]);
      toast.success("批次状态已刷新");
    } catch (error: unknown) {
      toast.error(`刷新失败: ${getAppErrorMessage(error)}`);
    }
  };

  const currentBatch = batchQuery.data ?? null;
  const artifacts = useMemo(
    () =>
      trackedBatchId
        ? mergeHotmailBatchArtifacts(batchQuery.data?.artifacts ?? [], artifactsQuery.data ?? [])
        : [],
    [trackedBatchId, batchQuery.data?.artifacts, artifactsQuery.data],
  );
  const statusMeta = formatHotmailBatchStatus(currentBatch);
  const hasPendingHandoff = hasHotmailPendingHandoff(currentBatch);
  const hasPendingLocalHandoff = hasHotmailPendingLocalHandoff(currentBatch);
  const localHandoffAction = useMemo(
    () => buildHotmailLocalHandoffActionState(currentBatch, isDesktopRuntime),
    [currentBatch, isDesktopRuntime],
  );
  const webLocalHandoffAction = useMemo(
    () => buildHotmailWebLocalHandoffActionState(currentBatch, isDesktopRuntime),
    [currentBatch, isDesktopRuntime],
  );
  const handoffAccessUrl = useMemo(
    () =>
      typeof window === "undefined"
        ? ""
        : buildHotmailHandoffAccessUrl(currentBatch, window.location.href),
    [currentBatch],
  );
  const nativeVncEndpoint = useMemo(
    () =>
      typeof window === "undefined"
        ? ""
        : buildHotmailNativeVncEndpoint(currentBatch, window.location.href),
    [currentBatch],
  );
  const progress = useMemo(
    () =>
      currentBatch
        ? getHotmailBatchProgress({
            total: currentBatch.total,
            completed: currentBatch.completed,
          })
        : "0%",
    [currentBatch],
  );
  const logs = currentBatch?.logs || [];

  return (
    <div className="space-y-6 p-6">
      <Card className="glass-card border-border/60">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mail className="h-5 w-5" />
            Hotmail 自动注册
          </CardTitle>
          <CardDescription>在真实前端里直接发起和跟踪 Hotmail 批量注册。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
            <div className="space-y-2">
              <Label htmlFor="hotmail-count">数量</Label>
              <Input
                id="hotmail-count"
                inputMode="numeric"
                value={count}
                onChange={(event) => setCount(event.target.value.replace(/[^\d]/g, ""))}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="hotmail-concurrency">并发</Label>
              <Input
                id="hotmail-concurrency"
                inputMode="numeric"
                value={concurrency}
                onChange={(event) => setConcurrency(event.target.value.replace(/[^\d]/g, ""))}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="hotmail-interval-min">最小间隔</Label>
              <Input
                id="hotmail-interval-min"
                inputMode="numeric"
                value={intervalMin}
                onChange={(event) => setIntervalMin(event.target.value.replace(/[^\d]/g, ""))}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="hotmail-interval-max">最大间隔</Label>
              <Input
                id="hotmail-interval-max"
                inputMode="numeric"
                value={intervalMax}
                onChange={(event) => setIntervalMax(event.target.value.replace(/[^\d]/g, ""))}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="hotmail-proxy">代理</Label>
              <Input
                id="hotmail-proxy"
                placeholder="http://user:pass@host:port"
                value={proxy}
                onChange={(event) => setProxy(event.target.value)}
              />
            </div>
          </div>

          <div className="flex flex-wrap gap-3">
            <Button
              className="gap-2"
              onClick={() => void createMutation.mutateAsync()}
              disabled={createMutation.isPending}
            >
              {createMutation.isPending ? (
                <LoaderCircle className="h-4 w-4 animate-spin" />
              ) : (
                <PlayCircle className="h-4 w-4" />
              )}
              开始批次
            </Button>
            <Button
              variant="outline"
              className="gap-2"
              onClick={() => void cancelMutation.mutateAsync()}
              disabled={!trackedBatchId || cancelMutation.isPending || !shouldPollHotmailBatch(currentBatch)}
            >
              {cancelMutation.isPending ? (
                <LoaderCircle className="h-4 w-4 animate-spin" />
              ) : (
                <Square className="h-4 w-4" />
              )}
              取消批次
            </Button>
          </div>

          <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto_auto]">
            <div className="space-y-2">
              <Label htmlFor="hotmail-batch-id">批次 ID</Label>
              <Input
                id="hotmail-batch-id"
                placeholder="可粘贴已有批次 ID 继续跟踪"
                value={batchIdInput}
                onChange={(event) => setBatchIdInput(event.target.value)}
              />
            </div>
            <div className="flex items-end">
              <Button variant="secondary" className="w-full" onClick={handleTrackBatch}>
                跟踪已有批次
              </Button>
            </div>
            <div className="flex items-end">
              <Button
                variant="outline"
                className="w-full gap-2"
                onClick={() => void handleRefresh()}
                disabled={!trackedBatchId || batchQuery.isFetching || artifactsQuery.isFetching}
              >
                <RefreshCw className="h-4 w-4" />
                刷新
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
        <Card className="glass-card border-border/60">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Archive className="h-5 w-5" />
              批次状态
            </CardTitle>
            <CardDescription>当前只跟踪一个 Hotmail 批次，可手动切换批次 ID。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {batchQuery.isLoading && trackedBatchId ? (
              <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                {Array.from({ length: 4 }).map((_, index) => (
                  <Skeleton key={index} className="h-20 rounded-xl" />
                ))}
              </div>
            ) : (
              <>
                <div className="flex flex-wrap items-center gap-3">
                  <Badge className={statusMeta.className}>{statusMeta.label}</Badge>
                  <span className="text-sm text-muted-foreground">
                    批次 ID: {trackedBatchId || "--"}
                  </span>
                  <span className="text-sm text-muted-foreground">进度: {progress}</span>
                </div>
                <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                  {[
                    { label: "总数", value: currentBatch?.total ?? 0 },
                    { label: "已完成", value: currentBatch?.completed ?? 0 },
                    { label: "成功", value: currentBatch?.success ?? 0 },
                    { label: "失败", value: currentBatch?.failed ?? 0 },
                  ].map((item) => (
                    <div
                      key={item.label}
                      className="rounded-2xl border border-border/60 bg-background/60 p-4"
                    >
                      <div className="text-sm text-muted-foreground">{item.label}</div>
                      <div className="mt-2 text-2xl font-semibold">{item.value}</div>
                    </div>
                  ))}
                </div>
                {hasPendingHandoff ? (
                  <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-900 dark:text-amber-100">
                    <div className="flex items-center gap-2 font-medium">
                      <AlertTriangle className="h-4 w-4" />
                      微软要求人工验证，当前批次已暂停
                    </div>
                    <div className="mt-3 space-y-2 text-sm leading-6">
                      <p>
                        {localHandoffAction.enabled
                          ? "优先点击下面的“本地接管（推荐）”，在你本机启动专用浏览器窗口处理微软验证；处理完成后，回到这里点击“继续注册”。"
                          : webLocalHandoffAction.enabled
                            ? "优先点击下面的“本机接管（Web）”，当前浏览器会调用你这台机器上的本地 helper 启动专用窗口处理微软验证；处理完成后，回到这里点击“继续注册”。"
                            : "优先用原生 VNC 客户端连接运行 register 服务的那台主机，处理当前 Playwright 停留的微软验证页；处理完成后，回到这里点击“继续注册”。"}
                      </p>
                      <p>
                        微软这个长按按钮对远程输入很敏感，浏览器里的 noVNC 容易因为抖动或延迟反复
                        提示重试。原生 VNC 客户端通常比 noVNC 稳定很多。
                      </p>
                      {!isDesktopRuntime && webLocalHandoffAction.enabled ? (
                        <p>
                          Web 模式下需要先在当前访问页面的这台机器上启动 Hotmail 本机接管 helper，
                          默认地址是 <span className="font-mono text-xs">http://127.0.0.1:16788</span>。
                        </p>
                      ) : null}
                      {!localHandoffAction.enabled && isDesktopRuntime && hasPendingLocalHandoff ? (
                        <p>{localHandoffAction.reason}</p>
                      ) : null}
                      {!webLocalHandoffAction.enabled && !isDesktopRuntime && hasPendingLocalHandoff ? (
                        <p>{webLocalHandoffAction.reason}</p>
                      ) : null}
                      {currentBatch?.handoffInstructions ? (
                        <p>{currentBatch.handoffInstructions}</p>
                      ) : null}
                      {currentBatch?.handoffTitle ? (
                        <p className="font-mono text-xs text-amber-800/80 dark:text-amber-200/80">
                          当前页面: {currentBatch.handoffTitle}
                        </p>
                      ) : null}
                      {handoffAccessUrl ? (
                        <p className="break-all font-mono text-xs text-amber-800/80 dark:text-amber-200/80">
                          noVNC 地址: {handoffAccessUrl}
                        </p>
                      ) : null}
                      {nativeVncEndpoint ? (
                        <p className="break-all font-mono text-xs text-amber-800/80 dark:text-amber-200/80">
                          原生 VNC 地址: {nativeVncEndpoint}
                        </p>
                      ) : null}
                    </div>
                    <div className="mt-4 flex flex-wrap gap-3">
                      {isDesktopRuntime ? (
                        <Button
                          className="gap-2"
                          onClick={() => void localHandoffMutation.mutateAsync()}
                          disabled={!localHandoffAction.enabled || localHandoffMutation.isPending}
                        >
                          {localHandoffMutation.isPending ? (
                            <LoaderCircle className="h-4 w-4 animate-spin" />
                          ) : (
                            <Laptop className="h-4 w-4" />
                          )}
                          本地接管（推荐）
                        </Button>
                      ) : hasPendingLocalHandoff ? (
                        <Button
                          className="gap-2"
                          onClick={() => void webLocalHandoffMutation.mutateAsync()}
                          disabled={!webLocalHandoffAction.enabled || webLocalHandoffMutation.isPending}
                        >
                          {webLocalHandoffMutation.isPending ? (
                            <LoaderCircle className="h-4 w-4 animate-spin" />
                          ) : (
                            <Laptop className="h-4 w-4" />
                          )}
                          本机接管（Web）
                        </Button>
                      ) : null}
                      <Button
                        variant="secondary"
                        className="gap-2"
                        onClick={() => window.open(handoffAccessUrl, "_blank", "noopener,noreferrer")}
                        disabled={!handoffAccessUrl}
                      >
                        <ExternalLink className="h-4 w-4" />
                        打开接管页面
                      </Button>
                      <Button
                        className="gap-2"
                        onClick={() => void continueMutation.mutateAsync()}
                        disabled={continueMutation.isPending || !hasPendingHandoff}
                      >
                        {continueMutation.isPending ? (
                          <LoaderCircle className="h-4 w-4 animate-spin" />
                        ) : (
                          <PlayCircle className="h-4 w-4" />
                        )}
                        我已处理，继续注册
                      </Button>
                      <Button
                        variant="outline"
                        className="gap-2"
                        onClick={() => void abandonMutation.mutateAsync()}
                        disabled={abandonMutation.isPending || !hasPendingHandoff}
                      >
                        {abandonMutation.isPending ? (
                          <LoaderCircle className="h-4 w-4 animate-spin" />
                        ) : (
                          <Square className="h-4 w-4" />
                        )}
                        放弃本次
                      </Button>
                    </div>
                    {showWebHelperGuide && !isDesktopRuntime ? (
                      <div className="mt-4 rounded-xl border border-border/60 bg-background/70 p-4 text-sm text-muted-foreground">
                        <p className="font-medium text-foreground">
                          请先在当前访问页面的这台机器上启动 Hotmail 本机接管 helper。
                        </p>
                        <p className="mt-2">
                          健康检查地址：
                          <span className="ml-2 font-mono text-xs">http://127.0.0.1:16788/health</span>
                        </p>
                        <p className="mt-2">推荐启动步骤：</p>
                        <pre className="mt-2 overflow-x-auto rounded-lg border border-border/60 bg-background/80 p-3 text-xs">
{`cd tools/hotmail_local_helper
python3 -m venv .venv
source .venv/bin/activate
pip install -r ../../vendor/codex-register/requirements.txt
playwright install chromium
cd ../..
python3 -m tools.hotmail_local_helper`}
                        </pre>
                        <p className="mt-2">
                          helper 启动后，再回到这里点击“本机接管（Web）”。
                        </p>
                      </div>
                    ) : null}
                  </div>
                ) : null}
                <div className="rounded-2xl border border-border/60 bg-background/40 p-4">
                  <div className="text-sm font-medium">运行日志</div>
                  {logs.length > 0 ? (
                    <div className="mt-3 max-h-64 space-y-2 overflow-auto text-sm">
                      {logs.map((line, index) => (
                        <div
                          key={`${index}-${line}`}
                          className={
                            classifyHotmailLogLine(line) === "challenge"
                              ? "rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 font-mono text-xs text-amber-700 dark:text-amber-300"
                              : "rounded-lg border border-border/50 bg-background/70 px-3 py-2 font-mono text-xs"
                          }
                        >
                          {line}
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="mt-3 text-sm text-muted-foreground">暂无日志</div>
                  )}
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card className="glass-card border-border/60">
          <CardHeader>
            <CardTitle>产物文件</CardTitle>
            <CardDescription>成功批次会在这里展示导出文件。</CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>文件名</TableHead>
                  <TableHead>路径</TableHead>
                  <TableHead className="text-right">大小</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {artifacts.length > 0 ? (
                  artifacts.map((artifact) => (
                    <TableRow key={`${artifact.path}-${artifact.filename}`}>
                      <TableCell className="font-medium">{artifact.filename || "--"}</TableCell>
                      <TableCell className="max-w-[320px] truncate font-mono text-xs text-muted-foreground">
                        {artifact.path || "--"}
                      </TableCell>
                      <TableCell className="text-right text-muted-foreground">
                        {formatArtifactSize(artifact.size)}
                      </TableCell>
                    </TableRow>
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={3} className="py-8 text-center text-muted-foreground">
                      {trackedBatchId
                        ? "当前批次还没有可展示的产物"
                        : "先启动一个批次，或输入已有批次 ID 继续查看"}
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
