export type RegisterChannel = "standard" | "browserbase_ddg";
export type RegisterModeOption = "single" | "batch" | "outlook-batch";

export function getRegisterChannelLabel(channel: string | null | undefined): string {
  if (String(channel || "").trim().toLowerCase() === "browserbase_ddg") {
    return "Browserbase-DDG 注册";
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
