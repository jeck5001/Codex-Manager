"use client";

import { useMemo } from "react";
import {
  Activity,
  BrainCircuit,
  CheckCircle2,
  Database,
  DollarSign,
  ExternalLink,
  PieChart,
  RefreshCw,
  RotateCcw,
  ShieldAlert,
  Sparkles,
  Trash2,
  Users,
  Wrench,
  XCircle,
  Zap,
  type LucideIcon,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { useDashboardStats } from "@/hooks/useDashboardStats";
import { accountClient } from "@/lib/api/account-client";
import { serviceClient } from "@/lib/api/service-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import { cn } from "@/lib/utils";
import {
  formatCompactNumber,
  formatHealthTierLabel,
  formatTsFromSeconds,
  healthTierToneClass,
} from "@/lib/utils/usage";

interface StatProgressCardProps {
  title: string;
  value: number;
  total: number;
  icon: LucideIcon;
  color: string;
  sub: string;
  onClick?: () => void;
}

interface PercentBarProps {
  label: string;
  value: number | null | undefined;
  tone?: "default" | "green" | "blue";
}

interface AccountHighlightCardProps {
  title: string;
  name: string;
  subtitle: string;
  tone?: "green" | "blue";
  progressLabel?: string;
  progressValue?: number | null | undefined;
  healthTier?: "healthy" | "warning" | "risky";
  healthScore?: number;
  onOpenAccount?: () => void;
  onOpenLogs?: () => void;
}

interface MiniTrendChartProps {
  points: Array<{
    bucketTs: number;
    requestCount: number;
    errorRate: number;
  }>;
}

function openFailureDrilldown(
  router: ReturnType<typeof useRouter>,
  code: string,
  label: string,
) {
  const normalized = String(code || "").trim().toLowerCase();
  const accountParams = new URLSearchParams();
  const logParams = new URLSearchParams();

  switch (normalized) {
    case "register_email_otp_timeout":
    case "register_email_otp_invalid":
    case "register_phone_required":
    case "register_proxy_error":
      router.push(`/register?status=failed&failureCode=${encodeURIComponent(normalized)}`);
      return;
    case "account_deactivated":
    case "workspace_deactivated":
      accountParams.set("status", "deactivated");
      accountParams.set(
        "statusReason",
        normalized === "workspace_deactivated" ? "检测到工作区已停用" : "检测到账号已停用"
      );
      router.push(`/accounts?${accountParams.toString()}`);
      return;
    case "refresh_token_expired":
      accountParams.set("statusReason", "Refresh 已过期");
      router.push(`/accounts?${accountParams.toString()}`);
      return;
    case "refresh_token_reused":
      accountParams.set("statusReason", "Refresh 已复用");
      router.push(`/accounts?${accountParams.toString()}`);
      return;
    case "refresh_token_invalidated":
      accountParams.set("statusReason", "Refresh 已失效");
      router.push(`/accounts?${accountParams.toString()}`);
      return;
    case "refresh_token_invalid":
      accountParams.set("statusReason", "Refresh 刷新失败");
      router.push(`/accounts?${accountParams.toString()}`);
      return;
    case "usage_unauthorized":
      accountParams.set("statusReason", "授权失效");
      router.push(`/accounts?${accountParams.toString()}`);
      return;
    case "usage_forbidden":
      logParams.set("query", "status:403");
      logParams.set("statusFilter", "4xx");
      router.push(`/logs?${logParams.toString()}`);
      return;
    case "usage_rate_limited":
      logParams.set("query", "status:429");
      logParams.set("statusFilter", "4xx");
      router.push(`/logs?${logParams.toString()}`);
      return;
    case "usage_upstream_server_error":
      logParams.set("query", "status:5xx");
      logParams.set("statusFilter", "5xx");
      router.push(`/logs?${logParams.toString()}`);
      return;
    case "network_timeout":
      logParams.set("query", "error:timeout");
      router.push(`/logs?${logParams.toString()}`);
      return;
    case "network_dns":
      logParams.set("query", "error:dns");
      router.push(`/logs?${logParams.toString()}`);
      return;
    case "network_connection":
      logParams.set("query", "error:connect");
      router.push(`/logs?${logParams.toString()}`);
      return;
    default:
      if (label) {
        logParams.set("query", label);
      }
      router.push(logParams.size > 0 ? `/logs?${logParams.toString()}` : "/logs");
    }
}

function describeFailureDrilldownTarget(code: string): string {
  switch (String(code || "").trim().toLowerCase()) {
    case "account_deactivated":
    case "workspace_deactivated":
    case "refresh_token_expired":
    case "refresh_token_reused":
    case "refresh_token_invalidated":
    case "refresh_token_invalid":
    case "usage_unauthorized":
      return "查看账号页";
    case "register_email_otp_timeout":
    case "register_email_otp_invalid":
    case "register_phone_required":
    case "register_proxy_error":
      return "查看注册页";
    default:
      return "查看日志页";
  }
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

function countRecoverableRegisterFailures(
  items: Array<{ code: string; count: number }>
): number {
  return items.reduce((sum, item) => {
    if (!String(item.code || "").trim().toLowerCase().startsWith("register_")) {
      return sum;
    }
    return recommendedRetryStrategyForFailure(item.code) == null
      ? sum
      : sum + Math.max(0, item.count || 0);
  }, 0);
}

function formatPercent(value: number | null | undefined): string {
  return value == null ? "--" : `${Math.max(0, Math.round(value))}%`;
}

function formatPredictionDuration(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "--";
  if (value <= 0) return "已触发";
  if (value < 1) {
    return `${Math.max(1, Math.round(value * 60))} 分钟`;
  }
  if (value < 24) {
    return `${value.toFixed(value >= 10 ? 0 : 1).replace(/\.0$/, "")} 小时`;
  }
  const days = value / 24;
  return `${days.toFixed(days >= 10 ? 0 : 1).replace(/\.0$/, "")} 天`;
}

function formatPredictionBucket(value: string | null | undefined): string {
  if (!value) return "未知窗口";
  return value === "secondary" ? "7天窗口" : "5小时窗口";
}

function formatLatency(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "N/A";
  return `${Math.max(0, Math.round(value))} ms`;
}

function formatQps(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "0.00";
  return value.toFixed(value >= 10 ? 1 : 2);
}

function formatRate(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "0%";
  return `${value.toFixed(value >= 10 ? 0 : 1).replace(/\.0$/, "")}%`;
}

function formatHealthcheckRate(
  successCount: number | null | undefined,
  sampledAccounts: number | null | undefined
): string {
  if (!sampledAccounts || sampledAccounts <= 0) {
    return "--";
  }
  return formatRate(((successCount ?? 0) / sampledAccounts) * 100);
}

function buildSparklinePoints(values: number[], width: number, height: number): string {
  if (!values.length) return "";
  const max = Math.max(...values, 1);
  if (values.length === 1) {
    return `0,${height / 2} ${width},${height / 2}`;
  }
  return values
    .map((value, index) => {
      const x = (index / (values.length - 1)) * width;
      const y = height - (Math.max(0, value) / max) * height;
      return `${x},${y}`;
    })
    .join(" ");
}

function MiniTrendChart({ points }: MiniTrendChartProps) {
  const width = 640;
  const height = 180;
  const requestValues = points.map((item) => item.requestCount);
  const errorValues = points.map((item) => item.errorRate);
  const requestLine = buildSparklinePoints(requestValues, width, height);
  const errorLine = buildSparklinePoints(errorValues, width, height);
  const totalRequests = requestValues.reduce((sum, value) => sum + value, 0);
  const latest = points.at(-1);

  if (!points.length || totalRequests <= 0) {
    return (
      <div className="flex h-[180px] items-center justify-center rounded-2xl border border-dashed border-border/50 bg-muted/20 text-sm text-muted-foreground">
        最近 1 小时暂无请求趋势
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="overflow-hidden rounded-2xl border border-border/40 bg-background/40 px-3 py-4">
        <svg viewBox={`0 0 ${width} ${height}`} className="h-[180px] w-full">
          {[0.25, 0.5, 0.75].map((ratio) => (
            <line
              key={ratio}
              x1="0"
              x2={width}
              y1={height * ratio}
              y2={height * ratio}
              stroke="currentColor"
              strokeOpacity="0.08"
              strokeWidth="1"
            />
          ))}
          <polyline
            fill="none"
            stroke="rgb(59 130 246)"
            strokeWidth="3"
            strokeLinecap="round"
            strokeLinejoin="round"
            points={requestLine}
          />
          <polyline
            fill="none"
            stroke="rgb(244 63 94)"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            points={errorLine}
          />
        </svg>
      </div>
      <div className="flex flex-wrap items-center gap-3 text-[11px] text-muted-foreground">
        <span className="inline-flex items-center gap-2">
          <span className="h-2.5 w-2.5 rounded-full bg-blue-500" />
          请求量
        </span>
        <span className="inline-flex items-center gap-2">
          <span className="h-2.5 w-2.5 rounded-full bg-rose-500" />
          错误率
        </span>
        {latest ? (
          <span className="rounded-full bg-muted/50 px-2 py-1">
            最近 1 分钟 {latest.requestCount} 次请求 / 错误率 {formatRate(latest.errorRate)}
          </span>
        ) : null}
      </div>
    </div>
  );
}

function PercentBar({ label, value, tone = "default" }: PercentBarProps) {
  const normalized = value == null ? 0 : Math.max(0, Math.min(100, Math.round(value)));
  const colorClass =
    tone === "green"
      ? "bg-green-500"
      : tone === "blue"
        ? "bg-blue-500"
        : "bg-primary";

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between text-[10px]">
        <span className="text-muted-foreground">{label}</span>
        <span className="font-semibold">{formatPercent(value)}</span>
      </div>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted/60">
        <div
          className={cn("h-full rounded-full transition-all", colorClass)}
          style={{ width: `${normalized}%` }}
        />
      </div>
    </div>
  );
}

function quotaTrackClass(tone: "green" | "blue") {
  return tone === "blue" ? "bg-blue-500/20" : "bg-green-500/20";
}

function quotaIndicatorClass(tone: "green" | "blue") {
  return tone === "blue" ? "bg-blue-500" : "bg-green-500";
}

function AccountHighlightCard({
  title,
  name,
  subtitle,
  tone = "green",
  progressLabel,
  progressValue,
  healthTier = "healthy",
  healthScore,
  onOpenAccount,
  onOpenLogs,
}: AccountHighlightCardProps) {
  const iconToneClass =
    tone === "blue"
      ? "bg-blue-500/20 text-blue-500"
      : "bg-green-500/20 text-green-500";

  return (
    <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
      <div className="flex items-center gap-4">
        <div
          className={cn(
            "flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl",
            iconToneClass,
          )}
        >
          <CheckCircle2 className="h-5 w-5" />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-[11px] font-medium text-muted-foreground">{title}</p>
          <p className="truncate text-sm font-semibold leading-5">{name}</p>
          <p className="truncate text-xs text-muted-foreground">{subtitle}</p>
        </div>
        {healthScore != null ? (
          <Badge
            variant="secondary"
            className={cn("shrink-0 px-2 py-0.5 text-[10px]", healthTierToneClass(healthTier))}
          >
            {formatHealthTierLabel(healthTier)} {healthScore}
          </Badge>
        ) : null}
      </div>
      {progressLabel ? (
        <div className="mt-3 border-t border-border/40 pt-3">
          <PercentBar label={progressLabel} value={progressValue} tone={tone} />
        </div>
      ) : null}
      {onOpenAccount || onOpenLogs ? (
        <div className="mt-3 flex flex-wrap gap-2 border-t border-border/40 pt-3">
          {onOpenAccount ? (
            <Button
              variant="outline"
              size="sm"
              className="h-8 rounded-lg px-3 text-xs"
              onClick={onOpenAccount}
            >
              查看账号
            </Button>
          ) : null}
          {onOpenLogs ? (
            <Button
              variant="ghost"
              size="sm"
              className="h-8 rounded-lg px-3 text-xs"
              onClick={onOpenLogs}
            >
              查看日志
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function StatProgressCard({
  title,
  value,
  total,
  icon: Icon,
  color,
  sub,
  onClick,
}: StatProgressCardProps) {
  const percentage = total > 0 ? Math.min(Math.round((value / total) * 100), 100) : 0;

  return (
    <Card
      className={cn(
        "glass-card overflow-hidden border-none shadow-md backdrop-blur-md transition-all hover:scale-[1.02]",
        onClick ? "cursor-pointer hover:shadow-lg" : "",
      )}
      onClick={onClick}
    >
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        <Icon className={cn("h-4 w-4", color)} />
      </CardHeader>
      <CardContent className="space-y-3">
        <div>
          <div className="text-2xl font-bold">{value}</div>
          <p className="mt-1 text-[10px] text-muted-foreground">{sub}</p>
        </div>
        <div className="space-y-1">
          <div className="flex items-center justify-between text-[10px]">
            <span className="text-muted-foreground">占比</span>
            <span className="font-mono font-medium">{percentage}%</span>
          </div>
          <Progress value={percentage} className="h-1.5" />
        </div>
      </CardContent>
    </Card>
  );
}

export default function DashboardPage() {
  const router = useRouter();
  const queryClient = useQueryClient();
  const {
    accounts,
    stats,
    currentAccount,
    recommendations,
    failureReasonSummary,
    governanceSummary,
    operationAudits,
    dashboardHealth,
    dashboardTrend,
    requestLogs,
    isLoading,
    isDashboardLoading,
    isServiceReady,
  } = useDashboardStats();
  const poolPrimary = stats.poolRemain?.primary ?? 0;
  const poolSecondary = stats.poolRemain?.secondary ?? 0;
  const usagePrediction = stats.usagePrediction;
  const recoverableRegisterFailures = countRecoverableRegisterFailures(
    failureReasonSummary,
  );
  const healthBuckets = dashboardHealth?.accountStatusBuckets || [];
  const gatewayMetrics = dashboardHealth?.gatewayMetrics;
  const recentHealthcheck = dashboardHealth?.recentHealthcheck;
  const trendPoints = dashboardTrend?.points || [];
  const dashboardSectionLoading = isLoading || isDashboardLoading;
  const cooldownReasonSummary = useMemo(() => {
    const counts = new Map<string, number>();
    for (const account of accounts) {
      if (!account.isInCooldown) {
        continue;
      }
      const label = String(account.cooldownReason || "临时冷却").trim() || "临时冷却";
      counts.set(label, (counts.get(label) || 0) + 1);
    }
    return Array.from(counts.entries())
      .map(([label, count]) => ({ label, count }))
      .sort((left, right) => {
        if (right.count !== left.count) {
          return right.count - left.count;
        }
        return left.label.localeCompare(right.label, "zh-CN");
      });
  }, [accounts]);
  const protectedAccounts = useMemo(
    () => accounts.filter((account) => account.isNewAccountProtected),
    [accounts]
  );
  const protectedEndingSoonest = useMemo(() => {
    return protectedAccounts.reduce<number | null>((soonest, account) => {
      if (!account.newAccountProtectionUntil) {
        return soonest;
      }
      if (soonest == null || account.newAccountProtectionUntil < soonest) {
        return account.newAccountProtectionUntil;
      }
      return soonest;
    }, null);
  }, [protectedAccounts]);
  const topCooldownReason = cooldownReasonSummary[0] ?? null;
  const topGovernanceReason = governanceSummary[0] ?? null;
  const primaryRiskSummary = topCooldownReason
    ? `当前冷却账号主要集中在「${topCooldownReason.label}」，建议先排查该类异常并等待冷却释放。`
    : topGovernanceReason
      ? `最近自动治理主要由「${topGovernanceReason.label}」触发，建议优先复核对应账号。`
      : protectedAccounts.length > 0
        ? `当前有 ${protectedAccounts.length} 个新号处于保护期，路由已自动降优先级，避免过早消耗。`
        : healthBuckets.find((item) => item.key === "quota_exhausted")?.count
          ? "存在额度耗尽账号，建议补池或切换策略。"
          : healthBuckets.find((item) => item.key === "unavailable")?.count
            ? "存在不可用账号，建议查看授权与网络链路。"
            : "主链路暂无明显异常，适合继续观察自动治理结果。";

  const invalidateOperationalViews = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["accounts"] }),
      queryClient.invalidateQueries({ queryKey: ["usage"] }),
      queryClient.invalidateQueries({ queryKey: ["usage-aggregate"] }),
      queryClient.invalidateQueries({ queryKey: ["register-tasks"] }),
      queryClient.invalidateQueries({ queryKey: ["register-stats"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
      queryClient.invalidateQueries({ queryKey: ["logs"] }),
      queryClient.invalidateQueries({ queryKey: ["dashboard-health"] }),
      queryClient.invalidateQueries({ queryKey: ["dashboard-trend"] }),
    ]);
  };

  const refreshUsageMutation = useMutation({
    mutationFn: () => accountClient.refreshUsage(),
    onSuccess: async () => {
      await invalidateOperationalViews();
      toast.success("已开始刷新全部账号用量");
    },
    onError: (error: unknown) => {
      toast.error(`刷新全部用量失败: ${getAppErrorMessage(error)}`);
    },
  });

  const cleanupUnavailableMutation = useMutation({
    mutationFn: () => accountClient.deleteUnavailableFree(),
    onSuccess: async (result: { deleted?: number }) => {
      await invalidateOperationalViews();
      const deleted = Number(result?.deleted || 0);
      toast.success(
        deleted > 0
          ? `已清理 ${deleted} 个不可用免费账号`
          : "没有可清理的不可用免费账号"
      );
    },
    onError: (error: unknown) => {
      toast.error(`清理不可用免费号失败: ${getAppErrorMessage(error)}`);
    },
  });

  const syncFreeProxyMutation = useMutation({
    mutationFn: () =>
      serviceClient.syncFreeProxyPool({ syncRegisterProxyPool: true }),
    onSuccess: async (result) => {
      await invalidateOperationalViews();
      toast.success(
        `已同步 ${result.appliedCount} 个代理，注册代理池总数 ${result.registerProxyTotalCount}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`同步 freeproxy 失败: ${getAppErrorMessage(error)}`);
    },
  });

  const retryRecoverableRegisterMutation = useMutation({
    mutationFn: async () => {
      const result = await accountClient.listRegisterTasks({
        page: 1,
        pageSize: 100,
        status: "failed",
      });
      const recoverableTasks = result.tasks.filter(
        (task) => recommendedRetryStrategyForFailure(task.failureCode) != null,
      );
      for (const task of recoverableTasks) {
        const strategy = recommendedRetryStrategyForFailure(task.failureCode);
        if (!strategy) {
          continue;
        }
        await accountClient.retryRegisterTask(task.taskUuid, strategy);
      }
      return { totalFailed: result.tasks.length, retried: recoverableTasks.length };
    },
    onSuccess: async (result) => {
      await invalidateOperationalViews();
      toast.success(
        result.retried > 0
          ? `已按策略重试 ${result.retried} 个失败注册任务`
          : `最近 100 条失败注册里没有可自动恢复的任务（共 ${result.totalFailed} 条失败）`
      );
    },
    onError: (error: unknown) => {
      toast.error(`批量重试失败注册任务失败: ${getAppErrorMessage(error)}`);
    },
  });

  const openAccountById = (accountId: string) => {
    const params = new URLSearchParams();
    params.set("query", accountId);
    router.push(`/accounts?${params.toString()}`);
  };

  const openAccountsByStatus = (
    status:
      | "all"
      | "available"
      | "low_quota"
      | "cooldown"
      | "protected"
      | "deactivated"
      | "governed",
  ) => {
    if (status === "all") {
      router.push("/accounts");
      return;
    }
    const params = new URLSearchParams();
    params.set("status", status);
    router.push(`/accounts?${params.toString()}`);
  };

  const openCooldownAccounts = (cooldownReason?: string) => {
    const params = new URLSearchParams();
    params.set("status", "cooldown");
    if (cooldownReason) {
      params.set("cooldownReason", cooldownReason);
    }
    router.push(`/accounts?${params.toString()}`);
  };

  const openLogsByAccountId = (accountId: string) => {
    const params = new URLSearchParams();
    params.set("query", `account:=${accountId}`);
    router.push(`/logs?${params.toString()}`);
  };

  const openGovernedAccounts = (governanceReason?: string) => {
    const params = new URLSearchParams();
    params.set("status", "governed");
    if (governanceReason) {
      params.set("governanceReason", governanceReason);
    }
    router.push(`/accounts?${params.toString()}`);
  };

  const toolboxActions = [
    {
      title: "刷新全部用量",
      description: "重新拉取当前号池的额度和状态，适合刚同步账号或代理后执行。",
      icon: RefreshCw,
      tone: "text-sky-500",
      disabled: !isServiceReady || refreshUsageMutation.isPending,
      pending: refreshUsageMutation.isPending,
      onClick: () => refreshUsageMutation.mutate(),
    },
    {
      title: "重试可恢复注册失败",
      description: "扫描最近 100 条失败注册，按验证码或代理策略自动批量重试。",
      icon: RotateCcw,
      tone: "text-emerald-500",
      disabled: !isServiceReady || retryRecoverableRegisterMutation.isPending,
      pending: retryRecoverableRegisterMutation.isPending,
      onClick: () => retryRecoverableRegisterMutation.mutate(),
    },
    {
      title: "清理不可用免费号",
      description: "移除已不可用且无保留价值的免费账号，减少坏号污染。",
      icon: Trash2,
      tone: "text-amber-500",
      disabled: !isServiceReady || cleanupUnavailableMutation.isPending,
      pending: cleanupUnavailableMutation.isPending,
      onClick: () => cleanupUnavailableMutation.mutate(),
    },
    {
      title: "同步 freeproxy",
      description: "从 freeproxy 拉取最新代理，并同步写入注册代理池。",
      icon: Sparkles,
      tone: "text-fuchsia-500",
      disabled: !isServiceReady || syncFreeProxyMutation.isPending,
      pending: syncFreeProxyMutation.isPending,
      onClick: () => syncFreeProxyMutation.mutate(),
    },
  ] as const;

  return (
    <div className="space-y-6 animate-in fade-in duration-700">
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {isLoading ? (
          Array.from({ length: 4 }).map((_, index) => (
            <Skeleton key={index} className="h-36 w-full rounded-2xl" />
          ))
        ) : (
          <>
            <Card
              className="glass-card cursor-pointer overflow-hidden border-none shadow-md backdrop-blur-md transition-all hover:scale-[1.02] hover:shadow-lg"
              onClick={() => openAccountsByStatus("all")}
            >
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">总账号数</CardTitle>
                <Users className="h-4 w-4 text-blue-500" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats.total}</div>
                <p className="mt-1 text-[10px] text-muted-foreground">池中所有配置账号</p>
                <div className="mt-4 flex w-fit items-center gap-2 rounded-full bg-blue-500/10 px-2 py-0.5 text-[10px] text-blue-600 dark:text-blue-400">
                  <Activity className="h-3 w-3" />
                  最近日志 {requestLogs.length} 条
                </div>
                <div className="mt-3 flex flex-wrap gap-2 text-[10px]">
                  <span className="rounded-full bg-emerald-500/10 px-2 py-0.5 text-emerald-700 dark:text-emerald-300">
                    优秀 {stats.healthy}
                  </span>
                  <span className="rounded-full bg-amber-500/10 px-2 py-0.5 text-amber-700 dark:text-amber-300">
                    预警 {stats.warning}
                  </span>
                  <span className="rounded-full bg-rose-500/10 px-2 py-0.5 text-rose-700 dark:text-rose-300">
                    风险 {stats.risky}
                  </span>
                  <span className="rounded-full bg-fuchsia-500/10 px-2 py-0.5 text-fuchsia-700 dark:text-fuchsia-300">
                    隔离 {stats.isolated}
                  </span>
                </div>
              </CardContent>
            </Card>

            <StatProgressCard
              title="可用账号"
              value={stats.available}
              total={stats.total}
              icon={CheckCircle2}
              color="text-green-500"
              sub="当前健康可调用的账号"
              onClick={() => openAccountsByStatus("available")}
            />

            <StatProgressCard
              title="不可用账号"
              value={stats.unavailable}
              total={stats.total}
              icon={XCircle}
              color="text-red-500"
              sub={`额度耗尽、授权失效或已隔离（当前隔离 ${stats.isolated}）`}
              onClick={() => openAccountsByStatus("deactivated")}
            />

            <Card
              className="cursor-pointer overflow-hidden border-none bg-primary/10 shadow-md backdrop-blur-md transition-all hover:scale-[1.02] hover:shadow-lg"
              onClick={() => openAccountsByStatus("low_quota")}
            >
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium text-primary">账号池剩余</CardTitle>
                <PieChart className="h-4 w-4 text-primary" />
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-[10px]">
                    <span className="text-muted-foreground">5小时内</span>
                    <span className="font-bold">{formatPercent(stats.poolRemain?.primary)}</span>
                  </div>
                  <Progress
                    value={poolPrimary}
                    trackClassName={quotaTrackClass("green")}
                    indicatorClassName={quotaIndicatorClass("green")}
                  />
                </div>
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-[10px]">
                    <span className="text-muted-foreground">7天内</span>
                    <span className="font-bold">{formatPercent(stats.poolRemain?.secondary)}</span>
                  </div>
                  <Progress
                    value={poolSecondary}
                    trackClassName={quotaTrackClass("blue")}
                    indicatorClassName={quotaIndicatorClass("blue")}
                  />
                </div>
              </CardContent>
            </Card>
          </>
        )}
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.15fr,0.85fr]">
        {dashboardSectionLoading ? (
          <>
            <Skeleton className="h-[320px] w-full rounded-2xl" />
            <div className="grid gap-4">
              <Skeleton className="h-[148px] w-full rounded-2xl" />
              <Skeleton className="h-[148px] w-full rounded-2xl" />
            </div>
          </>
        ) : (
          <>
            <Card className="glass-card border-none shadow-md">
              <CardHeader className="flex flex-row items-center justify-between gap-4">
                <div>
                  <CardTitle className="text-base font-semibold">实时健康仪表盘</CardTitle>
                  <p className="mt-1 text-xs text-muted-foreground">
                    汇总账号池状态、最近 5 分钟网关健康指标，以及最近 1 小时请求趋势。
                  </p>
                </div>
                <Badge variant="secondary" className="bg-primary/10 text-primary">
                  <Activity className="mr-1 h-3.5 w-3.5" />
                  30 秒刷新
                </Badge>
              </CardHeader>
              <CardContent className="space-y-5">
                <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-5">
                  {healthBuckets.map((bucket) => (
                    <div
                      key={bucket.key}
                      className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm"
                    >
                      <div className="flex items-center justify-between gap-3">
                        <div>
                          <p className="text-[11px] font-medium text-muted-foreground">
                            {bucket.label}
                          </p>
                          <p className="mt-1 text-2xl font-bold">{bucket.count}</p>
                        </div>
                        <Badge variant="secondary" className="shrink-0">
                          {bucket.percent}%
                        </Badge>
                      </div>
                      <div className="mt-3 h-1.5 w-full overflow-hidden rounded-full bg-muted/60">
                        <div
                          className="h-full rounded-full bg-primary transition-all"
                          style={{ width: `${Math.max(0, Math.min(100, bucket.percent))}%` }}
                        />
                      </div>
                    </div>
                  ))}
                </div>
                <MiniTrendChart points={trendPoints} />
              </CardContent>
            </Card>

            <div className="grid gap-4">
              <Card className="glass-card border-none shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">最近 5 分钟网关指标</CardTitle>
                </CardHeader>
                <CardContent className="grid gap-3 sm:grid-cols-3">
                  <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                    <p className="text-[11px] font-medium text-muted-foreground">当前 QPS</p>
                    <p className="mt-2 text-2xl font-bold">{formatQps(gatewayMetrics?.qps)}</p>
                    <p className="mt-2 text-[11px] text-muted-foreground">
                      {gatewayMetrics?.totalRequests ?? 0} 次请求 / 5 分钟
                    </p>
                  </div>
                  <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                    <p className="text-[11px] font-medium text-muted-foreground">成功率</p>
                    <p className="mt-2 text-2xl font-bold">
                      {formatRate(gatewayMetrics?.successRate)}
                    </p>
                    <p className="mt-2 text-[11px] text-muted-foreground">
                      成功 {gatewayMetrics?.successRequests ?? 0} / 错误{" "}
                      {gatewayMetrics?.errorRequests ?? 0}
                    </p>
                  </div>
                  <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                    <p className="text-[11px] font-medium text-muted-foreground">P95 延迟</p>
                    <p className="mt-2 text-2xl font-bold">
                      {formatLatency(gatewayMetrics?.p95LatencyMs)}
                    </p>
                    <p className="mt-2 text-[11px] text-muted-foreground">
                      P50 {formatLatency(gatewayMetrics?.p50LatencyMs)} / P99{" "}
                      {formatLatency(gatewayMetrics?.p99LatencyMs)}
                    </p>
                  </div>
                </CardContent>
              </Card>

              <Card className="glass-card border-none shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">观测摘要</CardTitle>
                </CardHeader>
                <CardContent className="space-y-3 text-sm">
                  <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                    <p className="text-[11px] font-medium text-muted-foreground">
                      当前主要风险
                    </p>
                    <p className="mt-2 font-medium">{primaryRiskSummary}</p>
                  </div>
                  <div className="grid gap-3 sm:grid-cols-3">
                    <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        冷却中账号
                      </p>
                      <p className="mt-2 text-xl font-bold">
                        {healthBuckets.find((item) => item.key === "cooldown")?.count ?? 0}
                      </p>
                    </div>
                    <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        最新刷新时间
                      </p>
                      <p className="mt-2 text-sm font-medium">
                        {formatTsFromSeconds(dashboardHealth?.generatedAt, "刚启动")}
                      </p>
                    </div>
                    <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        最近巡检
                      </p>
                      <p className="mt-2 text-sm font-semibold">
                        {formatHealthcheckRate(
                          recentHealthcheck?.successCount,
                          recentHealthcheck?.sampledAccounts
                        )}
                      </p>
                      <p className="mt-2 text-[11px] text-muted-foreground">
                        通过 {recentHealthcheck?.successCount ?? 0} / 抽检{" "}
                        {recentHealthcheck?.sampledAccounts ?? 0}
                      </p>
                      <p className="mt-1 text-[11px] text-muted-foreground">
                        {formatTsFromSeconds(recentHealthcheck?.finishedAt, "尚未执行")}
                      </p>
                    </div>
                  </div>
                  <div className="grid gap-3 sm:grid-cols-3">
                    <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        冷却原因热点
                      </p>
                      <p className="mt-2 text-sm font-semibold">
                        {topCooldownReason?.label || "暂无冷却账号"}
                      </p>
                      <p className="mt-2 text-[11px] text-muted-foreground">
                        {topCooldownReason
                          ? `${topCooldownReason.count} 个账号处于该类冷却`
                          : "当前没有账号处于 cooldown"}
                      </p>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="mt-3 h-auto px-0 text-xs text-sky-600 hover:bg-transparent hover:text-sky-600 dark:text-sky-400 dark:hover:text-sky-400"
                        onClick={() => openCooldownAccounts(topCooldownReason?.label)}
                      >
                        查看账号页
                      </Button>
                    </div>
                    <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        最近治理原因
                      </p>
                      <p className="mt-2 text-sm font-semibold">
                        {topGovernanceReason?.label || "最近无治理命中"}
                      </p>
                      <p className="mt-2 text-[11px] text-muted-foreground">
                        {topGovernanceReason
                          ? `${topGovernanceReason.affectedAccounts} 个账号受影响`
                          : "最近 24 小时没有自动治理事件"}
                      </p>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="mt-3 h-auto px-0 text-xs text-rose-600 hover:bg-transparent hover:text-rose-600 dark:text-rose-400 dark:hover:text-rose-400"
                        onClick={() =>
                          topGovernanceReason
                            ? openGovernedAccounts(topGovernanceReason.label)
                            : openGovernedAccounts()
                        }
                      >
                        查看治理账号
                      </Button>
                    </div>
                    <div className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm">
                      <p className="text-[11px] font-medium text-muted-foreground">
                        新号保护概览
                      </p>
                      <p className="mt-2 text-sm font-semibold">
                        {protectedAccounts.length > 0
                          ? `${protectedAccounts.length} 个新号保护中`
                          : "当前无保护中新号"}
                      </p>
                      <p className="mt-2 text-[11px] text-muted-foreground">
                        {protectedAccounts.length > 0
                          ? `最早释放时间 ${formatTsFromSeconds(
                              protectedEndingSoonest,
                              "--"
                            )}`
                          : "保护期账号会自动排在成熟账号之后"}
                      </p>
                      <p className="mt-3 text-[11px] text-muted-foreground">
                        {protectedAccounts.length > 0
                          ? "首页已识别保护态，账号页可继续查看单号明细。"
                          : "新注册账号进入保护期后会在这里汇总展示。"}
                      </p>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="mt-3 h-auto px-0 text-xs text-cyan-700 hover:bg-transparent hover:text-cyan-700 dark:text-cyan-300 dark:hover:text-cyan-300"
                        onClick={() => openAccountsByStatus("protected")}
                      >
                        查看保护账号
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </div>
          </>
        )}
      </div>

      <Card className="glass-card border-none shadow-md">
        <CardHeader className="flex flex-row items-center justify-between gap-4">
          <div>
            <CardTitle className="text-base font-semibold">一键修复工具箱</CardTitle>
            <p className="mt-1 text-xs text-muted-foreground">
              把常用修复动作集中到首页，适合代理失效、注册堆积或号池状态异常时快速处置。
            </p>
          </div>
          <Badge variant="secondary" className="bg-primary/10 text-primary">
            <Wrench className="mr-1 h-3.5 w-3.5" />
            运维修复
          </Badge>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
            {toolboxActions.map((action) => (
              <div
                key={action.title}
                className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className={cn("rounded-2xl bg-background/80 p-2", action.tone)}>
                    <action.icon className="h-4 w-4" />
                  </div>
                  {action.pending ? <Badge variant="secondary">处理中</Badge> : null}
                </div>
                <p className="mt-3 text-sm font-semibold">{action.title}</p>
                <p className="mt-2 min-h-[54px] text-[11px] leading-5 text-muted-foreground">
                  {action.description}
                </p>
                <Button
                  className="mt-3 w-full"
                  variant="outline"
                  disabled={action.disabled}
                  onClick={action.onClick}
                >
                  {action.pending ? "执行中..." : "立即执行"}
                </Button>
              </div>
            ))}
          </div>
          <div className="mt-3 flex flex-wrap gap-2 text-[11px] text-muted-foreground">
            <span className="rounded-full bg-muted/50 px-2 py-1">
              可恢复失败注册估算：{recoverableRegisterFailures} 条
            </span>
            <span className="rounded-full bg-muted/50 px-2 py-1">
              当前隔离账号：{stats.isolated}
            </span>
            <span className="rounded-full bg-muted/50 px-2 py-1">
              最近治理命中：{stats.recentGovernanceTotal}
            </span>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-2">
        {isLoading ? (
          Array.from({ length: 2 }).map((_, index) => (
            <Skeleton key={index} className="h-36 w-full rounded-2xl" />
          ))
        ) : (
          <>
            <Card className="glass-card overflow-hidden border-none shadow-md backdrop-blur-md">
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <div>
                  <CardTitle className="text-sm font-medium">跌破保护阈值预计</CardTitle>
                  <p className="mt-1 text-[10px] text-muted-foreground">
                    按当前号池消耗速度，估算剩余额度跌破保护阈值的时间。
                  </p>
                </div>
                <ShieldAlert className="h-4 w-4 text-amber-500" />
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="text-2xl font-bold">
                  {formatPredictionDuration(usagePrediction.estimatedHoursToThreshold)}
                </div>
                <div className="flex flex-wrap gap-2 text-[10px]">
                  <span className="rounded-full bg-amber-500/10 px-2 py-0.5 text-amber-700 dark:text-amber-300">
                    阈值 {usagePrediction.quotaProtectionThresholdPercent}%
                  </span>
                  <span className="rounded-full bg-blue-500/10 px-2 py-0.5 text-blue-700 dark:text-blue-300">
                    受限于 {formatPredictionBucket(usagePrediction.thresholdLimitedBy)}
                  </span>
                  <span
                    className={cn(
                      "rounded-full px-2 py-0.5",
                      usagePrediction.quotaProtectionEnabled
                        ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
                        : "bg-muted text-muted-foreground"
                    )}
                  >
                    {usagePrediction.quotaProtectionEnabled
                      ? "保护已开启"
                      : "保护未开启，仅按阈值测算"}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground">
                  当前满足保护阈值的可路由账号还有 {usagePrediction.readyAccountCount} 个。
                </p>
              </CardContent>
            </Card>

            <Card className="glass-card overflow-hidden border-none shadow-md backdrop-blur-md">
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <div>
                  <CardTitle className="text-sm font-medium">号池可支撑时间</CardTitle>
                  <p className="mt-1 text-[10px] text-muted-foreground">
                    不看保护阈值，按当前耗用速度估算整个号池被吃空前还能撑多久。
                  </p>
                </div>
                <Activity className="h-4 w-4 text-blue-500" />
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="text-2xl font-bold">
                  {formatPredictionDuration(usagePrediction.estimatedHoursToPoolExhaustion)}
                </div>
                <div className="flex flex-wrap gap-2 text-[10px]">
                  <span className="rounded-full bg-blue-500/10 px-2 py-0.5 text-blue-700 dark:text-blue-300">
                    受限于 {formatPredictionBucket(usagePrediction.poolLimitedBy)}
                  </span>
                  <span className="rounded-full bg-emerald-500/10 px-2 py-0.5 text-emerald-700 dark:text-emerald-300">
                    可用池 {stats.available} 个
                  </span>
                </div>
                <p className="text-xs text-muted-foreground">
                  这是保守版预测，基于最近一次额度快照倒推平均消耗速度，适合看补池时机，不代表精确 SLA。
                </p>
              </CardContent>
            </Card>
          </>
        )}
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {[ 
          {
            title: "今日令牌",
            value: formatCompactNumber(stats.todayTokens, "0"),
            icon: Zap,
            color: "text-yellow-500",
            sub: "输入 + 输出合计",
          },
          {
            title: "缓存令牌",
            value: formatCompactNumber(stats.cachedTokens, "0"),
            icon: Database,
            color: "text-indigo-500",
            sub: "上下文缓存命中",
          },
          {
            title: "推理令牌",
            value: formatCompactNumber(stats.reasoningTokens, "0"),
            icon: BrainCircuit,
            color: "text-purple-500",
            sub: "大模型思考过程",
          },
          {
            title: "预计费用",
            value: `$${Number(stats.todayCost || 0).toFixed(2)}`,
            icon: DollarSign,
            color: "text-emerald-500",
            sub: "按官价估算",
          },
        ].map((card) => (
          isLoading ? (
            <Skeleton key={card.title} className="h-32 w-full rounded-2xl" />
          ) : (
            <Card
              key={card.title}
              className="glass-card overflow-hidden border-none shadow-md backdrop-blur-md transition-all hover:scale-[1.02]"
            >
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">{card.title}</CardTitle>
                <card.icon className={cn("h-4 w-4", card.color)} />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{card.value}</div>
                <p className="mt-1 text-[10px] text-muted-foreground">{card.sub}</p>
              </CardContent>
            </Card>
          )
        ))}
      </div>

      <Card className="glass-card border-none shadow-md">
        <CardHeader className="flex flex-row items-center justify-between gap-4">
          <div>
            <CardTitle className="text-base font-semibold">近24小时失败原因</CardTitle>
            <p className="mt-1 text-xs text-muted-foreground">
              先把失败原因收敛清楚，后面健康分、自动隔离和自动补池都会基于这里继续演进。
            </p>
          </div>
          <div className="flex items-center gap-2 rounded-full bg-amber-500/10 px-3 py-1 text-xs font-medium text-amber-600 dark:text-amber-400">
            <ShieldAlert className="h-3.5 w-3.5" />
            失败事件 {stats.recentFailureTotal}
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {Array.from({ length: 4 }).map((_, index) => (
                <Skeleton key={index} className="h-24 w-full rounded-2xl" />
              ))}
            </div>
          ) : failureReasonSummary.length > 0 ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {failureReasonSummary.slice(0, 4).map((item) => (
                <button
                  type="button"
                  key={item.code}
                  className="rounded-2xl border border-border/40 bg-accent/20 p-4 text-left shadow-sm transition-colors hover:border-amber-400/40 hover:bg-amber-500/5"
                  onClick={() => openFailureDrilldown(router, item.code, item.label)}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="truncate text-sm font-semibold">{item.label}</p>
                      <p className="mt-1 text-[11px] text-muted-foreground">
                        影响账号 {item.affectedAccounts}
                      </p>
                    </div>
                    <div className="rounded-full bg-background/80 px-2 py-1 text-xs font-bold">
                      {item.count}
                    </div>
                  </div>
                  <div className="mt-4 flex items-center justify-between text-[11px] text-muted-foreground">
                    <span>最近一次</span>
                    <span>{formatTsFromSeconds(item.lastSeenAt, "--")}</span>
                  </div>
                  <div className="mt-2 flex items-center gap-1 text-[11px] text-amber-600 dark:text-amber-400">
                    <ExternalLink className="h-3 w-3" />
                    <span>{describeFailureDrilldownTarget(item.code)}</span>
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div className="rounded-2xl bg-accent/20 p-4 text-sm text-muted-foreground">
              最近 24 小时没有记录到账号刷新失败事件。
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="glass-card border-none shadow-md">
        <CardHeader className="flex flex-row items-center justify-between gap-4">
          <div>
            <CardTitle className="text-base font-semibold">最近操作审计</CardTitle>
            <p className="mt-1 text-xs text-muted-foreground">
              这里记录最近触发的关键运维动作，先覆盖高频修复类操作，方便回溯系统刚刚做了什么。
            </p>
          </div>
          <Badge variant="secondary" className="bg-blue-500/10 text-blue-600 dark:text-blue-300">
            最近 {operationAudits.length} 条
          </Badge>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {Array.from({ length: 4 }).map((_, index) => (
                <Skeleton key={index} className="h-24 rounded-2xl" />
              ))}
            </div>
          ) : operationAudits.length > 0 ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {operationAudits.map((item) => (
                <div
                  key={`${item.action}-${item.createdAt || 0}`}
                  className="rounded-2xl border border-border/40 bg-accent/20 p-4 shadow-sm"
                >
                  <div className="flex items-start justify-between gap-3">
                    <p className="text-sm font-semibold">{item.label}</p>
                    <span className="rounded-full bg-background/80 px-2 py-1 text-[10px] text-muted-foreground">
                      {formatTsFromSeconds(item.createdAt, "--")}
                    </span>
                  </div>
                  <p className="mt-2 text-[11px] leading-5 text-muted-foreground">
                    {item.detail || "--"}
                  </p>
                </div>
              ))}
            </div>
          ) : (
            <div className="rounded-2xl bg-accent/20 p-4 text-sm text-muted-foreground">
              还没有记录到可展示的运维操作审计事件。
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="glass-card border-none shadow-md">
        <CardHeader className="flex flex-row items-center justify-between gap-4">
          <div>
            <CardTitle className="text-base font-semibold">近24小时自动治理</CardTitle>
            <p className="mt-1 text-xs text-muted-foreground">
              这里展示后台自动停用或标记停用的命中结果，方便判断治理策略是否过于激进或过于保守。
            </p>
          </div>
          <Button
            variant="ghost"
            className="h-auto rounded-full bg-rose-500/10 px-3 py-1 text-xs font-medium text-rose-600 hover:bg-rose-500/15 hover:text-rose-600 dark:text-rose-400 dark:hover:text-rose-400"
            onClick={() => openGovernedAccounts()}
          >
            <ShieldAlert className="mr-2 h-3.5 w-3.5" />
            治理命中 {stats.recentGovernanceTotal}
          </Button>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
              {Array.from({ length: 3 }).map((_, index) => (
                <Skeleton key={index} className="h-24 w-full rounded-2xl" />
              ))}
            </div>
          ) : governanceSummary.length > 0 ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
              {governanceSummary.slice(0, 3).map((item) => (
                <button
                  type="button"
                  key={item.code}
                  className="rounded-2xl border border-border/40 bg-accent/20 p-4 text-left shadow-sm transition-colors hover:border-rose-400/40 hover:bg-rose-500/5"
                  onClick={() => openGovernedAccounts(item.label)}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="truncate text-sm font-semibold">{item.label}</p>
                      <p className="mt-1 text-[11px] text-muted-foreground">
                        影响账号 {item.affectedAccounts}
                      </p>
                    </div>
                    <div className="rounded-full bg-background/80 px-2 py-1 text-xs font-bold">
                      {item.count}
                    </div>
                  </div>
                  <div className="mt-4 flex items-center justify-between text-[11px] text-muted-foreground">
                    <span>目标状态 {item.targetStatus}</span>
                    <span>{formatTsFromSeconds(item.lastSeenAt, "--")}</span>
                  </div>
                  <div className="mt-2 flex items-center gap-1 text-[11px] text-rose-600 dark:text-rose-400">
                    <ExternalLink className="h-3 w-3" />
                    <span>查看账号页</span>
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div className="rounded-2xl bg-accent/20 p-4 text-sm text-muted-foreground">
              最近 24 小时没有记录到自动治理命中事件。
            </div>
          )}
        </CardContent>
      </Card>

      <div className="grid gap-6 md:grid-cols-2">
        <Card className="glass-card min-h-[300px] border-none shadow-md">
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="text-base font-semibold">当前活跃账号</CardTitle>
          </CardHeader>
          <CardContent className="flex min-h-[200px] flex-col justify-start">
            {isLoading ? (
              <div className="space-y-4">
                <Skeleton className="h-28 w-full rounded-2xl" />
                <div className="grid grid-cols-2 gap-4">
                  <Skeleton className="h-32 w-full rounded-xl" />
                  <Skeleton className="h-32 w-full rounded-xl" />
                </div>
              </div>
            ) : currentAccount ? (
              <div className="space-y-4">
                <AccountHighlightCard
                  title="当前活跃账号"
                  name={currentAccount.name}
                  subtitle={currentAccount.id}
                  tone="green"
                  healthTier={currentAccount.healthTier}
                  healthScore={currentAccount.healthScore}
                  onOpenAccount={() => openAccountById(currentAccount.id)}
                  onOpenLogs={() => openLogsByAccountId(currentAccount.id)}
                />
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div className="space-y-3 rounded-xl bg-muted/30 p-4">
                    <p className="text-xs text-muted-foreground">5小时剩余</p>
                    <p className="text-lg font-bold">{formatPercent(currentAccount.primaryRemainPercent)}</p>
                    <PercentBar label="剩余额度" value={currentAccount.primaryRemainPercent} tone="green" />
                  </div>
                  <div className="space-y-3 rounded-xl bg-muted/30 p-4">
                    <p className="text-xs text-muted-foreground">7天剩余</p>
                    <p className="text-lg font-bold">{formatPercent(currentAccount.secondaryRemainPercent)}</p>
                    <PercentBar label="剩余额度" value={currentAccount.secondaryRemainPercent} tone="blue" />
                  </div>
                </div>
              </div>
            ) : (
              <div className="flex h-full flex-col items-center justify-center gap-2 text-sm text-muted-foreground">
                <div className="rounded-full bg-accent/30 p-4 animate-pulse">
                  <Activity className="h-8 w-8 opacity-20" />
                </div>
                <p>{isServiceReady ? "暂无可识别的活跃账号" : "正在等待服务连接"}</p>
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="glass-card min-h-[300px] border-none shadow-md">
          <CardHeader>
            <CardTitle className="text-base font-semibold">智能推荐</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <p className="text-xs text-muted-foreground">
              基于当前配额，系统会优先推荐剩余额度更高且仍可参与路由的账号。
            </p>
            {isLoading ? (
              <div className="space-y-4">
                <Skeleton className="h-28 w-full rounded-2xl" />
                <Skeleton className="h-28 w-full rounded-2xl" />
              </div>
            ) : recommendations.primaryPick || recommendations.secondaryPick ? (
              <>
                {recommendations.primaryPick ? (
                  <AccountHighlightCard
                    title="5小时优先账号"
                    name={recommendations.primaryPick.name}
                    subtitle={recommendations.primaryPick.id}
                    tone="green"
                    progressLabel="剩余额度"
                    progressValue={recommendations.primaryPick.primaryRemainPercent}
                    healthTier={recommendations.primaryPick.healthTier}
                    healthScore={recommendations.primaryPick.healthScore}
                    onOpenAccount={() =>
                      openAccountById(recommendations.primaryPick!.id)
                    }
                    onOpenLogs={() =>
                      openLogsByAccountId(recommendations.primaryPick!.id)
                    }
                  />
                ) : null}
                {recommendations.secondaryPick ? (
                  <AccountHighlightCard
                    title="7天优先账号"
                    name={recommendations.secondaryPick.name}
                    subtitle={recommendations.secondaryPick.id}
                    tone="blue"
                    progressLabel="剩余额度"
                    progressValue={recommendations.secondaryPick.secondaryRemainPercent}
                    healthTier={recommendations.secondaryPick.healthTier}
                    healthScore={recommendations.secondaryPick.healthScore}
                    onOpenAccount={() =>
                      openAccountById(recommendations.secondaryPick!.id)
                    }
                    onOpenLogs={() =>
                      openLogsByAccountId(recommendations.secondaryPick!.id)
                    }
                  />
                ) : null}
              </>
            ) : (
              <div className="rounded-xl bg-accent/20 p-4 text-sm text-muted-foreground">
                {isServiceReady ? "当前没有可推荐的可用账号。" : "正在等待服务连接。"}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
