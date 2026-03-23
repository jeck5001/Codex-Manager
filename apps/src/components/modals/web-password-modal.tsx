"use client";

import { useEffect, useState } from "react";
import Image from "next/image";
import {
  Copy,
  KeyRound,
  LoaderCircle,
  QrCode,
  ShieldAlert,
  ShieldCheck,
  ShieldOff,
  Trash2,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { appClient } from "@/lib/api/app-client";
import { serviceClient } from "@/lib/api/service-client";
import { useAppStore } from "@/lib/store/useAppStore";
import type { AppSettings, WebAuthTwoFactorSetupResult } from "@/types";
import { toast } from "sonner";

interface WebPasswordModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function WebPasswordModal({ open, onOpenChange }: WebPasswordModalProps) {
  const { appSettings, setAppSettings } = useAppStore();
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [setupCode, setSetupCode] = useState("");
  const [disableCode, setDisableCode] = useState("");
  const [setupData, setSetupData] = useState<WebAuthTwoFactorSetupResult | null>(null);
  const [isSavingPassword, setIsSavingPassword] = useState(false);
  const [isManagingTwoFactor, setIsManagingTwoFactor] = useState(false);

  useEffect(() => {
    if (!open) {
      setPassword("");
      setConfirmPassword("");
      setSetupCode("");
      setDisableCode("");
      setSetupData(null);
      return;
    }

    let cancelled = false;
    const syncSettings = async () => {
      try {
        const settings = await appClient.getSettings();
        if (!cancelled) {
          setAppSettings(settings);
        }
      } catch (err: unknown) {
        if (!cancelled) {
          toast.error(
            `读取密码状态失败: ${err instanceof Error ? err.message : String(err)}`
          );
        }
      }
    };

    void syncSettings();

    return () => {
      cancelled = true;
    };
  }, [open, setAppSettings]);

  const refreshSettings = async (): Promise<AppSettings> => {
    const settings = await appClient.getSettings();
    setAppSettings(settings);
    return settings;
  };

  const copyText = async (value: string, successText: string) => {
    try {
      await navigator.clipboard.writeText(value);
      toast.success(successText);
    } catch (err: unknown) {
      toast.error(`复制失败: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const handleSave = async () => {
    if (!password) {
      toast.error("请输入密码");
      return;
    }
    if (password !== confirmPassword) {
      toast.error("两次输入的密码不一致");
      return;
    }

    setIsSavingPassword(true);
    try {
      const settings = await appClient.setSettings({ webAccessPassword: password });
      setAppSettings(settings);
      toast.success("访问密码已设置");
      setPassword("");
      setConfirmPassword("");
    } catch (err: unknown) {
      toast.error(`保存失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsSavingPassword(false);
    }
  };

  const handleClear = async () => {
    setIsSavingPassword(true);
    try {
      const settings = await appClient.setSettings({ webAccessPassword: "" });
      setAppSettings(settings);
      setSetupData(null);
      setSetupCode("");
      setDisableCode("");
      toast.success("访问密码与 2FA 已清除");
    } catch (err: unknown) {
      toast.error(`清除失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsSavingPassword(false);
    }
  };

  const handleStartTwoFactorSetup = async () => {
    setIsManagingTwoFactor(true);
    try {
      const result = await serviceClient.setupWebAuthTwoFactor();
      setSetupData(result);
      setSetupCode("");
      toast.success("已生成新的 2FA 绑定信息");
    } catch (err: unknown) {
      toast.error(`生成绑定信息失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsManagingTwoFactor(false);
    }
  };

  const handleVerifyTwoFactorSetup = async () => {
    if (!setupData) {
      toast.error("请先生成二维码");
      return;
    }
    if (!setupCode.trim()) {
      toast.error("请输入验证码");
      return;
    }

    setIsManagingTwoFactor(true);
    try {
      const result = await serviceClient.verifyWebAuthTwoFactor({
        setupToken: setupData.setupToken,
        code: setupCode.trim(),
      });
      await refreshSettings();
      setSetupCode("");
      toast.success(
        result.enabled
          ? "2FA 已启用，请立即保存恢复码"
          : "验证码已校验"
      );
    } catch (err: unknown) {
      toast.error(`启用失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsManagingTwoFactor(false);
    }
  };

  const handleDisableTwoFactor = async () => {
    if (!disableCode.trim()) {
      toast.error("请输入验证码或恢复码");
      return;
    }

    setIsManagingTwoFactor(true);
    try {
      await serviceClient.disableWebAuthTwoFactor({ code: disableCode.trim() });
      await refreshSettings();
      setDisableCode("");
      setSetupData(null);
      toast.success("2FA 已停用");
    } catch (err: unknown) {
      toast.error(`停用失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsManagingTwoFactor(false);
    }
  };

  const loading = isSavingPassword || isManagingTwoFactor;
  const recoveryCodesText = setupData?.recoveryCodes.join("\n") ?? "";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[640px]">
        <DialogHeader>
          <div className="mb-2 flex items-center gap-3">
            <div className="rounded-full bg-primary/10 p-2">
              <KeyRound className="h-5 w-5 text-primary" />
            </div>
            <DialogTitle>访问密码与 2FA</DialogTitle>
          </div>
          <DialogDescription>
            Web 管理页与桌面端共用同一份安全配置。建议先设置访问密码，再绑定 TOTP 二步验证保护登录主链路。
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          <div className="grid gap-3 md:grid-cols-2">
            <div className="flex items-center gap-3 rounded-lg border border-green-500/20 bg-green-500/10 p-3 text-sm text-green-600 dark:text-green-400">
              {appSettings.webAccessPasswordConfigured ? (
                <ShieldCheck className="h-4 w-4" />
              ) : (
                <ShieldAlert className="h-4 w-4" />
              )}
              <span>
                {appSettings.webAccessPasswordConfigured
                  ? "当前已启用访问密码保护"
                  : "当前未设置访问密码，Web 管理页处于公开状态"}
              </span>
            </div>
            <div
              className={`flex items-center gap-3 rounded-lg border p-3 text-sm ${
                appSettings.webAccessTwoFactorEnabled
                  ? "border-sky-500/20 bg-sky-500/10 text-sky-600 dark:text-sky-400"
                  : "border-zinc-500/20 bg-zinc-500/10 text-zinc-600 dark:text-zinc-300"
              }`}
            >
              {appSettings.webAccessTwoFactorEnabled ? (
                <ShieldCheck className="h-4 w-4" />
              ) : (
                <ShieldOff className="h-4 w-4" />
              )}
              <span>
                {appSettings.webAccessTwoFactorEnabled
                  ? `2FA 已启用，剩余 ${appSettings.webAccessRecoveryCodesRemaining} 个恢复码`
                  : "当前未启用 2FA，登录仍为单密码验证"}
              </span>
            </div>
          </div>

          <div className="grid gap-2">
            <Label htmlFor="password">新密码</Label>
            <Input
              id="password"
              type="password"
              placeholder="请输入新密码"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="confirm">确认新密码</Label>
            <Input
              id="confirm"
              type="password"
              placeholder="请再次输入新密码"
              value={confirmPassword}
              onChange={(event) => setConfirmPassword(event.target.value)}
            />
          </div>

          {appSettings.webAccessPasswordConfigured && (
            <div className="grid gap-4 rounded-xl border border-border/60 bg-muted/20 p-4">
              <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-sm font-medium">
                    <QrCode className="h-4 w-4" />
                    二步验证绑定
                  </div>
                  <p className="text-sm text-muted-foreground">
                    扫描二维码后输入 6 位验证码完成绑定。重新生成并确认后，会替换现有 secret 和恢复码。
                  </p>
                </div>
                <Button
                  variant="outline"
                  onClick={handleStartTwoFactorSetup}
                  disabled={loading}
                >
                  {isManagingTwoFactor ? (
                    <LoaderCircle className="mr-2 h-4 w-4 animate-spin" />
                  ) : null}
                  {setupData ? "重新生成二维码" : "开始绑定 2FA"}
                </Button>
              </div>

              {setupData ? (
                <div className="grid gap-4 md:grid-cols-[180px_minmax(0,1fr)]">
                  <div className="overflow-hidden rounded-xl border bg-white p-3">
                    <Image
                      src={setupData.qrCodeDataUrl}
                      alt="2FA QR Code"
                      width={160}
                      height={160}
                      unoptimized
                      className="h-full w-full rounded-lg object-contain"
                    />
                  </div>
                  <div className="grid gap-3">
                    <div className="grid gap-2">
                      <Label>手动输入 Secret</Label>
                      <div className="flex gap-2">
                        <Input value={setupData.secret} readOnly />
                        <Button
                          type="button"
                          variant="outline"
                          onClick={() =>
                            copyText(setupData.secret, "已复制 2FA Secret")
                          }
                        >
                          <Copy className="mr-2 h-4 w-4" />
                          复制
                        </Button>
                      </div>
                    </div>

                    <div className="grid gap-2">
                      <div className="flex items-center justify-between gap-3">
                        <Label>恢复码</Label>
                        <Button
                          type="button"
                          variant="outline"
                          onClick={() =>
                            copyText(recoveryCodesText, "已复制恢复码")
                          }
                        >
                          <Copy className="mr-2 h-4 w-4" />
                          复制全部
                        </Button>
                      </div>
                      <Textarea value={recoveryCodesText} readOnly rows={6} />
                      <p className="text-xs text-muted-foreground">
                        恢复码仅在这里完整展示一次，请在确认启用前妥善保存。
                      </p>
                    </div>

                    <div className="grid gap-2">
                      <div className="flex items-center justify-between gap-3">
                        <Label>OTPAuth URL</Label>
                        <Button
                          type="button"
                          variant="outline"
                          onClick={() =>
                            copyText(setupData.otpAuthUrl, "已复制 OTPAuth URL")
                          }
                        >
                          <Copy className="mr-2 h-4 w-4" />
                          复制链接
                        </Button>
                      </div>
                      <Input value={setupData.otpAuthUrl} readOnly />
                    </div>

                    <div className="grid gap-2">
                      <Label htmlFor="two-factor-code">验证码</Label>
                      <Input
                        id="two-factor-code"
                        inputMode="numeric"
                        placeholder="请输入 6 位验证码"
                        value={setupCode}
                        onChange={(event) => setSetupCode(event.target.value)}
                      />
                    </div>

                    <Button onClick={handleVerifyTwoFactorSetup} disabled={loading}>
                      {isManagingTwoFactor ? (
                        <LoaderCircle className="mr-2 h-4 w-4 animate-spin" />
                      ) : null}
                      确认启用 2FA
                    </Button>
                  </div>
                </div>
              ) : null}

              {appSettings.webAccessTwoFactorEnabled ? (
                <div className="grid gap-3 rounded-lg border border-amber-500/20 bg-amber-500/10 p-4">
                  <div className="space-y-1">
                    <div className="text-sm font-medium text-amber-700 dark:text-amber-300">
                      停用 2FA
                    </div>
                    <p className="text-sm text-amber-700/80 dark:text-amber-300/80">
                      输入当前验证码或任一未使用的恢复码后，可立即停用 2FA。
                    </p>
                  </div>
                  <Input
                    placeholder="请输入验证码或恢复码"
                    value={disableCode}
                    onChange={(event) => setDisableCode(event.target.value)}
                  />
                  <Button
                    variant="outline"
                    onClick={handleDisableTwoFactor}
                    disabled={loading}
                    className="justify-start"
                  >
                    {isManagingTwoFactor ? (
                      <LoaderCircle className="mr-2 h-4 w-4 animate-spin" />
                    ) : null}
                    停用 2FA
                  </Button>
                </div>
              ) : null}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          {appSettings.webAccessPasswordConfigured ? (
            <Button
              variant="ghost"
              onClick={handleClear}
              disabled={loading}
              className="text-destructive hover:bg-destructive/10 hover:text-destructive"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              清除密码
            </Button>
          ) : null}
          <Button onClick={handleSave} disabled={loading}>
            {isSavingPassword ? (
              <LoaderCircle className="mr-2 h-4 w-4 animate-spin" />
            ) : null}
            保存设置
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
