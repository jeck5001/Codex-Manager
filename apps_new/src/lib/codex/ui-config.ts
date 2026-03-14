import type { LucideIcon } from "lucide-react";
import { Activity, Blocks, KeyRound, Settings2, SquareTerminal } from "lucide-react";

import type { PageId, ThemeId } from "@/lib/codex/types";

export type NavItem = {
  page: PageId;
  title: string;
  description: string;
  icon: LucideIcon;
};

export type ThemeOption = {
  id: ThemeId;
  label: string;
  description: string;
  accent: string;
};

export const navItems: NavItem[] = [
  {
    page: "dashboard",
    title: "仪表盘",
    description: "账号池健康度、当日用量与推荐视图",
    icon: SquareTerminal,
  },
  {
    page: "accounts",
    title: "账号管理",
    description: "搜索、分页、批量操作与优先账号控制",
    icon: Blocks,
  },
  {
    page: "apikeys",
    title: "平台密钥",
    description: "协议、模型和推理等级管理",
    icon: KeyRound,
  },
  {
    page: "requestlogs",
    title: "请求日志",
    description: "状态筛选、路径复制与错误追踪",
    icon: Activity,
  },
  {
    page: "settings",
    title: "设置",
    description: "服务、传输、后台任务与环境覆盖",
    icon: Settings2,
  },
];

export const themeOptions: ThemeOption[] = [
  { id: "tech", label: "企业蓝", description: "数据密集型默认风格", accent: "#1e40af" },
  { id: "dark", label: "夜航蓝", description: "深色监控模式", accent: "#38bdf8" },
  { id: "business", label: "事务金", description: "稳重商务观感", accent: "#b98b2f" },
  { id: "mint", label: "薄荷绿", description: "轻量与健康感", accent: "#11b981" },
  { id: "sunset", label: "晚霞橙", description: "高提醒度动作色", accent: "#f97316" },
  { id: "grape", label: "葡萄灰紫", description: "辅助主题", accent: "#7c4dff" },
  { id: "ocean", label: "海湾青", description: "冷静数据视图", accent: "#0284c7" },
  { id: "forest", label: "松林绿", description: "长时工作模式", accent: "#2f9e44" },
  { id: "rose", label: "玫瑰粉", description: "轻强调主题", accent: "#e11d78" },
  { id: "slate", label: "石板灰", description: "深灰冷静模式", accent: "#334155" },
  { id: "aurora", label: "极光青", description: "高亮展示主题", accent: "#0ea5e9" },
] as const;

export const pageMeta: Record<PageId, { title: string; description: string; kicker: string }> = {
  dashboard: {
    title: "仪表盘",
    description: "以账号池健康度、令牌消耗和手动锁定状态为中心的总览台。",
    kicker: "Operations",
  },
  accounts: {
    title: "账号管理",
    description: "处理授权、导入、批量删除、配额追踪与优先账号锁定。",
    kicker: "Accounts",
  },
  apikeys: {
    title: "平台密钥",
    description: "集中维护协议、模型覆盖、推理等级和明文复制能力。",
    kicker: "Gateway Keys",
  },
  requestlogs: {
    title: "请求日志",
    description: "查看请求路径、上游映射、状态码与故障明细。",
    kicker: "Traffic",
  },
  settings: {
    title: "设置",
    description: "服务地址、传输、安全、后台任务与环境覆盖的统一配置面板。",
    kicker: "Runtime",
  },
};
