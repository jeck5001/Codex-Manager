"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { accountClient } from "@/lib/api/account-client";
import { appClient } from "@/lib/api/app-client";
import { getAppErrorMessage } from "@/lib/api/transport";

function formatPlanTypeLabel(planType: string | null | undefined): string {
  const normalized = String(planType || "").trim().toLowerCase();
  if (normalized === "team") return "Team";
  if (normalized === "plus") return "Plus";
  if (normalized === "free") return "Free";
  return normalized || "未知";
}

export function useAccountPayments() {
  const queryClient = useQueryClient();

  const generatePaymentLinkMutation = useMutation({
    mutationFn: (payload: Parameters<typeof accountClient.generatePaymentLink>[0]) =>
      accountClient.generatePaymentLink(payload),
    onError: (error: unknown) => {
      toast.error(`生成支付链接失败: ${getAppErrorMessage(error)}`);
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
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
    },
    onError: (error: unknown) => {
      toast.error(`订阅检测失败: ${getAppErrorMessage(error)}`);
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
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
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

  const uploadToTeamManagerMutation = useMutation({
    mutationFn: (accountId: string) => accountClient.uploadToTeamManager(accountId),
    onSuccess: async (result) => {
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(`${result.accountName || result.accountId} 已上传到 Team Manager`);
    },
    onError: (error: unknown) => {
      toast.error(`上传 Team Manager 失败: ${getAppErrorMessage(error)}`);
    },
  });

  const setOfficialPromoLinkMutation = useMutation({
    mutationFn: ({
      accountId,
      link,
    }: {
      accountId: string;
      link?: string | null;
    }) => accountClient.setOfficialPromoLink(accountId, link),
    onSuccess: async (result) => {
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
      if (result?.officialPromoLink) {
        toast.success(`${result.accountName || result.accountId} 的官方优惠链接已保存`);
      } else {
        toast.success(`${result.accountName || result.accountId} 的官方优惠链接已清空`);
      }
    },
    onError: (error: unknown) => {
      toast.error(`保存官方优惠链接失败: ${getAppErrorMessage(error)}`);
    },
  });

  const testTeamManagerMutation = useMutation({
    mutationFn: ({
      apiUrl,
      apiKey,
    }: {
      apiUrl?: string | null;
      apiKey?: string | null;
    }) => accountClient.testTeamManager(apiUrl, apiKey),
    onSuccess: (result) => {
      if (result?.success) {
        toast.success(result.message || "Team Manager 连接测试成功");
      } else {
        toast.error(result?.message || "Team Manager 连接测试失败");
      }
    },
    onError: (error: unknown) => {
      toast.error(`测试 Team Manager 失败: ${getAppErrorMessage(error)}`);
    },
  });

  return {
    generatePaymentLink: generatePaymentLinkMutation.mutateAsync,
    checkSubscription: checkSubscriptionMutation.mutateAsync,
    markSubscription: markSubscriptionMutation.mutateAsync,
    setOfficialPromoLink: setOfficialPromoLinkMutation.mutateAsync,
    uploadToTeamManager: uploadToTeamManagerMutation.mutateAsync,
    testTeamManager: testTeamManagerMutation.mutateAsync,
    openPaymentLink: async (url: string, incognito = false) => {
      if (incognito) {
        await appClient.openInBrowserIncognito(url);
      } else {
        await appClient.openInBrowser(url);
      }
    },
    isGeneratingPaymentLink: generatePaymentLinkMutation.isPending,
    isCheckingSubscription: checkSubscriptionMutation.isPending,
    isMarkingSubscription: markSubscriptionMutation.isPending,
    isSettingOfficialPromoLink: setOfficialPromoLinkMutation.isPending,
    isUploadingToTeamManager: uploadToTeamManagerMutation.isPending,
    isTestingTeamManager: testTeamManagerMutation.isPending,
    generatingAccountId:
      generatePaymentLinkMutation.isPending &&
      generatePaymentLinkMutation.variables?.accountId
        ? generatePaymentLinkMutation.variables.accountId
        : "",
    checkingAccountId:
      checkSubscriptionMutation.isPending &&
      checkSubscriptionMutation.variables?.accountId
        ? checkSubscriptionMutation.variables.accountId
        : "",
    formatPlanTypeLabel,
  };
}
