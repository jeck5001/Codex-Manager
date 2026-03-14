"use client";

import { 
  Dialog, 
  DialogContent, 
  DialogDescription, 
  DialogHeader, 
  DialogTitle,
  DialogFooter
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Account } from "@/types";
import { Clock, Calendar, RefreshCw, Database } from "lucide-react";
import { format } from "date-fns";
import { cn } from "@/lib/utils";

interface UsageModalProps {
  account: Account | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onRefresh: (id: string) => void;
  isRefreshing: boolean;
}

function UsageDetailRow({ label, used, total, resetsAt, icon: Icon }: { label: string, used: number, total: number, resetsAt?: string, icon: any }) {
  const percentage = total > 0 ? Math.min(Math.round((used / total) * 100), 100) : 0;
  const remaining = Math.max(total - used, 0);
  
  return (
    <div className="space-y-3 p-4 rounded-2xl bg-accent/10 border border-primary/5">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="p-1.5 rounded-lg bg-primary/10">
            <Icon className="h-4 w-4 text-primary" />
          </div>
          <span className="font-semibold">{label}</span>
        </div>
        <div className="text-right">
          <span className="text-lg font-bold">{remaining}</span>
          <span className="text-muted-foreground text-xs ml-1">/ {total} 剩余</span>
        </div>
      </div>
      
      <Progress value={percentage} className="h-2" />
      
      <div className="flex justify-between items-center text-[10px] text-muted-foreground">
        <span>已使用 {percentage}%</span>
        {resetsAt && (
          <span className="flex items-center gap-1">
            <Clock className="h-2.5 w-2.5" /> 
            重置时间: {(() => {
              try {
                const date = new Date(resetsAt);
                if (isNaN(date.getTime())) return "未知";
                return format(date, "HH:mm:ss");
              } catch {
                return "格式错误";
              }
            })()}
          </span>
        )}
      </div>
    </div>
  );
}

export default function UsageModal({ account, open, onOpenChange, onRefresh, isRefreshing }: UsageModalProps) {
  if (!account) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[450px] glass-card border-none p-6">
        <DialogHeader>
          <div className="flex items-center gap-3 mb-2">
            <div className="p-2 rounded-full bg-primary/10 text-primary">
              <Database className="h-5 w-5" />
            </div>
            <DialogTitle>用量详细查询</DialogTitle>
          </div>
          <DialogDescription className="font-medium text-foreground/80">
            账号: {account.name} ({account.id.slice(0, 8)}...)
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          <UsageDetailRow 
            label="5小时额度" 
            used={account.usage?.used || 0} 
            total={account.usage?.total || 0} 
            resetsAt={account.usage?.refresh_at}
            icon={Clock} 
          />
          
          <UsageDetailRow 
            label="7天周期额度" 
            used={account.usage?.used || 0} // Placeholder for secondary usage
            total={account.usage?.total || 0} 
            icon={Calendar} 
          />

          <div className="text-center">
            <p className="text-[10px] text-muted-foreground italic">
              数据捕获于: {account.last_refresh_at || "未知时间"}
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>关闭</Button>
          <Button 
            onClick={() => onRefresh(account.id)} 
            disabled={isRefreshing}
            className="gap-2"
          >
            <RefreshCw className={cn("h-4 w-4", isRefreshing && "animate-spin")} />
            {isRefreshing ? "正在刷新..." : "立即刷新"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
