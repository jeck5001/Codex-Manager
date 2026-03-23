"use client";

import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Download, RefreshCw, ShieldCheck } from "lucide-react";
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
import { isTauriRuntime } from "@/lib/api/transport";
import { formatTsFromSeconds } from "@/lib/utils/usage";
import { AuditLogItem } from "@/types";

type AuditActionFilter =
  | "all"
  | "create"
  | "upsert"
  | "update"
  | "set"
  | "delete"
  | "clear"
  | "run"
  | "import"
  | "sync";

type AuditObjectFilter =
  | "all"
  | "account"
  | "api_key"
  | "alert_rule"
  | "alert_channel"
  | "gateway_settings"
  | "app_settings"
  | "service_settings"
  | "healthcheck"
  | "web_auth"
  | "model_pricing"
  | "request_log"
  | "email_service"
  | "register_task"
  | "register_batch";

function parseDateTimeLocalValue(value: string): number | null {
  const trimmed = String(value || "").trim();
  if (!trimmed) return null;
  const timestamp = Date.parse(trimmed);
  if (Number.isNaN(timestamp)) return null;
  return Math.floor(timestamp / 1000);
}

function toDateTimeLocalValue(timestamp: number | null): string {
  if (timestamp == null || timestamp <= 0) return "";
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) return "";
  const offset = date.getTimezoneOffset();
  const localDate = new Date(date.getTime() - offset * 60_000);
  return localDate.toISOString().slice(0, 16);
}

function readChangePreview(item: AuditLogItem): string {
  const changes = item.changes ?? {};
  const before =
    changes.before && typeof changes.before === "object" && !Array.isArray(changes.before)
      ? (changes.before as Record<string, unknown>)
      : null;
  const after =
    changes.after && typeof changes.after === "object" && !Array.isArray(changes.after)
      ? (changes.after as Record<string, unknown>)
      : null;

  for (const key of ["status", "enabled", "strategy", "proxyUrl", "modelSlug", "expiresAt"]) {
    const beforeValue = before?.[key];
    const afterValue = after?.[key];
    if (beforeValue !== undefined || afterValue !== undefined) {
      return `${key}: ${String(beforeValue ?? "-")} -> ${String(afterValue ?? "-")}`;
    }
  }

  const params =
    changes.params && typeof changes.params === "object" && !Array.isArray(changes.params)
      ? (changes.params as Record<string, unknown>)
      : null;
  if (params) {
    const keys = Object.keys(params).filter((key) => key !== "addr").slice(0, 3);
    if (keys.length > 0) {
      return keys.map((key) => `${key}=${String(params[key] ?? "")}`).join(" / ");
    }
  }

  return "展开 JSON 查看完整 before / after 详情";
}

function ActionBadge({ action }: { action: string }) {
  const tone =
    action === "delete"
      ? "border-red-500/20 bg-red-500/10 text-red-500"
      : action === "create" || action === "upsert"
        ? "border-emerald-500/20 bg-emerald-500/10 text-emerald-500"
        : "border-sky-500/20 bg-sky-500/10 text-sky-500";
  return <Badge className={tone}>{action}</Badge>;
}

function AuditPageSkeleton() {
  return (
    <div className="space-y-5">
      <Skeleton className="h-32 w-full rounded-3xl" />
      <Skeleton className="h-[520px] w-full rounded-3xl" />
    </div>
  );
}

export default function AuditPage() {
  const [action, setAction] = useState<AuditActionFilter>("all");
  const [objectType, setObjectType] = useState<AuditObjectFilter>("all");
  const [objectId, setObjectId] = useState("");
  const [timeFromInput, setTimeFromInput] = useState("");
  const [timeToInput, setTimeToInput] = useState("");
  const [page, setPage] = useState(1);
  const pageSize = 20;

  const timeFrom = useMemo(() => parseDateTimeLocalValue(timeFromInput), [timeFromInput]);
  const timeTo = useMemo(() => parseDateTimeLocalValue(timeToInput), [timeToInput]);

  const auditQuery = useQuery({
    queryKey: ["audit", action, objectType, objectId, timeFrom, timeTo, page, pageSize],
    queryFn: () =>
      serviceClient.listAuditLogs({
        action: action === "all" ? "" : action,
        objectType: objectType === "all" ? "" : objectType,
        objectId,
        timeFrom,
        timeTo,
        page,
        pageSize,
      }),
    placeholderData: (previous) => previous,
  });

  const items = auditQuery.data?.items || [];
  const totalPages = Math.max(1, Math.ceil((auditQuery.data?.total || 0) / pageSize));

  const filterSummary = useMemo(() => {
    return [
      action !== "all" ? `动作 ${action}` : null,
      objectType !== "all" ? `对象 ${objectType}` : null,
      objectId.trim() ? `对象 ID ${objectId.trim()}` : null,
      timeFrom ? `起始 ${timeFromInput.replace("T", " ")}` : null,
      timeTo ? `结束 ${timeToInput.replace("T", " ")}` : null,
    ]
      .filter(Boolean)
      .join(" / ");
  }, [action, objectType, objectId, timeFrom, timeTo, timeFromInput, timeToInput]);

  const handleExport = async (format: "csv" | "json") => {
    try {
      const params = {
        format,
        action: action === "all" ? "" : action,
        objectType: objectType === "all" ? "" : objectType,
        objectId,
        timeFrom,
        timeTo,
      };
      if (!isTauriRuntime()) {
        await serviceClient.downloadAuditLogsViaHttp(params);
      } else {
        const result = await serviceClient.exportAuditLogs(params);
        const blob = new Blob([result.content], {
          type:
            result.format === "json"
              ? "application/json;charset=utf-8"
              : "text/csv;charset=utf-8",
        });
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement("a");
        anchor.href = url;
        anchor.download = result.fileName || "codexmanager-auditlogs.csv";
        anchor.click();
        URL.revokeObjectURL(url);
      }
      toast.success(`审计日志 ${format.toUpperCase()} 导出已开始`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "导出失败");
    }
  };

  if (auditQuery.isLoading && !auditQuery.data) {
    return <AuditPageSkeleton />;
  }

  return (
    <div className="space-y-5">
      <Card className="glass-card border-none shadow-sm">
        <CardHeader className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="space-y-2">
            <div className="inline-flex items-center gap-2 rounded-full bg-sky-500/10 px-3 py-1 text-xs font-medium text-sky-600">
              <ShieldCheck className="h-3.5 w-3.5" />
              管理写操作追踪
            </div>
            <CardTitle className="text-2xl">审计日志工作台</CardTitle>
            <p className="text-sm text-muted-foreground">
              查看账号、平台密钥、告警配置、网关设置等管理写操作，直接回溯 before / after 变化。
            </p>
            <p className="text-xs text-muted-foreground">
              {filterSummary || "当前显示全部审计记录"}，共 {auditQuery.data?.total || 0} 条。
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              className="gap-2"
              onClick={() => auditQuery.refetch()}
              disabled={auditQuery.isFetching}
            >
              <RefreshCw className={`h-4 w-4 ${auditQuery.isFetching ? "animate-spin" : ""}`} />
              刷新
            </Button>
            <Button variant="outline" className="gap-2" onClick={() => handleExport("csv")}>
              <Download className="h-4 w-4" />
              导出 CSV
            </Button>
            <Button variant="outline" className="gap-2" onClick={() => handleExport("json")}>
              <Download className="h-4 w-4" />
              导出 JSON
            </Button>
          </div>
        </CardHeader>
        <CardContent className="grid gap-3 md:grid-cols-2 xl:grid-cols-5">
          <Select
            value={action}
            onValueChange={(value) => {
              setAction(value as AuditActionFilter);
              setPage(1);
            }}
          >
            <SelectTrigger>
              <SelectValue placeholder="动作类型" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部动作</SelectItem>
              <SelectItem value="create">create</SelectItem>
              <SelectItem value="upsert">upsert</SelectItem>
              <SelectItem value="update">update</SelectItem>
              <SelectItem value="set">set</SelectItem>
              <SelectItem value="delete">delete</SelectItem>
              <SelectItem value="clear">clear</SelectItem>
              <SelectItem value="run">run</SelectItem>
              <SelectItem value="import">import</SelectItem>
              <SelectItem value="sync">sync</SelectItem>
            </SelectContent>
          </Select>
          <Select
            value={objectType}
            onValueChange={(value) => {
              setObjectType(value as AuditObjectFilter);
              setPage(1);
            }}
          >
            <SelectTrigger>
              <SelectValue placeholder="对象类型" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部对象</SelectItem>
              <SelectItem value="account">account</SelectItem>
              <SelectItem value="api_key">api_key</SelectItem>
              <SelectItem value="alert_rule">alert_rule</SelectItem>
              <SelectItem value="alert_channel">alert_channel</SelectItem>
              <SelectItem value="gateway_settings">gateway_settings</SelectItem>
              <SelectItem value="app_settings">app_settings</SelectItem>
              <SelectItem value="service_settings">service_settings</SelectItem>
              <SelectItem value="healthcheck">healthcheck</SelectItem>
              <SelectItem value="web_auth">web_auth</SelectItem>
              <SelectItem value="model_pricing">model_pricing</SelectItem>
              <SelectItem value="request_log">request_log</SelectItem>
              <SelectItem value="email_service">email_service</SelectItem>
              <SelectItem value="register_task">register_task</SelectItem>
              <SelectItem value="register_batch">register_batch</SelectItem>
            </SelectContent>
          </Select>
          <Input
            value={objectId}
            onChange={(event) => {
              setObjectId(event.target.value);
              setPage(1);
            }}
            placeholder="对象 ID"
          />
          <Input
            type="datetime-local"
            value={timeFromInput}
            max={toDateTimeLocalValue(timeTo)}
            onChange={(event) => {
              setTimeFromInput(event.target.value);
              setPage(1);
            }}
          />
          <Input
            type="datetime-local"
            value={timeToInput}
            min={toDateTimeLocalValue(timeFrom)}
            onChange={(event) => {
              setTimeToInput(event.target.value);
              setPage(1);
            }}
          />
        </CardContent>
      </Card>

      <Card className="glass-card border-none shadow-sm">
        <CardContent className="p-0">
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="min-w-[170px]">时间</TableHead>
                  <TableHead className="min-w-[110px]">动作</TableHead>
                  <TableHead className="min-w-[140px]">对象</TableHead>
                  <TableHead className="min-w-[180px]">对象 ID</TableHead>
                  <TableHead className="min-w-[120px]">操作者</TableHead>
                  <TableHead className="min-w-[360px]">变更摘要</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {items.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={6} className="py-16 text-center text-muted-foreground">
                      当前筛选条件下暂无审计日志
                    </TableCell>
                  </TableRow>
                ) : (
                  items.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell className="text-sm text-muted-foreground">
                        {formatTsFromSeconds(item.createdAt)}
                      </TableCell>
                      <TableCell>
                        <ActionBadge action={item.action} />
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary">{item.objectType}</Badge>
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {item.objectId || "-"}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">{item.operator}</Badge>
                      </TableCell>
                      <TableCell className="space-y-2 py-4">
                        <p className="text-sm">{readChangePreview(item)}</p>
                        <details className="rounded-2xl border border-border/50 bg-background/35 p-3">
                          <summary className="cursor-pointer text-xs text-muted-foreground">
                            展开 JSON
                          </summary>
                          <pre className="mt-3 overflow-x-auto whitespace-pre-wrap break-all text-xs text-muted-foreground">
                            {JSON.stringify(item.changes ?? {}, null, 2)}
                          </pre>
                        </details>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          第 {auditQuery.data?.page || page} / {totalPages} 页
        </p>
        <div className="flex gap-2">
          <Button
            variant="outline"
            disabled={page <= 1}
            onClick={() => setPage((value) => Math.max(1, value - 1))}
          >
            上一页
          </Button>
          <Button
            variant="outline"
            disabled={page >= totalPages}
            onClick={() => setPage((value) => Math.min(totalPages, value + 1))}
          >
            下一页
          </Button>
        </div>
      </div>
    </div>
  );
}
