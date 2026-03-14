"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { serviceClient } from "@/lib/api/service-client";
import { useAppStore } from "@/lib/store/useAppStore";
import { 
  Table, 
  TableBody, 
  TableCell, 
  TableHead, 
  TableHeader, 
  TableRow 
} from "@/components/ui/table";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { 
  RefreshCw, 
  Trash2, 
  Search, 
  Filter,
  CheckCircle2,
  AlertCircle,
  Clock
} from "lucide-react";
import { useState } from "react";
import { format } from "date-fns";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "sonner";

export default function LogsPage() {
  const { serviceStatus } = useAppStore();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("all");

  const { data: logs, isLoading } = useQuery({
    queryKey: ["logs", search, filter],
    queryFn: () => serviceClient.listRequestLogs(search, 100),
    enabled: serviceStatus.connected,
    refetchInterval: 5000,
  });

  const clearMutation = useMutation({
    mutationFn: () => serviceClient.clearRequestLogs(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["logs"] });
      toast.success("日志已清空");
    },
  });

  const filteredLogs = logs?.filter((log: any) => {
    if (filter === "all") return true;
    if (filter === "2xx") return log.status >= 200 && log.status < 300;
    if (filter === "4xx") return log.status >= 400 && log.status < 500;
    if (filter === "5xx") return log.status >= 500;
    return true;
  }) || [];

  const getStatusBadge = (status: number) => {
    if (status >= 200 && status < 300) return <Badge className="bg-green-500/10 text-green-500 border-green-500/20">{status}</Badge>;
    if (status >= 400 && status < 500) return <Badge className="bg-yellow-500/10 text-yellow-500 border-yellow-500/20">{status}</Badge>;
    return <Badge className="bg-red-500/10 text-red-500 border-red-500/20">{status}</Badge>;
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div className="flex flex-1 items-center gap-2 max-w-md">
          <div className="relative w-full">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="搜索方法、路径或密钥..."
              className="pl-9 h-10"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <div className="flex border rounded-lg p-1 bg-card">
            {["all", "2xx", "4xx", "5xx"].map((f) => (
              <button
                key={f}
                onClick={() => setFilter(f)}
                className={`px-3 py-1 text-xs rounded-md transition-all ${
                  filter === f ? "bg-primary text-primary-foreground shadow-sm" : "text-muted-foreground hover:bg-muted"
                }`}
              >
                {f.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => queryClient.invalidateQueries({ queryKey: ["logs"] })}>
            <RefreshCw className="h-4 w-4 mr-2" /> 刷新
          </Button>
          <Button variant="destructive" size="sm" onClick={() => clearMutation.mutate()}>
            <Trash2 className="h-4 w-4 mr-2" /> 清空日志
          </Button>
        </div>
      </div>

      <Card className="border-none bg-card/50 shadow-md">
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[180px]">时间</TableHead>
                <TableHead>方法 / 路径</TableHead>
                <TableHead>模型</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>延迟</TableHead>
                <TableHead>错误信息</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                Array.from({ length: 10 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell><Skeleton className="h-4 w-32" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-40" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-24" /></TableCell>
                    <TableCell><Skeleton className="h-6 w-12 rounded-full" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-12" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-full" /></TableCell>
                  </TableRow>
                ))
              ) : filteredLogs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} className="h-32 text-center text-muted-foreground">
                    {!serviceStatus.connected ? "服务未连接，无法获取日志" : "暂无请求日志"}
                  </TableCell>
                </TableRow>
              ) : (
                filteredLogs.map((log: any) => (
                  <TableRow key={log.id} className="text-xs group hover:bg-muted/30">
                    <TableCell className="text-muted-foreground font-mono">
                      {format(new Date(log.timestamp), "yyyy-MM-dd HH:mm:ss")}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-bold text-primary">{log.method}</span>
                        <span className="text-muted-foreground truncate max-w-[200px]">{log.path}</span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="font-normal text-[10px]">{log.model || "-"}</Badge>
                    </TableCell>
                    <TableCell>{getStatusBadge(log.status)}</TableCell>
                    <TableCell className="text-muted-foreground">
                       <div className="flex items-center gap-1">
                         <Clock className="h-3 w-3" />
                         {log.latency_ms ? `${log.latency_ms}ms` : "-"}
                       </div>
                    </TableCell>
                    <TableCell className="text-red-400 max-w-[300px] truncate" title={log.error}>
                      {log.error || "-"}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}
