"use client";

import { useMemo } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { accountClient } from "@/lib/api/account-client";
import { attachUsagesToAccounts } from "@/lib/api/normalize";
import { serviceClient } from "@/lib/api/service-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import { useAppStore } from "@/lib/store/useAppStore";

type ImportByDirectoryResult = Awaited<ReturnType<typeof accountClient.importByDirectory>>;
type ImportByFileResult = Awaited<ReturnType<typeof accountClient.importByFile>>;
type ExportResult = Awaited<ReturnType<typeof accountClient.export>>;
type DeleteUnavailableFreeResult = { deleted?: number };
type DeleteBannedAccountsResult = { deleted?: number };
type SubscriptionCheckResult = Awaited<
  ReturnType<typeof accountClient.checkSubscription>
>;
type SubscriptionCheckManyResult = Awaited<
  ReturnType<typeof accountClient.checkSubscriptions>
>;
type TeamManagerUploadResult = Awaited<
  ReturnType<typeof accountClient.uploadToTeamManager>
>;
type TeamManagerUploadManyResult = Awaited<
  ReturnType<typeof accountClient.uploadManyToTeamManager>
>;
type BatchMarkSubscriptionResult = {
  successCount: number;
  failedCount: number;
};

function isAccountRefreshBlocked(status: string | null | undefined): boolean {
  return String(status || "").trim().toLowerCase() === "disabled";
}

function isRefreshTokenExpiredError(error: unknown): boolean {
  return getAppErrorMessage(error)
    .toLowerCase()
    .includes("refresh token failed with status 401");
}

function buildImportSummaryMessage(result: ImportByDirectoryResult): string {
  const total = Number(result?.total || 0);
  const created = Number(result?.created || 0);
  const updated = Number(result?.updated || 0);
  const failed = Number(result?.failed || 0);
  return `导入完成：共${total}，新增${created}，更新${updated}，失败${failed}`;
}

function formatUsageRefreshErrorMessage(error: unknown): string {
  const message = getAppErrorMessage(error);
  if (message.toLowerCase().includes("your openai account has been deactivated")) {
    return "账号已被 OpenAI 停用，已自动标记到停用列表";
  }
  if (isRefreshTokenExpiredError(error)) {
    return "账号长期未登录，refresh 已过期，已改为不可用状态";
  }
  return message;
}

function formatPlanTypeLabel(planType: string | null | undefined): string {
  const normalized = String(planType || "").trim().toLowerCase();
  if (normalized === "team") return "Team";
  if (normalized === "plus") return "Plus";
  if (normalized === "free") return "Free";
  return normalized || "未知";
}

export function useAccounts() {
  const queryClient = useQueryClient();
  const serviceStatus = useAppStore((state) => state.serviceStatus);

  const accountsQuery = useQuery({
    queryKey: ["accounts", "list"],
    queryFn: () => accountClient.list(),
    retry: 1,
  });

  const usagesQuery = useQuery({
    queryKey: ["usage", "list"],
    queryFn: () => accountClient.listUsage(),
    retry: 1,
  });

  const manualPreferredAccountQuery = useQuery({
    queryKey: ["gateway", "manual-account", serviceStatus.addr || null],
    queryFn: () => serviceClient.getManualPreferredAccountId(),
    enabled: serviceStatus.connected,
    retry: 1,
  });

  const accounts = useMemo(() => {
    return attachUsagesToAccounts(
      accountsQuery.data?.items || [],
      usagesQuery.data || []
    );
  }, [accountsQuery.data?.items, usagesQuery.data]);

  const groups = useMemo(() => {
    const map = new Map<string, number>();
    for (const account of accounts) {
      const group = account.group || "默认";
      map.set(group, (map.get(group) || 0) + 1);
    }
    return Array.from(map.entries())
      .sort((left, right) => left[0].localeCompare(right[0], "zh-Hans-CN"))
      .map(([label, count]) => ({ label, count }));
  }, [accounts]);

  const invalidateAll = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["accounts"] }),
      queryClient.invalidateQueries({ queryKey: ["usage"] }),
      queryClient.invalidateQueries({ queryKey: ["usage-aggregate"] }),
      queryClient.invalidateQueries({ queryKey: ["today-summary"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
      queryClient.invalidateQueries({ queryKey: ["gateway", "manual-account"] }),
      queryClient.invalidateQueries({ queryKey: ["logs"] }),
    ]);
  };

  const invalidateManualPreferred = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["gateway", "manual-account"] }),
      queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
    ]);
  };

  const refreshAccountMutation = useMutation({
    mutationFn: (accountId: string) => accountClient.refreshUsage(accountId),
    onSuccess: () => {
      toast.success("账号用量已刷新");
    },
    onError: (error: unknown) => {
      if (isRefreshTokenExpiredError(error)) {
        return;
      }
      toast.error(`刷新失败: ${formatUsageRefreshErrorMessage(error)}`);
    },
    onSettled: async () => {
      await invalidateAll();
    },
  });

  const recoverAccountAuthMutation = useMutation({
    mutationFn: ({
      accountId,
      openBrowser,
    }: {
      accountId: string;
      openBrowser: boolean;
    }) => accountClient.recoverAccountAuth(accountId, openBrowser),
  });

  const refreshAllMutation = useMutation({
    mutationFn: () => accountClient.refreshUsage(),
    onSuccess: () => {
      toast.success("账号用量已刷新");
    },
    onError: (error: unknown) => {
      toast.error(`刷新失败: ${formatUsageRefreshErrorMessage(error)}`);
    },
    onSettled: async () => {
      await invalidateAll();
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (accountId: string) => accountClient.delete(accountId),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("账号已删除");
    },
    onError: (error: unknown) => {
      toast.error(`删除失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteManyMutation = useMutation({
    mutationFn: (accountIds: string[]) => accountClient.deleteMany(accountIds),
    onSuccess: async (_result, accountIds) => {
      await invalidateAll();
      toast.success(`已删除 ${accountIds.length} 个账号`);
    },
    onError: (error: unknown) => {
      toast.error(`批量删除失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteUnavailableFreeMutation = useMutation({
    mutationFn: () => accountClient.deleteUnavailableFree(),
    onSuccess: async (result: DeleteUnavailableFreeResult) => {
      await invalidateAll();
      const deleted = Number(result?.deleted || 0);
      if (deleted > 0) {
        toast.success(`已移除 ${deleted} 个不可用免费账号`);
      } else {
        toast.success("未发现可清理的不可用免费账号");
      }
    },
    onError: (error: unknown) => {
      toast.error(`清理失败: ${getAppErrorMessage(error)}`);
    },
  });

  const deleteBannedMutation = useMutation({
    mutationFn: () => accountClient.deleteBanned(),
    onSuccess: async (result: DeleteBannedAccountsResult) => {
      await invalidateAll();
      const deleted = Number(result?.deleted || 0);
      if (deleted > 0) {
        toast.success(`已清理 ${deleted} 个封禁账号`);
      } else {
        toast.success("未发现可清理的封禁账号");
      }
    },
    onError: (error: unknown) => {
      toast.error(`清理失败: ${getAppErrorMessage(error)}`);
    },
  });

  const updateAccountSortMutation = useMutation({
    mutationFn: ({ accountId, sort }: { accountId: string; sort: number }) =>
      accountClient.updateSort(accountId, sort),
    onSuccess: async () => {
      await invalidateAll();
      toast.success("账号顺序已更新");
    },
    onError: (error: unknown) => {
      toast.error(`更新顺序失败: ${getAppErrorMessage(error)}`);
    },
  });

  const toggleAccountStatusMutation = useMutation({
    mutationFn: ({
      accountId,
      enabled,
    }: {
      accountId: string;
      enabled: boolean;
      sourceStatus?: string | null;
    }) =>
      enabled
        ? accountClient.enableAccount(accountId)
        : accountClient.disableAccount(accountId),
    onSuccess: async (_result, variables) => {
      await invalidateAll();
      const normalizedSourceStatus = String(variables.sourceStatus || "")
        .trim()
        .toLowerCase();
      toast.success(
        variables.enabled
          ? normalizedSourceStatus === "inactive" ||
            normalizedSourceStatus === "deactivated"
            ? "账号已恢复"
            : "账号已启用"
          : "账号已禁用"
      );
    },
    onError: (error: unknown, variables) => {
      const normalizedSourceStatus = String(variables.sourceStatus || "")
        .trim()
        .toLowerCase();
      const actionLabel = variables.enabled
        ? normalizedSourceStatus === "inactive" ||
          normalizedSourceStatus === "deactivated"
          ? "恢复"
          : "启用"
        : "禁用";
      toast.error(
        `${actionLabel}账号失败: ${getAppErrorMessage(error)}`
      );
    },
  });

  const bulkToggleAccountStatusMutation = useMutation({
    mutationFn: ({
      accountIds,
      enabled,
    }: {
      accountIds: string[];
      enabled: boolean;
      scopeLabel?: string;
    }) =>
      accountClient.updateManyStatus(accountIds, enabled ? "active" : "disabled"),
    onSuccess: async (result, variables) => {
      await invalidateAll();
      const scopeLabel = variables.scopeLabel || "账号";
      const actionLabel = variables.enabled ? "启用" : "禁用";
      const updated = Number(result?.updated || 0);
      const skipped = Number(result?.skipped || 0);
      const failed = Number(result?.failed || 0);
      toast.success(
        `${actionLabel}${scopeLabel}完成：更新 ${updated}，跳过 ${skipped}，失败 ${failed}`
      );
    },
    onError: (error: unknown, variables) => {
      const actionLabel = variables.enabled ? "启用" : "禁用";
      toast.error(`${actionLabel}账号失败: ${getAppErrorMessage(error)}`);
    },
  });

  const bulkUpdateTagsMutation = useMutation({
    mutationFn: ({
      accountIds,
      tags,
    }: {
      accountIds: string[];
      tags: string[] | string | null;
    }) => accountClient.updateManyTags(accountIds, tags),
    onSuccess: async (result, variables) => {
      await invalidateAll();
      const updated = Number(result?.updated || 0);
      const skipped = Number(result?.skipped || 0);
      const failed = Number(result?.failed || 0);
      const normalizedTags = Array.isArray(variables.tags)
        ? variables.tags.map((item) => String(item || "").trim()).filter(Boolean)
        : String(variables.tags || "")
            .split(",")
            .map((item) => item.trim())
            .filter(Boolean);
      toast.success(
        normalizedTags.length > 0
          ? `标签更新完成：更新 ${updated}，跳过 ${skipped}，失败 ${failed}`
          : `标签清空完成：更新 ${updated}，跳过 ${skipped}，失败 ${failed}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`批量更新标签失败: ${getAppErrorMessage(error)}`);
    },
  });

  const checkSubscriptionMutation = useMutation({
    mutationFn: ({
      accountId,
      proxy,
    }: {
      accountId: string;
      proxy?: string | null;
    }) => accountClient.checkSubscription(accountId, proxy),
    onSuccess: async (result: SubscriptionCheckResult) => {
      await invalidateAll();
      toast.success(
        `${result.accountName || result.accountId} 当前订阅：${formatPlanTypeLabel(
          result.planType
        )}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`订阅检测失败: ${getAppErrorMessage(error)}`);
    },
  });

  const checkSubscriptionsMutation = useMutation({
    mutationFn: ({
      accountIds,
      proxy,
    }: {
      accountIds: string[];
      proxy?: string | null;
    }) => accountClient.checkSubscriptions(accountIds, proxy),
    onSuccess: async (result: SubscriptionCheckManyResult) => {
      await invalidateAll();
      const details = Array.isArray(result?.details) ? result.details : [];
      const planSummary = details
        .filter((item) => item?.success)
        .reduce<Record<string, number>>((summary, item) => {
          const key = formatPlanTypeLabel(item.planType);
          summary[key] = (summary[key] || 0) + 1;
          return summary;
        }, {});
      const planText = Object.entries(planSummary)
        .map(([label, count]) => `${label} ${count}`)
        .join("，");
      toast.success(
        `订阅检测完成：成功 ${Number(result?.successCount || 0)}，失败 ${Number(
          result?.failedCount || 0
        )}${planText ? `，${planText}` : ""}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`批量订阅检测失败: ${getAppErrorMessage(error)}`);
    },
  });

  const markSubscriptionMutation = useMutation({
    mutationFn: ({
      accountId,
      planType,
    }: {
      accountId: string;
      planType: "free" | "plus" | "team";
    }) => accountClient.markSubscription(accountId, planType),
    onSuccess: async (result) => {
      await invalidateAll();
      toast.success(
        `${result.accountName || result.accountId} 已标记为 ${formatPlanTypeLabel(
          result.planType
        )}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`标记订阅失败: ${getAppErrorMessage(error)}`);
    },
  });

  const markManySubscriptionsMutation = useMutation({
    mutationFn: async ({
      accountIds,
      planType,
    }: {
      accountIds: string[];
      planType: "free" | "plus" | "team";
    }): Promise<BatchMarkSubscriptionResult> => {
      const results = await Promise.allSettled(
        accountIds.map((accountId) => accountClient.markSubscription(accountId, planType))
      );
      return results.reduce<BatchMarkSubscriptionResult>(
        (summary, result) => {
          if (result.status === "fulfilled") {
            summary.successCount += 1;
          } else {
            summary.failedCount += 1;
          }
          return summary;
        },
        { successCount: 0, failedCount: 0 }
      );
    },
    onSuccess: async (result, variables) => {
      await invalidateAll();
      toast.success(
        `批量标记 ${formatPlanTypeLabel(variables.planType)} 完成：成功 ${result.successCount}，失败 ${result.failedCount}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`批量标记订阅失败: ${getAppErrorMessage(error)}`);
    },
  });

  const uploadToTeamManagerMutation = useMutation({
    mutationFn: (accountId: string) => accountClient.uploadToTeamManager(accountId),
    onSuccess: async (result: TeamManagerUploadResult) => {
      await invalidateAll();
      toast.success(`${result.accountName || result.accountId} 已上传到 Team Manager`);
    },
    onError: (error: unknown) => {
      toast.error(`上传 Team Manager 失败: ${getAppErrorMessage(error)}`);
    },
  });

  const uploadManyToTeamManagerMutation = useMutation({
    mutationFn: (accountIds: string[]) => accountClient.uploadManyToTeamManager(accountIds),
    onSuccess: async (result: TeamManagerUploadManyResult) => {
      await invalidateAll();
      toast.success(
        `上传 Team Manager 完成：成功 ${Number(result?.successCount || 0)}，失败 ${Number(
          result?.failedCount || 0
        )}，跳过 ${Number(result?.skippedCount || 0)}`
      );
    },
    onError: (error: unknown) => {
      toast.error(`批量上传 Team Manager 失败: ${getAppErrorMessage(error)}`);
    },
  });

  const importByDirectoryMutation = useMutation({
    mutationFn: () => accountClient.importByDirectory(),
    onSuccess: async (result: ImportByDirectoryResult) => {
      if (result?.canceled) {
        toast.info("已取消导入");
        return;
      }
      await invalidateAll();
      toast.success(buildImportSummaryMessage(result));
    },
    onError: (error: unknown) => {
      toast.error(`导入失败: ${getAppErrorMessage(error)}`);
    },
  });

  const importByFileMutation = useMutation({
    mutationFn: () => accountClient.importByFile(),
    onSuccess: async (result: ImportByFileResult) => {
      if (result?.canceled) {
        toast.info("已取消导入");
        return;
      }
      await invalidateAll();
      toast.success(buildImportSummaryMessage(result));
    },
    onError: (error: unknown) => {
      toast.error(`导入失败: ${getAppErrorMessage(error)}`);
    },
  });

  const exportMutation = useMutation({
    mutationFn: (accountIds: string[] = []) => accountClient.export(accountIds),
    onSuccess: (result: ExportResult, accountIds) => {
      if (result?.canceled) {
        toast.info("已取消导出");
        return;
      }
      const exported = Number(result?.exported || 0);
      const outputDir = String(result?.outputDir || "").trim();
      const selectedCount = Array.isArray(accountIds) ? accountIds.length : 0;
      toast.success(
        outputDir
          ? selectedCount > 0
            ? `已导出选中的 ${exported} 个账号到 ${outputDir}`
            : `已导出 ${exported} 个账号到 ${outputDir}`
          : selectedCount > 0
            ? `已导出选中的 ${exported} 个账号`
            : `已导出 ${exported} 个账号`
      );
    },
    onError: (error: unknown) => {
      toast.error(`导出失败: ${getAppErrorMessage(error)}`);
    },
  });

  const setManualPreferredMutation = useMutation({
    mutationFn: (accountId: string) => serviceClient.setManualPreferredAccount(accountId),
    onSuccess: async () => {
      await invalidateManualPreferred();
      toast.success("已设为优先账号");
    },
    onError: (error: unknown) => {
      toast.error(`设置优先账号失败: ${getAppErrorMessage(error)}`);
    },
  });

  const clearManualPreferredMutation = useMutation({
    mutationFn: () => serviceClient.clearManualPreferredAccount(),
    onSuccess: async () => {
      await invalidateManualPreferred();
      toast.success("已取消优先账号");
    },
    onError: (error: unknown) => {
      toast.error(`取消优先账号失败: ${getAppErrorMessage(error)}`);
    },
  });

  const recoverAccountAfterRefreshFailure = async (accountId: string) => {
    toast.info("检测到 refresh 已过期，正在自动恢复登录...");
    const recovery = await recoverAccountAuthMutation.mutateAsync({
      accountId,
      openBrowser: false,
    });
    if (recovery.warning) {
      toast.warning(recovery.warning);
    }

    if (recovery.status !== "recovered") {
      throw new Error(
        getAppErrorMessage(
          recovery.warning || "自动恢复未返回成功状态"
        )
      );
    }
    await accountClient.refreshUsage(recovery.accountId || accountId);
    await invalidateAll();
    toast.success("已自动恢复登录并刷新用量");
  };

  return {
    accounts,
    groups,
    total: accountsQuery.data?.total || accounts.length,
    isLoading: accountsQuery.isLoading || usagesQuery.isLoading,
    manualPreferredAccountId: manualPreferredAccountQuery.data || "",
    refreshAccount: (accountId: string) => {
      void refreshAccountMutation.mutateAsync(accountId).catch(async (error: unknown) => {
        if (!isRefreshTokenExpiredError(error)) {
          return;
        }
        try {
          await recoverAccountAfterRefreshFailure(accountId);
        } catch (recoveryError: unknown) {
          toast.error(`自动登录失败: ${getAppErrorMessage(recoveryError)}`);
          await invalidateAll();
        }
      });
    },
    refreshAllAccounts: () => {
      if (!accounts.some((account) => !isAccountRefreshBlocked(account.status))) {
        toast.info("当前没有可刷新的账号");
        return;
      }
      refreshAllMutation.mutate();
    },
    deleteAccount: (accountId: string) => deleteMutation.mutate(accountId),
    deleteManyAccounts: (accountIds: string[]) => deleteManyMutation.mutate(accountIds),
    deleteUnavailableFree: () => deleteUnavailableFreeMutation.mutate(),
    deleteBannedAccounts: () => deleteBannedMutation.mutate(),
    importByFile: () => importByFileMutation.mutate(),
    importByDirectory: () => importByDirectoryMutation.mutate(),
    exportAccounts: (accountIds?: string[]) => exportMutation.mutate(accountIds ?? []),
    setPreferredAccount: (accountId: string) => setManualPreferredMutation.mutate(accountId),
    clearPreferredAccount: () => clearManualPreferredMutation.mutate(),
    updateAccountSort: (accountId: string, sort: number) =>
      updateAccountSortMutation.mutateAsync({ accountId, sort }),
    toggleAccountStatus: (
      accountId: string,
      enabled: boolean,
      sourceStatus?: string | null
    ) => toggleAccountStatusMutation.mutate({ accountId, enabled, sourceStatus }),
    checkSubscription: (accountId: string, proxy?: string | null) =>
      checkSubscriptionMutation.mutate({ accountId, proxy }),
    checkSubscriptions: (accountIds: string[], proxy?: string | null) =>
      checkSubscriptionsMutation.mutate({ accountIds, proxy }),
    markSubscription: (accountId: string, planType: "free" | "plus" | "team") =>
      markSubscriptionMutation.mutate({ accountId, planType }),
    markManySubscriptions: (
      accountIds: string[],
      planType: "free" | "plus" | "team"
    ) => markManySubscriptionsMutation.mutate({ accountIds, planType }),
    uploadToTeamManager: (accountId: string) =>
      uploadToTeamManagerMutation.mutate(accountId),
    uploadManyToTeamManager: (accountIds: string[]) =>
      uploadManyToTeamManagerMutation.mutate(accountIds),
    bulkToggleAccountStatus: (
      accountIds: string[],
      enabled: boolean,
      scopeLabel?: string
    ) => bulkToggleAccountStatusMutation.mutate({ accountIds, enabled, scopeLabel }),
    updateManyTags: (accountIds: string[], tags: string[] | string | null) =>
      bulkUpdateTagsMutation.mutate({ accountIds, tags }),
    isRefreshingAccountId:
      recoverAccountAuthMutation.isPending &&
      recoverAccountAuthMutation.variables &&
      typeof recoverAccountAuthMutation.variables === "object" &&
      "accountId" in recoverAccountAuthMutation.variables
        ? String(
            (recoverAccountAuthMutation.variables as { accountId?: unknown }).accountId || ""
          )
        : refreshAccountMutation.isPending &&
            typeof refreshAccountMutation.variables === "string"
        ? refreshAccountMutation.variables
        : "",
    isRefreshingAllAccounts: refreshAllMutation.isPending,
    isExporting: exportMutation.isPending,
    isDeletingMany: deleteManyMutation.isPending,
    isDeletingBanned: deleteBannedMutation.isPending,
    isDeletingUnavailableFree: deleteUnavailableFreeMutation.isPending,
    isUpdatingPreferred:
      setManualPreferredMutation.isPending || clearManualPreferredMutation.isPending,
    isUpdatingSortAccountId:
      updateAccountSortMutation.isPending &&
      updateAccountSortMutation.variables &&
      typeof updateAccountSortMutation.variables === "object" &&
      "accountId" in updateAccountSortMutation.variables
        ? String(
            (updateAccountSortMutation.variables as { accountId?: unknown }).accountId || ""
          )
        : "",
    isUpdatingStatusAccountId:
      toggleAccountStatusMutation.isPending &&
      toggleAccountStatusMutation.variables &&
      typeof toggleAccountStatusMutation.variables === "object" &&
      "accountId" in toggleAccountStatusMutation.variables
        ? String(
            (toggleAccountStatusMutation.variables as { accountId?: unknown }).accountId || ""
          )
        : "",
    isBulkUpdatingStatus: bulkToggleAccountStatusMutation.isPending,
    isBulkUpdatingTags: bulkUpdateTagsMutation.isPending,
    isCheckingSubscriptionAccountId:
      checkSubscriptionMutation.isPending &&
      checkSubscriptionMutation.variables &&
      typeof checkSubscriptionMutation.variables === "object" &&
      "accountId" in checkSubscriptionMutation.variables
        ? String(
            (checkSubscriptionMutation.variables as { accountId?: unknown }).accountId || ""
          )
        : "",
    isCheckingSubscriptions: checkSubscriptionsMutation.isPending,
    isMarkingSubscriptionAccountId:
      markSubscriptionMutation.isPending &&
      markSubscriptionMutation.variables &&
      typeof markSubscriptionMutation.variables === "object" &&
      "accountId" in markSubscriptionMutation.variables
        ? String(
            (markSubscriptionMutation.variables as { accountId?: unknown }).accountId || ""
          )
        : "",
    isUploadingTeamManagerAccountId:
      uploadToTeamManagerMutation.isPending &&
      typeof uploadToTeamManagerMutation.variables === "string"
        ? uploadToTeamManagerMutation.variables
        : "",
    isUploadingManyToTeamManager: uploadManyToTeamManagerMutation.isPending,
    isMarkingManySubscriptions: markManySubscriptionsMutation.isPending,
  };
}
