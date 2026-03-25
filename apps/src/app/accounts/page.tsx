"use client";

import { useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import {
  BadgeCheck,
  BarChart3,
  CreditCard,
  Download,
  PencilLine,
  ExternalLink,
  FileUp,
  FolderOpen,
  MoreVertical,
  Pin,
  Plus,
  Power,
  PowerOff,
  RefreshCw,
  Search,
  ShieldCheck,
  Trash2,
  X,
  type LucideIcon,
} from "lucide-react";
import { toast } from "sonner";
import { AddAccountModal } from "@/components/modals/add-account-modal";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import UsageModal from "@/components/modals/usage-modal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useAccounts } from "@/hooks/useAccounts";
import { cn } from "@/lib/utils";
import {
  formatTsFromSeconds,
  formatHealthTierLabel,
  healthTierToneClass,
  isPrimaryWindowOnlyUsage,
  isSecondaryWindowOnlyUsage,
} from "@/lib/utils/usage";
import { Account } from "@/types";

type StatusFilter =
  | "all"
  | "available"
  | "low_quota"
  | "cooldown"
  | "protected"
  | "deactivated"
  | "isolated"
  | "governed";

function normalizeStatusFilter(value: string | null | undefined): StatusFilter {
  switch (String(value || "").trim().toLowerCase()) {
    case "available":
      return "available";
    case "low_quota":
      return "low_quota";
    case "cooldown":
      return "cooldown";
    case "protected":
      return "protected";
    case "deactivated":
      return "deactivated";
    case "isolated":
      return "isolated";
    case "governed":
      return "governed";
    default:
      return "all";
  }
}

function normalizeGovernanceFilter(value: string | null | undefined): string {
  const nextValue = String(value || "").trim();
  if (!nextValue || nextValue === "all") {
    return "all";
  }
  return nextValue;
}

function normalizeStatusReasonFilter(value: string | null | undefined): string {
  const nextValue = String(value || "").trim();
  if (!nextValue || nextValue === "all") {
    return "all";
  }
  return nextValue;
}

function normalizeTagFilter(value: string | null | undefined): string {
  const nextValue = String(value || "").trim();
  if (!nextValue || nextValue === "all") {
    return "all";
  }
  return nextValue;
}

function normalizeCooldownReasonFilter(value: string | null | undefined): string {
  const nextValue = String(value || "").trim();
  if (!nextValue || nextValue === "all") {
    return "all";
  }
  return nextValue;
}

function formatGroupFilterLabel(value: string) {
  const nextValue = String(value || "").trim();
  if (!nextValue || nextValue === "all") {
    return "全部分组";
  }
  return nextValue;
}

interface QuotaProgressProps {
  label: string;
  remainPercent: number | null;
  resetsAt?: number | null;
  icon: LucideIcon;
  tone: "green" | "blue";
  emptyText?: string;
}

function QuotaProgress({
  label,
  remainPercent,
  resetsAt = null,
  icon: Icon,
  tone,
  emptyText = "--",
}: QuotaProgressProps) {
  const value = remainPercent ?? 0;
  const trackClassName = tone === "blue" ? "bg-blue-500/20" : "bg-green-500/20";
  const indicatorClassName = tone === "blue" ? "bg-blue-500" : "bg-green-500";

  return (
    <div className="flex min-w-[120px] flex-col gap-1">
      <div className="flex items-center justify-between text-[10px]">
        <div className="flex items-center gap-1 text-muted-foreground">
          <Icon className="h-3 w-3" />
          <span>{label}</span>
        </div>
        <span className="font-medium">
          {remainPercent == null ? emptyText : `${value}%`}
        </span>
      </div>
      <Progress
        value={value}
        trackClassName={trackClassName}
        indicatorClassName={indicatorClassName}
      />
      {remainPercent != null ? (
        <span className="text-[10px] text-muted-foreground">
          重置: {formatTsFromSeconds(resetsAt, "--")}
        </span>
      ) : null}
    </div>
  );
}

function getQuotaResetTs(
  account: Account,
  bucket: "primary" | "secondary",
): number | null {
  const usage = account.usage;
  if (!usage) {
    return null;
  }
  const secondaryOnly = isSecondaryWindowOnlyUsage(usage);
  const primaryOnly = isPrimaryWindowOnlyUsage(usage);

  if (bucket === "primary") {
    return secondaryOnly ? null : usage.resetsAt;
  }
  if (secondaryOnly) {
    return usage.resetsAt;
  }
  if (primaryOnly) {
    return null;
  }
  return usage.secondaryResetsAt;
}

function getAccountStatusAction(account: Account): {
  enable: boolean;
  label: string;
  icon: LucideIcon;
} {
  const normalizedStatus = String(account.status || "").trim().toLowerCase();
  if (normalizedStatus === "disabled") {
    return { enable: true, label: "启用账号", icon: Power };
  }
  if (normalizedStatus === "inactive" || normalizedStatus === "deactivated") {
    return { enable: true, label: "恢复账号", icon: Power };
  }
  return { enable: false, label: "禁用账号", icon: PowerOff };
}

function getSharedTags(accounts: Account[]): string[] {
  if (accounts.length === 0) {
    return [];
  }

  const [firstAccount, ...restAccounts] = accounts;
  const shared = new Set(
    firstAccount.tags
      .map((tag) => String(tag || "").trim())
      .filter(Boolean),
  );

  for (const account of restAccounts) {
    const currentTags = new Set(
      account.tags
        .map((tag) => String(tag || "").trim())
        .filter(Boolean),
    );
    for (const tag of Array.from(shared)) {
      if (!currentTags.has(tag)) {
        shared.delete(tag);
      }
    }
    if (shared.size === 0) {
      break;
    }
  }

  return Array.from(shared).sort((left, right) =>
    left.localeCompare(right, "zh-CN"),
  );
}

export default function AccountsPage() {
  const router = useRouter();
  const {
    accounts,
    groups,
    isLoading,
    refreshAccount,
    refreshAllAccounts,
    deleteAccount,
    deleteManyAccounts,
    deleteUnavailableFree,
    deleteBannedAccounts,
    importByFile,
    importByDirectory,
    exportAccounts,
    isRefreshingAccountId,
    isRefreshingAllAccounts,
    isExporting,
    isDeletingMany,
    isDeletingBanned,
    isDeletingUnavailableFree,
    manualPreferredAccountId,
    setPreferredAccount,
    clearPreferredAccount,
    isUpdatingPreferred,
    updateAccountSort,
    isUpdatingSortAccountId,
    toggleAccountStatus,
    isUpdatingStatusAccountId,
    bulkToggleAccountStatus,
    isBulkUpdatingStatus,
    updateManyTags,
    isBulkUpdatingTags,
    checkSubscription,
    checkSubscriptions,
    markSubscription,
    markManySubscriptions,
    uploadToTeamManager,
    uploadManyToTeamManager,
    isCheckingSubscriptionAccountId,
    isCheckingSubscriptions,
    isMarkingSubscriptionAccountId,
    isMarkingManySubscriptions,
    isUploadingTeamManagerAccountId,
    isUploadingManyToTeamManager,
  } = useAccounts();

  const [search, setSearch] = useState("");
  const [groupFilter, setGroupFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [governanceFilter, setGovernanceFilter] = useState<string>("all");
  const [statusReasonFilter, setStatusReasonFilter] = useState<string>("all");
  const [cooldownReasonFilter, setCooldownReasonFilter] = useState<string>("all");
  const [tagFilter, setTagFilter] = useState<string>("all");
  const [pageSize, setPageSize] = useState("20");
  const [page, setPage] = useState(1);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [addAccountModalOpen, setAddAccountModalOpen] = useState(false);
  const [usageModalOpen, setUsageModalOpen] = useState(false);
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [sortDraft, setSortDraft] = useState("");
  const [sortDialogState, setSortDialogState] = useState<{
    accountId: string;
    accountName: string;
    currentSort: number;
  } | null>(null);
  const [markSubscriptionDialogState, setMarkSubscriptionDialogState] = useState<
    | {
        kind: "single";
        accountId: string;
        accountName: string;
      }
    | {
        kind: "selected";
        accountIds: string[];
        count: number;
      }
    | null
  >(null);
  const [markSubscriptionPlanType, setMarkSubscriptionPlanType] = useState<
    "free" | "plus" | "team"
  >("plus");
  const [deleteDialogState, setDeleteDialogState] = useState<
    | { kind: "single"; account: Account }
    | { kind: "selected"; ids: string[]; count: number }
    | null
  >(null);
  const [tagDialogState, setTagDialogState] = useState<
    | { kind: "selected"; ids: string[]; count: number }
    | null
  >(null);
  const [tagDraft, setTagDraft] = useState("");

  const scopedAccounts = useMemo(() => {
    return accounts.filter((account) => {
      const matchSearch =
        !search ||
        account.name.toLowerCase().includes(search.toLowerCase()) ||
        account.id.toLowerCase().includes(search.toLowerCase());
      const matchGroup =
        groupFilter === "all" || (account.group || "默认") === groupFilter;
      return matchSearch && matchGroup;
    });
  }, [accounts, groupFilter, search]);

  const filteredAccounts = useMemo(() => {
    return scopedAccounts.filter((account) => {
      const matchStatus =
        statusFilter === "all" ||
        (statusFilter === "available" && account.isAvailable) ||
        (statusFilter === "low_quota" && account.isLowQuota) ||
        (statusFilter === "cooldown" && account.isInCooldown) ||
        (statusFilter === "protected" && account.isNewAccountProtected) ||
        (statusFilter === "deactivated" && account.isDeactivated) ||
        (statusFilter === "isolated" && account.isIsolated) ||
        (statusFilter === "governed" && Boolean(account.lastGovernanceReason));
      const matchGovernance =
        governanceFilter === "all" ||
        account.lastGovernanceReason === governanceFilter;
      const matchStatusReason =
        statusReasonFilter === "all" ||
        account.lastStatusReason === statusReasonFilter;
      const matchCooldownReason =
        cooldownReasonFilter === "all" ||
        (account.isInCooldown &&
          (account.cooldownReason || "临时冷却") === cooldownReasonFilter);
      const matchTag =
        tagFilter === "all" || account.tags.includes(tagFilter);
      return (
        matchStatus &&
        matchGovernance &&
        matchStatusReason &&
        matchCooldownReason &&
        matchTag
      );
    });
  }, [
    cooldownReasonFilter,
    governanceFilter,
    scopedAccounts,
    statusFilter,
    statusReasonFilter,
    tagFilter,
  ]);

  const pageSizeNumber = Number(pageSize) || 20;
  const totalPages = Math.max(
    1,
    Math.ceil(filteredAccounts.length / pageSizeNumber),
  );
  const safePage = Math.min(page, totalPages);
  const accountIdSet = useMemo(
    () => new Set(accounts.map((account) => account.id)),
    [accounts],
  );
  const effectiveSelectedIds = useMemo(
    () => selectedIds.filter((id) => accountIdSet.has(id)),
    [accountIdSet, selectedIds],
  );
  const selectedAccounts = useMemo(
    () => accounts.filter((account) => effectiveSelectedIds.includes(account.id)),
    [accounts, effectiveSelectedIds],
  );
  const filteredLowQuotaAccounts = useMemo(
    () => filteredAccounts.filter((account) => account.isLowQuota),
    [filteredAccounts],
  );
  const scopedGovernedAccounts = useMemo(
    () => scopedAccounts.filter((account) => Boolean(account.lastGovernanceReason)),
    [scopedAccounts],
  );
  const governanceOptions = useMemo(() => {
    const counts = new Map<string, number>();
    for (const account of scopedGovernedAccounts) {
      const label = String(account.lastGovernanceReason || "").trim();
      if (!label) continue;
      counts.set(label, (counts.get(label) || 0) + 1);
    }
    return Array.from(counts.entries())
      .map(([label, count]) => ({ label, count }))
      .sort((left, right) => {
        if (right.count !== left.count) {
          return right.count - left.count;
        }
        return left.label.localeCompare(right.label, "zh-CN");
      });
  }, [scopedGovernedAccounts]);
  const cooldownOptions = useMemo(() => {
    const counts = new Map<string, number>();
    for (const account of scopedAccounts) {
      if (!account.isInCooldown) continue;
      const label = String(account.cooldownReason || "临时冷却").trim() || "临时冷却";
      counts.set(label, (counts.get(label) || 0) + 1);
    }
    return Array.from(counts.entries())
      .map(([label, count]) => ({ label, count }))
      .sort((left, right) => {
        if (right.count !== left.count) {
          return right.count - left.count;
        }
        return left.label.localeCompare(right.label, "zh-CN");
      });
  }, [scopedAccounts]);
  const statusReasonOptions = useMemo(() => {
    const counts = new Map<string, number>();
    for (const account of scopedAccounts) {
      const label = String(account.lastStatusReason || "").trim();
      if (!label) continue;
      counts.set(label, (counts.get(label) || 0) + 1);
    }
    return Array.from(counts.entries())
      .map(([label, count]) => ({ label, count }))
      .sort((left, right) => {
        if (right.count !== left.count) {
          return right.count - left.count;
        }
        return left.label.localeCompare(right.label, "zh-CN");
      });
  }, [scopedAccounts]);
  const tagOptions = useMemo(() => {
    const counts = new Map<string, number>();
    for (const account of scopedAccounts) {
      for (const tag of account.tags) {
        const label = String(tag || "").trim();
        if (!label) continue;
        counts.set(label, (counts.get(label) || 0) + 1);
      }
    }
    return Array.from(counts.entries())
      .map(([label, count]) => ({ label, count }))
      .sort((left, right) => {
        if (right.count !== left.count) {
          return right.count - left.count;
        }
        return left.label.localeCompare(right.label, "zh-CN");
      });
  }, [scopedAccounts]);
  const selectedEnableIds = useMemo(
    () =>
      selectedAccounts
        .filter((account) => {
          const action = getAccountStatusAction(account);
          return action.enable;
        })
        .map((account) => account.id),
    [selectedAccounts],
  );
  const selectedDisableIds = useMemo(
    () =>
      selectedAccounts
        .filter((account) => !getAccountStatusAction(account).enable)
        .map((account) => account.id),
    [selectedAccounts],
  );
  const lowQuotaEnableIds = useMemo(
    () =>
      filteredLowQuotaAccounts
        .filter((account) => getAccountStatusAction(account).enable)
        .map((account) => account.id),
    [filteredLowQuotaAccounts],
  );
  const lowQuotaDisableIds = useMemo(
    () =>
      filteredLowQuotaAccounts
        .filter((account) => !getAccountStatusAction(account).enable)
        .map((account) => account.id),
    [filteredLowQuotaAccounts],
  );
  const governedEnableIds = useMemo(
    () =>
      scopedGovernedAccounts
        .filter((account) => getAccountStatusAction(account).enable)
        .map((account) => account.id),
    [scopedGovernedAccounts],
  );
  const scopedBannedAccounts = useMemo(
    () => scopedAccounts.filter((account) => account.isDeactivated),
    [scopedAccounts],
  );

  const visibleAccounts = useMemo(() => {
    const offset = (safePage - 1) * pageSizeNumber;
    return filteredAccounts.slice(offset, offset + pageSizeNumber);
  }, [filteredAccounts, pageSizeNumber, safePage]);
  const activeFilterItems = useMemo(() => {
    const items: Array<{ key: string; label: string }> = [];
    if (search.trim()) {
      items.push({ key: "search", label: `搜索 ${search.trim()}` });
    }
    if (groupFilter !== "all") {
      items.push({ key: "group", label: `分组 ${groupFilter}` });
    }
    if (statusFilter !== "all") {
      const statusLabelMap: Record<StatusFilter, string> = {
        all: "全部",
        available: "可用",
        low_quota: "低配额",
        cooldown: "冷却中",
        protected: "新号保护",
        deactivated: "已停用",
        isolated: "隔离中",
        governed: "最近治理",
      };
      items.push({
        key: "status",
        label: `状态 ${statusLabelMap[statusFilter]}`,
      });
    }
    if (governanceFilter !== "all") {
      items.push({ key: "governance", label: `治理 ${governanceFilter}` });
    }
    if (statusReasonFilter !== "all") {
      items.push({ key: "statusReason", label: `状态原因 ${statusReasonFilter}` });
    }
    if (cooldownReasonFilter !== "all") {
      items.push({ key: "cooldownReason", label: `冷却原因 ${cooldownReasonFilter}` });
    }
    if (tagFilter !== "all") {
      items.push({ key: "tag", label: `标签 ${tagFilter}` });
    }
    return items;
  }, [
    cooldownReasonFilter,
    governanceFilter,
    groupFilter,
    search,
    statusFilter,
    statusReasonFilter,
    tagFilter,
  ]);

  const selectedAccount = useMemo(
    () => accounts.find((account) => account.id === selectedAccountId) ?? null,
    [accounts, selectedAccountId],
  );

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const params = new URLSearchParams(window.location.search);
    queueMicrotask(() => {
      setSearch(String(params.get("query") || "").trim());
      setGroupFilter(String(params.get("group") || "all").trim() || "all");
      setStatusFilter(normalizeStatusFilter(params.get("status")));
      setGovernanceFilter(
        normalizeGovernanceFilter(params.get("governanceReason")),
      );
      setStatusReasonFilter(
        normalizeStatusReasonFilter(params.get("statusReason")),
      );
      setCooldownReasonFilter(
        normalizeCooldownReasonFilter(params.get("cooldownReason")),
      );
      setTagFilter(normalizeTagFilter(params.get("tag")));
    });
  }, []);

  const handleSearchChange = (value: string) => {
    setSearch(value);
    setPage(1);
  };

  const handleGroupFilterChange = (value: string | null) => {
    setGroupFilter(value || "all");
    setPage(1);
  };

  const handleStatusFilterChange = (value: StatusFilter) => {
    setStatusFilter(value);
    if (value !== "governed") {
      setGovernanceFilter("all");
    }
    if (value !== "cooldown") {
      setCooldownReasonFilter("all");
    }
    setPage(1);
  };

  const handleGovernanceFilterChange = (value: string | null) => {
    const nextValue = normalizeGovernanceFilter(value);
    setGovernanceFilter(nextValue);
    if (nextValue !== "all") {
      setStatusFilter("governed");
    }
    setPage(1);
  };

  const handleStatusReasonFilterChange = (value: string | null) => {
    setStatusReasonFilter(normalizeStatusReasonFilter(value));
    setPage(1);
  };

  const handleCooldownReasonFilterChange = (value: string | null) => {
    const nextValue = normalizeCooldownReasonFilter(value);
    setCooldownReasonFilter(nextValue);
    if (nextValue !== "all") {
      setStatusFilter("cooldown");
    }
    setPage(1);
  };

  const handleTagFilterChange = (value: string | null) => {
    setTagFilter(normalizeTagFilter(value));
    setPage(1);
  };

  const handlePageSizeChange = (value: string | null) => {
    setPageSize(value || "20");
    setPage(1);
  };

  const handleClearFilters = () => {
    setSearch("");
    setGroupFilter("all");
    setStatusFilter("all");
    setGovernanceFilter("all");
    setStatusReasonFilter("all");
    setCooldownReasonFilter("all");
    setTagFilter("all");
    setPage(1);
    router.push("/accounts");
  };

  const handleRemoveFilterItem = (key: string) => {
    let nextSearch = search;
    let nextGroupFilter = groupFilter;
    let nextStatusFilter = statusFilter;
    let nextGovernanceFilter = governanceFilter;
    let nextStatusReasonFilter = statusReasonFilter;
    let nextCooldownReasonFilter = cooldownReasonFilter;
    let nextTagFilter = tagFilter;
    if (key === "search") {
      nextSearch = "";
      setSearch("");
    } else if (key === "group") {
      nextGroupFilter = "all";
      setGroupFilter("all");
    } else if (key === "status") {
      nextStatusFilter = "all";
      nextGovernanceFilter = "all";
      nextCooldownReasonFilter = "all";
      setStatusFilter("all");
      setGovernanceFilter("all");
      setCooldownReasonFilter("all");
    } else if (key === "governance") {
      nextGovernanceFilter = "all";
      setGovernanceFilter("all");
    } else if (key === "statusReason") {
      nextStatusReasonFilter = "all";
      setStatusReasonFilter("all");
    } else if (key === "cooldownReason") {
      nextCooldownReasonFilter = "all";
      setCooldownReasonFilter("all");
    } else if (key === "tag") {
      nextTagFilter = "all";
      setTagFilter("all");
    }
    setPage(1);
    const params = new URLSearchParams();
    if (nextStatusFilter !== "all") {
      params.set("status", nextStatusFilter);
    }
    if (nextGovernanceFilter !== "all") {
      params.set("governanceReason", nextGovernanceFilter);
    }
    if (nextStatusReasonFilter !== "all") {
      params.set("statusReason", nextStatusReasonFilter);
    }
    if (nextCooldownReasonFilter !== "all") {
      params.set("cooldownReason", nextCooldownReasonFilter);
    }
    if (nextTagFilter !== "all") {
      params.set("tag", nextTagFilter);
    }
    if (nextSearch.trim()) {
      params.set("query", nextSearch.trim());
    }
    if (nextGroupFilter !== "all") {
      params.set("group", nextGroupFilter);
    }
    router.push(
      params.size > 0 ? `/accounts?${params.toString()}` : "/accounts",
    );
  };

  const toggleSelect = (id: string) => {
    setSelectedIds((current) =>
      current.includes(id)
        ? current.filter((item) => item !== id)
        : [...current, id],
    );
  };

  const toggleSelectAllVisible = () => {
    const visibleIds = visibleAccounts.map((account) => account.id);
    const allSelected = visibleIds.every((id) =>
      effectiveSelectedIds.includes(id),
    );
    setSelectedIds((current) => {
      if (allSelected) {
        return current.filter((id) => !visibleIds.includes(id));
      }
      return Array.from(new Set([...current, ...visibleIds]));
    });
  };

  const openUsage = (account: Account) => {
    setSelectedAccountId(account.id);
    setUsageModalOpen(true);
  };

  const handleDeleteSelected = () => {
    if (!effectiveSelectedIds.length) {
      toast.error("请先选择要删除的账号");
      return;
    }
    setDeleteDialogState({
      kind: "selected",
      ids: [...effectiveSelectedIds],
      count: effectiveSelectedIds.length,
    });
  };

  const handleDeleteSingle = (account: Account) => {
    setDeleteDialogState({ kind: "single", account });
  };

  const handleBulkToggleSelected = (enabled: boolean) => {
    const targetIds = enabled ? selectedEnableIds : selectedDisableIds;
    if (!effectiveSelectedIds.length) {
      toast.error("请先选择要操作的账号");
      return;
    }
    if (!targetIds.length) {
      toast.info(enabled ? "选中账号里没有可启用项" : "选中账号里没有可禁用项");
      return;
    }
    bulkToggleAccountStatus(targetIds, enabled, "选中账号");
  };

  const handleBulkToggleLowQuota = (enabled: boolean) => {
    const targetIds = enabled ? lowQuotaEnableIds : lowQuotaDisableIds;
    if (!filteredLowQuotaAccounts.length) {
      toast.info("当前筛选范围内没有低配额账号");
      return;
    }
    if (!targetIds.length) {
      toast.info(enabled ? "低配额账号里没有可启用项" : "低配额账号里没有可禁用项");
      return;
    }
    bulkToggleAccountStatus(targetIds, enabled, "低配额账号");
  };

  const handleBulkRestoreGoverned = () => {
    if (!scopedGovernedAccounts.length) {
      toast.info("当前搜索或分组范围内没有最近自动治理账号");
      return;
    }
    if (!governedEnableIds.length) {
      toast.info("最近自动治理账号里没有可恢复项");
      return;
    }
    bulkToggleAccountStatus(governedEnableIds, true, "最近自动治理账号");
  };

  const handleBatchCheckSubscription = () => {
    if (!effectiveSelectedIds.length) {
      toast.error("请先选择要检测的账号");
      return;
    }
    checkSubscriptions(effectiveSelectedIds);
  };

  const handleBatchUploadTeamManager = () => {
    if (!effectiveSelectedIds.length) {
      toast.error("请先选择要上传的账号");
      return;
    }
    uploadManyToTeamManager(effectiveSelectedIds);
  };

  const openSingleMarkSubscriptionDialog = (account: Account) => {
    setMarkSubscriptionDialogState({
      kind: "single",
      accountId: account.id,
      accountName: account.name,
    });
    const currentPlanType = String(account.subscriptionPlanType || "").trim().toLowerCase();
    if (currentPlanType === "free" || currentPlanType === "plus" || currentPlanType === "team") {
      setMarkSubscriptionPlanType(currentPlanType as "free" | "plus" | "team");
      return;
    }
    setMarkSubscriptionPlanType("plus");
  };

  const openBatchMarkSubscriptionDialog = () => {
    if (!effectiveSelectedIds.length) {
      toast.error("请先选择要标记的账号");
      return;
    }
    setMarkSubscriptionDialogState({
      kind: "selected",
      accountIds: [...effectiveSelectedIds],
      count: effectiveSelectedIds.length,
    });
    setMarkSubscriptionPlanType("plus");
  };

  const handleConfirmMarkSubscription = () => {
    if (!markSubscriptionDialogState) return;
    if (markSubscriptionDialogState.kind === "single") {
      markSubscription(
        markSubscriptionDialogState.accountId,
        markSubscriptionPlanType
      );
      setMarkSubscriptionDialogState(null);
      return;
    }
    markManySubscriptions(
      markSubscriptionDialogState.accountIds,
      markSubscriptionPlanType
    );
    setMarkSubscriptionDialogState(null);
  };

  const openSortEditor = (account: Account) => {
    setSortDialogState({
      accountId: account.id,
      accountName: account.name,
      currentSort: account.priority,
    });
    setSortDraft(String(account.priority));
  };

  const handleConfirmSort = async () => {
    if (!sortDialogState) return;

    const raw = sortDraft.trim();
    if (!raw) {
      toast.error("请输入顺序值");
      return;
    }

    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      toast.error("顺序必须是数字");
      return;
    }

    const nextSort = Math.max(0, Math.trunc(parsed));
    if (nextSort === sortDialogState.currentSort) {
      setSortDialogState(null);
      return;
    }

    try {
      await updateAccountSort(sortDialogState.accountId, nextSort);
      setSortDialogState(null);
    } catch {
      // mutation 已统一处理 toast，这里保持弹窗不关闭
    }
  };

  const handleConfirmDelete = () => {
    if (!deleteDialogState) return;
    if (deleteDialogState.kind === "single") {
      deleteAccount(deleteDialogState.account.id);
      return;
    }
    deleteManyAccounts(deleteDialogState.ids);
    setSelectedIds((current) =>
      current.filter((id) => !deleteDialogState.ids.includes(id)),
    );
  };

  const openBatchTagDialog = () => {
    if (!effectiveSelectedIds.length) {
      toast.error("请先选择要打标签的账号");
      return;
    }
    setTagDialogState({
      kind: "selected",
      ids: [...effectiveSelectedIds],
      count: effectiveSelectedIds.length,
    });
    const commonTags = getSharedTags(selectedAccounts);
    setTagDraft(commonTags.join(", "));
  };

  const handleConfirmTags = () => {
    if (!tagDialogState) return;
    const tags = tagDraft
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
    updateManyTags(tagDialogState.ids, tags);
    setTagDialogState(null);
  };

  return (
    <div className="space-y-6">
      <Card className="glass-card border-none shadow-md backdrop-blur-md">
        <CardContent className="grid gap-3 pt-0 lg:grid-cols-[200px_auto_minmax(0,1fr)_auto] lg:items-center">
          <div className="min-w-0">
            <Input
              placeholder="搜索账号名 / 编号..."
              className="glass-card h-10 rounded-xl px-3"
              value={search}
              onChange={(event) => handleSearchChange(event.target.value)}
            />
          </div>

          <div className="flex shrink-0 items-center gap-3">
            <Select value={groupFilter} onValueChange={handleGroupFilterChange}>
              <SelectTrigger className="h-10 w-[140px] shrink-0 rounded-xl bg-card/50">
                <SelectValue placeholder="全部分组">
                  {(value) => formatGroupFilterLabel(String(value || ""))}
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">
                  全部分组 ({accounts.length})
                </SelectItem>
                {groups.map((group) => (
                  <SelectItem key={group.label} value={group.label}>
                    {group.label} ({group.count})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <div className="flex shrink-0 items-center gap-1 rounded-xl border border-border/60 bg-muted/30 p-1">
              {[
                { id: "all", label: "全部" },
                { id: "available", label: "可用" },
                { id: "low_quota", label: "低配额" },
                { id: "cooldown", label: "冷却中" },
                { id: "protected", label: "新号保护" },
                { id: "deactivated", label: "已停用" },
                { id: "isolated", label: "隔离中" },
                { id: "governed", label: "最近治理" },
              ].map((filter) => (
                <button
                  key={filter.id}
                  onClick={() =>
                    handleStatusFilterChange(filter.id as StatusFilter)
                  }
                  className={cn(
                    "rounded-lg px-3 py-1.5 text-xs font-semibold transition-all",
                    statusFilter === filter.id
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-background/60 hover:text-foreground",
                  )}
                >
                  {filter.label}
                </button>
              ))}
            </div>
            {governanceOptions.length > 0 ? (
              <Select
                value={governanceFilter}
                onValueChange={handleGovernanceFilterChange}
              >
                <SelectTrigger className="h-10 w-[200px] shrink-0 rounded-xl bg-card/50">
                  <SelectValue placeholder="治理原因">
                    {(value) => {
                      const nextValue = String(value || "").trim();
                      if (!nextValue || nextValue === "all") {
                        return `全部治理原因 (${scopedGovernedAccounts.length})`;
                      }
                      return nextValue;
                    }}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">
                    全部治理原因 ({scopedGovernedAccounts.length})
                  </SelectItem>
                  {governanceOptions.map((option) => (
                    <SelectItem key={option.label} value={option.label}>
                      {option.label} ({option.count})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : null}
            {cooldownOptions.length > 0 ? (
              <Select
                value={cooldownReasonFilter}
                onValueChange={handleCooldownReasonFilterChange}
              >
                <SelectTrigger className="h-10 w-[220px] shrink-0 rounded-xl bg-card/50">
                  <SelectValue placeholder="冷却原因">
                    {(value) => {
                      const nextValue = String(value || "").trim();
                      if (!nextValue || nextValue === "all") {
                        return `全部冷却原因 (${cooldownOptions.length})`;
                      }
                      return nextValue;
                    }}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">
                    全部冷却原因 ({cooldownOptions.length})
                  </SelectItem>
                  {cooldownOptions.map((option) => (
                    <SelectItem key={option.label} value={option.label}>
                      {option.label} ({option.count})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : null}
            {statusReasonOptions.length > 0 ? (
              <Select
                value={statusReasonFilter}
                onValueChange={handleStatusReasonFilterChange}
              >
                <SelectTrigger className="h-10 w-[200px] shrink-0 rounded-xl bg-card/50">
                  <SelectValue placeholder="状态原因">
                    {(value) => {
                      const nextValue = String(value || "").trim();
                      if (!nextValue || nextValue === "all") {
                        return `全部状态原因 (${statusReasonOptions.length})`;
                      }
                      return nextValue;
                    }}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">
                    全部状态原因 ({statusReasonOptions.length})
                  </SelectItem>
                  {statusReasonOptions.map((option) => (
                    <SelectItem key={option.label} value={option.label}>
                      {option.label} ({option.count})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : null}
            {tagOptions.length > 0 ? (
              <Select
                value={tagFilter}
                onValueChange={handleTagFilterChange}
              >
                <SelectTrigger className="h-10 w-[180px] shrink-0 rounded-xl bg-card/50">
                  <SelectValue placeholder="标签">
                    {(value) => {
                      const nextValue = String(value || "").trim();
                      if (!nextValue || nextValue === "all") {
                        return `全部标签 (${tagOptions.length})`;
                      }
                      return nextValue;
                    }}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">
                    全部标签 ({tagOptions.length})
                  </SelectItem>
                  {tagOptions.map((option) => (
                    <SelectItem key={option.label} value={option.label}>
                      {option.label} ({option.count})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : null}
          </div>

          <div className="hidden min-w-0 lg:block" />

          <div className="ml-auto flex shrink-0 items-center gap-2 lg:ml-0 lg:justify-self-end">
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Button
                  variant="outline"
                  className="glass-card h-10 min-w-[50px] justify-between gap-2 rounded-xl px-3.5"
                  render={<span />}
                  nativeButton={false}
                >
                  <span className="flex items-center gap-2">
                    <span className="text-sm font-medium">账号操作</span>
                    {effectiveSelectedIds.length > 0 ? (
                      <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-semibold text-primary">
                        {effectiveSelectedIds.length}
                      </span>
                    ) : null}
                  </span>
                  <MoreVertical className="h-4 w-4 text-muted-foreground" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent
                align="end"
                className="w-64 rounded-xl border border-border/70 bg-popover/95 p-2 shadow-xl backdrop-blur-md"
              >
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="px-2 py-1 text-[11px] uppercase tracking-[0.16em] text-muted-foreground/80">
                    账号管理
                  </DropdownMenuLabel>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    onClick={() => setAddAccountModalOpen(true)}
                  >
                    <Plus className="mr-2 h-4 w-4" /> 添加账号
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    onClick={() => importByFile()}
                  >
                    <FileUp className="mr-2 h-4 w-4" /> 按文件导入
                    <DropdownMenuShortcut>FILE</DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    onClick={() => importByDirectory()}
                  >
                    <FolderOpen className="mr-2 h-4 w-4" /> 按文件夹导入
                    <DropdownMenuShortcut>DIR</DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={isExporting}
                    onClick={() => exportAccounts(effectiveSelectedIds)}
                  >
                    <Download className="mr-2 h-4 w-4" />
                    导出账号
                    <DropdownMenuShortcut>
                      {isExporting ? "..." : "ZIP"}
                    </DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!effectiveSelectedIds.length || isBulkUpdatingTags}
                    onClick={openBatchTagDialog}
                  >
                    <Pin className="mr-2 h-4 w-4" /> 批量设置标签
                    <DropdownMenuShortcut>{effectiveSelectedIds.length || "-"}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="px-2 py-1 text-[11px] uppercase tracking-[0.16em] text-muted-foreground/80">
                    状态
                  </DropdownMenuLabel>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!selectedEnableIds.length || isBulkUpdatingStatus}
                    onClick={() => handleBulkToggleSelected(true)}
                  >
                    <Power className="mr-2 h-4 w-4" /> 启用选中账号
                    <DropdownMenuShortcut>{selectedEnableIds.length || "-"}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!selectedDisableIds.length || isBulkUpdatingStatus}
                    onClick={() => handleBulkToggleSelected(false)}
                  >
                    <PowerOff className="mr-2 h-4 w-4" /> 禁用选中账号
                    <DropdownMenuShortcut>{selectedDisableIds.length || "-"}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!lowQuotaEnableIds.length || isBulkUpdatingStatus}
                    onClick={() => handleBulkToggleLowQuota(true)}
                  >
                    <Power className="mr-2 h-4 w-4" /> 启用低配额账号
                    <DropdownMenuShortcut>{lowQuotaEnableIds.length || "-"}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!lowQuotaDisableIds.length || isBulkUpdatingStatus}
                    onClick={() => handleBulkToggleLowQuota(false)}
                  >
                    <PowerOff className="mr-2 h-4 w-4" /> 禁用低配额账号
                    <DropdownMenuShortcut>{lowQuotaDisableIds.length || "-"}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!governedEnableIds.length || isBulkUpdatingStatus}
                    onClick={handleBulkRestoreGoverned}
                  >
                    <Power className="mr-2 h-4 w-4" /> 恢复最近治理账号
                    <DropdownMenuShortcut>{governedEnableIds.length || "-"}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!effectiveSelectedIds.length || isCheckingSubscriptions}
                    onClick={handleBatchCheckSubscription}
                  >
                    <ShieldCheck className="mr-2 h-4 w-4" /> 检测选中订阅
                    <DropdownMenuShortcut>
                      {effectiveSelectedIds.length || "-"}
                    </DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!effectiveSelectedIds.length || isMarkingManySubscriptions}
                    onClick={openBatchMarkSubscriptionDialog}
                  >
                    <BadgeCheck className="mr-2 h-4 w-4" /> 标记选中订阅
                    <DropdownMenuShortcut>
                      {effectiveSelectedIds.length || "-"}
                    </DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    disabled={!effectiveSelectedIds.length || isUploadingManyToTeamManager}
                    onClick={handleBatchUploadTeamManager}
                  >
                    <FileUp className="mr-2 h-4 w-4" /> 上传选中到 TM
                    <DropdownMenuShortcut>
                      {effectiveSelectedIds.length || "-"}
                    </DropdownMenuShortcut>
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="px-2 py-1 text-[11px] uppercase tracking-[0.16em] text-muted-foreground/80">
                    支付
                  </DropdownMenuLabel>
                  <DropdownMenuItem
                    className="h-9 rounded-lg px-2"
                    onClick={() => router.push("/payment")}
                  >
                    <CreditCard className="mr-2 h-4 w-4" /> 打开支付中心
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="px-2 py-1 text-[11px] uppercase tracking-[0.16em] text-muted-foreground/80">
                    清理
                  </DropdownMenuLabel>
                  <DropdownMenuItem
                    disabled={!effectiveSelectedIds.length || isDeletingMany}
                    variant="destructive"
                    className="h-9 rounded-lg px-2"
                    onClick={handleDeleteSelected}
                  >
                    <Trash2 className="mr-2 h-4 w-4" /> 删除选中账号
                    <DropdownMenuShortcut>
                      {effectiveSelectedIds.length || "-"}
                    </DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    disabled={!scopedBannedAccounts.length || isDeletingBanned}
                    variant="destructive"
                    className="h-9 rounded-lg px-2"
                    onClick={() => deleteBannedAccounts()}
                  >
                    <Trash2 className="mr-2 h-4 w-4" /> 一键清理封禁账号
                    <DropdownMenuShortcut>
                      {isDeletingBanned ? "..." : scopedBannedAccounts.length || "-"}
                    </DropdownMenuShortcut>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    variant="destructive"
                    className="h-9 rounded-lg px-2"
                    disabled={isDeletingUnavailableFree}
                    onClick={() => deleteUnavailableFree()}
                  >
                    <Trash2 className="mr-2 h-4 w-4" /> 一键清理不可用免费
                    {isDeletingUnavailableFree ? (
                      <DropdownMenuShortcut>...</DropdownMenuShortcut>
                    ) : null}
                  </DropdownMenuItem>
                </DropdownMenuGroup>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button
              className="h-10 gap-2 rounded-xl shadow-lg shadow-primary/20"
              onClick={() => refreshAllAccounts()}
              disabled={isRefreshingAllAccounts}
            >
              <RefreshCw
                className={cn(
                  "h-4 w-4",
                  isRefreshingAllAccounts && "animate-spin",
                )}
              />
              刷新账号用量
            </Button>
          </div>
        </CardContent>
      </Card>

      {activeFilterItems.length > 0 ? (
        <div className="flex flex-wrap items-center gap-2 px-1">
          <span className="text-xs font-medium text-muted-foreground">
            当前筛选:
          </span>
          {activeFilterItems.map((item) => (
            <button
              type="button"
              key={item.key}
              className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-2.5 py-1 text-[11px] text-primary transition-colors hover:bg-primary/15"
              onClick={() => handleRemoveFilterItem(item.key)}
              title={`移除筛选：${item.label}`}
            >
              <span>{item.label}</span>
              <X className="h-3 w-3" />
            </button>
          ))}
          <Button
            variant="ghost"
            size="sm"
            className="h-7 rounded-full px-3 text-xs"
            onClick={handleClearFilters}
          >
            清空筛选
          </Button>
        </div>
      ) : null}

      <Card className="glass-card overflow-hidden border-none py-0 shadow-xl backdrop-blur-md">
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-12 text-center">
                  <Checkbox
                    checked={
                      visibleAccounts.length > 0 &&
                      visibleAccounts.every((account) =>
                        effectiveSelectedIds.includes(account.id),
                      )
                    }
                    onCheckedChange={toggleSelectAllVisible}
                  />
                </TableHead>
                <TableHead className="max-w-[220px]">账号信息</TableHead>
                <TableHead>5h 额度</TableHead>
                <TableHead>7d 额度</TableHead>
                <TableHead className="w-20">顺序</TableHead>
                <TableHead>状态</TableHead>
                <TableHead className="text-center">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                Array.from({ length: 5 }).map((_, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      <Skeleton className="mx-auto h-4 w-4" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-4 w-32" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-4 w-24" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-4 w-24" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-4 w-10" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-6 w-16 rounded-full" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="mx-auto h-8 w-24" />
                    </TableCell>
                  </TableRow>
                ))
              ) : visibleAccounts.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="h-48 text-center">
                    <div className="flex flex-col items-center justify-center gap-2 text-muted-foreground">
                      <Search className="h-8 w-8 opacity-20" />
                      <p>未找到符合条件的账号</p>
                    </div>
                  </TableCell>
                </TableRow>
              ) : (
                visibleAccounts.map((account) => {
                  const primaryWindowOnly = isPrimaryWindowOnlyUsage(
                    account.usage,
                  );
                  const secondaryWindowOnly = isSecondaryWindowOnlyUsage(
                    account.usage,
                  );
                  const statusAction = getAccountStatusAction(account);
                  const StatusActionIcon = statusAction.icon;
                  return (
                    <TableRow key={account.id} className="group">
                      <TableCell className="text-center">
                        <Checkbox
                          checked={effectiveSelectedIds.includes(account.id)}
                          onCheckedChange={() => toggleSelect(account.id)}
                        />
                      </TableCell>
                      <TableCell className="max-w-[220px]">
                        <div className="flex flex-col overflow-hidden">
                          <div className="flex items-center gap-2 overflow-hidden">
                            <span className="truncate text-sm font-semibold">
                              {account.name}
                            </span>
                            <Badge
                              variant="secondary"
                              className="h-4 shrink-0 bg-accent/50 px-1.5 text-[9px]"
                            >
                              {account.group || "默认"}
                            </Badge>
                            {manualPreferredAccountId === account.id ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-amber-500/15 px-1.5 text-[9px] text-amber-700 dark:text-amber-300"
                              >
                                优先
                              </Badge>
                            ) : null}
                            {account.subscriptionPlanType ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-sky-500/15 px-1.5 text-[9px] text-sky-700 dark:text-sky-300"
                              >
                                {String(account.subscriptionPlanType).toUpperCase()}
                              </Badge>
                            ) : null}
                            {account.tags.slice(0, 2).map((tag) => (
                              <Badge
                                key={`${account.id}-${tag}`}
                                variant="secondary"
                                className="h-4 shrink-0 bg-violet-500/15 px-1.5 text-[9px] text-violet-700 dark:text-violet-300"
                              >
                                #{tag}
                              </Badge>
                            ))}
                            {account.tags.length > 2 ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-muted/50 px-1.5 text-[9px]"
                              >
                                +{account.tags.length - 2}
                              </Badge>
                            ) : null}
                            {account.isInCooldown ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-amber-500/15 px-1.5 text-[9px] text-amber-700 dark:text-amber-300"
                              >
                                冷却中
                              </Badge>
                            ) : null}
                            {account.isNewAccountProtected ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-cyan-500/15 px-1.5 text-[9px] text-cyan-700 dark:text-cyan-300"
                              >
                                新号保护
                              </Badge>
                            ) : null}
                            <Badge
                              variant="secondary"
                              className={cn(
                                "h-4 shrink-0 px-1.5 text-[9px]",
                                healthTierToneClass(account.healthTier)
                              )}
                            >
                              {formatHealthTierLabel(account.healthTier)} {account.healthScore}
                            </Badge>
                            {account.isIsolated && account.lastIsolationReason ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-rose-500/20 px-1.5 text-[9px] text-rose-700 dark:text-rose-300"
                              >
                                隔离 {account.lastIsolationReason}
                              </Badge>
                            ) : account.lastGovernanceReason ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-rose-500/15 px-1.5 text-[9px] text-rose-700 dark:text-rose-300"
                              >
                                治理 {account.lastGovernanceReason}
                              </Badge>
                            ) : null}
                            {account.teamManagerUploadedAt ? (
                              <Badge
                                variant="secondary"
                                className="h-4 shrink-0 bg-emerald-500/15 px-1.5 text-[9px] text-emerald-700 dark:text-emerald-300"
                              >
                                TM
                              </Badge>
                            ) : null}
                          </div>
                          <span className="truncate font-mono text-[10px] uppercase text-muted-foreground opacity-60">
                            {account.id.slice(0, 16)}...
                          </span>
                          <span className="mt-1 text-[10px] text-muted-foreground">
                            最近刷新:{" "}
                            {formatTsFromSeconds(
                              account.lastRefreshAt,
                              "从未刷新",
                            )}
                          </span>
                          {account.lastStatusReason ? (
                            <span className="text-[10px] text-muted-foreground">
                              最近状态: {account.lastStatusReason}
                              {account.lastStatusChangedAt
                                ? ` · ${formatTsFromSeconds(account.lastStatusChangedAt, "--")}`
                                : ""}
                            </span>
                          ) : null}
                          {account.lastIsolationReason ? (
                            <span className="text-[10px] text-rose-600 dark:text-rose-400">
                              隔离原因: {account.lastIsolationReason}
                              {account.lastIsolationAt
                                ? ` · ${formatTsFromSeconds(account.lastIsolationAt, "--")}`
                                : ""}
                            </span>
                          ) : null}
                          {account.isInCooldown ? (
                            <span className="text-[10px] text-amber-600 dark:text-amber-400">
                              冷却中: {account.cooldownReason || "临时冷却"}
                              {account.cooldownUntil
                                ? ` · 至 ${formatTsFromSeconds(account.cooldownUntil, "--")}`
                                : ""}
                            </span>
                          ) : null}
                          {account.isNewAccountProtected ? (
                            <span className="text-[10px] text-cyan-700 dark:text-cyan-300">
                              {account.newAccountProtectionReason || "新号保护期内，已自动降优先级"}
                              {account.newAccountProtectionUntil
                                ? ` · 至 ${formatTsFromSeconds(account.newAccountProtectionUntil, "--")}`
                                : ""}
                            </span>
                          ) : null}
                          {account.subscriptionUpdatedAt ? (
                            <span className="text-[10px] text-muted-foreground">
                              订阅标记: {formatTsFromSeconds(account.subscriptionUpdatedAt, "--")}
                            </span>
                          ) : null}
                          {account.tags.length > 0 ? (
                            <span className="text-[10px] text-muted-foreground">
                              标签: {account.tags.join(", ")}
                            </span>
                          ) : null}
                        </div>
                      </TableCell>
                      <TableCell>
                        <QuotaProgress
                          label="5小时"
                          remainPercent={account.primaryRemainPercent}
                          resetsAt={getQuotaResetTs(account, "primary")}
                          icon={RefreshCw}
                          tone="green"
                          emptyText={secondaryWindowOnly ? "未提供" : "--"}
                        />
                      </TableCell>
                      <TableCell>
                        <QuotaProgress
                          label="7天"
                          remainPercent={account.secondaryRemainPercent}
                          resetsAt={getQuotaResetTs(account, "secondary")}
                          icon={RefreshCw}
                          tone="blue"
                          emptyText={primaryWindowOnly ? "未提供" : "--"}
                        />
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          <span className="rounded bg-muted/50 px-2 py-0.5 font-mono text-xs">
                            {account.priority}
                          </span>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7 text-muted-foreground transition-colors hover:text-primary"
                            disabled={isUpdatingSortAccountId === account.id}
                            onClick={() => openSortEditor(account)}
                            title="编辑顺序"
                          >
                            <PencilLine className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col gap-1.5">
                          <div className="flex items-center gap-1.5">
                            <Badge
                              variant="secondary"
                              className={cn(
                                "w-fit px-1.5 text-[9px]",
                                healthTierToneClass(account.healthTier)
                              )}
                            >
                              健康 {account.healthScore}
                            </Badge>
                          </div>
                          <div
                            className="flex items-center gap-1.5"
                          >
                            <div
                              className={cn(
                                "h-1.5 w-1.5 rounded-full",
                                account.isAvailable
                                  ? "bg-green-500"
                                  : "bg-red-500",
                              )}
                            />
                            <span
                              className={cn(
                                "text-[11px] font-medium",
                                account.isAvailable
                                  ? "text-green-600 dark:text-green-400"
                                  : "text-red-600 dark:text-red-400",
                              )}
                            >
                              {account.availabilityText}
                            </span>
                          </div>
                          {account.isIsolated && account.lastIsolationReason ? (
                            <div className="rounded-md bg-rose-500/15 px-2 py-1 text-[10px] text-rose-700 dark:text-rose-300">
                              当前隔离: {account.lastIsolationReason}
                              {account.lastIsolationAt
                                ? ` · ${formatTsFromSeconds(account.lastIsolationAt, "--")}`
                                : ""}
                            </div>
                          ) : account.isInCooldown ? (
                            <div className="rounded-md bg-amber-500/10 px-2 py-1 text-[10px] text-amber-700 dark:text-amber-300">
                              冷却中: {account.cooldownReason || "临时冷却"}
                              {account.cooldownUntil
                                ? ` · 至 ${formatTsFromSeconds(account.cooldownUntil, "--")}`
                                : ""}
                            </div>
                          ) : account.isNewAccountProtected ? (
                            <div className="rounded-md bg-cyan-500/10 px-2 py-1 text-[10px] text-cyan-700 dark:text-cyan-300">
                              {account.newAccountProtectionReason || "新号保护期内，已自动降优先级"}
                              {account.newAccountProtectionUntil
                                ? ` · 至 ${formatTsFromSeconds(account.newAccountProtectionUntil, "--")}`
                                : ""}
                            </div>
                          ) : account.lastGovernanceReason ? (
                            <div className="rounded-md bg-rose-500/10 px-2 py-1 text-[10px] text-rose-700 dark:text-rose-300">
                              自动治理: {account.lastGovernanceReason}
                              {account.lastGovernanceAt
                                ? ` · ${formatTsFromSeconds(account.lastGovernanceAt, "--")}`
                                : ""}
                            </div>
                          ) : account.lastStatusReason ? (
                            <div className="rounded-md bg-muted/40 px-2 py-1 text-[10px] text-muted-foreground">
                              最近状态: {account.lastStatusReason}
                            </div>
                          ) : null}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="table-action-cell gap-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-muted-foreground transition-colors hover:text-primary"
                            onClick={() => openUsage(account)}
                            title="用量详情"
                          >
                            <BarChart3 className="h-4 w-4" />
                          </Button>
                          <DropdownMenu>
                            <DropdownMenuTrigger>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-8 w-8"
                                render={<span />}
                                nativeButton={false}
                              >
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuItem
                                className="gap-2"
                                disabled={isUpdatingPreferred}
                                onClick={() =>
                                  manualPreferredAccountId === account.id
                                    ? clearPreferredAccount()
                                    : setPreferredAccount(account.id)
                                }
                              >
                                <Pin className="h-4 w-4" />
                                {manualPreferredAccountId === account.id
                                  ? "取消优先"
                                  : "设为优先"}
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                className="gap-2"
                                disabled={
                                  isUpdatingStatusAccountId === account.id
                                }
                                onClick={() =>
                                  toggleAccountStatus(
                                    account.id,
                                    statusAction.enable,
                                    account.status,
                                  )
                                }
                              >
                                <StatusActionIcon className="h-4 w-4" />
                                {statusAction.label}
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                className="gap-2"
                                onClick={() =>
                                  router.push(
                                    `/logs?query=${encodeURIComponent(account.id)}`,
                                  )
                                }
                              >
                                <ExternalLink className="h-4 w-4" /> 详情与日志
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                className="gap-2"
                                disabled={isCheckingSubscriptionAccountId === account.id}
                                onClick={() => checkSubscription(account.id)}
                              >
                                <ShieldCheck className="h-4 w-4" />
                                检测订阅
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                className="gap-2"
                                disabled={isMarkingSubscriptionAccountId === account.id}
                                onClick={() => openSingleMarkSubscriptionDialog(account)}
                              >
                                <BadgeCheck className="h-4 w-4" />
                                标记订阅
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                className="gap-2"
                                disabled={isUploadingTeamManagerAccountId === account.id}
                                onClick={() => uploadToTeamManager(account.id)}
                              >
                                <FileUp className="h-4 w-4" />
                                上传到 Team Manager
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                className="gap-2"
                                onClick={() =>
                                  router.push(
                                    `/payment?accountId=${encodeURIComponent(account.id)}`,
                                  )
                                }
                              >
                                <CreditCard className="h-4 w-4" />
                                去支付中心
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                className="gap-2 text-red-500"
                                onClick={() => handleDeleteSingle(account)}
                              >
                                <Trash2 className="h-4 w-4" /> 删除
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <div className="flex items-center justify-between px-2">
        <div className="text-xs text-muted-foreground">
          共 {filteredAccounts.length} 个账号
          {effectiveSelectedIds.length > 0 ? (
            <span className="ml-1 text-primary">
              (已选择 {effectiveSelectedIds.length} 个)
            </span>
          ) : null}
        </div>
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <span className="whitespace-nowrap text-xs text-muted-foreground">
              每页显示
            </span>
            <Select value={pageSize} onValueChange={handlePageSizeChange}>
              <SelectTrigger className="h-8 w-[70px] text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {["5", "10", "20", "50", "100", "500"].map((value) => (
                  <SelectItem key={value} value={value}>
                    {value}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              className="h-8 px-3 text-xs"
              disabled={safePage <= 1}
              onClick={() => setPage((current) => Math.max(1, current - 1))}
            >
              上一页
            </Button>
            <div className="min-w-[60px] text-center text-xs font-medium">
              第 {safePage} / {totalPages} 页
            </div>
            <Button
              variant="outline"
              size="sm"
              className="h-8 px-3 text-xs"
              disabled={safePage >= totalPages}
              onClick={() =>
                setPage((current) => Math.min(totalPages, current + 1))
              }
            >
              下一页
            </Button>
          </div>
        </div>
      </div>

      {addAccountModalOpen ? (
        <AddAccountModal
          open={addAccountModalOpen}
          onOpenChange={setAddAccountModalOpen}
        />
      ) : null}
      <UsageModal
        account={selectedAccount}
        open={usageModalOpen}
        onOpenChange={(open) => {
          setUsageModalOpen(open);
          if (!open) {
            setSelectedAccountId("");
          }
        }}
        onRefresh={refreshAccount}
        isRefreshing={
          isRefreshingAllAccounts ||
          (!!selectedAccount && isRefreshingAccountId === selectedAccount.id)
        }
      />
      <ConfirmDialog
        open={Boolean(deleteDialogState)}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteDialogState(null);
          }
        }}
        title={
          deleteDialogState?.kind === "single" ? "删除账号" : "批量删除账号"
        }
        description={
          deleteDialogState?.kind === "single"
            ? `确定删除账号 ${deleteDialogState.account.name} 吗？删除后不可恢复。`
            : `确定删除选中的 ${deleteDialogState?.count || 0} 个账号吗？删除后不可恢复。`
        }
        confirmText="删除"
        confirmVariant="destructive"
        onConfirm={handleConfirmDelete}
      />
      <Dialog
        open={Boolean(tagDialogState)}
        onOpenChange={(open) => {
          if (!open) {
            setTagDialogState(null);
          }
        }}
      >
        <DialogContent className="glass-card border-none sm:max-w-md">
          <DialogHeader>
            <DialogTitle>批量设置标签</DialogTitle>
            <DialogDescription>
              {tagDialogState
                ? `为选中的 ${tagDialogState.count} 个账号设置标签。多个标签使用英文逗号分隔，留空表示清空标签。`
                : "为选中账号设置统一标签。"}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-2 py-2">
            <Label htmlFor="batch-account-tags-input">标签</Label>
            <Input
              id="batch-account-tags-input"
              value={tagDraft}
              disabled={isBulkUpdatingTags}
              onChange={(event) => setTagDraft(event.target.value)}
              placeholder="free, imported, high-quality"
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  handleConfirmTags();
                }
              }}
            />
            <p className="text-[11px] text-muted-foreground">
              建议使用短标签，如 `free`、`team`、`risk`、`imported`。
            </p>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              disabled={isBulkUpdatingTags}
              onClick={() => setTagDialogState(null)}
            >
              取消
            </Button>
            <Button
              disabled={isBulkUpdatingTags}
              onClick={handleConfirmTags}
            >
              {isBulkUpdatingTags ? "保存中..." : "保存标签"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog
        open={Boolean(markSubscriptionDialogState)}
        onOpenChange={(open) => {
          if (!open) {
            setMarkSubscriptionDialogState(null);
          }
        }}
      >
        <DialogContent className="glass-card border-none sm:max-w-md">
          <DialogHeader>
            <DialogTitle>标记订阅</DialogTitle>
            <DialogDescription>
              {markSubscriptionDialogState?.kind === "single"
                ? `为账号 ${markSubscriptionDialogState.accountName} 设置订阅类型。`
                : `为选中的 ${markSubscriptionDialogState?.count || 0} 个账号统一设置订阅类型。`}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 py-2">
            <Label htmlFor="mark-subscription-plan-type">订阅类型</Label>
            <Select
              value={markSubscriptionPlanType}
              onValueChange={(value) =>
                setMarkSubscriptionPlanType((value || "plus") as "free" | "plus" | "team")
              }
            >
              <SelectTrigger id="mark-subscription-plan-type" className="h-11 rounded-xl">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="free">Free</SelectItem>
                <SelectItem value="plus">Plus</SelectItem>
                <SelectItem value="team">Team</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setMarkSubscriptionDialogState(null)}
            >
              取消
            </Button>
            <Button
              onClick={handleConfirmMarkSubscription}
              disabled={isMarkingManySubscriptions}
            >
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog
        open={Boolean(sortDialogState)}
        onOpenChange={(open) => {
          if (!open && !isUpdatingSortAccountId) {
            setSortDialogState(null);
          }
        }}
      >
        <DialogContent className="glass-card border-none sm:max-w-[420px]">
          <DialogHeader>
            <DialogTitle>编辑账号顺序</DialogTitle>
            <DialogDescription>
              {sortDialogState
                ? `修改 ${sortDialogState.accountName} 的排序值。值越小越靠前。`
                : "修改账号的排序值。"}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-2 py-2">
            <Label htmlFor="account-sort-input">顺序值</Label>
            <Input
              id="account-sort-input"
              type="number"
              min={0}
              step={1}
              value={sortDraft}
              disabled={Boolean(isUpdatingSortAccountId)}
              onChange={(event) => setSortDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  void handleConfirmSort();
                }
              }}
            />
            <p className="text-[11px] text-muted-foreground">
              仅修改当前账号的排序值，不会自动重排其它账号。
            </p>
          </div>
          <DialogFooter className="gap-2 sm:gap-2">
            <Button
              variant="outline"
              disabled={Boolean(isUpdatingSortAccountId)}
              onClick={() => setSortDialogState(null)}
            >
              取消
            </Button>
            <Button
              disabled={Boolean(isUpdatingSortAccountId)}
              onClick={() => void handleConfirmSort()}
            >
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
