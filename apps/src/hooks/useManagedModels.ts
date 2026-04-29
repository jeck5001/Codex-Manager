"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { accountClient, ManagedModelPayload } from "@/lib/api/account-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import { useAppStore } from "@/lib/store/useAppStore";
import { ManagedModelCatalog, ManagedModelInfo } from "@/types";

type BatchDeleteManagedModelsResult = {
  deleted: string[];
  failed: Array<{ slug: string; reason: string }>;
};

function buildManagedModelQueryKey(addr: string) {
  return ["managed-model-catalog", addr || ""] as const;
}

function t(message: string, params?: Record<string, string | number>) {
  if (!params) return message;
  return message.replace(/\{(\w+)\}/g, (_, key) => String(params[key] ?? ""));
}

export function useManagedModels() {
  const queryClient = useQueryClient();
  const serviceStatus = useAppStore((state) => state.serviceStatus);
  const isServiceReady = serviceStatus.connected;
  const queryKey = buildManagedModelQueryKey(serviceStatus.addr);

  const query = useQuery({
    queryKey,
    queryFn: () => accountClient.listManagedModels(false),
    enabled: isServiceReady,
    retry: 1,
  });

  const refreshMutation = useMutation({
    mutationFn: () => accountClient.listManagedModels(true),
  });
  const saveMutation = useMutation({
    mutationFn: (payload: ManagedModelPayload) => accountClient.saveManagedModel(payload),
  });
  const deleteMutation = useMutation({
    mutationFn: (slug: string) => accountClient.deleteManagedModel(slug),
  });

  const setCatalog = (catalog: ManagedModelCatalog) => {
    queryClient.setQueryData(queryKey, catalog);
  };

  const invalidateAll = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey }),
      queryClient.invalidateQueries({ queryKey: ["apikey-models"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  };

  const ensureServiceReady = (actionLabel: string): boolean => {
    if (isServiceReady) {
      return true;
    }
    toast.info(`${t("服务未连接，暂时无法")} ${t(actionLabel)}`);
    return false;
  };

  const reloadManagedCatalog = async (): Promise<ManagedModelCatalog> => {
    const catalog = await accountClient.listManagedModels(false);
    setCatalog(catalog);
    return catalog;
  };

  const refreshRemote = async (): Promise<ManagedModelCatalog | null> => {
    if (!ensureServiceReady("刷新模型目录")) {
      return null;
    }
    try {
      const catalog = await refreshMutation.mutateAsync();
      setCatalog(catalog);
      await invalidateAll();
      toast.success(t("模型目录已刷新"));
      return catalog;
    } catch (error) {
      toast.error(`${t("刷新模型失败")}: ${getAppErrorMessage(error)}`);
      return null;
    }
  };

  const saveModel = async (
    payload: ManagedModelPayload
  ): Promise<ManagedModelInfo | null> => {
    if (!ensureServiceReady("保存模型")) {
      return null;
    }
    try {
      const saved = await saveMutation.mutateAsync(payload);
      await reloadManagedCatalog();
      await invalidateAll();
      toast.success(t("模型已保存"));
      return saved;
    } catch (error) {
      toast.error(`${t("保存模型失败")}: ${getAppErrorMessage(error)}`);
      return null;
    }
  };

  const deleteModel = async (slug: string): Promise<boolean> => {
    if (!ensureServiceReady("删除模型")) {
      return false;
    }
    try {
      await deleteMutation.mutateAsync(slug);
      await reloadManagedCatalog();
      await invalidateAll();
      toast.success(t("模型已删除"));
      return true;
    } catch (error) {
      toast.error(`${t("删除模型失败")}: ${getAppErrorMessage(error)}`);
      return false;
    }
  };

  const deleteModels = async (
    slugs: string[]
  ): Promise<BatchDeleteManagedModelsResult> => {
    if (!ensureServiceReady("批量删除模型")) {
      return { deleted: [], failed: [] };
    }
    const normalizedSlugs = Array.from(
      new Set(
        slugs
          .map((slug) => String(slug || "").trim())
          .filter(Boolean)
      )
    );

    const deleted: string[] = [];
    const failed: Array<{ slug: string; reason: string }> = [];
    for (const slug of normalizedSlugs) {
      try {
        await deleteMutation.mutateAsync(slug);
        deleted.push(slug);
      } catch (error) {
        failed.push({ slug, reason: getAppErrorMessage(error) });
      }
    }

    if (deleted.length > 0) {
      await reloadManagedCatalog();
      await invalidateAll();
    }

    if (deleted.length > 0 && failed.length === 0) {
      toast.success(t("已删除 {count} 个模型", { count: deleted.length }));
    } else if (deleted.length > 0) {
      toast.warning(
        t("批量删除完成：成功{success}个，失败{failed}个", {
          success: deleted.length,
          failed: failed.length,
        })
      );
    } else if (failed.length > 0) {
      toast.error(`${t("批量删除失败")}: ${failed[0].slug} - ${failed[0].reason}`);
    }

    return { deleted, failed };
  };

  return {
    models: query.data?.items || [],
    catalog: query.data || { items: [] },
    isLoading: isServiceReady && query.isLoading,
    isServiceReady,
    refreshRemote,
    saveModel,
    deleteModel,
    deleteModels,
    isRefreshing: refreshMutation.isPending,
    isSaving: saveMutation.isPending,
    isDeleting: deleteMutation.isPending,
  };
}
