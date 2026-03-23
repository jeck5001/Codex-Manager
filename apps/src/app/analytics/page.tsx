"use client";

import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Activity,
  CalendarRange,
  Flame,
  Grid2X2,
  LineChart,
  Loader2,
  Orbit,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { serviceClient } from "@/lib/api/service-client";
import { useAppStore } from "@/lib/store/useAppStore";
import type {
  HeatmapCellItem,
  HeatmapTrendResult,
  ModelTrendResult,
  RequestTrendResult,
} from "@/types";

const WEEKDAY_LABELS = ["日", "一", "二", "三", "四", "五", "六"];

function formatRangeLabel(start: number, end: number) {
  if (!start || !end) return "暂无时间范围";
  const startLabel = new Date(start * 1000).toLocaleDateString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
  });
  const endLabel = new Date(end * 1000).toLocaleDateString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
  });
  return `${startLabel} - ${endLabel}`;
}

function buildPolyline(values: number[], width: number, height: number) {
  if (!values.length) return "";
  const maxValue = Math.max(...values, 1);
  const step = values.length <= 1 ? width : width / (values.length - 1);
  return values
    .map((value, index) => {
      const x = index * step;
      const y = height - (value / maxValue) * height;
      return `${x},${y}`;
    })
    .join(" ");
}

function formatBucketLabel(bucket: string, granularity: string) {
  if (granularity === "month") return bucket;
  if (granularity === "week") return bucket.replace(/^(\d{4})-/, "");
  return bucket.slice(5);
}

function RequestTrendChart({
  data,
  granularity,
}: {
  data: RequestTrendResult | undefined;
  granularity: string;
}) {
  const items = data?.items ?? [];
  if (!items.length) {
    return (
      <div className="flex h-[280px] items-center justify-center rounded-3xl border border-dashed border-border/60 bg-background/20 text-sm text-muted-foreground">
        当前范围暂无请求趋势
      </div>
    );
  }

  const recentItems = items.slice(-16);
  const requestPoints = buildPolyline(
    recentItems.map((item) => item.requestCount),
    640,
    180
  );
  const successPoints = recentItems
    .map((item, index) => {
      const step = recentItems.length <= 1 ? 640 : 640 / (recentItems.length - 1);
      const x = index * step;
      const y = 180 - (item.successRate / 100) * 180;
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <div className="space-y-4">
      <div className="rounded-[28px] border border-border/50 bg-background/20 p-4">
        <svg viewBox="0 0 640 220" className="h-[240px] w-full">
          <defs>
            <linearGradient id="requestArea" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.35" />
              <stop offset="100%" stopColor="#38bdf8" stopOpacity="0.02" />
            </linearGradient>
          </defs>
          {[0, 25, 50, 75, 100].map((tick) => {
            const y = 180 - (tick / 100) * 180;
            return (
              <g key={tick}>
                <line
                  x1="0"
                  y1={y}
                  x2="640"
                  y2={y}
                  stroke="currentColor"
                  strokeOpacity="0.08"
                />
                <text x="644" y={y + 4} className="fill-muted-foreground text-[10px]">
                  {tick}%
                </text>
              </g>
            );
          })}
          <polyline
            points={`0,180 ${requestPoints} 640,180`}
            fill="url(#requestArea)"
            stroke="none"
          />
          <polyline
            points={requestPoints}
            fill="none"
            stroke="#0f766e"
            strokeWidth="3"
            strokeLinejoin="round"
            strokeLinecap="round"
          />
          <polyline
            points={successPoints}
            fill="none"
            stroke="#f97316"
            strokeWidth="2.5"
            strokeDasharray="6 6"
            strokeLinejoin="round"
            strokeLinecap="round"
          />
          {recentItems.map((item, index) => {
            const step = recentItems.length <= 1 ? 640 : 640 / (recentItems.length - 1);
            const x = index * step;
            return (
              <text
                key={`${item.bucket}-${x}`}
                x={x}
                y="212"
                textAnchor={index === 0 ? "start" : index === recentItems.length - 1 ? "end" : "middle"}
                className="fill-muted-foreground text-[10px]"
              >
                {formatBucketLabel(item.bucket, granularity)}
              </text>
            );
          })}
        </svg>
      </div>
      <div className="flex flex-wrap gap-3 text-xs text-muted-foreground">
        <span className="inline-flex items-center gap-2 rounded-full bg-muted/50 px-3 py-1">
          <span className="h-2.5 w-2.5 rounded-full bg-teal-700" />
          请求量走势
        </span>
        <span className="inline-flex items-center gap-2 rounded-full bg-muted/50 px-3 py-1">
          <span className="h-2.5 w-2.5 rounded-full bg-orange-500" />
          成功率走势
        </span>
      </div>
    </div>
  );
}

function ModelTrendPanel({ data }: { data: ModelTrendResult | undefined }) {
  const items = (data?.items ?? []).slice(0, 8);
  const total = items.reduce((sum, item) => sum + item.requestCount, 0);

  if (!items.length || total <= 0) {
    return (
      <div className="flex h-[280px] items-center justify-center rounded-3xl border border-dashed border-border/60 bg-background/20 text-sm text-muted-foreground">
        当前范围暂无模型分布
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {items.map((item) => {
        const width = Math.max(8, (item.requestCount / total) * 100);
        return (
          <div
            key={item.model}
            className="rounded-3xl border border-border/50 bg-background/20 p-4"
          >
            <div className="mb-3 flex items-center justify-between gap-3">
              <span className="truncate font-mono text-sm">{item.model}</span>
              <Badge variant="secondary">{item.successRate.toFixed(1)}%</Badge>
            </div>
            <div className="h-3 overflow-hidden rounded-full bg-muted/50">
              <div
                className="h-full rounded-full bg-gradient-to-r from-sky-500 via-cyan-400 to-emerald-400"
                style={{ width: `${width}%` }}
              />
            </div>
            <div className="mt-3 flex items-center justify-between text-xs text-muted-foreground">
              <span>{item.requestCount.toLocaleString()} 次请求</span>
              <span>{item.successCount.toLocaleString()} 次成功</span>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function buildHeatmapLookup(items: HeatmapCellItem[]) {
  const lookup = new Map<string, HeatmapCellItem>();
  for (const item of items) {
    lookup.set(`${item.weekday}-${item.hour}`, item);
  }
  return lookup;
}

function HeatmapPanel({ data }: { data: HeatmapTrendResult | undefined }) {
  const items = data?.items ?? [];
  if (!items.length) {
    return (
      <div className="flex h-[320px] items-center justify-center rounded-3xl border border-dashed border-border/60 bg-background/20 text-sm text-muted-foreground">
        当前范围暂无热力图数据
      </div>
    );
  }

  const lookup = buildHeatmapLookup(items);
  const maxRequests = Math.max(...items.map((item) => item.requestCount), 1);

  return (
    <div className="space-y-4">
      <div className="overflow-x-auto rounded-[28px] border border-border/50 bg-background/20 p-4">
        <div className="grid min-w-[960px] grid-cols-[80px_repeat(24,minmax(0,1fr))] gap-2">
          <div />
          {Array.from({ length: 24 }, (_, hour) => (
            <div key={`hour-${hour}`} className="text-center text-[10px] text-muted-foreground">
              {hour}
            </div>
          ))}
          {WEEKDAY_LABELS.map((weekdayLabel, weekday) => (
            <div key={`row-${weekday}`} className="contents">
              <div className="flex items-center text-xs font-medium text-muted-foreground">
                星期{weekdayLabel}
              </div>
              {Array.from({ length: 24 }, (_, hour) => {
                const cell = lookup.get(`${weekday}-${hour}`);
                const intensity = cell ? cell.requestCount / maxRequests : 0;
                const alpha = cell ? 0.12 + intensity * 0.88 : 0.05;
                return (
                  <div
                    key={`${weekday}-${hour}`}
                    className="flex aspect-square items-center justify-center rounded-xl border border-border/20 text-[10px] text-foreground/80"
                    style={{
                      background: `rgba(14, 165, 233, ${alpha})`,
                    }}
                    title={
                      cell
                        ? `星期${weekdayLabel} ${hour}:00\n请求 ${cell.requestCount} 次\n成功率 ${cell.successRate.toFixed(1)}%`
                        : `星期${weekdayLabel} ${hour}:00\n暂无请求`
                    }
                  >
                    {cell?.requestCount ?? ""}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
        <span>颜色越深表示请求越密集</span>
        <Badge variant="outline">悬停可查看成功率</Badge>
      </div>
    </div>
  );
}

function SummarySkeleton() {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      {Array.from({ length: 4 }).map((_, index) => (
        <Card key={index} className="glass-card border-border/50">
          <CardHeader className="space-y-3">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-8 w-28" />
          </CardHeader>
        </Card>
      ))}
    </div>
  );
}

export default function AnalyticsPage() {
  const { serviceStatus } = useAppStore();
  const [preset, setPreset] = useState("30d");
  const [granularity, setGranularity] = useState("day");
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");

  const startTs = useMemo(() => {
    if (!startDate) return null;
    const parsed = new Date(`${startDate}T00:00:00`);
    return Number.isNaN(parsed.getTime()) ? null : Math.floor(parsed.getTime() / 1000);
  }, [startDate]);

  const endTs = useMemo(() => {
    if (!endDate) return null;
    const parsed = new Date(`${endDate}T23:59:59`);
    return Number.isNaN(parsed.getTime()) ? null : Math.floor(parsed.getTime() / 1000) + 1;
  }, [endDate]);

  const canQuery =
    serviceStatus.connected &&
    (preset !== "custom" || (startTs != null && endTs != null && endTs > startTs));

  const requestTrendQuery = useQuery({
    queryKey: ["analytics", "requests", preset, granularity, startTs, endTs],
    queryFn: () =>
      serviceClient.getRequestTrends({
        preset,
        granularity,
        startTs,
        endTs,
      }),
    enabled: canQuery,
    staleTime: 30_000,
    retry: 1,
  });

  const modelTrendQuery = useQuery({
    queryKey: ["analytics", "models", preset, startTs, endTs],
    queryFn: () =>
      serviceClient.getModelTrends({
        preset,
        startTs,
        endTs,
      }),
    enabled: canQuery,
    staleTime: 30_000,
    retry: 1,
  });

  const heatmapQuery = useQuery({
    queryKey: ["analytics", "heatmap", preset, startTs, endTs],
    queryFn: () =>
      serviceClient.getHeatmapTrends({
        preset,
        startTs,
        endTs,
      }),
    enabled: canQuery,
    staleTime: 30_000,
    retry: 1,
  });

  const summary = useMemo(() => {
    const requestData = requestTrendQuery.data;
    const modelData = modelTrendQuery.data;
    const heatmapData = heatmapQuery.data;
    const totalRequests =
      requestData?.items.reduce((sum, item) => sum + item.requestCount, 0) ?? 0;
    const totalSuccess =
      requestData?.items.reduce((sum, item) => sum + item.successCount, 0) ?? 0;
    const topModel = modelData?.items[0] ?? null;
    const hottestCell =
      heatmapData?.items.reduce<HeatmapCellItem | null>((best, item) => {
        if (!best || item.requestCount > best.requestCount) return item;
        return best;
      }, null) ?? null;

    return {
      totalRequests,
      successRate: totalRequests > 0 ? (totalSuccess / totalRequests) * 100 : 0,
      topModel,
      hottestCell,
      rangeLabel: formatRangeLabel(
        requestData?.rangeStart ?? 0,
        requestData?.rangeEnd ?? 0
      ),
    };
  }, [heatmapQuery.data, modelTrendQuery.data, requestTrendQuery.data]);

  const isLoading =
    requestTrendQuery.isLoading || modelTrendQuery.isLoading || heatmapQuery.isLoading;

  const handlePresetChange = (value: string | null) => {
    if (value) setPreset(value);
  };

  const handleGranularityChange = (value: string | null) => {
    if (value) setGranularity(value);
  };

  return (
    <div className="space-y-6">
      <Card className="glass-card relative overflow-hidden border-border/50">
        <div className="absolute inset-x-0 top-0 h-28 bg-[radial-gradient(circle_at_top_left,_rgba(14,165,233,0.28),transparent_52%),radial-gradient(circle_at_top_right,_rgba(16,185,129,0.22),transparent_44%)]" />
        <CardHeader className="relative space-y-4">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div className="space-y-3">
              <div className="inline-flex items-center gap-2 rounded-full border border-sky-400/30 bg-sky-500/10 px-3 py-1 text-xs text-sky-700 dark:text-sky-300">
                <Activity className="h-3.5 w-3.5" />
                最近 30/90 天真实网关趋势
              </div>
              <div>
                <CardTitle className="text-2xl">用量分析视图</CardTitle>
                <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
                  从请求日志直接聚合请求量、成功率、模型分布和时段热力图，帮助我们快速判断峰值时段和主要模型消耗结构。
                </p>
              </div>
            </div>
            <Badge variant="outline" className="h-8 rounded-full px-3">
              <CalendarRange className="mr-1.5 h-3.5 w-3.5" />
              {summary.rangeLabel}
            </Badge>
          </div>
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
            <div className="space-y-2">
              <div className="text-xs text-muted-foreground">分析范围</div>
              <Select value={preset} onValueChange={handlePresetChange}>
                <SelectTrigger className="rounded-2xl bg-background/60">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="30d">最近 30 天</SelectItem>
                  <SelectItem value="90d">最近 90 天</SelectItem>
                  <SelectItem value="custom">自定义区间</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <div className="text-xs text-muted-foreground">趋势粒度</div>
              <Select value={granularity} onValueChange={handleGranularityChange}>
                <SelectTrigger className="rounded-2xl bg-background/60">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="day">按天</SelectItem>
                  <SelectItem value="week">按周</SelectItem>
                  <SelectItem value="month">按月</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <div className="text-xs text-muted-foreground">开始日期</div>
              <Input
                type="date"
                value={startDate}
                onChange={(event) => setStartDate(event.target.value)}
                disabled={preset !== "custom"}
                className="rounded-2xl bg-background/60"
              />
            </div>
            <div className="space-y-2">
              <div className="text-xs text-muted-foreground">结束日期</div>
              <Input
                type="date"
                value={endDate}
                onChange={(event) => setEndDate(event.target.value)}
                disabled={preset !== "custom"}
                className="rounded-2xl bg-background/60"
              />
            </div>
          </div>
        </CardHeader>
      </Card>

      {!serviceStatus.connected ? (
        <Card className="glass-card border-border/50">
          <CardContent className="flex h-[220px] flex-col items-center justify-center gap-3 text-center">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            <div className="space-y-1">
              <div className="font-medium">服务未连接</div>
              <div className="text-sm text-muted-foreground">
                连接本地服务后，这里会自动加载真实趋势数据。
              </div>
            </div>
          </CardContent>
        </Card>
      ) : preset === "custom" && !canQuery ? (
        <Card className="glass-card border-border/50">
          <CardContent className="flex h-[220px] flex-col items-center justify-center gap-3 text-center">
            <CalendarRange className="h-8 w-8 text-muted-foreground" />
            <div className="space-y-1">
              <div className="font-medium">需要补全自定义时间范围</div>
              <div className="text-sm text-muted-foreground">
                请选择合法的开始和结束日期后再加载趋势分析。
              </div>
            </div>
          </CardContent>
        </Card>
      ) : (
        <>
          {isLoading ? (
            <SummarySkeleton />
          ) : (
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
              <Card className="glass-card border-border/50">
                <CardHeader className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">总请求量</span>
                    <LineChart className="h-4 w-4 text-sky-500" />
                  </div>
                  <div className="text-3xl font-semibold">
                    {summary.totalRequests.toLocaleString()}
                  </div>
                </CardHeader>
              </Card>
              <Card className="glass-card border-border/50">
                <CardHeader className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">整体成功率</span>
                    <Activity className="h-4 w-4 text-emerald-500" />
                  </div>
                  <div className="text-3xl font-semibold">
                    {summary.successRate.toFixed(1)}%
                  </div>
                </CardHeader>
              </Card>
              <Card className="glass-card border-border/50">
                <CardHeader className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">主力模型</span>
                    <Orbit className="h-4 w-4 text-indigo-500" />
                  </div>
                  <div className="truncate text-2xl font-semibold">
                    {summary.topModel?.model || "暂无数据"}
                  </div>
                </CardHeader>
              </Card>
              <Card className="glass-card border-border/50">
                <CardHeader className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">最热时段</span>
                    <Flame className="h-4 w-4 text-orange-500" />
                  </div>
                  <div className="text-2xl font-semibold">
                    {summary.hottestCell
                      ? `周${WEEKDAY_LABELS[summary.hottestCell.weekday]} ${summary.hottestCell.hour}:00`
                      : "暂无数据"}
                  </div>
                </CardHeader>
              </Card>
            </div>
          )}

          <div className="grid gap-6 xl:grid-cols-[minmax(0,1.25fr)_minmax(320px,0.75fr)]">
            <Card className="glass-card border-border/50">
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-lg">
                  <LineChart className="h-5 w-5 text-sky-500" />
                  请求量与成功率趋势
                </CardTitle>
              </CardHeader>
              <CardContent>
                {requestTrendQuery.isLoading ? (
                  <Skeleton className="h-[280px] w-full rounded-3xl" />
                ) : (
                  <RequestTrendChart
                    data={requestTrendQuery.data}
                    granularity={granularity}
                  />
                )}
              </CardContent>
            </Card>

            <Card className="glass-card border-border/50">
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-lg">
                  <Orbit className="h-5 w-5 text-indigo-500" />
                  模型分布
                </CardTitle>
              </CardHeader>
              <CardContent>
                {modelTrendQuery.isLoading ? (
                  <Skeleton className="h-[280px] w-full rounded-3xl" />
                ) : (
                  <ModelTrendPanel data={modelTrendQuery.data} />
                )}
              </CardContent>
            </Card>
          </div>

          <Card className="glass-card border-border/50">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-lg">
                <Grid2X2 className="h-5 w-5 text-cyan-500" />
                请求热力图
              </CardTitle>
            </CardHeader>
            <CardContent>
              {heatmapQuery.isLoading ? (
                <Skeleton className="h-[320px] w-full rounded-3xl" />
              ) : (
                <HeatmapPanel data={heatmapQuery.data} />
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
