"use client";

import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BarChart3,
  CalendarRange,
  Download,
  Loader2,
  PiggyBank,
  Plus,
  ReceiptText,
  Save,
  TableProperties,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { serviceClient } from "@/lib/api/service-client";
import { useAppStore } from "@/lib/store/useAppStore";
import type { CostSummaryDayItem, CostSummaryModelItem, ModelPricingItem } from "@/types";

type DraftPricingRow = {
  id: string;
  modelSlug: string;
  inputPricePer1k: string;
  outputPricePer1k: string;
};

function createDraftRow(item?: Partial<ModelPricingItem>): DraftPricingRow {
  const seed = Math.random().toString(36).slice(2, 10);
  return {
    id: `${item?.modelSlug || "pricing"}-${seed}`,
    modelSlug: item?.modelSlug || "",
    inputPricePer1k:
      item?.inputPricePer1k != null ? String(item.inputPricePer1k) : "",
    outputPricePer1k:
      item?.outputPricePer1k != null ? String(item.outputPricePer1k) : "",
  };
}

function formatUsdPer1k(value: string): string {
  const num = Number(value);
  if (!Number.isFinite(num) || num < 0) return "$0.0000 / 1K";
  return `$${num.toFixed(num >= 1 ? 2 : 4)} / 1K`;
}

function formatUsd(value: number): string {
  return `$${value.toFixed(value >= 10 ? 2 : 4)}`;
}

function DailyCostBarChart({ items }: { items: CostSummaryDayItem[] }) {
  const normalizedItems = items.slice(-14);
  const maxValue = Math.max(
    ...normalizedItems.map((item) => item.estimatedCostUsd),
    0
  );

  if (!normalizedItems.length || maxValue <= 0) {
    return (
      <div className="flex h-[220px] items-center justify-center rounded-3xl border border-dashed border-border/60 bg-background/20 text-sm text-muted-foreground">
        当前范围暂无每日费用数据
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex h-[220px] items-end gap-2 rounded-3xl border border-border/50 bg-background/20 px-4 py-5">
        {normalizedItems.map((item) => {
          const height = Math.max(10, (item.estimatedCostUsd / maxValue) * 100);
          return (
            <div key={item.day} className="flex min-w-0 flex-1 flex-col items-center gap-2">
              <div className="text-[10px] text-muted-foreground">
                {formatUsd(item.estimatedCostUsd)}
              </div>
              <div className="flex h-full w-full items-end">
                <div
                  className="w-full rounded-t-2xl bg-gradient-to-t from-primary via-primary/80 to-sky-400/90 shadow-[0_8px_24px_rgba(59,130,246,0.18)]"
                  style={{ height: `${height}%` }}
                />
              </div>
              <div className="text-[10px] text-muted-foreground">
                {item.day.slice(5)}
              </div>
            </div>
          );
        })}
      </div>
      <div className="flex flex-wrap items-center gap-3 text-[11px] text-muted-foreground">
        <span className="inline-flex items-center gap-2">
          <span className="h-2.5 w-2.5 rounded-full bg-primary" />
          最近 {normalizedItems.length} 天每日费用
        </span>
        <span className="rounded-full bg-muted/50 px-2 py-1">
          峰值 {formatUsd(maxValue)}
        </span>
      </div>
    </div>
  );
}

function buildPieSegments(items: CostSummaryModelItem[]) {
  const total = items.reduce((sum, item) => sum + item.estimatedCostUsd, 0);
  if (total <= 0) return [];

  let offset = 0;
  const palette = [
    "#3b82f6",
    "#14b8a6",
    "#f59e0b",
    "#f43f5e",
    "#8b5cf6",
    "#22c55e",
  ];

  return items.map((item, index) => {
    const share = item.estimatedCostUsd / total;
    const start = offset * 360;
    offset += share;
    const end = offset * 360;
    return {
      ...item,
      share,
      color: palette[index % palette.length],
      gradient: `${palette[index % palette.length]} ${start}deg ${end}deg`,
    };
  });
}

function ModelDistributionChart({ items }: { items: CostSummaryModelItem[] }) {
  const topItems = items.slice(0, 6);
  const segments = buildPieSegments(topItems);

  if (!segments.length) {
    return (
      <div className="flex h-[220px] items-center justify-center rounded-3xl border border-dashed border-border/60 bg-background/20 text-sm text-muted-foreground">
        当前范围暂无模型费用分布
      </div>
    );
  }

  const background = `conic-gradient(${segments
    .map((segment) => segment.gradient)
    .join(", ")})`;

  return (
    <div className="grid gap-5 lg:grid-cols-[180px_minmax(0,1fr)] lg:items-center">
      <div className="mx-auto flex h-[180px] w-[180px] items-center justify-center rounded-full border border-border/50 bg-background/20 p-4">
        <div
          className="relative h-full w-full rounded-full"
          style={{ background }}
        >
          <div className="absolute inset-[24%] rounded-full border border-border/50 bg-background/95 shadow-inner" />
        </div>
      </div>
      <div className="space-y-3">
        {segments.map((item) => (
          <div
            key={item.model}
            className="rounded-2xl border border-border/50 bg-background/20 px-4 py-3"
          >
            <div className="flex items-center justify-between gap-3">
              <div className="flex min-w-0 items-center gap-3">
                <span
                  className="h-3 w-3 shrink-0 rounded-full"
                  style={{ backgroundColor: item.color }}
                />
                <span className="truncate font-mono text-xs">{item.model}</span>
              </div>
              <span className="text-xs font-semibold">
                {(item.share * 100).toFixed(item.share >= 0.1 ? 1 : 2)}%
              </span>
            </div>
            <div className="mt-2 flex items-center justify-between text-xs text-muted-foreground">
              <span>{formatUsd(item.estimatedCostUsd)}</span>
              <span>{item.totalTokens.toLocaleString()} tokens</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function CostsPage() {
  const { serviceStatus } = useAppStore();
  const queryClient = useQueryClient();
  const [draftRows, setDraftRows] = useState<DraftPricingRow[] | null>(null);
  const [preset, setPreset] = useState("month");
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");

  const pricingQuery = useQuery({
    queryKey: ["costs", "model-pricing"],
    queryFn: () => serviceClient.getCostModelPricing(),
    enabled: serviceStatus.connected,
    retry: 1,
    staleTime: 30_000,
  });

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

  const summaryQuery = useQuery({
    queryKey: ["costs", "summary", preset, startTs, endTs],
    queryFn: () =>
      serviceClient.getCostSummary({
        preset,
        startTs,
        endTs,
      }),
    enabled:
      serviceStatus.connected &&
      (preset !== "custom" || (startTs != null && endTs != null && endTs > startTs)),
    retry: 1,
    staleTime: 30_000,
  });

  const serverRows = useMemo(
    () =>
      pricingQuery.data && pricingQuery.data.length > 0
        ? pricingQuery.data.map((item) => createDraftRow(item))
        : [createDraftRow()],
    [pricingQuery.data]
  );
  const rows = draftRows ?? serverRows;

  const saveMutation = useMutation({
    mutationFn: async () => {
      const items = rows
        .map((row) => ({
          modelSlug: row.modelSlug.trim(),
          inputPricePer1k: Number(row.inputPricePer1k),
          outputPricePer1k: Number(row.outputPricePer1k),
          updatedAt: null,
        }))
        .filter((item) => item.modelSlug.length > 0);

      if (items.some((item) => !Number.isFinite(item.inputPricePer1k) || item.inputPricePer1k < 0)) {
        throw new Error("输入单价必须是大于等于 0 的数字");
      }
      if (
        items.some(
          (item) => !Number.isFinite(item.outputPricePer1k) || item.outputPricePer1k < 0
        )
      ) {
        throw new Error("输出单价必须是大于等于 0 的数字");
      }
      if (new Set(items.map((item) => item.modelSlug)).size !== items.length) {
        throw new Error("模型名称不能重复");
      }

      await serviceClient.setCostModelPricing(items);
      return items;
    },
    onSuccess: async (items) => {
      queryClient.setQueryData<ModelPricingItem[]>(["costs", "model-pricing"], () =>
        items.map((item) => ({
          ...item,
          updatedAt: null,
        }))
      );
      setDraftRows(null);
      await queryClient.invalidateQueries({ queryKey: ["costs", "model-pricing"] });
      toast.success("模型单价已保存");
    },
    onError: (error: unknown) => {
      toast.error(error instanceof Error ? error.message : "保存失败");
    },
  });

  const exportMutation = useMutation({
    mutationFn: () =>
      serviceClient.exportCostSummary({
        preset,
        startTs,
        endTs,
      }),
    onSuccess: (result) => {
      const blob = new Blob([result.content], {
        type: "text/csv;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = result.fileName || "codexmanager-costs.csv";
      anchor.style.display = "none";
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 0);
      toast.success("CSV 已导出");
    },
    onError: (error: unknown) => {
      toast.error(error instanceof Error ? error.message : "导出失败");
    },
  });

  const updateRows = (updater: (current: DraftPricingRow[]) => DraftPricingRow[]) => {
    setDraftRows((current) => updater(current ?? serverRows));
  };

  const updateRow = (id: string, patch: Partial<DraftPricingRow>) => {
    updateRows((current) =>
      current.map((row) => (row.id === id ? { ...row, ...patch } : row))
    );
  };

  const removeRow = (id: string) => {
    updateRows((current) => {
      if (current.length <= 1) {
        return [createDraftRow()];
      }
      return current.filter((row) => row.id !== id);
    });
  };

  const isLoading = serviceStatus.connected && pricingQuery.isLoading;
  const summary = summaryQuery.data;
  const topKey = summary?.byKey[0];
  const topModel = summary?.byModel[0];

  return (
    <div className="animate-in space-y-6 fade-in duration-500">
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.1fr)_minmax(320px,0.9fr)]">
        <Card className="glass-card border-none shadow-xl">
          <CardHeader className="space-y-3">
            <div className="flex items-start gap-3">
              <div className="flex h-11 w-11 items-center justify-center rounded-2xl bg-primary/12 text-primary">
                <ReceiptText className="h-5 w-5" />
              </div>
              <div className="space-y-1">
                <CardTitle className="text-xl">费用统计工作台</CardTitle>
                <p className="text-sm text-muted-foreground">
                  统一查看模型成本配置、时间范围汇总、图表趋势与 CSV 报表导出。
                </p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="grid gap-3 md:grid-cols-3">
            <div className="rounded-3xl border border-primary/15 bg-primary/8 p-4">
              <div className="mb-2 flex items-center gap-2 text-primary">
                <PiggyBank className="h-4 w-4" />
                <span className="text-sm font-semibold">模型单价</span>
              </div>
              <p className="text-sm text-muted-foreground">
                已接入真实 RPC，可直接维护模型输入 / 输出单价。
              </p>
            </div>
            <div className="rounded-3xl border border-border/60 bg-background/35 p-4">
              <div className="mb-2 flex items-center gap-2">
                <BarChart3 className="h-4 w-4 text-emerald-500" />
                <span className="text-sm font-semibold">费用汇总</span>
              </div>
              <p className="text-sm text-muted-foreground">
                现在已经支持按时间范围读取按 Key / 模型 / 日期聚合的数据。
              </p>
            </div>
            <div className="rounded-3xl border border-border/60 bg-background/35 p-4">
              <div className="mb-2 flex items-center gap-2">
                <Save className="h-4 w-4 text-amber-500" />
                <span className="text-sm font-semibold">导出报表</span>
              </div>
              <p className="text-sm text-muted-foreground">
                当前范围的费用汇总可直接导出为 CSV，便于复盘与审计。
              </p>
            </div>
          </CardContent>
        </Card>

        <Card className="glass-card border-none shadow-lg">
          <CardHeader>
            <CardTitle className="text-base">当前进度</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm text-muted-foreground">
            <div className="flex items-center justify-between rounded-2xl border border-border/60 bg-background/30 px-4 py-3">
              <span>费用统计页入口</span>
              <Badge className="border-green-500/20 bg-green-500/10 text-green-500">
                已接通
              </Badge>
            </div>
            <div className="flex items-center justify-between rounded-2xl border border-border/60 bg-background/30 px-4 py-3">
              <span>模型单价配置</span>
              <Badge className="border-green-500/20 bg-green-500/10 text-green-500">
                可保存
              </Badge>
            </div>
            <div className="flex items-center justify-between rounded-2xl border border-border/60 bg-background/30 px-4 py-3">
              <span>费用汇总 RPC</span>
              <Badge className="border-green-500/20 bg-green-500/10 text-green-500">
                已接通
              </Badge>
            </div>
            <div className="flex items-center justify-between rounded-2xl border border-border/60 bg-background/30 px-4 py-3">
              <span>图表 / CSV 导出</span>
              <Badge className="border-green-500/20 bg-green-500/10 text-green-500">
                已接通
              </Badge>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card className="glass-card border-none shadow-lg">
        <CardHeader className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <CardTitle className="text-lg">费用汇总</CardTitle>
            <p className="text-sm text-muted-foreground">
              按范围查看 API Key、模型与日期维度的 token 与费用聚合。
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              className="rounded-xl"
              disabled={
                !serviceStatus.connected ||
                exportMutation.isPending ||
                (preset === "custom" &&
                  (startTs == null || endTs == null || endTs <= startTs))
              }
              onClick={() => exportMutation.mutate()}
            >
              {exportMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Download className="mr-2 h-4 w-4" />
              )}
              导出 CSV
            </Button>
            <Select
              value={preset}
              onValueChange={(value) => setPreset(value ?? "month")}
            >
              <SelectTrigger className="h-10 w-[140px] rounded-xl">
                <SelectValue placeholder="选择范围" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="today">今日</SelectItem>
                <SelectItem value="week">本周</SelectItem>
                <SelectItem value="month">本月</SelectItem>
                <SelectItem value="custom">自定义</SelectItem>
              </SelectContent>
            </Select>
            {preset === "custom" ? (
              <>
                <Input
                  type="date"
                  value={startDate}
                  onChange={(event) => setStartDate(event.target.value)}
                  className="h-10 w-[160px] rounded-xl"
                />
                <Input
                  type="date"
                  value={endDate}
                  onChange={(event) => setEndDate(event.target.value)}
                  className="h-10 w-[160px] rounded-xl"
                />
              </>
            ) : null}
          </div>
        </CardHeader>
        <CardContent className="space-y-5">
          {!serviceStatus.connected ? (
            <div className="rounded-3xl border border-dashed border-border/70 bg-background/20 px-5 py-10 text-center text-sm text-muted-foreground">
              服务未连接，暂时无法读取费用汇总。
            </div>
          ) : summaryQuery.isLoading ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {Array.from({ length: 4 }).map((_, index) => (
                <Skeleton key={index} className="h-28 rounded-3xl" />
              ))}
            </div>
          ) : summary ? (
            <>
              <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                <div className="rounded-3xl border border-primary/15 bg-primary/8 p-4">
                  <div className="mb-2 flex items-center gap-2 text-primary">
                    <PiggyBank className="h-4 w-4" />
                    <span className="text-sm font-semibold">总费用</span>
                  </div>
                  <div className="text-2xl font-semibold">
                    {formatUsd(summary.total.estimatedCostUsd)}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {summary.total.totalTokens.toLocaleString()} tokens
                  </div>
                </div>
                <div className="rounded-3xl border border-border/60 bg-background/30 p-4">
                  <div className="mb-2 flex items-center gap-2">
                    <CalendarRange className="h-4 w-4 text-emerald-500" />
                    <span className="text-sm font-semibold">请求数</span>
                  </div>
                  <div className="text-2xl font-semibold">
                    {summary.total.requestCount.toLocaleString()}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    输入 {summary.total.inputTokens.toLocaleString()} / 输出{" "}
                    {summary.total.outputTokens.toLocaleString()}
                  </div>
                </div>
                <div className="rounded-3xl border border-border/60 bg-background/30 p-4">
                  <div className="mb-2 flex items-center gap-2">
                    <ReceiptText className="h-4 w-4 text-amber-500" />
                    <span className="text-sm font-semibold">最高费用 Key</span>
                  </div>
                  <div className="truncate text-lg font-semibold">
                    {topKey?.keyId || "-"}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {topKey ? formatUsd(topKey.estimatedCostUsd) : "暂无数据"}
                  </div>
                </div>
                <div className="rounded-3xl border border-border/60 bg-background/30 p-4">
                  <div className="mb-2 flex items-center gap-2">
                    <TableProperties className="h-4 w-4 text-sky-500" />
                    <span className="text-sm font-semibold">最高费用模型</span>
                  </div>
                  <div className="truncate text-lg font-semibold">
                    {topModel?.model || "-"}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {topModel ? formatUsd(topModel.estimatedCostUsd) : "暂无数据"}
                  </div>
                </div>
              </div>

              <div className="grid gap-4 xl:grid-cols-2">
                <Card className="border-border/60 bg-background/20">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm">每日费用趋势</CardTitle>
                    <p className="text-xs text-muted-foreground">
                      基于当前筛选范围展示最近 14 天的费用柱状分布。
                    </p>
                  </CardHeader>
                  <CardContent>
                    <DailyCostBarChart items={summary.byDay} />
                  </CardContent>
                </Card>

                <Card className="border-border/60 bg-background/20">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm">模型费用分布</CardTitle>
                    <p className="text-xs text-muted-foreground">
                      展示当前范围费用最高的 6 个模型占比。
                    </p>
                  </CardHeader>
                  <CardContent>
                    <ModelDistributionChart items={summary.byModel} />
                  </CardContent>
                </Card>
              </div>

              <div className="grid gap-4 xl:grid-cols-3">
                <Card className="border-border/60 bg-background/20">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm">按 Key 汇总</CardTitle>
                  </CardHeader>
                  <CardContent className="px-0">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Key</TableHead>
                          <TableHead>费用</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {summary.byKey.slice(0, 6).map((item) => (
                          <TableRow key={item.keyId}>
                            <TableCell className="font-mono text-xs">{item.keyId}</TableCell>
                            <TableCell>{formatUsd(item.estimatedCostUsd)}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </CardContent>
                </Card>

                <Card className="border-border/60 bg-background/20">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm">按模型汇总</CardTitle>
                  </CardHeader>
                  <CardContent className="px-0">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>模型</TableHead>
                          <TableHead>费用</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {summary.byModel.slice(0, 6).map((item) => (
                          <TableRow key={item.model}>
                            <TableCell className="font-mono text-xs">{item.model}</TableCell>
                            <TableCell>{formatUsd(item.estimatedCostUsd)}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </CardContent>
                </Card>

                <Card className="border-border/60 bg-background/20">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm">按日期汇总</CardTitle>
                  </CardHeader>
                  <CardContent className="px-0">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>日期</TableHead>
                          <TableHead>费用</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {summary.byDay.slice(-7).map((item) => (
                          <TableRow key={item.day}>
                            <TableCell className="font-mono text-xs">{item.day}</TableCell>
                            <TableCell>{formatUsd(item.estimatedCostUsd)}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </CardContent>
                </Card>
              </div>
            </>
          ) : (
            <div className="rounded-3xl border border-dashed border-border/70 bg-background/20 px-5 py-10 text-center text-sm text-muted-foreground">
              当前范围暂无费用数据。
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="glass-card border-none shadow-lg">
        <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <CardTitle className="text-lg">模型单价配置</CardTitle>
            <p className="text-sm text-muted-foreground">
              单位为 USD / 1K tokens，保存后将作为后续费用估算与报表聚合的基础。
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              className="rounded-xl"
              onClick={() => updateRows((current) => [...current, createDraftRow()])}
            >
              <Plus className="mr-2 h-4 w-4" />
              添加模型
            </Button>
            <Button
              className="rounded-xl"
              disabled={!serviceStatus.connected || saveMutation.isPending}
              onClick={() => saveMutation.mutate()}
            >
              {saveMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Save className="mr-2 h-4 w-4" />
              )}
              保存配置
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {!serviceStatus.connected ? (
            <div className="rounded-3xl border border-dashed border-border/70 bg-background/20 px-5 py-10 text-center text-sm text-muted-foreground">
              服务未连接，暂时无法加载费用配置。
            </div>
          ) : isLoading ? (
            <div className="space-y-3">
              {Array.from({ length: 3 }).map((_, index) => (
                <Skeleton key={index} className="h-24 rounded-3xl" />
              ))}
            </div>
          ) : (
            rows.map((row, index) => (
              <div
                key={row.id}
                className="grid gap-4 rounded-3xl border border-border/60 bg-background/30 p-4 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)_minmax(0,1fr)_auto]"
              >
                <div className="space-y-2">
                  <div className="text-xs font-medium tracking-[0.16em] text-muted-foreground uppercase">
                    模型标识 #{index + 1}
                  </div>
                  <Input
                    placeholder="例如 o3 / gpt-4o"
                    value={row.modelSlug}
                    onChange={(event) =>
                      updateRow(row.id, { modelSlug: event.target.value })
                    }
                    className="h-11 rounded-2xl"
                  />
                </div>
                <div className="space-y-2">
                  <div className="text-xs font-medium tracking-[0.16em] text-muted-foreground uppercase">
                    输入单价
                  </div>
                  <Input
                    inputMode="decimal"
                    placeholder="0.0200"
                    value={row.inputPricePer1k}
                    onChange={(event) =>
                      updateRow(row.id, { inputPricePer1k: event.target.value })
                    }
                    className="h-11 rounded-2xl"
                  />
                  <div className="text-[11px] text-muted-foreground">
                    {formatUsdPer1k(row.inputPricePer1k)}
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="text-xs font-medium tracking-[0.16em] text-muted-foreground uppercase">
                    输出单价
                  </div>
                  <Input
                    inputMode="decimal"
                    placeholder="0.0800"
                    value={row.outputPricePer1k}
                    onChange={(event) =>
                      updateRow(row.id, { outputPricePer1k: event.target.value })
                    }
                    className="h-11 rounded-2xl"
                  />
                  <div className="text-[11px] text-muted-foreground">
                    {formatUsdPer1k(row.outputPricePer1k)}
                  </div>
                </div>
                <div className="flex items-start justify-end">
                  <Button
                    variant="ghost"
                    size="icon"
                    className="mt-6 rounded-2xl text-muted-foreground hover:text-red-500"
                    onClick={() => removeRow(row.id)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
