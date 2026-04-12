"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { accountClient, ManagedModelPayload } from "@/lib/api/account-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import { useDesktopPageActive } from "@/hooks/useDesktopPageActive";
import { useDeferredDesktopActivation } from "@/hooks/useDeferredDesktopActivation";
import { useRuntimeCapabilities } from "@/hooks/useRuntimeCapabilities";
import { useI18n } from "@/lib/i18n/provider";
import { useAppStore } from "@/lib/store/useAppStore";

const MANAGED_MODEL_QUERY_KEY = ["managed-model-catalog"];

export function useManagedModels() {
  const queryClient = useQueryClient();
  const { t } = useI18n();
  const serviceStatus = useAppStore((state) => state.serviceStatus);
  const { canAccessManagementRpc } = useRuntimeCapabilities();
  const isServiceReady = canAccessManagementRpc && serviceStatus.connected;
  const isPageActive = useDesktopPageActive("/models/");
  const isQueryEnabled = useDeferredDesktopActivation(isServiceReady && isPageActive);

  const ensureServiceReady = (actionLabel: string): boolean => {
    if (isServiceReady) {
      return true;
    }
    toast.info(`${t("服务未连接，暂时无法")} ${t(actionLabel)}`);
    return false;
  };

  const query = useQuery({
    queryKey: MANAGED_MODEL_QUERY_KEY,
    queryFn: () => accountClient.listManagedModels(false),
    enabled: isQueryEnabled,
    retry: 1,
  });

  const invalidateAll = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: MANAGED_MODEL_QUERY_KEY }),
      queryClient.invalidateQueries({ queryKey: ["apikey-models"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  };

  const refreshMutation = useMutation({
    mutationFn: (refreshRemote: boolean) => accountClient.listManagedModels(refreshRemote),
    onSuccess: async (catalog) => {
      queryClient.setQueryData(MANAGED_MODEL_QUERY_KEY, catalog);
      await invalidateAll();
      toast.success(t("模型目录已刷新"));
    },
    onError: (error: unknown) => {
      toast.error(`${t("刷新模型失败")}: ${getAppErrorMessage(error)}`);
    },
  });

  const saveMutation = useMutation({
    mutationFn: (params: ManagedModelPayload) => accountClient.saveManagedModel(params),
    onSuccess: async () => {
      await invalidateAll();
      toast.success(t("模型已保存"));
    },
    onError: (error: unknown) => {
      toast.error(`${t("保存模型失败")}: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (slug: string) => accountClient.deleteManagedModel(slug),
    onSuccess: async () => {
      await invalidateAll();
      toast.success(t("模型已删除"));
    },
    onError: (error: unknown) => {
      toast.error(`${t("删除模型失败")}: ${getAppErrorMessage(error)}`);
    },
  });

  return {
    models: query.data?.items || [],
    catalog: query.data || { items: [] },
    isLoading: isServiceReady && (!isQueryEnabled || query.isLoading),
    isServiceReady,
    refreshRemote: async () => {
      if (!ensureServiceReady("刷新模型")) return null;
      return refreshMutation.mutateAsync(true);
    },
    refreshLocal: async () => {
      if (!ensureServiceReady("读取模型")) return null;
      return refreshMutation.mutateAsync(false);
    },
    saveModel: async (params: ManagedModelPayload) => {
      if (!ensureServiceReady("保存模型")) return null;
      return saveMutation.mutateAsync(params);
    },
    deleteModel: async (slug: string) => {
      if (!ensureServiceReady("删除模型")) return false;
      await deleteMutation.mutateAsync(slug);
      return true;
    },
    isRefreshing: refreshMutation.isPending,
    isSaving: saveMutation.isPending,
    isDeleting: deleteMutation.isPending,
  };
}
