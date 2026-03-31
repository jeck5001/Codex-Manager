"use client";

import { useMemo } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { accountClient } from "@/lib/api/account-client";
import { getAppErrorMessage } from "@/lib/api/transport";

type RegisterTaskListParams = Parameters<typeof accountClient.listRegisterTasks>[0];
type NormalizedRegisterTaskListParams = {
  page: number;
  pageSize: number;
  status: string | null;
};

function buildTaskListQueryKey(filters: NormalizedRegisterTaskListParams) {
  return [
    "register-tasks",
    filters.status || "all",
    filters.page,
    filters.pageSize,
  ] as const;
}

export function useRegisterTasks(filters: RegisterTaskListParams = {}) {
  const queryClient = useQueryClient();
  const normalizedFilters = useMemo(
    () => ({
      page: Math.max(1, filters.page ?? 1),
      pageSize: Math.max(1, filters.pageSize ?? 20),
      status: filters.status || null,
    }),
    [filters.page, filters.pageSize, filters.status],
  );

  const tasksQuery = useQuery({
    queryKey: buildTaskListQueryKey(normalizedFilters),
    queryFn: () => accountClient.listRegisterTasks(normalizedFilters),
    retry: 1,
    refetchInterval: (query) => {
      const tasks = query.state.data?.tasks || [];
      return tasks.some((task) => {
        const status = String(task.status || "").trim().toLowerCase();
        return status === "pending" || status === "running";
      })
        ? 3000
        : false;
    },
  });

  const statsQuery = useQuery({
    queryKey: ["register-stats"],
    queryFn: () => accountClient.getRegisterStats(),
    retry: 1,
    refetchInterval: 10000,
  });

  const invalidateAll = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["register-tasks"] }),
      queryClient.invalidateQueries({ queryKey: ["register-stats"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  };

  const cancelMutation = useMutation({
    mutationFn: (taskUuid: string) => accountClient.cancelRegisterTask(taskUuid),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("已提交取消请求");
    },
    onError: (error: unknown) => {
      toast.error(`取消失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (taskUuid: string) => accountClient.deleteRegisterTask(taskUuid),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("任务已删除");
    },
    onError: (error: unknown) => {
      toast.error(`删除失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteManyMutation = useMutation({
    mutationFn: (taskUuids: string[]) => accountClient.deleteRegisterTasks(taskUuids),
    onSuccess: async (result) => {
      await invalidateAll();
      if (result.failedCount > 0) {
        toast.warning(`已删除 ${result.deletedCount} 条，${result.failedCount} 条删除失败`);
        return;
      }
      toast.success(`已删除 ${result.deletedCount} 条任务`);
    },
    onError: (error: unknown) => {
      toast.error(`批量删除失败: ${getAppErrorMessage(error)}`);
    },
  });

  const retryMutation = useMutation({
    mutationFn: ({ taskUuid, strategy }: { taskUuid: string; strategy?: string | null }) =>
      accountClient.retryRegisterTask(taskUuid, strategy),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("已重新发起注册任务");
    },
    onError: (error: unknown) => {
      toast.error(`重新发起失败: ${getAppErrorMessage(error)}`);
    },
  });

  return {
    tasks: tasksQuery.data?.tasks || [],
    total: tasksQuery.data?.total || 0,
    stats: statsQuery.data || null,
    isLoading: tasksQuery.isLoading,
    isStatsLoading: statsQuery.isLoading,
    refetchTasks: tasksQuery.refetch,
    cancelTask: cancelMutation.mutateAsync,
    retryTask: (taskUuid: string, strategy?: string | null) =>
      retryMutation.mutateAsync({ taskUuid, strategy }),
    deleteTask: deleteMutation.mutateAsync,
    deleteTasks: deleteManyMutation.mutateAsync,
    isCancelling: cancelMutation.isPending,
    isRetrying: retryMutation.isPending,
    isDeleting: deleteMutation.isPending,
    isDeletingMany: deleteManyMutation.isPending,
  };
}
