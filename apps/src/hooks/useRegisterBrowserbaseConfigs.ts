"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { accountClient } from "@/lib/api/account-client";
import { getAppErrorMessage } from "@/lib/api/transport";

export function useRegisterBrowserbaseConfigs() {
  const queryClient = useQueryClient();

  const configsQuery = useQuery({
    queryKey: ["register-browserbase-configs"],
    queryFn: () => accountClient.listRegisterBrowserbaseConfigs(),
    retry: 1,
  });

  const invalidateAll = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["register-browserbase-configs"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  };

  const createMutation = useMutation({
    mutationFn: (payload: Parameters<typeof accountClient.createRegisterBrowserbaseConfig>[0]) =>
      accountClient.createRegisterBrowserbaseConfig(payload),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("Browserbase 配置已创建");
    },
    onError: (error: unknown) => {
      toast.error(`创建失败: ${getAppErrorMessage(error)}`);
    },
  });

  const updateMutation = useMutation({
    mutationFn: (payload: Parameters<typeof accountClient.updateRegisterBrowserbaseConfig>[0]) =>
      accountClient.updateRegisterBrowserbaseConfig(payload),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("Browserbase 配置已更新");
    },
    onError: (error: unknown) => {
      toast.error(`更新失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (configId: number) => accountClient.deleteRegisterBrowserbaseConfig(configId),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("Browserbase 配置已删除");
    },
    onError: (error: unknown) => {
      toast.error(`删除失败: ${getAppErrorMessage(error)}`);
    },
  });

  const readFullMutation = useMutation({
    mutationFn: (configId: number) => accountClient.readRegisterBrowserbaseConfigFull(configId),
    onError: (error: unknown) => {
      toast.error(`读取详情失败: ${getAppErrorMessage(error)}`);
    },
  });

  return {
    configs: configsQuery.data?.configs || [],
    total: configsQuery.data?.total || 0,
    isLoading: configsQuery.isLoading,
    refetchConfigs: configsQuery.refetch,
    createBrowserbaseConfig: createMutation.mutateAsync,
    updateBrowserbaseConfig: updateMutation.mutateAsync,
    deleteBrowserbaseConfig: deleteMutation.mutateAsync,
    readBrowserbaseConfigFull: readFullMutation.mutateAsync,
    isCreating: createMutation.isPending,
    isUpdating: updateMutation.isPending,
    isDeleting: deleteMutation.isPending,
    isReadingFull: readFullMutation.isPending,
  };
}
