"use client";

import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  buildStartupSnapshotQueryKey,
  hasStartupSnapshotSignal,
  STARTUP_SNAPSHOT_REQUEST_LOG_LIMIT,
  STARTUP_SNAPSHOT_STALE_TIME,
  STARTUP_SNAPSHOT_WARMUP_INTERVAL_MS,
  STARTUP_SNAPSHOT_WARMUP_TIMEOUT_MS,
} from "@/lib/api/startup-snapshot";
import { pickCurrentAccountId } from "@/lib/api/startup-snapshot-state";
import { serviceClient } from "@/lib/api/service-client";
import { useAppStore } from "@/lib/store/useAppStore";
import { pickBestRecommendations } from "@/lib/utils/usage";

export function useDashboardStats() {
  const serviceStatus = useAppStore((state) => state.serviceStatus);
  const isServiceReady = serviceStatus.connected;
  const warmupStartedAtRef = useRef<number | null>(null);

  useEffect(() => {
    if (!isServiceReady) {
      warmupStartedAtRef.current = null;
      return;
    }
    warmupStartedAtRef.current = Date.now();
  }, [isServiceReady, serviceStatus.addr]);

  const snapshotQuery = useQuery({
    queryKey: buildStartupSnapshotQueryKey(
      serviceStatus.addr,
      STARTUP_SNAPSHOT_REQUEST_LOG_LIMIT
    ),
    queryFn: () =>
      serviceClient.getStartupSnapshot({
        requestLogLimit: STARTUP_SNAPSHOT_REQUEST_LOG_LIMIT,
      }),
    enabled: isServiceReady,
    retry: 1,
    staleTime: STARTUP_SNAPSHOT_STALE_TIME,
    refetchInterval: (query) => {
      if (!isServiceReady) return false;
      const startedAt = warmupStartedAtRef.current;
      if (startedAt == null) return false;
      if (Date.now() - startedAt >= STARTUP_SNAPSHOT_WARMUP_TIMEOUT_MS) {
        warmupStartedAtRef.current = null;
        return false;
      }

      const snapshot = query.state.data;
      if (!snapshot || snapshot.accounts.length === 0) {
        return false;
      }

      return hasStartupSnapshotSignal(snapshot)
        ? false
        : STARTUP_SNAPSHOT_WARMUP_INTERVAL_MS;
    },
    refetchIntervalInBackground: true,
  });
  const dashboardHealthQuery = useQuery({
    queryKey: ["dashboard-health", serviceStatus.addr],
    queryFn: () => serviceClient.getDashboardHealth(),
    enabled: isServiceReady,
    retry: 1,
    staleTime: 15_000,
    refetchInterval: isServiceReady ? 30_000 : false,
    refetchIntervalInBackground: true,
  });
  const dashboardTrendQuery = useQuery({
    queryKey: ["dashboard-trend", serviceStatus.addr],
    queryFn: () => serviceClient.getDashboardTrend(),
    enabled: isServiceReady,
    retry: 1,
    staleTime: 15_000,
    refetchInterval: isServiceReady ? 30_000 : false,
    refetchIntervalInBackground: true,
  });

  const data = snapshotQuery.data;
  const accounts = data?.accounts || [];
  const hasStartupSignal = hasStartupSnapshotSignal(data);
  const shouldWarmupPoll =
    isServiceReady &&
    accounts.length > 0 &&
    !hasStartupSignal &&
    snapshotQuery.isFetching;
  const totalAccounts = accounts.length;
  const availableAccounts = accounts.filter((item) => item.isAvailable).length;
  const unavailableAccounts = totalAccounts - availableAccounts;
  const currentAccountId = pickCurrentAccountId(
    accounts,
    data?.latestRequestAccountId ?? null,
    data?.manualRouteAccountIds ?? []
  );
  const currentAccount =
    accounts.find((item) => item.id === currentAccountId) ?? null;
  const recommendations = pickBestRecommendations(accounts);
  const failureReasonSummary = data?.failureReasonSummary || [];
  const governanceSummary = data?.governanceSummary || [];
  const operationAudits = data?.operationAudits || [];
  const recentFailureTotal = failureReasonSummary.reduce(
    (sum, item) => sum + item.count,
    0
  );
  const recentGovernanceTotal = governanceSummary.reduce(
    (sum, item) => sum + item.count,
    0
  );
  const healthyAccounts = accounts.filter((item) => item.healthTier === "healthy").length;
  const warningAccounts = accounts.filter((item) => item.healthTier === "warning").length;
  const riskyAccounts = accounts.filter((item) => item.healthTier === "risky").length;
  const isolatedAccounts = accounts.filter((item) => item.isIsolated).length;

  return {
    accounts,
    stats: {
      total: totalAccounts,
      available: availableAccounts,
      unavailable: unavailableAccounts,
      todayTokens: data?.requestLogTodaySummary.todayTokens || 0,
      cachedTokens: data?.requestLogTodaySummary.cachedInputTokens || 0,
      reasoningTokens: data?.requestLogTodaySummary.reasoningOutputTokens || 0,
      todayCost: data?.requestLogTodaySummary.estimatedCost || 0,
      poolRemain: {
        primary: data?.usageAggregateSummary.primaryRemainPercent ?? null,
        secondary: data?.usageAggregateSummary.secondaryRemainPercent ?? null,
        primaryKnownCount: data?.usageAggregateSummary.primaryKnownCount ?? 0,
        primaryBucketCount: data?.usageAggregateSummary.primaryBucketCount ?? 0,
        secondaryKnownCount: data?.usageAggregateSummary.secondaryKnownCount ?? 0,
        secondaryBucketCount: data?.usageAggregateSummary.secondaryBucketCount ?? 0,
      },
      usagePrediction: {
        quotaProtectionEnabled:
          data?.usagePredictionSummary.quotaProtectionEnabled ?? false,
        quotaProtectionThresholdPercent:
          data?.usagePredictionSummary.quotaProtectionThresholdPercent ?? 0,
        readyAccountCount: data?.usagePredictionSummary.readyAccountCount ?? 0,
        estimatedHoursToThreshold:
          data?.usagePredictionSummary.estimatedHoursToThreshold ?? null,
        estimatedHoursToPoolExhaustion:
          data?.usagePredictionSummary.estimatedHoursToPoolExhaustion ?? null,
        thresholdLimitedBy:
          data?.usagePredictionSummary.thresholdLimitedBy ?? null,
        poolLimitedBy: data?.usagePredictionSummary.poolLimitedBy ?? null,
      },
      healthy: healthyAccounts,
      warning: warningAccounts,
      risky: riskyAccounts,
      isolated: isolatedAccounts,
      recentFailureTotal,
      recentGovernanceTotal,
    },
    currentAccount,
    recommendations,
    failureReasonSummary,
    governanceSummary,
    operationAudits,
    dashboardHealth: dashboardHealthQuery.data ?? null,
    dashboardTrend: dashboardTrendQuery.data ?? null,
    requestLogCount: data?.recentRequestLogCount ?? 0,
    isLoading: !isServiceReady || snapshotQuery.isPending || shouldWarmupPoll,
    isDashboardLoading:
      isServiceReady &&
      (dashboardHealthQuery.isPending || dashboardTrendQuery.isPending),
    isSyncingSnapshot: shouldWarmupPoll,
    isServiceReady,
    isError: snapshotQuery.isError,
    error: snapshotQuery.error,
  };
}
