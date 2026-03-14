"use client";

import { useDashboardStats } from "@/hooks/useDashboardStats";
import { 
  Card, 
  CardContent, 
  CardHeader, 
  CardTitle 
} from "@/components/ui/card";
import { 
  Users, 
  CheckCircle2, 
  XCircle, 
  Zap, 
  Database, 
  BrainCircuit, 
  DollarSign, 
  PieChart 
} from "lucide-react";
import { Skeleton } from "@/components/ui/skeleton";

export default function DashboardPage() {
  const { stats, isLoading } = useDashboardStats();

  const cards = [
    { title: "总账号数", value: stats.total, icon: Users, color: "text-blue-500", sub: "池中所有账号" },
    { title: "可用账号", value: stats.available, icon: CheckCircle2, color: "text-green-500", sub: "当前状态正常" },
    { title: "不可用账号", value: stats.unavailable, icon: XCircle, color: "text-red-500", sub: "额度耗尽或异常" },
    { title: "今日令牌", value: stats.todayTokens?.toLocaleString(), icon: Zap, color: "text-yellow-500", sub: "输入 + 输出合计" },
    { title: "缓存令牌", value: stats.cachedTokens?.toLocaleString(), icon: Database, color: "text-indigo-500", sub: "上下文缓存命中" },
    { title: "推理令牌", value: stats.reasoningTokens?.toLocaleString(), icon: BrainCircuit, color: "text-purple-500", sub: "大模型思考过程" },
    { title: "预计费用", value: `$${stats.todayCost?.toFixed(2)}`, icon: DollarSign, color: "text-emerald-500", sub: "按官价估算" },
  ];

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {isLoading ? (
          Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-32 w-full rounded-xl" />
          ))
        ) : (
          cards.map((card, i) => (
            <Card key={i} className="overflow-hidden border-none bg-card/50 shadow-md backdrop-blur-sm transition-all hover:scale-[1.02]">
              <CardHeader className="flex flex-row items-center justify-between pb-2 space-y-0">
                <CardTitle className="text-sm font-medium">{card.title}</CardTitle>
                <card.icon className={`h-4 w-4 ${card.color}`} />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{card.value}</div>
                <p className="text-xs text-muted-foreground mt-1">{card.sub}</p>
              </CardContent>
            </Card>
          ))
        )}

        <Card className="overflow-hidden border-none bg-primary/10 shadow-md backdrop-blur-sm transition-all hover:scale-[1.02]">
          <CardHeader className="flex flex-row items-center justify-between pb-2 space-y-0">
            <CardTitle className="text-sm font-medium">账号池剩余</CardTitle>
            <PieChart className="h-4 w-4 text-primary" />
          </CardHeader>
          <CardContent>
            <div className="flex flex-col gap-1">
              <div className="flex items-center justify-between">
                <span className="text-xs text-muted-foreground">5小时内</span>
                <span className="text-sm font-bold">{stats.poolRemain.primary}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs text-muted-foreground">7天内</span>
                <span className="text-sm font-bold">{stats.poolRemain.secondary}</span>
              </div>
            </div>
            <p className="text-[10px] text-muted-foreground mt-2 opacity-70 text-center">按已刷新账号聚合</p>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card className="min-h-[300px] border-none bg-card/50 shadow-md">
          <CardHeader>
            <CardTitle className="text-base font-semibold">当前活跃账号</CardTitle>
          </CardHeader>
          <CardContent className="flex items-center justify-center text-muted-foreground text-sm">
            暂无活跃请求
          </CardContent>
        </Card>

        <Card className="min-h-[300px] border-none bg-card/50 shadow-md">
          <CardHeader>
            <CardTitle className="text-base font-semibold">智能推荐</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
             <div className="rounded-lg border border-dashed p-4 text-center text-sm text-muted-foreground">
               基于当前配额，系统建议优先使用可用性更稳定的账号。
             </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
