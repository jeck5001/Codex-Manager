"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { accountClient } from "@/lib/api/account-client";
import { toast } from "sonner";
import { useMemo } from "react";

export function useAccounts() {
  const queryClient = useQueryClient();

  const accountsQuery = useQuery({
    queryKey: ["accounts"],
    queryFn: () => accountClient.list(),
    retry: 1,
  });

  const accounts = useMemo(() => {
    return Array.isArray(accountsQuery.data) ? accountsQuery.data : [];
  }, [accountsQuery.data]);

  // Derived groups
  const groups = useMemo(() => {
    if (!Array.isArray(accounts)) return [];
    const map: Record<string, number> = {};
    accounts.forEach(acc => {
      const g = acc.group || "默认";
      map[g] = (map[g] || 0) + 1;
    });
    return Object.entries(map).map(([label, count]) => ({ label, count }));
  }, [accounts]);

  const refreshMutation = useMutation({
    mutationFn: (id: string) => accountClient.refreshUsage(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success("刷新成功");
    },
    onError: (err: any) => {
      toast.error(`刷新失败: ${err.message}`);
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => accountClient.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success("账号已删除");
    },
  });

  const deleteUnavailableFreeMutation = useMutation({
    mutationFn: () => accountClient.deleteUnavailableFree(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(`清理完成`);
    },
  });

  const importByDirectoryMutation = useMutation({
    mutationFn: () => accountClient.importByDirectory(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success("文件夹导入请求已发送");
    },
  });

  return {
    accounts,
    groups,
    isLoading: accountsQuery.isLoading,
    refreshAccount: refreshMutation.mutate,
    deleteAccount: deleteMutation.mutate,
    deleteUnavailableFree: deleteUnavailableFreeMutation.mutate,
    importByDirectory: importByDirectoryMutation.mutate,
    isRefreshing: refreshMutation.isPending,
  };
}
