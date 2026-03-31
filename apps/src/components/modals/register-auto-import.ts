export function getRegisterSubmitLabel(autoImport: boolean): string {
  return autoImport ? "开始注册并导入" : "开始注册";
}

export function buildManualImportSummary(title: string, accountCount: number): string {
  const normalizedTitle = String(title || "").trim() || "注册";
  const normalizedCount = Number.isFinite(accountCount) ? Math.max(0, Math.trunc(accountCount)) : 0;
  if (normalizedCount <= 1) {
    return `${normalizedTitle}已完成，未自动入池，可在注册中心手动加入号池`;
  }
  return `${normalizedTitle}已完成，共 ${normalizedCount} 个账号未自动入池，可在注册中心手动加入号池`;
}
