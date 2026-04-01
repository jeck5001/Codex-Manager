export type RegisterChannel = "standard" | "browserbase_ddg" | "any_auto";
export type RegisterModeOption = "single" | "batch" | "outlook-batch";

export function getRegisterChannelLabel(channel: string | null | undefined): string {
  const normalized = String(channel || "").trim().toLowerCase();
  if (normalized === "browserbase_ddg") {
    return "Browserbase-DDG 注册";
  }
  if (normalized === "any_auto") {
    return "Any-Auto 注册";
  }
  return "标准注册";
}

export function canUseOutlookBatchRegisterMode(channel: string | null | undefined): boolean {
  return String(channel || "").trim().toLowerCase() !== "browserbase_ddg";
}

export function sanitizeRegisterModeForChannel(
  channel: string | null | undefined,
  mode: RegisterModeOption,
): RegisterModeOption {
  if (!canUseOutlookBatchRegisterMode(channel) && mode === "outlook-batch") {
    return "single";
  }
  return mode;
}
