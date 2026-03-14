"use client";

import { useState, useMemo } from "react";
import { useAccounts } from "@/hooks/useAccounts";
import { 
  Table, 
  TableBody, 
  TableCell, 
  TableHead, 
  TableHeader, 
  TableRow 
} from "@/components/ui/table";
import { 
  Card, 
  CardContent 
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { 
  Plus, 
  RefreshCw, 
  Search, 
  MoreVertical, 
  Trash2, 
  ExternalLink,
  FolderOpen,
  Download,
  Upload,
  ArrowUpDown,
  Filter
} from "lucide-react";
import { 
  DropdownMenu, 
  DropdownMenuContent, 
  DropdownMenuItem, 
  DropdownMenuTrigger,
  DropdownMenuSeparator
} from "@/components/ui/dropdown-menu";
import { 
  Select, 
  SelectContent, 
  SelectItem, 
  SelectTrigger, 
  SelectValue 
} from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { AddAccountModal } from "@/components/modals/add-account-modal";
import { cn } from "@/lib/utils";

type StatusFilter = "all" | "available" | "low_quota";

export default function AccountsPage() {
  const { accounts, groups, isLoading, refreshAccount, deleteAccount, deleteUnavailableFree, importByDirectory } = useAccounts();
  const [search, setSearch] = useState("");
  const [groupFilter, setGroupFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [pageSize, setPageSize] = useState("20");
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [addAccountModalOpen, setAddAccountModalOpen] = useState(false);

  // Advanced Filtering
  const filteredAccounts = useMemo(() => {
    return accounts.filter(acc => {
      const matchSearch = !search || 
        acc.name.toLowerCase().includes(search.toLowerCase()) ||
        acc.id.toLowerCase().includes(search.toLowerCase());
      
      const matchGroup = groupFilter === "all" || (acc.group || "默认") === groupFilter;
      
      const matchStatus = statusFilter === "all" || 
        (statusFilter === "available" && acc.is_available) ||
        (statusFilter === "low_quota" && acc.is_low_quota);

      return matchSearch && matchGroup && matchStatus;
    });
  }, [accounts, search, groupFilter, statusFilter]);

  const toggleSelect = (id: string) => {
    setSelectedIds(prev => 
      prev.includes(id) ? prev.filter(i => i !== id) : [...prev, id]
    );
  };

  const toggleSelectAll = () => {
    setSelectedIds(prev => 
      prev.length === filteredAccounts.length ? [] : filteredAccounts.map(a => a.id)
    );
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <div className="relative w-64">
              <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="搜索账号名 / 编号..."
                className="pl-9 h-10 bg-card/50"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>

            <Select value={groupFilter} onValueChange={(val) => val && setGroupFilter(val)}>
              <SelectTrigger className="w-[160px] h-10 bg-card/50">
                <SelectValue placeholder="全部分组" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部分组 ({accounts.length})</SelectItem>
                {groups.map(g => (
                  <SelectItem key={g.label} value={g.label}>{g.label} ({g.count})</SelectItem>
                ))}
              </SelectContent>
            </Select>

            <div className="flex items-center rounded-lg border bg-muted/30 p-1">
              {[
                { id: "all", label: "全部" },
                { id: "available", label: "可用" },
                { id: "low_quota", label: "低配额" },
              ].map((f) => (
                <button
                  key={f.id}
                  onClick={() => setStatusFilter(f.id as StatusFilter)}
                  className={cn(
                    "px-4 py-1.5 text-xs font-medium rounded-md transition-all",
                    statusFilter === f.id ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  {f.label}
                </button>
              ))}
            </div>
          </div>

          <div className="flex items-center gap-2">
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button variant="outline" className="gap-2 h-10">
                  账号操作 <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuItem onClick={() => setAddAccountModalOpen(true)}>
                  <Plus className="h-4 w-4 mr-2" /> 添加账号
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => importByDirectory()}>
                  <FolderOpen className="h-4 w-4 mr-2" /> 按文件夹导入
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem className="text-primary">
                  <RefreshCw className="h-4 w-4 mr-2" /> 刷新所有账号
                </DropdownMenuItem>
                <DropdownMenuItem className={cn(selectedIds.length === 0 && "opacity-50")}>
                  <Download className="h-4 w-4 mr-2" /> 导出选中账号
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={selectedIds.length === 0} className="text-destructive">
                  <Trash2 className="h-4 w-4 mr-2" /> 删除选中账号
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => deleteUnavailableFree()} className="text-destructive">
                  <Trash2 className="h-4 w-4 mr-2" /> 一键清理不可用免费
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button className="gap-2 h-10 shadow-lg shadow-primary/20">
              <RefreshCw className="h-4 w-4" /> 刷新所有
            </Button>
          </div>
        </div>
      </div>

      <Card className="border-none bg-card/50 shadow-xl backdrop-blur-md overflow-hidden">
        <CardContent className="p-0">
          <Table>
            <TableHeader className="bg-muted/30">
              <TableRow>
                <TableHead className="w-12 text-center">
                  <Checkbox 
                    checked={filteredAccounts.length > 0 && selectedIds.length === filteredAccounts.length}
                    onCheckedChange={toggleSelectAll}
                  />
                </TableHead>
                <TableHead>账号名称</TableHead>
                <TableHead>分组</TableHead>
                <TableHead className="w-24">
                  <div className="flex items-center gap-1">
                    顺序 <ArrowUpDown className="h-3 w-3" />
                  </div>
                </TableHead>
                <TableHead>状态</TableHead>
                <TableHead>最近刷新</TableHead>
                <TableHead className="text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell><Skeleton className="h-4 w-4 mx-auto" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-32" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-20" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-12" /></TableCell>
                    <TableCell><Skeleton className="h-6 w-16 rounded-full" /></TableCell>
                    <TableCell><Skeleton className="h-4 w-24" /></TableCell>
                    <TableCell className="text-right"><Skeleton className="h-8 w-8 ml-auto" /></TableCell>
                  </TableRow>
                ))
              ) : filteredAccounts.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="h-48 text-center">
                    <div className="flex flex-col items-center justify-center text-muted-foreground gap-2">
                      <Search className="h-8 w-8 opacity-20" />
                      <p>未找到符合条件的账号</p>
                    </div>
                  </TableCell>
                </TableRow>
              ) : (
                filteredAccounts.map((account) => (
                  <TableRow key={account.id} className="group transition-colors hover:bg-muted/30">
                    <TableCell className="text-center">
                      <Checkbox 
                        checked={selectedIds.includes(account.id)}
                        onCheckedChange={() => toggleSelect(account.id)}
                      />
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-semibold text-sm">{account.name}</span>
                        <span className="text-[10px] text-muted-foreground font-mono opacity-60 uppercase">{account.id.slice(0, 8)}...</span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="font-normal px-2 py-0 h-5 bg-accent/50 text-accent-foreground border-none">
                        {account.group || "默认"}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <span className="text-xs font-mono bg-muted/50 px-2 py-0.5 rounded">{account.priority || 0}</span>
                    </TableCell>
                    <TableCell>
                      {account.is_available ? (
                        <div className="flex items-center gap-1.5">
                          <div className="h-1.5 w-1.5 rounded-full bg-green-500" />
                          <span className="text-xs text-green-600 dark:text-green-400 font-medium">可用</span>
                        </div>
                      ) : (
                        <div className="flex items-center gap-1.5">
                          <div className="h-1.5 w-1.5 rounded-full bg-red-500" />
                          <span className="text-xs text-red-600 dark:text-red-400 font-medium">不可用</span>
                        </div>
                      )}
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {account.last_refresh_at || "从未刷新"}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <Button 
                          variant="ghost" 
                          size="icon" 
                          className="h-8 w-8 text-muted-foreground hover:text-primary"
                          onClick={() => refreshAccount(account.id)}
                        >
                          <RefreshCw className="h-4 w-4" />
                        </Button>
                        <DropdownMenu>
                          <DropdownMenuTrigger>
                            <Button variant="ghost" size="icon" className="h-8 w-8">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem className="gap-2">
                              <ExternalLink className="h-4 w-4" /> 详情与日志
                            </DropdownMenuItem>
                            <DropdownMenuItem className="gap-2 text-red-500" onClick={() => deleteAccount(account.id)}>
                              <Trash2 className="h-4 w-4" /> 删除账号
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
      
      <div className="flex items-center justify-between px-2">
        <div className="text-xs text-muted-foreground">
          共 {filteredAccounts.length} 个账号 {selectedIds.length > 0 && <span className="text-primary ml-1">(已选择 {selectedIds.length} 个)</span>}
        </div>
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground whitespace-nowrap">每页显示</span>
            <Select value={pageSize} onValueChange={(val) => val && setPageSize(val)}>
              <SelectTrigger className="h-8 w-[70px] text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {["5", "10", "20", "50", "100", "500"].map(v => (
                  <SelectItem key={v} value={v}>{v}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex items-center gap-2">
             <Button variant="outline" size="sm" className="h-8 px-3 text-xs" disabled>上一页</Button>
             <div className="text-xs font-medium min-w-[60px] text-center">第 1 / 1 页</div>
             <Button variant="outline" size="sm" className="h-8 px-3 text-xs" disabled>下一页</Button>
          </div>
        </div>
      </div>

      <AddAccountModal 
        open={addAccountModalOpen} 
        onOpenChange={setAddAccountModalOpen} 
      />
    </div>
  );
}
