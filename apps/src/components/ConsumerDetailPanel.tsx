"use client";

import { useQuery } from "@tanstack/react-query";
import { Loader2, X } from "lucide-react";
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

export default function ConsumerDetailPanel({
  keyId,
  keyName,
  preset,
  onClose,
}: {
  keyId: string;
  keyName: string;
  preset: string;
  onClose: () => void;
}) {
  const overviewQuery = useQuery({
    queryKey: ["consumer-overview", keyId, preset],
    queryFn: () => serviceClient.getConsumerOverview({ keyId, preset }),
    staleTime: 30_000,
  });

  const trendQuery = useQuery({
    queryKey: ["consumer-trend", keyId, preset],
    queryFn: () => serviceClient.getConsumerTrend({ keyId, preset }),
    staleTime: 30_000,
  });

  const modelQuery = useQuery({
    queryKey: ["consumer-model-breakdown", keyId, preset],
    queryFn: () => serviceClient.getConsumerModelBreakdown({ keyId, preset }),
    staleTime: 30_000,
  });

  const isLoading = overviewQuery.isLoading || trendQuery.isLoading || modelQuery.isLoading;
  const overview = overviewQuery.data;
  const trendItems = trendQuery.data?.items ?? [];
  const modelItems = modelQuery.data?.items ?? [];

  return (
    <div className="space-y-5 rounded-3xl border border-border/50 bg-card/50 p-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-0.5">
          <h3 className="text-base font-semibold">
            消费者详情
          </h3>
          <p className="text-xs text-muted-foreground truncate max-w-[300px]">
            {keyName || keyId}
          </p>
        </div>
        <button
          onClick={onClose}
          className="rounded-full p-1.5 text-muted-foreground hover:bg-muted/60 hover:text-foreground transition-colors"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      {isLoading ? (
        <div className="flex h-40 items-center justify-center">
          <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <>
          {/* Overview cards */}
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Card className="rounded-2xl border-border/40 bg-background/40">
              <CardContent className="p-3">
                <p className="text-[11px] text-muted-foreground">总请求</p>
                <p className="text-lg font-bold tabular-nums">
                  {(overview?.requestCount ?? 0).toLocaleString()}
                </p>
              </CardContent>
            </Card>
            <Card className="rounded-2xl border-border/40 bg-background/40">
              <CardContent className="p-3">
                <p className="text-[11px] text-muted-foreground">总成本</p>
                <p className="text-lg font-bold tabular-nums">
                  {formatUsd(overview?.estimatedCostUsd ?? 0)}
                </p>
              </CardContent>
            </Card>
            <Card className="rounded-2xl border-border/40 bg-background/40">
              <CardContent className="p-3">
                <p className="text-[11px] text-muted-foreground">成功率</p>
                <p className="text-lg font-bold tabular-nums">
                  {overview?.successRate ?? 0}%
                </p>
              </CardContent>
            </Card>
            <Card className="rounded-2xl border-border/40 bg-background/40">
              <CardContent className="p-3">
                <p className="text-[11px] text-muted-foreground">平均延迟</p>
                <p className="text-lg font-bold tabular-nums">
                  {overview?.avgDurationMs != null
                    ? `${overview.avgDurationMs.toFixed(0)} ms`
                    : "-"}
                </p>
              </CardContent>
            </Card>
          </div>

          {/* Daily cost trend */}
          <div className="space-y-2">
            <h4 className="text-sm font-medium text-muted-foreground">每日成本趋势</h4>
            <DailyTrendChart items={trendItems} />
          </div>

          {/* Model breakdown */}
          <div className="space-y-2">
            <h4 className="text-sm font-medium text-muted-foreground">模型用量分布</h4>
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
                      <TableHead className="text-xs text-right">Token</TableHead>
                      <TableHead className="text-xs text-right">成本</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {modelItems.map((item) => (
                      <TableRow key={item.model}>
                        <TableCell className="text-xs font-mono truncate max-w-[180px]">
                          {item.model}
                        </TableCell>
                        <TableCell className="text-xs text-right tabular-nums">
                          {item.requestCount.toLocaleString()}
                        </TableCell>
                        <TableCell className="text-xs text-right tabular-nums">
                          {formatTokens(item.totalTokens)}
                        </TableCell>
                        <TableCell className="text-xs text-right tabular-nums">
                          {formatUsd(item.estimatedCostUsd)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

function DailyTrendChart({
  items,
}: {
  items: { day: string; estimatedCostUsd: number }[];
}) {
  const normalizedItems = items.slice(-14);
  const maxValue = Math.max(
    ...normalizedItems.map((item) => item.estimatedCostUsd),
    0
  );

  if (!normalizedItems.length || maxValue <= 0) {
    return (
      <div className="flex h-[180px] items-center justify-center rounded-2xl border border-dashed border-border/60 bg-background/20 text-sm text-muted-foreground">
        当前范围暂无趋势数据
      </div>
    );
  }

  return (
    <div className="flex h-[180px] items-end gap-1.5 rounded-2xl border border-border/40 bg-background/20 px-3 py-4">
      {normalizedItems.map((item) => {
        const height = Math.max(8, (item.estimatedCostUsd / maxValue) * 100);
        return (
          <div
            key={item.day}
            className="flex min-w-0 flex-1 flex-col items-center gap-1.5"
          >
            <div className="text-[9px] text-muted-foreground tabular-nums">
              {formatUsd(item.estimatedCostUsd)}
            </div>
            <div className="flex h-full w-full items-end">
              <div
                className="w-full rounded-t-xl bg-gradient-to-t from-primary via-primary/80 to-sky-400/90"
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
