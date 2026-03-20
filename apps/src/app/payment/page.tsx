"use client";

import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import {
  BadgeCheck,
  Copy,
  CreditCard,
  ExternalLink,
  Globe2,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { useAccounts } from "@/hooks/useAccounts";
import { useAccountPayments } from "@/hooks/useAccountPayments";
import { appClient } from "@/lib/api/app-client";

export default function PaymentPage() {
  const router = useRouter();
  const { accounts, isLoading } = useAccounts();
  const {
    generatePaymentLink,
    checkSubscription,
    markSubscription,
    uploadToTeamManager,
    openPaymentLink,
    isGeneratingPaymentLink,
    isCheckingSubscription,
    isMarkingSubscription,
    isUploadingToTeamManager,
    formatPlanTypeLabel,
  } = useAccountPayments();
  const { data: settingsSnapshot } = useQuery({
    queryKey: ["payment-settings"],
    queryFn: () => appClient.getSettings(),
  });

  const [accountSearch, setAccountSearch] = useState("");
  const deferredAccountSearch = useDeferredValue(accountSearch);
  const [initialAccountId, setInitialAccountId] = useState("");
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [planType, setPlanType] = useState<"plus" | "team">("plus");
  const [workspaceName, setWorkspaceName] = useState("MyTeam");
  const [priceInterval, setPriceInterval] = useState<"month" | "year">("month");
  const [seatQuantity, setSeatQuantity] = useState("5");
  const [country, setCountry] = useState("SG");
  const [proxy, setProxy] = useState("");
  const [generatedLink, setGeneratedLink] = useState("");
  const [generatedAccountName, setGeneratedAccountName] = useState("");
  const [detectedPlanType, setDetectedPlanType] = useState("");
  const [detectedRawPlanType, setDetectedRawPlanType] = useState("");
  const [manualPlanType, setManualPlanType] = useState<"free" | "plus" | "team">("plus");

  const filteredAccounts = useMemo(() => {
    const keyword = deferredAccountSearch.trim().toLowerCase();
    if (!keyword) return accounts;
    return accounts.filter((account) => {
      return (
        account.name.toLowerCase().includes(keyword) ||
        account.id.toLowerCase().includes(keyword) ||
        (account.group || "").toLowerCase().includes(keyword)
      );
    });
  }, [accounts, deferredAccountSearch]);

  const selectedAccount = useMemo(
    () => accounts.find((account) => account.id === selectedAccountId) ?? null,
    [accounts, selectedAccountId],
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    setInitialAccountId(params.get("accountId") || "");
  }, []);

  useEffect(() => {
    if (initialAccountId && accounts.some((account) => account.id === initialAccountId)) {
      setSelectedAccountId(initialAccountId);
      return;
    }
    if (!selectedAccountId && accounts.length > 0) {
      setSelectedAccountId(accounts[0].id);
    }
  }, [accounts, initialAccountId, selectedAccountId]);

  useEffect(() => {
    if (!detectedPlanType) return;
    const normalized = String(detectedPlanType).trim().toLowerCase();
    if (normalized === "free" || normalized === "plus" || normalized === "team") {
      setManualPlanType(normalized as "free" | "plus" | "team");
    }
  }, [detectedPlanType]);

  const handleCheckSubscription = async () => {
    if (!selectedAccountId) {
      toast.error("请先选择账号");
      return;
    }
    const result = await checkSubscription({
      accountId: selectedAccountId,
      proxy: proxy.trim() || null,
    });
    setDetectedPlanType(String(result.planType || ""));
    setDetectedRawPlanType(String(result.rawPlanType || ""));
    toast.success(
      `${result.accountName || selectedAccount?.name || selectedAccountId} 当前订阅：${formatPlanTypeLabel(
        result.planType
      )}`
    );
  };

  const handleGenerateLink = async () => {
    if (!selectedAccountId) {
      toast.error("请先选择账号");
      return;
    }
    const seatNumber = Number(seatQuantity);
    const result = await generatePaymentLink({
      accountId: selectedAccountId,
      planType,
      workspaceName: workspaceName.trim() || "MyTeam",
      priceInterval,
      seatQuantity: Number.isFinite(seatNumber) ? seatNumber : 5,
      country: country.trim().toUpperCase() || "SG",
      proxy: proxy.trim() || null,
    });
    setGeneratedLink(result.link);
    setGeneratedAccountName(result.accountName);
    toast.success(`${result.accountName} 的 ${formatPlanTypeLabel(result.planType)} 支付链接已生成`);
  };

  const handleMarkSubscription = async () => {
    if (!selectedAccountId) {
      toast.error("请先选择账号");
      return;
    }
    const result = await markSubscription({
      accountId: selectedAccountId,
      planType: manualPlanType,
    });
    setDetectedPlanType(String(result.planType || ""));
  };

  const handleUploadTeamManager = async () => {
    if (!selectedAccountId) {
      toast.error("请先选择账号");
      return;
    }
    await uploadToTeamManager(selectedAccountId);
  };

  const copyGeneratedLink = async () => {
    if (!generatedLink) {
      toast.error("当前还没有支付链接");
      return;
    }
    await navigator.clipboard.writeText(generatedLink);
    toast.success("支付链接已复制");
  };

  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(360px,0.95fr)]">
      <Card className="glass-card border-none shadow-xl">
        <CardHeader className="space-y-3">
          <div className="flex items-center gap-3">
            <div className="flex h-11 w-11 items-center justify-center rounded-2xl bg-primary/12 text-primary">
              <CreditCard className="h-5 w-5" />
            </div>
            <div>
              <CardTitle className="text-xl">支付中心</CardTitle>
              <p className="text-sm text-muted-foreground">
                在主项目里直接生成 Plus / Team 支付链接，并检测账号当前订阅状态。
              </p>
            </div>
          </div>
        </CardHeader>
        <CardContent className="grid gap-5">
          <div className="grid gap-2">
            <Label htmlFor="payment-account-search">账号筛选</Label>
            <Input
              id="payment-account-search"
              placeholder="搜索账号名 / 账号 ID / 分组"
              value={accountSearch}
              onChange={(event) => setAccountSearch(event.target.value)}
            />
          </div>

          <div className="grid gap-2">
            <Label>选择账号</Label>
            {isLoading ? (
              <Skeleton className="h-10 w-full rounded-xl" />
            ) : (
              <Select
                value={selectedAccountId}
                onValueChange={(value) => setSelectedAccountId(value || "")}
              >
                <SelectTrigger className="h-11 rounded-xl">
                  <SelectValue placeholder="选择一个账号" />
                </SelectTrigger>
                <SelectContent>
                  {filteredAccounts.map((account) => (
                    <SelectItem key={account.id} value={account.id}>
                      {account.name} · {account.group || "默认"}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="grid gap-2">
              <Label>套餐类型</Label>
              <Select
                value={planType}
                onValueChange={(value) =>
                  setPlanType((value || "plus") as "plus" | "team")
                }
              >
                <SelectTrigger className="h-11 rounded-xl">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="plus">ChatGPT Plus</SelectItem>
                  <SelectItem value="team">ChatGPT Team</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="grid gap-2">
              <Label>计费国家</Label>
              <Input
                placeholder="SG / US / JP"
                value={country}
                onChange={(event) => setCountry(event.target.value.toUpperCase())}
              />
            </div>
          </div>

          {planType === "team" ? (
            <div className="grid gap-4 md:grid-cols-3">
              <div className="grid gap-2 md:col-span-2">
                <Label>工作区名称</Label>
                <Input
                  placeholder="MyTeam"
                  value={workspaceName}
                  onChange={(event) => setWorkspaceName(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label>席位数</Label>
                <Input
                  type="number"
                  min={1}
                  value={seatQuantity}
                  onChange={(event) => setSeatQuantity(event.target.value)}
                />
              </div>

              <div className="grid gap-2 md:col-span-3">
                <Label>计费周期</Label>
                <Select
                  value={priceInterval}
                  onValueChange={(value) =>
                    setPriceInterval((value || "month") as "month" | "year")
                  }
                >
                  <SelectTrigger className="h-11 rounded-xl">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="month">按月</SelectItem>
                    <SelectItem value="year">按年</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          ) : null}

          <div className="grid gap-2">
            <Label>代理（可选）</Label>
            <Input
              placeholder="http://127.0.0.1:7890"
              value={proxy}
              onChange={(event) => setProxy(event.target.value)}
            />
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <Button
              className="h-11 gap-2 rounded-xl px-5"
              disabled={!selectedAccountId || isGeneratingPaymentLink}
              onClick={() => void handleGenerateLink()}
            >
              <Sparkles className="h-4 w-4" />
              {isGeneratingPaymentLink ? "生成中..." : "生成支付链接"}
            </Button>
            <Button
              variant="outline"
              className="h-11 gap-2 rounded-xl px-5"
              disabled={!selectedAccountId || isCheckingSubscription}
              onClick={() => void handleCheckSubscription()}
            >
              <ShieldCheck className="h-4 w-4" />
              {isCheckingSubscription ? "检测中..." : "检测订阅状态"}
            </Button>
          </div>

          <div className="grid gap-4 rounded-2xl border border-border/60 bg-card/45 p-4 md:grid-cols-[minmax(0,1fr)_auto_auto]">
            <div className="grid gap-2">
              <Label>手动订阅标记</Label>
              <Select
                value={manualPlanType}
                onValueChange={(value) =>
                  setManualPlanType((value || "plus") as "free" | "plus" | "team")
                }
              >
                <SelectTrigger className="h-11 rounded-xl">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="free">Free</SelectItem>
                  <SelectItem value="plus">Plus</SelectItem>
                  <SelectItem value="team">Team</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <Button
              variant="outline"
              className="h-11 gap-2 rounded-xl px-5 self-end"
              disabled={!selectedAccountId || isMarkingSubscription}
              onClick={() => void handleMarkSubscription()}
            >
              <BadgeCheck className="h-4 w-4" />
              {isMarkingSubscription ? "保存中..." : "保存订阅标记"}
            </Button>
            <Button
              variant="outline"
              className="h-11 gap-2 rounded-xl px-5 self-end"
              disabled={!selectedAccountId || isUploadingToTeamManager}
              onClick={() => void handleUploadTeamManager()}
            >
              <ExternalLink className="h-4 w-4" />
              {isUploadingToTeamManager ? "上传中..." : "上传 Team Manager"}
            </Button>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-6">
        <Card className="glass-card border-none shadow-lg">
          <CardHeader className="space-y-3">
            <CardTitle className="flex items-center gap-2 text-base">
              <BadgeCheck className="h-4 w-4 text-primary" />
              账号概览
            </CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 text-sm">
            <div className="rounded-2xl border border-border/60 bg-card/50 p-4">
              <div className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                当前账号
              </div>
              <div className="mt-2 text-base font-semibold">
                {selectedAccount?.name || "未选择"}
              </div>
              <div className="mt-1 break-all font-mono text-[11px] text-muted-foreground">
                {selectedAccount?.id || "--"}
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-1">
              <div className="rounded-2xl border border-border/60 bg-card/50 p-4">
                <div className="flex items-center gap-2 text-xs uppercase tracking-[0.16em] text-muted-foreground">
                  <ShieldCheck className="h-3.5 w-3.5" />
                  订阅状态
                </div>
                <div className="mt-3 text-lg font-semibold">
                  {detectedPlanType ? formatPlanTypeLabel(detectedPlanType) : "待检测"}
                </div>
              <div className="mt-1 text-xs text-muted-foreground">
                  原始 plan_type: {detectedRawPlanType || "--"}
              </div>
            </div>

              <div className="rounded-2xl border border-border/60 bg-card/50 p-4">
                <div className="flex items-center gap-2 text-xs uppercase tracking-[0.16em] text-muted-foreground">
                  <Globe2 className="h-3.5 w-3.5" />
                  当前套餐表单
                </div>
                <div className="mt-3 text-lg font-semibold">
                  {planType === "team" ? "ChatGPT Team" : "ChatGPT Plus"}
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  国家 {country || "--"}{planType === "team" ? ` · ${workspaceName || "MyTeam"}` : ""}
                </div>
              </div>
            </div>

            <div className="rounded-2xl border border-border/60 bg-card/50 p-4">
              <div className="flex items-center gap-2 text-xs uppercase tracking-[0.16em] text-muted-foreground">
                <ExternalLink className="h-3.5 w-3.5" />
                Team Manager
              </div>
              <div className="mt-3 text-lg font-semibold">
                {settingsSnapshot?.teamManagerEnabled ? "已启用" : "未启用"}
              </div>
              <div className="mt-1 text-xs text-muted-foreground">
                {settingsSnapshot?.teamManagerApiUrl || "未配置 API URL"}
              </div>
              <div className="mt-3 flex gap-2">
                <Button
                  variant="outline"
                  className="h-9 rounded-xl"
                  onClick={() => router.push("/settings")}
                >
                  前往设置
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="glass-card border-none shadow-lg">
          <CardHeader className="space-y-3">
            <CardTitle className="flex items-center gap-2 text-base">
              <ExternalLink className="h-4 w-4 text-primary" />
              支付链接
            </CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            <div className="rounded-2xl border border-dashed border-border/70 bg-card/40 p-4">
              <div className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                最近生成
              </div>
              <div className="mt-2 text-sm font-medium">
                {generatedAccountName || "尚未生成"}
              </div>
              <div className="mt-2 break-all font-mono text-[12px] text-muted-foreground">
                {generatedLink || "生成后会显示在这里"}
              </div>
            </div>

            <div className="flex flex-wrap gap-3">
              <Button
                variant="outline"
                className="h-10 gap-2 rounded-xl"
                disabled={!generatedLink}
                onClick={() => void copyGeneratedLink()}
              >
                <Copy className="h-4 w-4" />
                复制链接
              </Button>
              <Button
                variant="outline"
                className="h-10 gap-2 rounded-xl"
                disabled={!generatedLink}
                onClick={() => void openPaymentLink(generatedLink, false)}
              >
                <ExternalLink className="h-4 w-4" />
                普通打开
              </Button>
              <Button
                className="h-10 gap-2 rounded-xl"
                disabled={!generatedLink}
                onClick={() => void openPaymentLink(generatedLink, true)}
              >
                <ShieldCheck className="h-4 w-4" />
                无痕打开
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
