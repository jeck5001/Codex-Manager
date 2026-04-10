"use client";

import { useMemo } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { accountClient } from "@/lib/api/account-client";
import { getAppErrorMessage } from "@/lib/api/transport";

type RegisterEmailServiceListParams = Parameters<
  typeof accountClient.listRegisterEmailServices
>[0];
type RegisterEmailServiceCreatePayload = Parameters<
  typeof accountClient.createRegisterEmailService
>[0];
type RegisterEmailServiceUpdatePayload = Parameters<
  typeof accountClient.updateRegisterEmailService
>[0];
type RegisterOutlookBatchImportPayload = Parameters<
  typeof accountClient.outlookBatchImportRegisterEmailServices
>[0];
type RegisterTempMailCloudflareSettingsPayload = Parameters<
  typeof accountClient.setRegisterTempMailCloudflareSettings
>[0];

function buildListQueryKey(filters: RegisterEmailServiceListParams) {
  return [
    "register-email-services",
    filters?.serviceType || "all",
    filters?.enabledOnly === true ? "enabled" : "all-status",
  ] as const;
}

export function useRegisterEmailServices(filters: RegisterEmailServiceListParams = {}) {
  const queryClient = useQueryClient();
  const normalizedFilters = useMemo(
    () => ({
      serviceType: filters?.serviceType || null,
      enabledOnly: filters?.enabledOnly === true,
    }),
    [filters?.enabledOnly, filters?.serviceType]
  );

  const typesQuery = useQuery({
    queryKey: ["register-email-service-types"],
    queryFn: () => accountClient.getRegisterEmailServiceTypes(),
    retry: 1,
  });

  const servicesQuery = useQuery({
    queryKey: buildListQueryKey(normalizedFilters),
    queryFn: () => accountClient.listRegisterEmailServices(normalizedFilters),
    retry: 1,
  });

  const statsQuery = useQuery({
    queryKey: ["register-email-service-stats"],
    queryFn: () => accountClient.getRegisterEmailServiceStats(),
    retry: 1,
  });

  const cloudflareSettingsQuery = useQuery({
    queryKey: ["register-temp-mail-cloudflare-settings"],
    queryFn: () => accountClient.getRegisterTempMailCloudflareSettings(),
    retry: 1,
  });

  const invalidateAll = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["register-email-service-types"] }),
      queryClient.invalidateQueries({ queryKey: ["register-email-services"] }),
      queryClient.invalidateQueries({ queryKey: ["register-email-service-stats"] }),
      queryClient.invalidateQueries({ queryKey: ["register-temp-mail-cloudflare-settings"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  };

  const saveCloudflareSettingsMutation = useMutation({
    mutationFn: (payload: RegisterTempMailCloudflareSettingsPayload) =>
      accountClient.setRegisterTempMailCloudflareSettings(payload),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["register-temp-mail-cloudflare-settings"],
      });
      toast.success("Cloudflare Temp-Mail 设置已保存");
    },
    onError: (error: unknown) => {
      toast.error(`保存 Cloudflare 设置失败: ${getAppErrorMessage(error)}`);
    },
  });

  const createMutation = useMutation({
    mutationFn: (payload: RegisterEmailServiceCreatePayload) =>
      accountClient.createRegisterEmailService(payload),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("邮箱服务已创建");
    },
    onError: (error: unknown) => {
      toast.error(`创建失败: ${getAppErrorMessage(error)}`);
    },
  });

  const updateMutation = useMutation({
    mutationFn: (payload: RegisterEmailServiceUpdatePayload) =>
      accountClient.updateRegisterEmailService(payload),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("邮箱服务已更新");
    },
    onError: (error: unknown) => {
      toast.error(`更新失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (serviceId: number) => accountClient.deleteRegisterEmailService(serviceId),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("邮箱服务已删除");
    },
    onError: (error: unknown) => {
      toast.error(`删除失败: ${getAppErrorMessage(error)}`);
    },
  });

  const readFullMutation = useMutation({
    mutationFn: (serviceId: number) => accountClient.readRegisterEmailServiceFull(serviceId),
    onError: (error: unknown) => {
      toast.error(`读取详情失败: ${getAppErrorMessage(error)}`);
    },
  });

  const testMutation = useMutation({
    mutationFn: (serviceId: number) => accountClient.testRegisterEmailService(serviceId),
    onSuccess: (result) => {
      if (result.success) {
        toast.success(result.message || "服务测试通过");
      } else {
        toast.error(result.message || "服务测试失败");
      }
    },
    onError: (error: unknown) => {
      toast.error(`测试失败: ${getAppErrorMessage(error)}`);
    },
  });

  const toggleEnabledMutation = useMutation({
    mutationFn: ({ serviceId, enabled }: { serviceId: number; enabled: boolean }) =>
      accountClient.setRegisterEmailServiceEnabled(serviceId, enabled),
    onSuccess: async (_result, variables) => {
      await invalidateAll();
      toast.success(variables.enabled ? "邮箱服务已启用" : "邮箱服务已禁用");
    },
    onError: (error: unknown, variables) => {
      toast.error(
        `${variables.enabled ? "启用" : "禁用"}失败: ${getAppErrorMessage(error)}`
      );
    },
  });

  const outlookBatchImportMutation = useMutation({
    mutationFn: (payload: RegisterOutlookBatchImportPayload) =>
      accountClient.outlookBatchImportRegisterEmailServices(payload),
    onSuccess: async (result) => {
      await invalidateAll();
      toast.success(
        `批量导入完成：成功 ${result.success}，失败 ${result.failed}`
      );
      if (result.errors.length > 0) {
        toast.info(`另有 ${result.errors.length} 条错误明细，请在弹窗中查看`);
      }
    },
    onError: (error: unknown) => {
      toast.error(`批量导入失败: ${getAppErrorMessage(error)}`);
    },
  });

  const batchDeleteOutlookMutation = useMutation({
    mutationFn: (serviceIds: number[]) =>
      accountClient.batchDeleteRegisterOutlookEmailServices(serviceIds),
    onSuccess: async (result) => {
      await invalidateAll();
      toast.success(result.message || `已删除 ${result.deleted} 个 Outlook 账户`);
    },
    onError: (error: unknown) => {
      toast.error(`批量删除失败: ${getAppErrorMessage(error)}`);
    },
  });

  const reorderMutation = useMutation({
    mutationFn: (serviceIds: number[]) =>
      accountClient.reorderRegisterEmailServices({ serviceIds }),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("邮箱服务优先级已更新");
    },
    onError: (error: unknown) => {
      toast.error(`更新顺序失败: ${getAppErrorMessage(error)}`);
    },
  });

  const testTempmailMutation = useMutation({
    mutationFn: (apiUrl?: string | null) => accountClient.testRegisterTempmail(apiUrl),
    onSuccess: (result) => {
      if (result.success) {
        toast.success(result.message || "Tempmail 连接正常");
      } else {
        toast.error(result.message || "Tempmail 连接失败");
      }
    },
    onError: (error: unknown) => {
      toast.error(`测试失败: ${getAppErrorMessage(error)}`);
    },
  });

  return {
    serviceTypes: typesQuery.data?.types || [],
    services: servicesQuery.data?.services || [],
    total: servicesQuery.data?.total || 0,
    stats: statsQuery.data || null,
    cloudflareSettings: cloudflareSettingsQuery.data || null,
    isLoading: servicesQuery.isLoading,
    isTypesLoading: typesQuery.isLoading,
    isStatsLoading: statsQuery.isLoading,
    isCloudflareSettingsLoading: cloudflareSettingsQuery.isLoading,
    refetchServices: servicesQuery.refetch,
    refetchCloudflareSettings: cloudflareSettingsQuery.refetch,
    createEmailService: createMutation.mutateAsync,
    updateEmailService: updateMutation.mutateAsync,
    saveCloudflareSettings: saveCloudflareSettingsMutation.mutateAsync,
    deleteEmailService: deleteMutation.mutate,
    readEmailServiceFull: readFullMutation.mutateAsync,
    testEmailService: testMutation.mutateAsync,
    setEmailServiceEnabled: toggleEnabledMutation.mutate,
    importOutlookServices: outlookBatchImportMutation.mutateAsync,
    batchDeleteOutlookServices: batchDeleteOutlookMutation.mutateAsync,
    reorderEmailServices: reorderMutation.mutateAsync,
    testTempmailConnection: testTempmailMutation.mutateAsync,
    isCreating: createMutation.isPending,
    isUpdating: updateMutation.isPending,
    isDeleting: deleteMutation.isPending,
    isReadingFull: readFullMutation.isPending,
    isTesting: testMutation.isPending,
    isToggling: toggleEnabledMutation.isPending,
    isImporting: outlookBatchImportMutation.isPending,
    isBatchDeletingOutlook: batchDeleteOutlookMutation.isPending,
    isReordering: reorderMutation.isPending,
    isTestingTempmail: testTempmailMutation.isPending,
    isSavingCloudflareSettings: saveCloudflareSettingsMutation.isPending,
    lastImportResult: outlookBatchImportMutation.data || null,
  };
}
