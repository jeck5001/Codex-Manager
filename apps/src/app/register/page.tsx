"use client";

import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  CalendarDays,
  CheckCircle2,
  Clock3,
  Eye,
  Layers3,
  Mail,
  Plus,
  RefreshCw,
  RotateCcw,
  Server,
  Trash2,
  UserPlus,
  Users,
  XCircle,
} from "lucide-react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { AddAccountModal } from "@/components/modals/add-account-modal";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
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
import { getAppErrorMessage } from "@/lib/api/transport";
import { formatApiDateTime } from "@/lib/utils/datetime";
import { cn } from "@/lib/utils";
import type { Account, RegisterTaskSnapshot } from "@/types";

type PendingAction =
  | { kind: "cancel"; task: RegisterTaskSnapshot }
  | { kind: "delete"; task: RegisterTaskSnapshot }
  | { kind: "batch-delete"; taskUuids: string[] }
  | null;

const PAGE_SIZE = 20;

const REGISTER_FAILURE_REASON_LABELS: Record<string, string> = {
  register_email_otp_timeout: "邮箱验证码超时",
  register_email_otp_invalid: "邮箱验证码错误或已过期",
  register_phone_required: "注册触发手机号验证",
  register_proxy_error: "注册代理异常",
};

const REGISTER_RETRY_STRATEGY_LABELS: Record<string, string> = {
  same: "原样重试",
  relax_email_wait: "延长验证码等待",
  latest_email_otp: "强制取最新验证码",
  refresh_proxy: "换代理重试",
};

function formatTimestamp(value: string) {
  return formatApiDateTime(value, { emptyLabel: "--", withSeconds: true });
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

function getImportStatusMeta(task: RegisterTaskSnapshot) {
  if (task.isImported) {
    return {
      label: "已入池",
      className: "border-green-500/20 bg-green-500/10 text-green-600 dark:text-green-400",
      detail: task.importedAccountId || null,
    };
  }
  if (task.requiresManualImport) {
    return {
      label: "待入池",
      className: "border-amber-500/20 bg-amber-500/10 text-amber-600 dark:text-amber-300",
      detail: "注册已完成，但尚未导入本地账号池",
    };
  }
  if (String(task.status || "").trim().toLowerCase() === "completed") {
    return {
      label: "无入池信息",
      className: "border-slate-500/20 bg-slate-500/10 text-slate-600 dark:text-slate-300",
      detail: "任务完成，但没有可用于导入的账号信息",
    };
  }
  return null;
}

function countByStatuses(source: Record<string, number>, keys: string[]) {
  return keys.reduce((sum, key) => sum + (source[key] || 0), 0);
}

function isTaskActive(status: string) {
  const normalized = String(status || "").trim().toLowerCase();
  return normalized === "pending" || normalized === "running";
}

function normalizeRegisterStatusFilter(value: string | null): string {
  if (value === "pending" || value === "running" || value === "completed" || value === "failed" || value === "cancelled") {
    return value;
  }
  return "all";
}

function normalizeRegisterImportFilter(value: string | null): string {
  if (value === "manual-import" || value === "imported" || value === "no-import-info") {
    return value;
  }
  return "all";
}

function resolveRegisterFailureReasonLabel(code: string, fallback?: string) {
  const normalized = String(code || "").trim().toLowerCase();
  if (normalized && REGISTER_FAILURE_REASON_LABELS[normalized]) {
    return REGISTER_FAILURE_REASON_LABELS[normalized];
  }
  return fallback || "失败原因";
}

function recommendedRetryStrategyForFailure(code: string): string | null {
  switch (String(code || "").trim().toLowerCase()) {
    case "register_email_otp_timeout":
      return "relax_email_wait";
    case "register_email_otp_invalid":
      return "latest_email_otp";
    case "register_proxy_error":
      return "refresh_proxy";
    case "register_phone_required":
      return null;
    default:
      return "same";
  }
}

function resolveRetryStrategyLabel(strategy: string | null) {
  if (!strategy) {
    return "不可自动重试";
  }
  return REGISTER_RETRY_STRATEGY_LABELS[strategy] || REGISTER_RETRY_STRATEGY_LABELS.same;
}

function describeRetryStrategy(strategy: string | null) {
  switch (strategy) {
    case "relax_email_wait":
      return "延长验证码等待时间，并缩短轮询间隔后重试。";
    case "latest_email_otp":
      return "强制以最新一封验证码为准，避免拿到旧验证码。";
    case "refresh_proxy":
      return "丢弃旧代理，重新挑选代理后再发起注册。";
    case "same":
      return "按原参数再跑一次，适合偶发网络抖动。";
    default:
      return "这类失败通常需要人工处理，不建议自动重试。";
  }
}

function isRetryableRegisterFailure(code: string) {
  return recommendedRetryStrategyForFailure(code) != null;
}

export default function RegisterPage() {
  const queryClient = useQueryClient();
  const [page, setPage] = useState(1);
  const [statusFilter, setStatusFilter] = useState("all");
  const [importFilter, setImportFilter] = useState("all");
  const [addModalOpen, setAddModalOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailTask, setDetailTask] = useState<RegisterTaskSnapshot | null>(null);
  const [isLoadingDetail, setIsLoadingDetail] = useState(false);
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [selectedTaskUuids, setSelectedTaskUuids] = useState<string[]>([]);
  const [monitorTaskUuid, setMonitorTaskUuid] = useState("");
  const [failureCodeFilter, setFailureCodeFilter] = useState("");
  const [importingTaskUuid, setImportingTaskUuid] = useState("");

  const {
    tasks,
    total,
    stats,
    isLoading,
    isStatsLoading,
    refetchTasks,
    cancelTask,
    retryTask,
    deleteTask,
    deleteTasks,
    isCancelling,
    isRetrying,
    isDeleting,
    isDeletingMany,
  } = useRegisterTasks({
    page,
    pageSize: PAGE_SIZE,
    status: statusFilter === "all" ? null : statusFilter,
  });

  const registerServicesQuery = useQuery({
    queryKey: ["register-available-services"],
    queryFn: () => accountClient.getRegisterAvailableServices(),
    retry: 1,
    staleTime: 30_000,
  });

  const registerOutlookAccountsQuery = useQuery({
    queryKey: ["register-outlook-accounts"],
    queryFn: () => accountClient.getRegisterOutlookAccounts(),
    retry: 1,
    staleTime: 30_000,
  });

  const recentAccountsQuery = useQuery({
    queryKey: ["register-recent-accounts"],
    queryFn: () => accountClient.list({ page: 1, pageSize: 8 }),
    retry: 1,
    staleTime: 15_000,
  });

  const latestTasksQuery = useQuery({
    queryKey: ["register-tasks", "latest-workbench"],
    queryFn: () => accountClient.listRegisterTasks({ page: 1, pageSize: 8, status: null }),
    retry: 1,
    refetchInterval: 3000,
  });

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const byStatus = stats?.byStatus || {};
  const runningCount = countByStatuses(byStatus, ["pending", "running"]);
  const completedCount = byStatus.completed || 0;
  const failedCount = countByStatuses(byStatus, ["failed", "cancelled"]);
  const latestTasks = useMemo(
    () => latestTasksQuery.data?.tasks || [],
    [latestTasksQuery.data?.tasks],
  );
  const activeTasks = useMemo(
    () => latestTasks.filter((task) => isTaskActive(task.status)),
    [latestTasks],
  );
  const recentAccounts = useMemo(
    () => recentAccountsQuery.data?.items || [],
    [recentAccountsQuery.data?.items],
  );
  const recentAccountPreview = useMemo(
    () => recentAccounts.slice(0, 4),
    [recentAccounts],
  );
  const filteredTasks = useMemo(
    () =>
      tasks.filter((task) => {
        if (failureCodeFilter) {
          const failureMatched =
            String(task.failureCode || "").trim().toLowerCase() === failureCodeFilter;
          if (!failureMatched) {
            return false;
          }
        }

        if (importFilter === "manual-import") {
          return task.requiresManualImport;
        }
        if (importFilter === "imported") {
          return task.isImported;
        }
        if (importFilter === "no-import-info") {
          return !task.requiresManualImport && !task.isImported;
        }
        return true;
      }),
    [failureCodeFilter, importFilter, tasks],
  );
  const recommendedRetryStrategy = useMemo(
    () => recommendedRetryStrategyForFailure(failureCodeFilter),
    [failureCodeFilter],
  );
  const selectableTaskUuids = useMemo(
    () =>
      filteredTasks
        .filter((task) => String(task.status || "").trim().toLowerCase() !== "running")
        .map((task) => task.taskUuid),
    [filteredTasks],
  );
  const allSelectableChecked =
    selectableTaskUuids.length > 0 &&
    selectableTaskUuids.every((taskUuid) => selectedTaskUuids.includes(taskUuid));
  const failedTaskStrategyGroups = useMemo(() => {
    const groups = new Map<
      string,
      {
        code: string;
        label: string;
        failedCount: number;
        retryableCount: number;
        blockedCount: number;
        strategy: string | null;
      }
    >();
    for (const task of tasks) {
      if (String(task.status || "").trim().toLowerCase() !== "failed") {
        continue;
      }
      const code = String(task.failureCode || "").trim().toLowerCase() || "unknown";
      const existing = groups.get(code);
      const strategy = recommendedRetryStrategyForFailure(code);
      const retryable = strategy != null;
      if (existing) {
        existing.failedCount += 1;
        if (retryable) {
          existing.retryableCount += 1;
        } else {
          existing.blockedCount += 1;
        }
        continue;
      }
      groups.set(code, {
        code,
        label: resolveRegisterFailureReasonLabel(code, task.failureLabel),
        failedCount: 1,
        retryableCount: retryable ? 1 : 0,
        blockedCount: retryable ? 0 : 1,
        strategy,
      });
    }
    return Array.from(groups.values()).sort((left, right) => {
      if (right.failedCount !== left.failedCount) {
        return right.failedCount - left.failedCount;
      }
      return left.label.localeCompare(right.label, "zh-CN");
    });
  }, [tasks]);
  const recoverableFailedTaskCount = useMemo(
    () =>
      failedTaskStrategyGroups.reduce(
        (sum, item) => sum + item.retryableCount,
        0,
      ),
    [failedTaskStrategyGroups],
  );

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const params = new URLSearchParams(window.location.search);
    const nextStatusFilter = normalizeRegisterStatusFilter(params.get("status"));
    const nextImportFilter = normalizeRegisterImportFilter(params.get("importStatus"));
    const nextFailureCode = (params.get("failureCode") || "").trim().toLowerCase();
    setStatusFilter((current) => (current === nextStatusFilter ? current : nextStatusFilter));
    setImportFilter((current) => (current === nextImportFilter ? current : nextImportFilter));
    setFailureCodeFilter((current) => (current === nextFailureCode ? current : nextFailureCode));
    setPage(1);
  }, []);

  useEffect(() => {
    setSelectedTaskUuids((current) =>
      current.filter((taskUuid) => selectableTaskUuids.includes(taskUuid)),
    );
  }, [selectableTaskUuids]);

  useEffect(() => {
    const candidates = activeTasks.length > 0 ? activeTasks : latestTasks;
    if (!candidates.length) {
      if (monitorTaskUuid) {
        setMonitorTaskUuid("");
      }
      return;
    }

    if (!candidates.some((task) => task.taskUuid === monitorTaskUuid)) {
      setMonitorTaskUuid(candidates[0].taskUuid);
    }
  }, [activeTasks, latestTasks, monitorTaskUuid]);

  const monitorTaskQuery = useQuery({
    queryKey: ["register-task-monitor", monitorTaskUuid],
    queryFn: () => accountClient.getRegisterTask(monitorTaskUuid),
    enabled: Boolean(monitorTaskUuid),
    retry: 1,
    refetchInterval: (query) => {
      const status = query.state.data?.status || "";
      return isTaskActive(status) ? 2000 : false;
    },
  });

  const workbenchCards = useMemo(
    () => [
      {
        title: "邮箱服务",
        value:
          (registerServicesQuery.data?.outlook.count || 0) +
          (registerServicesQuery.data?.customDomain.count || 0) +
          (registerServicesQuery.data?.generatorEmail.count || 0) +
          (registerServicesQuery.data?.mail33Imap.count || 0) +
          (registerServicesQuery.data?.tempMail.count || 0) +
          (registerServicesQuery.data?.tempmail.available ? 1 : 0),
        sub: "当前可用于自动注册的服务总数",
        icon: Server,
        tone: "text-sky-500",
      },
      {
        title: "Outlook 账号",
        value: registerOutlookAccountsQuery.data?.total || 0,
        sub: `未注册 ${registerOutlookAccountsQuery.data?.unregisteredCount || 0} 个`,
        icon: Mail,
        tone: "text-blue-500",
      },
      {
        title: "最近账号",
        value: recentAccounts.length,
        sub: "本地账号池最新加载的账号数",
        icon: Users,
        tone: "text-emerald-500",
      },
      {
        title: "活跃任务",
        value: activeTasks.length,
        sub: "排队中或正在执行的注册任务",
        icon: Clock3,
        tone: "text-amber-500",
      },
    ],
    [
      activeTasks.length,
      recentAccounts.length,
      registerOutlookAccountsQuery.data?.total,
      registerOutlookAccountsQuery.data?.unregisteredCount,
      registerServicesQuery.data?.customDomain.count,
      registerServicesQuery.data?.generatorEmail.count,
      registerServicesQuery.data?.mail33Imap.count,
      registerServicesQuery.data?.outlook.count,
      registerServicesQuery.data?.tempMail.count,
      registerServicesQuery.data?.tempmail.available,
    ],
  );

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
    if (pendingAction.kind === "batch-delete") {
      await deleteTasks(pendingAction.taskUuids);
      setSelectedTaskUuids([]);
      return;
    }
    await deleteTask(pendingAction.task.taskUuid);
  };

  const toggleTaskSelection = (taskUuid: string, checked: boolean) => {
    setSelectedTaskUuids((current) => {
      if (checked) {
        return current.includes(taskUuid) ? current : [...current, taskUuid];
      }
      return current.filter((item) => item !== taskUuid);
    });
  };

  const toggleSelectAllCurrentPage = (checked: boolean) => {
    setSelectedTaskUuids(checked ? selectableTaskUuids : []);
  };

  const handleImportTask = async (task: RegisterTaskSnapshot) => {
    if (!task.requiresManualImport || !task.taskUuid) {
      return;
    }
    setImportingTaskUuid(task.taskUuid);
    try {
      const imported = await accountClient.importRegisterTask(task.taskUuid);
      toast.success(`已加入号池：${imported.email || task.email || imported.accountId}`);
      await Promise.all([
        refetchTasks(),
        latestTasksQuery.refetch(),
        recentAccountsQuery.refetch(),
        queryClient.invalidateQueries({ queryKey: ["register-task-monitor"] }),
        queryClient.invalidateQueries({ queryKey: ["accounts"] }),
        queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
      ]);
      if (detailTask?.taskUuid === task.taskUuid) {
        setDetailTask(await accountClient.getRegisterTask(task.taskUuid));
      }
    } catch (error) {
      toast.error(`加入号池失败: ${getAppErrorMessage(error)}`);
    } finally {
      setImportingTaskUuid("");
    }
  };

  const handleRetryFailedTasksInView = async () => {
    const failedTasks = filteredTasks.filter(
      (task) =>
        String(task.status || "").trim().toLowerCase() === "failed" &&
        isRetryableRegisterFailure(task.failureCode),
    );
    if (failedTasks.length === 0) {
      return;
    }
    for (const task of failedTasks) {
      const strategy = recommendedRetryStrategyForFailure(task.failureCode);
      if (!strategy) {
        continue;
      }
      await retryTask(task.taskUuid, strategy);
    }
  };

  const handleRetryByFailureGroup = async (failureCode: string) => {
    const matchedTasks = tasks.filter(
      (task) =>
        String(task.status || "").trim().toLowerCase() === "failed" &&
        String(task.failureCode || "").trim().toLowerCase() ===
          String(failureCode || "").trim().toLowerCase(),
    );
    for (const task of matchedTasks) {
      const strategy = recommendedRetryStrategyForFailure(task.failureCode);
      if (!strategy) {
        continue;
      }
      await retryTask(task.taskUuid, strategy);
    }
  };

  const handleRetryRecoverableFailedTasks = async () => {
    for (const group of failedTaskStrategyGroups) {
      if (!group.strategy || group.retryableCount <= 0) {
        continue;
      }
      await handleRetryByFailureGroup(group.code);
    }
  };

  return (
    <div className="min-w-0 space-y-4 animate-in fade-in duration-700">
      <div className="grid min-w-0 gap-3 md:grid-cols-2 xl:grid-cols-4">
        {isStatsLoading
          ? Array.from({ length: 4 }).map((_, index) => (
              <Skeleton key={index} className="h-24 rounded-2xl" />
            ))
          : summaryCards.map((card) => (
              <Card
                key={card.title}
                className="glass-card overflow-hidden border-none py-0 shadow-md backdrop-blur-md"
              >
                <CardHeader className="flex flex-row items-center justify-between space-y-0 px-5 pb-1 pt-4">
                  <CardTitle className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
                    {card.title}
                  </CardTitle>
                  <card.icon className={cn("h-4 w-4", card.tone)} />
                </CardHeader>
                <CardContent className="px-5 pb-4 pt-0">
                  <div className="text-xl font-bold leading-none">{card.value}</div>
                  <p className="mt-2 line-clamp-1 text-[11px] text-muted-foreground">{card.sub}</p>
                </CardContent>
              </Card>
            ))}
      </div>

      <div className="grid min-w-0 gap-4">
        <Card className="glass-card min-w-0 overflow-hidden border-none shadow-xl backdrop-blur-md">
          <CardHeader className="border-b border-border/40 pb-4">
            <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
              <div>
                <CardTitle>注册工作台</CardTitle>
                <p className="mt-1 text-sm text-muted-foreground">
                  从这里发起自动注册，并快速查看服务、账号和活跃任务。
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <Button className="gap-2" onClick={() => setAddModalOpen(true)}>
                  <Plus className="h-4 w-4" />
                  启动注册
                </Button>
                <Button
                  variant="outline"
                  className="gap-2"
                  onClick={() => {
                    void registerServicesQuery.refetch();
                    void registerOutlookAccountsQuery.refetch();
                    void latestTasksQuery.refetch();
                    void recentAccountsQuery.refetch();
                  }}
                >
                  <RefreshCw className="h-4 w-4" />
                  刷新工作台
                </Button>
              </div>
            </div>
          </CardHeader>
          <CardContent className="space-y-3 pt-4">
            <div className="grid min-w-0 gap-3 sm:grid-cols-2 xl:grid-cols-4">
              {workbenchCards.map((card) => (
                <div
                  key={card.title}
                  className="rounded-xl border border-border/50 bg-muted/20 p-3 shadow-sm"
                >
                  <div className="flex items-center justify-between">
                    <p className="text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">
                      {card.title}
                    </p>
                    <card.icon className={cn("h-4 w-4", card.tone)} />
                  </div>
                  <div className="mt-2 text-xl font-bold leading-none">{card.value}</div>
                  <p className="mt-2 line-clamp-1 text-[11px] text-muted-foreground">{card.sub}</p>
                </div>
              ))}
            </div>

            <div className="grid min-w-0 gap-3 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.7fr)]">
              <div className="min-w-0 rounded-2xl border border-border/50 bg-muted/15 p-4">
                <div className="mb-3 flex min-w-0 flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
                  <div>
                    <h3 className="text-sm font-semibold">活跃任务监控</h3>
                    <p className="mt-1 text-xs text-muted-foreground">
                      默认跟踪最近的活跃任务；没有活跃任务时展示最近一条任务记录。
                    </p>
                  </div>
                  <div className="min-w-0 xl:min-w-[240px]">
                    <Select
                      value={monitorTaskUuid}
                      onValueChange={(value) => setMonitorTaskUuid(value || "")}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="选择要监控的任务" />
                      </SelectTrigger>
                      <SelectContent>
                        {(activeTasks.length > 0 ? activeTasks : latestTasks).map((task) => (
                          <SelectItem key={task.taskUuid} value={task.taskUuid}>
                            {`${getStatusMeta(task.status).label} · ${task.email || task.taskUuid.slice(0, 8)}`}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>

                {!monitorTaskUuid ? (
                  <div className="rounded-xl border border-dashed border-border/60 px-4 py-10 text-center text-sm text-muted-foreground">
                    暂无可监控的注册任务
                  </div>
                ) : monitorTaskQuery.isLoading || !monitorTaskQuery.data ? (
                  <div className="space-y-3">
                    <Skeleton className="h-20 rounded-xl" />
                    <Skeleton className="h-[260px] rounded-xl" />
                  </div>
                ) : (
                  <div className="space-y-3">
                    <div className="grid min-w-0 gap-3 md:grid-cols-3">
                      <div className="min-w-0 rounded-xl border border-border/50 bg-background/40 p-3">
                        <p className="text-xs text-muted-foreground">状态</p>
                        <Badge className={cn("mt-2 border", getStatusMeta(monitorTaskQuery.data.status).className)}>
                          {getStatusMeta(monitorTaskQuery.data.status).label}
                        </Badge>
                      </div>
                      <div className="min-w-0 rounded-xl border border-border/50 bg-background/40 p-3">
                        <p className="text-xs text-muted-foreground">邮箱</p>
                        <p className="mt-2 truncate text-sm font-medium">
                          {monitorTaskQuery.data.email || "--"}
                        </p>
                      </div>
                      <div className="min-w-0 rounded-xl border border-border/50 bg-background/40 p-3">
                        <p className="text-xs text-muted-foreground">创建时间</p>
                        <p className="mt-2 text-sm">{formatTimestamp(monitorTaskQuery.data.createdAt)}</p>
                      </div>
                    </div>
                    <Textarea
                      readOnly
                      value={monitorTaskQuery.data.logs.join("\n")}
                      className="min-h-[260px] resize-none overflow-auto whitespace-pre-wrap break-all font-mono text-[10px] leading-4 xl:min-h-[320px] [overflow-wrap:anywhere]"
                    />
                  </div>
                )}
              </div>

              <div className="min-w-0 space-y-3">
                <div className="min-w-0 rounded-2xl border border-border/50 bg-muted/15 p-4">
                  <div className="mb-3 flex items-center justify-between">
                    <h3 className="text-sm font-semibold">服务概览</h3>
                    <Layers3 className="h-4 w-4 text-muted-foreground" />
                  </div>
                  <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-2">
                    <div className="rounded-xl border border-border/50 bg-background/40 px-3 py-2.5">
                      <p className="text-[11px] text-muted-foreground">Tempmail</p>
                      <div className="mt-2">
                        <Badge variant={registerServicesQuery.data?.tempmail.available ? "default" : "secondary"}>
                          {registerServicesQuery.data?.tempmail.available ? "可用" : "不可用"}
                        </Badge>
                      </div>
                    </div>
                    <div className="rounded-xl border border-border/50 bg-background/40 px-3 py-2.5">
                      <p className="text-[11px] text-muted-foreground">Outlook</p>
                      <p className="mt-2 text-lg font-semibold">
                        {registerServicesQuery.data?.outlook.count || 0}
                      </p>
                    </div>
                    <div className="rounded-xl border border-border/50 bg-background/40 px-3 py-2.5">
                      <p className="text-[11px] text-muted-foreground">自定义域名</p>
                      <p className="mt-2 text-lg font-semibold">
                        {registerServicesQuery.data?.customDomain.count || 0}
                      </p>
                    </div>
                    <div className="rounded-xl border border-border/50 bg-background/40 px-3 py-2.5">
                      <p className="text-[11px] text-muted-foreground">33mail + IMAP</p>
                      <p className="mt-2 text-lg font-semibold">
                        {registerServicesQuery.data?.mail33Imap.count || 0}
                      </p>
                    </div>
                    <div className="rounded-xl border border-border/50 bg-background/40 px-3 py-2.5">
                      <p className="text-[11px] text-muted-foreground">Temp Mail</p>
                      <p className="mt-2 text-lg font-semibold">
                        {registerServicesQuery.data?.tempMail.count || 0}
                      </p>
                    </div>
                  </div>
                </div>

                <div className="min-w-0 rounded-2xl border border-border/50 bg-muted/15 p-4">
                  <div className="mb-3 flex items-center justify-between">
                    <h3 className="text-sm font-semibold">最近账号</h3>
                    <Badge variant="outline">{recentAccounts.length}</Badge>
                  </div>
                  <div className="space-y-2">
                    {recentAccountsQuery.isLoading ? (
                      Array.from({ length: 4 }).map((_, index) => (
                        <Skeleton key={index} className="h-11 rounded-xl" />
                      ))
                    ) : recentAccounts.length === 0 ? (
                      <div className="rounded-xl border border-dashed border-border/60 px-3 py-5 text-center text-sm text-muted-foreground">
                        本地账号池里还没有可展示的账号
                      </div>
                    ) : (
                      recentAccountPreview.map((account: Account) => (
                        <div
                          key={account.id}
                          className="rounded-xl border border-border/50 bg-background/40 px-3 py-2"
                        >
                          <div className="flex items-center justify-between gap-2">
                            <span className="truncate text-sm font-medium">{account.name}</span>
                            <Badge variant={account.isAvailable ? "default" : "secondary"}>
                              {account.isAvailable ? "可用" : "不可用"}
                            </Badge>
                          </div>
                          <p className="mt-1 truncate font-mono text-[11px] text-muted-foreground">
                            {account.id}
                          </p>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card className="glass-card min-w-0 overflow-hidden border-none shadow-xl backdrop-blur-md">
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
        <CardContent className="min-w-0 space-y-4 pt-4">
          <div className="grid min-w-0 gap-4 md:grid-cols-[220px_220px_minmax(0,1fr)]">
            <div className="min-w-0 space-y-2">
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
            <div className="min-w-0 space-y-2">
              <Label>入池状态</Label>
              <Select
                value={importFilter}
                onValueChange={(value) => {
                  setImportFilter(value || "all");
                  setPage(1);
                }}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">全部</SelectItem>
                  <SelectItem value="manual-import">待入池</SelectItem>
                  <SelectItem value="imported">已入池</SelectItem>
                  <SelectItem value="no-import-info">无入池信息</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="min-w-0 flex items-end justify-end">
              <div className="flex flex-wrap items-center justify-end gap-3">
                <div className="text-sm text-muted-foreground">
                  {failureCodeFilter || importFilter !== "all"
                    ? `当前页命中 ${filteredTasks.length} 条，共返回 ${total} 条任务`
                    : `共 ${total} 条任务，当前第 ${page} / ${totalPages} 页`}
                </div>
                {failureCodeFilter ? (
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={
                      isRetrying ||
                      !filteredTasks.some(
                        (task) => String(task.status || "").trim().toLowerCase() === "failed",
                      )
                    }
                    onClick={() => void handleRetryFailedTasksInView()}
                  >
                    按推荐策略重试当前页
                  </Button>
                ) : null}
              </div>
            </div>
          </div>

          {failureCodeFilter ? (
            <div className="rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-sm text-amber-700 dark:text-amber-300">
              已按失败原因筛选：{resolveRegisterFailureReasonLabel(
                failureCodeFilter,
                filteredTasks[0]?.failureLabel,
              )}，推荐策略：{resolveRetryStrategyLabel(recommendedRetryStrategy)}
            </div>
          ) : null}

          {failedTaskStrategyGroups.length > 0 ? (
            <div className="space-y-3 rounded-2xl border border-border/50 bg-muted/15 p-4">
              <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                <div>
                  <h3 className="text-sm font-semibold">失败重试策略中心</h3>
                  <p className="mt-1 text-xs text-muted-foreground">
                    基于当前页失败任务自动推荐处理方式。可恢复类型支持一键批量重试；手机号验证类会直接标记为不可自动重试。
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={isRetrying || recoverableFailedTaskCount <= 0}
                  onClick={() => void handleRetryRecoverableFailedTasks()}
                >
                  一键重试全部可恢复失败 ({recoverableFailedTaskCount})
                </Button>
              </div>
              <div className="grid gap-3 xl:grid-cols-2">
                {failedTaskStrategyGroups.map((group) => (
                  <div
                    key={group.code}
                    className={cn(
                      "rounded-xl border p-3 shadow-sm",
                      group.strategy
                        ? "border-emerald-500/20 bg-emerald-500/5"
                        : "border-rose-500/20 bg-rose-500/5",
                    )}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <p className="truncate text-sm font-semibold">{group.label}</p>
                        <p className="mt-1 text-[11px] text-muted-foreground">
                          推荐策略：{resolveRetryStrategyLabel(group.strategy)}
                        </p>
                      </div>
                      <Badge variant="secondary" className="shrink-0">
                        {group.failedCount} 条
                      </Badge>
                    </div>
                    <p className="mt-3 text-[11px] leading-5 text-muted-foreground">
                      {describeRetryStrategy(group.strategy)}
                    </p>
                    <div className="mt-3 flex flex-wrap gap-2 text-[11px]">
                      <span className="rounded-full bg-background/70 px-2 py-1 text-muted-foreground">
                        可重试 {group.retryableCount}
                      </span>
                      {group.blockedCount > 0 ? (
                        <span className="rounded-full bg-rose-500/10 px-2 py-1 text-rose-700 dark:text-rose-300">
                          不建议重试 {group.blockedCount}
                        </span>
                      ) : null}
                    </div>
                    <div className="mt-3 flex flex-wrap gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setFailureCodeFilter(group.code)}
                      >
                        只看这类失败
                      </Button>
                      <Button
                        size="sm"
                        disabled={!group.strategy || group.retryableCount <= 0 || isRetrying}
                        onClick={() => void handleRetryByFailureGroup(group.code)}
                      >
                        按推荐策略重试
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          <div className="min-w-0 overflow-x-auto rounded-xl border border-border/50">
            {selectedTaskUuids.length > 0 ? (
              <div className="flex items-center justify-between gap-3 border-b border-border/50 bg-muted/30 px-4 py-3">
                <div className="text-sm text-muted-foreground">
                  已选中 <span className="font-medium text-foreground">{selectedTaskUuids.length}</span> 条任务
                </div>
                <Button
                  variant="destructive"
                  size="sm"
                  disabled={isDeletingMany}
                  onClick={() =>
                    setPendingAction({ kind: "batch-delete", taskUuids: selectedTaskUuids })
                  }
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  批量删除
                </Button>
              </div>
            ) : null}
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-12">
                    <Checkbox
                      checked={allSelectableChecked}
                      disabled={selectableTaskUuids.length === 0}
                      onCheckedChange={(checked) => toggleSelectAllCurrentPage(checked === true)}
                      aria-label="全选当前页任务"
                    />
                  </TableHead>
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
                      <TableCell colSpan={7}>
                        <Skeleton className="h-8 w-full" />
                      </TableCell>
                    </TableRow>
                  ))
                ) : filteredTasks.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={7} className="h-40 text-center text-muted-foreground">
                      当前筛选下暂无注册任务
                    </TableCell>
                  </TableRow>
                ) : (
                  filteredTasks.map((task) => {
                    const statusMeta = getStatusMeta(task.status);
                    const importStatusMeta = getImportStatusMeta(task);
                    const normalizedStatus = String(task.status || "").trim().toLowerCase();
                    const canCancel =
                      normalizedStatus === "pending" || normalizedStatus === "running";
                    const canRetry =
                      normalizedStatus === "failed" &&
                      isRetryableRegisterFailure(task.failureCode);
                    const canDelete = normalizedStatus !== "running";
                    const isSelected = selectedTaskUuids.includes(task.taskUuid);
                    const retryStrategy = recommendedRetryStrategyForFailure(task.failureCode);
                    return (
                      <TableRow key={task.taskUuid}>
                        <TableCell>
                          <Checkbox
                            checked={isSelected}
                            disabled={!canDelete}
                            onCheckedChange={(checked) =>
                              toggleTaskSelection(task.taskUuid, checked === true)
                            }
                            aria-label={`选择任务 ${task.taskUuid}`}
                          />
                        </TableCell>
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
                          {importStatusMeta ? (
                            <div className="mt-1 flex items-center gap-2">
                              <Badge className={cn("border", importStatusMeta.className)}>
                                {importStatusMeta.label}
                              </Badge>
                            </div>
                          ) : null}
                          {task.failureLabel ? (
                            <div className="truncate text-[11px] text-amber-600 dark:text-amber-300">
                              {task.failureLabel}
                            </div>
                          ) : null}
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
                              title={
                                canRetry
                                  ? `按${resolveRetryStrategyLabel(retryStrategy)}重新发起`
                                  : "该失败类型不建议自动重试"
                              }
                              disabled={!canRetry || isRetrying}
                              onClick={() =>
                                retryStrategy
                                  ? void retryTask(task.taskUuid, retryStrategy)
                                  : undefined
                              }
                            >
                              <RotateCcw className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="icon"
                              title={
                                task.requiresManualImport
                                  ? "手动加入号池"
                                  : task.isImported
                                    ? "账号已在号池中"
                                    : "当前任务没有可导入账号"
                              }
                              disabled={!task.requiresManualImport || importingTaskUuid === task.taskUuid}
                              onClick={() => void handleImportTask(task)}
                            >
                              {task.isImported ? (
                                <CheckCircle2 className="h-4 w-4" />
                              ) : (
                                <UserPlus className="h-4 w-4" />
                              )}
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
        <DialogContent className="glass-card border-none p-4 sm:max-w-[760px] sm:p-6">
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
              {(() => {
                const importStatusMeta = getImportStatusMeta(detailTask);
                return importStatusMeta ? (
                  <div className="flex flex-col gap-3 rounded-lg border border-border/50 bg-muted/20 p-3 sm:flex-row sm:items-center sm:justify-between">
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground">号池状态</p>
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge className={cn("border", importStatusMeta.className)}>
                          {importStatusMeta.label}
                        </Badge>
                        <span className="text-xs text-muted-foreground">
                          {importStatusMeta.detail || "--"}
                        </span>
                      </div>
                    </div>
                    {detailTask.requiresManualImport ? (
                      <Button
                        size="sm"
                        className="gap-2"
                        disabled={importingTaskUuid === detailTask.taskUuid}
                        onClick={() => void handleImportTask(detailTask)}
                      >
                        <UserPlus className="h-4 w-4" />
                        手动加入号池
                      </Button>
                    ) : null}
                  </div>
                ) : null;
              })()}
              <div className="grid gap-3 sm:grid-cols-2">
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
                <div className="rounded-lg border border-border/50 bg-muted/20 p-3">
                  <p className="text-xs text-muted-foreground">号池账号</p>
                  <p className="mt-1 break-all font-mono text-xs">
                    {detailTask.importedAccountId || "--"}
                  </p>
                </div>
                <div className="rounded-lg border border-border/50 bg-muted/20 p-3 md:col-span-2">
                  <p className="text-xs text-muted-foreground">失败原因</p>
                  <p className="mt-1 text-sm">{detailTask.failureLabel || detailTask.errorMessage || "--"}</p>
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
        title={
          pendingAction?.kind === "cancel"
            ? "取消注册任务"
            : pendingAction?.kind === "batch-delete"
              ? "批量删除注册任务"
              : "删除注册任务"
        }
        description={
          pendingAction?.kind === "cancel"
            ? "确认要取消这个注册任务吗？如果任务已经接近完成，仍可能产生部分结果。"
            : pendingAction?.kind === "batch-delete"
              ? `确认要删除已选中的 ${pendingAction.taskUuids.length} 条注册任务吗？删除后将无法在列表中查看历史日志。`
              : "确认要删除这个注册任务吗？删除后将无法在列表中查看历史日志。"
        }
        confirmText={pendingAction?.kind === "cancel" ? "确认取消" : "确认删除"}
        confirmVariant={
          pendingAction?.kind === "delete" || pendingAction?.kind === "batch-delete"
            ? "destructive"
            : "default"
        }
        onConfirm={() => void handleConfirmAction()}
      />

      <AddAccountModal open={addModalOpen} onOpenChange={setAddModalOpen} />
    </div>
  );
}
