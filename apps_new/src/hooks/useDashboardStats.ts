"use client";

import { useQuery } from "@tanstack/react-query";
import { accountClient } from "@/lib/api/account-client";
import { serviceClient } from "@/lib/api/service-client";

export function useDashboardStats() {
  const accountsQuery = useQuery({
    queryKey: ["accounts"],
    queryFn: () => accountClient.list(),
    retry: 1,
  });

  const aggregateQuery = useQuery({
    queryKey: ["usage-aggregate"],
    queryFn: () => accountClient.aggregateUsage(),
    retry: 1,
  });

  const todaySummaryQuery = useQuery({
    queryKey: ["today-summary"],
    queryFn: () => serviceClient.getTodaySummary(),
    retry: 1,
  });

  const accounts = Array.isArray(accountsQuery.data) ? accountsQuery.data : [];
  const totalAccounts = accounts.length;
  const availableAccounts = accounts.filter(a => a.is_available).length;
  const unavailableAccounts = totalAccounts - availableAccounts;

  return {
    stats: {
      total: totalAccounts,
      available: availableAccounts,
      unavailable: unavailableAccounts,
      todayTokens: todaySummaryQuery.data?.total_tokens || 0,
      cachedTokens: todaySummaryQuery.data?.cached_input_tokens || 0,
      reasoningTokens: todaySummaryQuery.data?.reasoning_output_tokens || 0,
      todayCost: todaySummaryQuery.data?.estimated_cost || 0,
      poolRemain: aggregateQuery.data || { primary: "--", secondary: "--" },
    },
    isLoading: accountsQuery.isLoading || aggregateQuery.isLoading || todaySummaryQuery.isLoading,
  };
}
