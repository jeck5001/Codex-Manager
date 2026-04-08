"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import {
  Archive,
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
import { getAppErrorMessage } from "@/lib/api/transport";
import { accountClient } from "@/lib/api/account-client";
import type { RegisterHotmailArtifact, RegisterHotmailBatchSnapshot } from "@/types";
import {
  classifyHotmailLogLine,
  getHotmailBatchProgress,
  mergeHotmailBatchArtifacts,
  shouldPollHotmailBatch,
} from "./hotmail-batch-state";

const HOTMAIL_BATCH_STORAGE_KEY = "codexmanager.hotmail.activeBatchId";

function formatBatchStatus(batch: RegisterHotmailBatchSnapshot | null) {
  if (!batch) {
    return {
      label: "未开始",
      className: "border-border bg-muted/40 text-muted-foreground",
    };
  }
  if (batch.cancelled) {
    return {
      label: "已取消",
      className: "border-amber-500/20 bg-amber-500/10 text-amber-600 dark:text-amber-300",
    };
  }
  if (batch.finished) {
    return {
      label: "已完成",
      className: "border-green-500/20 bg-green-500/10 text-green-600 dark:text-green-400",
    };
  }
  return {
    label: "运行中",
    className: "border-blue-500/20 bg-blue-500/10 text-blue-600 dark:text-blue-400",
  };
}

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
  const [count, setCount] = useState("1");
  const [concurrency, setConcurrency] = useState("1");
  const [intervalMin, setIntervalMin] = useState("2");
  const [intervalMax, setIntervalMax] = useState("5");
  const [proxy, setProxy] = useState("");
  const [batchIdInput, setBatchIdInput] = useState("");
  const [trackedBatchId, setTrackedBatchId] = useState("");
  const [artifacts, setArtifacts] = useState<RegisterHotmailArtifact[]>([]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const savedBatchId = window.localStorage.getItem(HOTMAIL_BATCH_STORAGE_KEY) || "";
    if (savedBatchId) {
      setTrackedBatchId(savedBatchId);
      setBatchIdInput(savedBatchId);
    }
  }, []);

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

  useEffect(() => {
    if (!trackedBatchId) {
      setArtifacts([]);
      return;
    }
    const nextArtifacts = mergeHotmailBatchArtifacts(
      batchQuery.data?.artifacts ?? [],
      artifactsQuery.data ?? [],
    );
    setArtifacts((previous) => mergeHotmailBatchArtifacts(previous, nextArtifacts));
  }, [trackedBatchId, batchQuery.data?.artifacts, artifactsQuery.data]);

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
      setArtifacts(batch.artifacts || []);
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

  const handleTrackBatch = () => {
    const nextBatchId = batchIdInput.trim();
    if (!nextBatchId) {
      toast.error("请先输入批次 ID");
      return;
    }
    setTrackedBatchId(nextBatchId);
    setArtifacts([]);
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
  const statusMeta = formatBatchStatus(currentBatch);
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
