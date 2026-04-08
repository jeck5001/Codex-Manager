export const APP_NAV_ITEMS = [
  { id: "dashboard", name: "仪表盘", href: "/" },
  { id: "accounts", name: "账号管理", href: "/accounts/" },
  { id: "register", name: "注册中心", href: "/register/" },
  { id: "payment", name: "支付中心", href: "/payment/" },
  { id: "emailServices", name: "邮箱服务", href: "/email-services/" },
  { id: "hotmail", name: "Hotmail", href: "/hotmail/" },
  { id: "apiKeys", name: "平台密钥", href: "/apikeys/" },
  { id: "logs", name: "请求日志", href: "/logs/" },
  { id: "audit", name: "审计日志", href: "/audit/" },
  { id: "costs", name: "费用统计", href: "/costs/" },
  { id: "analytics", name: "用量分析", href: "/analytics/" },
  { id: "settings", name: "设置", href: "/settings/" },
] as const;

export type AppNavItemId = (typeof APP_NAV_ITEMS)[number]["id"];

export const APP_NAV_ALWAYS_VISIBLE_IDS: AppNavItemId[] = ["settings"];
const APP_NAV_UPGRADE_VISIBLE_IDS: AppNavItemId[] = ["hotmail"];

export const APP_NAV_DEFAULT_VISIBLE_IDS: AppNavItemId[] = APP_NAV_ITEMS.map(
  (item) => item.id
);

export function normalizeVisibleMenuItems(
  value: readonly string[] | null | undefined
): AppNavItemId[] {
  const allowed = new Set<string>(APP_NAV_ITEMS.map((item) => item.id));
  const items = Array.isArray(value) ? value : [];
  const normalized = items
    .map((item) => String(item || "").trim())
    .filter((item): item is AppNavItemId => Boolean(item) && allowed.has(item));

  const deduped = APP_NAV_ITEMS.filter(
    (item) =>
      normalized.includes(item.id) || APP_NAV_ALWAYS_VISIBLE_IDS.includes(item.id)
  ).map((item) => item.id);

  if (deduped.length === 0) {
    return [...APP_NAV_DEFAULT_VISIBLE_IDS];
  }

  const hasLegacyFullMenu = APP_NAV_ITEMS.filter(
    (item) => !APP_NAV_UPGRADE_VISIBLE_IDS.includes(item.id)
  ).every((item) => deduped.includes(item.id));

  if (!hasLegacyFullMenu) {
    return deduped;
  }

  return APP_NAV_ITEMS.filter(
    (item) =>
      deduped.includes(item.id) ||
      APP_NAV_ALWAYS_VISIBLE_IDS.includes(item.id) ||
      APP_NAV_UPGRADE_VISIBLE_IDS.includes(item.id)
  ).map((item) => item.id);
}
