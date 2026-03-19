"use client";

import { useMemo, useState } from "react";
import {
  Activity,
  CalendarDays,
  Eye,
  Plus,
  RefreshCw,
  Trash2,
  XCircle,
} from "lucide-react";
import { AddAccountModal } from "@/components/modals/add-account-modal";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { useRegisterTasks } from "@/hooks/useRegisterTasks";
import { accountClient } from "@/lib/api/account-client";
import { cn } from "@/lib/utils";
import type { RegisterTaskSnapshot } from "@/types";

type PendingAction =
  | { kind: "cancel"; task: RegisterTaskSnapshot }
  | { kind: "delete"; task: RegisterTaskSnapshot }
  | null;

const PAGE_SIZE = 20;

function formatTimestamp(value: string) {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function getStatusMeta(status: string) {
  const normalized = String(status || "").trim().toLowerCase();
  if (normalized === "completed") {
    return { label: "已完成", className: "border-green-500/20 bg-green-500/10 text-green-600 dark:text-green-400" };
  }
  if (normalized === "running") {
    return { label: "运行中", className: "border-blue-500/20 bg-blue-500/10 text-blue-600 dark:text-blue-400" };
  }
  if (normalized === "failed") {
    return { label: "失败", className: "border-red-500/20 bg-red-500/10 text-red-600 dark:text-red-400" };
  }
  if (normalized === "cancelled") {
    return { label: "已取消", className: "border-amber-500/20 bg-amber-500/10 text-amber-600 dark:text-amber-400" };
  }
  if (normalized === "pending") {
    return { label: "排队中", className: "border-slate-500/20 bg-slate-500/10 text-slate-600 dark:text-slate-300" };
  }
  return { label: status || "--", className: "border-border bg-muted/40 text-muted-foreground" };
}

function countByStatuses(source: Record<string, number>, keys: string[]) {
  return keys.reduce((sum, key) => sum + (source[key] || 0), 0);
}

export default function RegisterPage() {
  const [page, setPage] = useState(1);
  const [statusFilter, setStatusFilter] = useState("all");
  const [addModalOpen, setAddModalOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailTask, setDetailTask] = useState<RegisterTaskSnapshot | null>(null);
  const [isLoadingDetail, setIsLoadingDetail] = useState(false);
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);

  const {
    tasks,
    total,
    stats,
    isLoading,
    isStatsLoading,
    refetchTasks,
    cancelTask,
    deleteTask,
    isCancelling,
    isDeleting,
  } = useRegisterTasks({
    page,
    pageSize: PAGE_SIZE,
    status: statusFilter === "all" ? null : statusFilter,
  });

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const byStatus = stats?.byStatus || {};
  const runningCount = countByStatuses(byStatus, ["pending", "running"]);
  const completedCount = byStatus.completed || 0;
  const failedCount = countByStatuses(byStatus, ["failed", "cancelled"]);

  const summaryCards = useMemo(
    () => [
      {
        title: "今日注册",
        value: stats?.todayCount || 0,
        sub: "今天新建的注册任务数",
        icon: CalendarDays,
        tone: "text-blue-500",
      },
      {
        title: "运行中",
        value: runningCount,
        sub: "排队中与运行中的任务",
        icon: Activity,
        tone: "text-amber-500",
      },
      {
        title: "已完成",
        value: completedCount,
        sub: "已成功结束的任务",
        icon: RefreshCw,
        tone: "text-green-500",
      },
      {
        title: "失败/取消",
        value: failedCount,
        sub: "失败或被取消的任务",
        icon: XCircle,
        tone: "text-red-500",
      },
    ],
    [completedCount, failedCount, runningCount, stats?.todayCount],
  );

  const handleOpenDetail = async (taskUuid: string) => {
    setDetailOpen(true);
    setIsLoadingDetail(true);
    try {
      const task = await accountClient.getRegisterTask(taskUuid);
      setDetailTask(task);
    } finally {
      setIsLoadingDetail(false);
    }
  };

  const handleConfirmAction = async () => {
    if (!pendingAction) return;
    if (pendingAction.kind === "cancel") {
      await cancelTask(pendingAction.task.taskUuid);
      return;
    }
    await deleteTask(pendingAction.task.taskUuid);
  };

  return (
    <div className="space-y-6 animate-in fade-in duration-700">
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {isStatsLoading
          ? Array.from({ length: 4 }).map((_, index) => (
              <Skeleton key={index} className="h-32 rounded-2xl" />
            ))
          : summaryCards.map((card) => (
              <Card key={card.title} className="glass-card overflow-hidden border-none shadow-md backdrop-blur-md">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">{card.title}</CardTitle>
                  <card.icon className={cn("h-4 w-4", card.tone)} />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{card.value}</div>
                  <p className="mt-1 text-[11px] text-muted-foreground">{card.sub}</p>
                </CardContent>
              </Card>
            ))}
      </div>

      <Card className="glass-card overflow-hidden border-none shadow-xl backdrop-blur-md">
        <CardHeader className="border-b border-border/40">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div>
              <CardTitle>注册中心</CardTitle>
              <p className="mt-1 text-sm text-muted-foreground">
                查看注册任务历史、实时状态和日志；也可以直接从这里启动新的自动注册。
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                className="gap-2"
                onClick={() => void refetchTasks()}
              >
                <RefreshCw className="h-4 w-4" />
                刷新
              </Button>
              <Button className="gap-2" onClick={() => setAddModalOpen(true)}>
                <Plus className="h-4 w-4" />
                新建注册
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4 pt-4">
          <div className="grid gap-4 md:grid-cols-[220px_1fr]">
            <div className="space-y-2">
              <Label>任务状态</Label>
              <Select
                value={statusFilter}
                onValueChange={(value) => {
                  setStatusFilter(value || "all");
                  setPage(1);
                }}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">全部状态</SelectItem>
                  <SelectItem value="pending">排队中</SelectItem>
                  <SelectItem value="running">运行中</SelectItem>
                  <SelectItem value="completed">已完成</SelectItem>
                  <SelectItem value="failed">失败</SelectItem>
                  <SelectItem value="cancelled">已取消</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-end justify-end">
              <div className="text-sm text-muted-foreground">
                共 {total} 条任务，当前第 {page} / {totalPages} 页
              </div>
            </div>
          </div>

          <div className="overflow-hidden rounded-xl border border-border/50">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>创建时间</TableHead>
                  <TableHead>状态</TableHead>
                  <TableHead>邮箱</TableHead>
                  <TableHead>代理</TableHead>
                  <TableHead>任务编号</TableHead>
                  <TableHead className="text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 6 }).map((_, index) => (
                    <TableRow key={index}>
                      <TableCell colSpan={6}>
                        <Skeleton className="h-8 w-full" />
                      </TableCell>
                    </TableRow>
                  ))
                ) : tasks.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={6} className="h-40 text-center text-muted-foreground">
                      当前筛选下暂无注册任务
                    </TableCell>
                  </TableRow>
                ) : (
                  tasks.map((task) => {
                    const statusMeta = getStatusMeta(task.status);
                    const normalizedStatus = String(task.status || "").trim().toLowerCase();
                    const canCancel =
                      normalizedStatus === "pending" || normalizedStatus === "running";
                    const canDelete = normalizedStatus !== "running";
                    return (
                      <TableRow key={task.taskUuid}>
                        <TableCell className="text-sm text-muted-foreground">
                          {formatTimestamp(task.createdAt)}
                        </TableCell>
                        <TableCell>
                          <Badge className={cn("border", statusMeta.className)}>
                            {statusMeta.label}
                          </Badge>
                        </TableCell>
                        <TableCell className="max-w-[180px]">
                          <div className="truncate text-sm">{task.email || "--"}</div>
                          {task.errorMessage ? (
                            <div className="truncate text-[11px] text-red-500">
                              {task.errorMessage}
                            </div>
                          ) : null}
                        </TableCell>
                        <TableCell className="max-w-[180px]">
                          <span className="block truncate font-mono text-[11px] text-muted-foreground">
                            {task.proxy || "--"}
                          </span>
                        </TableCell>
                        <TableCell className="max-w-[180px]">
                          <span className="block truncate font-mono text-[11px] text-muted-foreground">
                            {task.taskUuid}
                          </span>
                        </TableCell>
                        <TableCell className="text-right">
                          <div className="flex justify-end gap-2">
                            <Button
                              variant="ghost"
                              size="icon"
                              title="查看详情"
                              onClick={() => void handleOpenDetail(task.taskUuid)}
                            >
                              <Eye className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="icon"
                              title="取消任务"
                              disabled={!canCancel || isCancelling}
                              onClick={() => setPendingAction({ kind: "cancel", task })}
                            >
                              <XCircle className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="icon"
                              title="删除任务"
                              disabled={!canDelete || isDeleting}
                              onClick={() => setPendingAction({ kind: "delete", task })}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  })
                )}
              </TableBody>
            </Table>
          </div>

          <div className="flex items-center justify-between">
            <Button
              variant="outline"
              disabled={page <= 1}
              onClick={() => setPage((current) => Math.max(1, current - 1))}
            >
              上一页
            </Button>
            <span className="text-sm text-muted-foreground">
              第 {page} 页 / 共 {totalPages} 页
            </span>
            <Button
              variant="outline"
              disabled={page >= totalPages}
              onClick={() => setPage((current) => Math.min(totalPages, current + 1))}
            >
              下一页
            </Button>
          </div>
        </CardContent>
      </Card>

      <Dialog open={detailOpen} onOpenChange={setDetailOpen}>
        <DialogContent className="glass-card max-h-[85vh] overflow-hidden border-none sm:max-w-[760px]">
          <DialogHeader>
            <DialogTitle>任务详情</DialogTitle>
            <DialogDescription>
              查看单个注册任务的实时快照和日志明细。
            </DialogDescription>
          </DialogHeader>
          {isLoadingDetail || !detailTask ? (
            <div className="space-y-3">
              <Skeleton className="h-6 w-48" />
              <Skeleton className="h-24 w-full" />
              <Skeleton className="h-64 w-full" />
            </div>
          ) : (
            <div className="space-y-4">
              <div className="grid gap-3 md:grid-cols-2">
                <div className="rounded-lg border border-border/50 bg-muted/20 p-3">
                  <p className="text-xs text-muted-foreground">任务编号</p>
                  <p className="mt-1 font-mono text-xs">{detailTask.taskUuid}</p>
                </div>
                <div className="rounded-lg border border-border/50 bg-muted/20 p-3">
                  <p className="text-xs text-muted-foreground">状态</p>
                  <p className="mt-1 text-sm font-medium">
                    {getStatusMeta(detailTask.status).label}
                  </p>
                </div>
                <div className="rounded-lg border border-border/50 bg-muted/20 p-3">
                  <p className="text-xs text-muted-foreground">创建时间</p>
                  <p className="mt-1 text-sm">{formatTimestamp(detailTask.createdAt)}</p>
                </div>
                <div className="rounded-lg border border-border/50 bg-muted/20 p-3">
                  <p className="text-xs text-muted-foreground">注册邮箱</p>
                  <p className="mt-1 text-sm">{detailTask.email || "--"}</p>
                </div>
              </div>
              <Textarea
                readOnly
                value={detailTask.logs.join("\n")}
                className="min-h-[360px] resize-none overflow-auto whitespace-pre-wrap break-all font-mono text-[10px] leading-4 [overflow-wrap:anywhere]"
              />
            </div>
          )}
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={pendingAction != null}
        onOpenChange={(open) => {
          if (!open) setPendingAction(null);
        }}
        title={pendingAction?.kind === "cancel" ? "取消注册任务" : "删除注册任务"}
        description={
          pendingAction?.kind === "cancel"
            ? "确认要取消这个注册任务吗？如果任务已经接近完成，仍可能产生部分结果。"
            : "确认要删除这个注册任务吗？删除后将无法在列表中查看历史日志。"
        }
        confirmText={pendingAction?.kind === "cancel" ? "确认取消" : "确认删除"}
        confirmVariant={pendingAction?.kind === "delete" ? "destructive" : "default"}
        onConfirm={() => void handleConfirmAction()}
      />

      <AddAccountModal open={addModalOpen} onOpenChange={setAddModalOpen} />
    </div>
  );
}
