"use client";

import { useQuery } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { serviceClient } from "@/lib/api/service-client";

function formatUsd(value: number): string {
  return `$${value.toFixed(value >= 10 ? 2 : 4)}`;
}

function formatTokens(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(value);
}

function hitRateColor(rate: number): string {
  if (rate >= 50) return "text-green-500";
  if (rate >= 20) return "text-yellow-500";
  return "text-red-400";
}

export default function CacheAnalyticsPanel({ preset }: { preset: string }) {
  const summaryQuery = useQuery({
    queryKey: ["cache-summary", preset],
    queryFn: () => serviceClient.getCacheAnalyticsSummary({ preset }),
    staleTime: 30_000,
  });

  const trendQuery = useQuery({
    queryKey: ["cache-trend", preset],
    queryFn: () => serviceClient.getCacheAnalyticsTrend({ preset }),
    staleTime: 30_000,
  });

  const modelQuery = useQuery({
    queryKey: ["cache-by-model", preset],
    queryFn: () => serviceClient.getCacheAnalyticsByModel({ preset }),
    staleTime: 30_000,
  });

  const isLoading = summaryQuery.isLoading || trendQuery.isLoading || modelQuery.isLoading;
  const summary = summaryQuery.data;
  const trendItems = trendQuery.data?.items ?? [];
  const modelItems = modelQuery.data?.items ?? [];

  if (isLoading) {
    return (
      <div className="flex h-40 items-center justify-center">
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      {/* Summary cards */}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <Card className="rounded-2xl border-border/40 bg-background/40">
          <CardContent className="p-3">
            <p className="text-[11px] text-muted-foreground">缓存命中率</p>
            <p className={`text-xl font-bold tabular-nums ${hitRateColor(summary?.hitRate ?? 0)}`}>
              {summary?.hitRate ?? 0}%
            </p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl border-border/40 bg-background/40">
          <CardContent className="p-3">
            <p className="text-[11px] text-muted-foreground">缓存请求</p>
            <p className="text-lg font-bold tabular-nums">
              {(summary?.cachedRequests ?? 0).toLocaleString()}
              <span className="text-xs font-normal text-muted-foreground">
                {" / "}{(summary?.totalRequests ?? 0).toLocaleString()}
              </span>
            </p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl border-border/40 bg-background/40">
          <CardContent className="p-3">
            <p className="text-[11px] text-muted-foreground">缓存 Token 比例</p>
            <p className="text-lg font-bold tabular-nums">
              {summary?.cacheTokenRatio ?? 0}%
            </p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl border-border/40 bg-background/40">
          <CardContent className="p-3">
            <p className="text-[11px] text-muted-foreground">估算节省</p>
            <p className="text-lg font-bold tabular-nums text-green-500">
              {formatUsd(summary?.estimatedSavingsUsd ?? 0)}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Daily hit rate trend */}
      <div className="space-y-2">
        <h4 className="text-sm font-medium text-muted-foreground">每日缓存命中率</h4>
        <HitRateTrendChart items={trendItems} />
      </div>

      {/* Model breakdown */}
      <div className="space-y-2">
        <h4 className="text-sm font-medium text-muted-foreground">按模型缓存效率</h4>
        {modelItems.length === 0 ? (
          <div className="flex h-20 items-center justify-center rounded-2xl border border-dashed border-border/60 text-sm text-muted-foreground">
            暂无模型数据
          </div>
        ) : (
          <div className="rounded-2xl border border-border/40 overflow-hidden">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="text-xs">模型</TableHead>
                  <TableHead className="text-xs text-right">请求数</TableHead>
                  <TableHead className="text-xs text-right">命中率</TableHead>
                  <TableHead className="text-xs text-right">缓存 Token</TableHead>
                  <TableHead className="text-xs text-right">估算节省</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {modelItems.map((item) => (
                  <TableRow key={item.model}>
                    <TableCell className="text-xs font-mono truncate max-w-[180px]">
                      {item.model}
                    </TableCell>
                    <TableCell className="text-xs text-right tabular-nums">
                      {item.cachedRequests.toLocaleString()} / {item.totalRequests.toLocaleString()}
                    </TableCell>
                    <TableCell className={`text-xs text-right tabular-nums font-medium ${hitRateColor(item.hitRate)}`}>
                      {item.hitRate}%
                    </TableCell>
                    <TableCell className="text-xs text-right tabular-nums">
                      {formatTokens(item.cachedInputTokens)}
                    </TableCell>
                    <TableCell className="text-xs text-right tabular-nums text-green-500">
                      {formatUsd(item.estimatedSavingsUsd)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
    </div>
  );
}

function HitRateTrendChart({
  items,
}: {
  items: { day: string; hitRate: number; cachedRequests: number; totalRequests: number }[];
}) {
  const normalizedItems = items.slice(-14);

  if (!normalizedItems.length) {
    return (
      <div className="flex h-[180px] items-center justify-center rounded-2xl border border-dashed border-border/60 bg-background/20 text-sm text-muted-foreground">
        当前范围暂无缓存趋势数据
      </div>
    );
  }

  return (
    <div className="flex h-[180px] items-end gap-1.5 rounded-2xl border border-border/40 bg-background/20 px-3 py-4">
      {normalizedItems.map((item) => {
        const height = Math.max(8, item.hitRate);
        const barColor =
          item.hitRate >= 50
            ? "from-green-500 via-green-400 to-emerald-300"
            : item.hitRate >= 20
              ? "from-yellow-500 via-yellow-400 to-amber-300"
              : "from-red-400 via-red-300 to-orange-300";
        return (
          <div
            key={item.day}
            className="flex min-w-0 flex-1 flex-col items-center gap-1.5"
          >
            <div className="text-[9px] text-muted-foreground tabular-nums">
              {item.hitRate}%
            </div>
            <div className="flex h-full w-full items-end">
              <div
                className={`w-full rounded-t-xl bg-gradient-to-t ${barColor}`}
                style={{ height: `${height}%` }}
              />
            </div>
            <div className="text-[9px] text-muted-foreground">
              {item.day.slice(5)}
            </div>
          </div>
        );
      })}
    </div>
  );
}
