"use client";

import {
  Activity,
  BrainCircuit,
  CheckCircle2,
  Database,
  DollarSign,
  ExternalLink,
  PieChart,
  ShieldAlert,
  Users,
  XCircle,
  Zap,
  type LucideIcon,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { useDashboardStats } from "@/hooks/useDashboardStats";
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
      accountParams.set("status", "deactivated");
      accountParams.set("statusReason", "检测到账号已停用");
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
  const {
    stats,
    currentAccount,
    recommendations,
    failureReasonSummary,
    governanceSummary,
    requestLogs,
    isLoading,
    isServiceReady,
  } = useDashboardStats();
  const poolPrimary = stats.poolRemain?.primary ?? 0;
  const poolSecondary = stats.poolRemain?.secondary ?? 0;
  const usagePrediction = stats.usagePrediction;

  const openAccountById = (accountId: string) => {
    const params = new URLSearchParams();
    params.set("query", accountId);
    router.push(`/accounts?${params.toString()}`);
  };

  const openAccountsByStatus = (
    status: "all" | "available" | "low_quota" | "deactivated" | "governed",
  ) => {
    if (status === "all") {
      router.push("/accounts");
      return;
    }
    const params = new URLSearchParams();
    params.set("status", status);
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
