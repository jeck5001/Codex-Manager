"use client";

import { useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  Mail,
  MoreVertical,
  PlayCircle,
  Plus,
  RefreshCw,
  ShieldCheck,
  Trash2,
  Upload,
  Wrench,
  XCircle,
} from "lucide-react";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { useRegisterEmailServices } from "@/hooks/useRegisterEmailServices";
import { formatApiDateTime } from "@/lib/utils/datetime";
import { cn } from "@/lib/utils";
import { toast } from "sonner";
import type {
  RegisterEmailService,
  RegisterEmailServiceField,
  RegisterEmailServiceType,
} from "@/types";

type ServiceFormMode = "create" | "edit";

type ServiceFormState = {
  mode: ServiceFormMode;
  serviceId: number | null;
  serviceType: string;
  name: string;
  enabled: boolean;
  priority: string;
  config: Record<string, unknown>;
};

type DeleteState =
  | { kind: "single"; service: RegisterEmailService }
  | { kind: "outlook-batch"; ids: number[]; count: number }
  | null;

const EMPTY_FORM: ServiceFormState = {
  mode: "create",
  serviceId: null,
  serviceType: "",
  name: "",
  enabled: true,
  priority: "0",
  config: {},
};

function formatServiceTypeLabel(type: RegisterEmailServiceType | undefined, value: string) {
  return type?.label || value || "未命名类型";
}

function formatTimestamp(value: string) {
  return formatApiDateTime(value, { emptyLabel: "未使用", withSeconds: false });
}

function getDefaultFieldValue(field: RegisterEmailServiceField) {
  if (
    typeof field.defaultValue === "string" ||
    typeof field.defaultValue === "number" ||
    typeof field.defaultValue === "boolean"
  ) {
    return field.defaultValue;
  }
  return "";
}

function buildFormConfig(
  typeMeta: RegisterEmailServiceType | undefined,
  rawConfig?: Record<string, unknown>
) {
  const nextConfig: Record<string, unknown> = { ...(rawConfig || {}) };
  for (const field of typeMeta?.configFields || []) {
    if (!(field.name in nextConfig)) {
      nextConfig[field.name] = getDefaultFieldValue(field);
    }
  }
  return nextConfig;
}

function createFormState(
  mode: ServiceFormMode,
  typeMeta: RegisterEmailServiceType | undefined,
  service?: RegisterEmailService
): ServiceFormState {
  return {
    mode,
    serviceId: service?.id ?? null,
    serviceType: service?.serviceType || typeMeta?.value || "",
    name: service?.name || "",
    enabled: service?.enabled ?? true,
    priority: String(service?.priority ?? 0),
    config: buildFormConfig(typeMeta, service?.config),
  };
}

function isBooleanField(field: RegisterEmailServiceField, value: unknown) {
  return typeof field.defaultValue === "boolean" || typeof value === "boolean";
}

function isNumberField(field: RegisterEmailServiceField, value: unknown) {
  return typeof field.defaultValue === "number" || typeof value === "number";
}

function stringifyFieldValue(value: unknown) {
  if (typeof value === "string" || typeof value === "number") {
    return String(value);
  }
  return "";
}

function buildConfigPayload(
  form: ServiceFormState,
  typeMeta: RegisterEmailServiceType | undefined
) {
  const fieldMap = new Map((typeMeta?.configFields || []).map((field) => [field.name, field]));
  const payload: Record<string, unknown> = {};

  for (const [key, rawValue] of Object.entries(form.config || {})) {
    const field = fieldMap.get(key);

    if (typeof rawValue === "boolean") {
      payload[key] = rawValue;
      continue;
    }

    const text = typeof rawValue === "string" || typeof rawValue === "number"
      ? String(rawValue).trim()
      : "";

    if (field && isNumberField(field, rawValue || field.defaultValue)) {
      if (text) {
        const parsed = Number(text);
        payload[key] = Number.isFinite(parsed) ? parsed : text;
      } else if (form.mode === "edit") {
        payload[key] = "";
      }
      continue;
    }

    if (text) {
      payload[key] = text;
      continue;
    }

    if (form.mode === "edit" && field) {
      payload[key] = "";
    }
  }

  return payload;
}

const SENSITIVE_CONFIG_KEYWORDS = [
  "password",
  "secret",
  "token",
  "api_key",
  "apikey",
  "access_token",
  "refresh_token",
  "client_secret",
];

function normalizeConfigKey(key: string) {
  return key.trim().toLowerCase().replace(/[\s-]+/g, "_");
}

function isSensitiveConfigField(
  key: string,
  field: RegisterEmailServiceField | undefined
) {
  if (field?.secret) {
    return true;
  }
  const normalized = normalizeConfigKey(key);
  return SENSITIVE_CONFIG_KEYWORDS.some((keyword) => normalized.includes(keyword));
}

function summarizeConfig(
  config: Record<string, unknown>,
  typeMeta: RegisterEmailServiceType | undefined
) {
  const entries = Object.entries(config || {}).filter(([, value]) => {
    if (typeof value === "boolean") {
      return value;
    }
    return value !== null && value !== undefined && String(value).trim() !== "";
  });

  if (!entries.length) return "无配置";

  const fieldMap = new Map((typeMeta?.configFields || []).map((field) => [field.name, field]));

  return entries
    .slice(0, 3)
    .map(([key, value]) => {
      const field = fieldMap.get(key);
      const displayKey = field?.label || key;
      if (typeof value === "boolean") {
        return `${displayKey}: 是`;
      }
      if (isSensitiveConfigField(key, field)) {
        return `${displayKey}: 已隐藏`;
      }
      return `${displayKey}: ${String(value)}`;
    })
    .join(" · ");
}

export default function EmailServicesPage() {
  const [search, setSearch] = useState("");
  const [serviceTypeFilter, setServiceTypeFilter] = useState("all");
  const [enabledOnly, setEnabledOnly] = useState(false);
  const [formOpen, setFormOpen] = useState(false);
  const [formState, setFormState] = useState<ServiceFormState>(EMPTY_FORM);
  const [isOpeningEdit, setIsOpeningEdit] = useState(false);
  const [deleteState, setDeleteState] = useState<DeleteState>(null);
  const [selectedOutlookIds, setSelectedOutlookIds] = useState<number[]>([]);
  const [outlookImportOpen, setOutlookImportOpen] = useState(false);
  const [outlookImportData, setOutlookImportData] = useState("");
  const [outlookImportEnabled, setOutlookImportEnabled] = useState(true);
  const [outlookImportPriority, setOutlookImportPriority] = useState("0");
  const [importResultText, setImportResultText] = useState("");
  const [tempmailTestOpen, setTempmailTestOpen] = useState(false);
  const [tempmailTestUrl, setTempmailTestUrl] = useState("");

  const {
    serviceTypes,
    services,
    stats,
    isLoading,
    isTypesLoading,
    isStatsLoading,
    refetchServices,
    createEmailService,
    updateEmailService,
    deleteEmailService,
    readEmailServiceFull,
    testEmailService,
    setEmailServiceEnabled,
    importOutlookServices,
    batchDeleteOutlookServices,
    reorderEmailServices,
    testTempmailConnection,
    isCreating,
    isUpdating,
    isDeleting,
    isReadingFull,
    isTesting,
    isToggling,
    isImporting,
    isBatchDeletingOutlook,
    isReordering,
    isTestingTempmail,
  } = useRegisterEmailServices({
    serviceType: serviceTypeFilter === "all" ? null : serviceTypeFilter,
    enabledOnly,
  });

  const serviceTypeMap = useMemo(
    () => new Map(serviceTypes.map((item) => [item.value, item])),
    [serviceTypes]
  );

  const filteredServices = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    if (!keyword) return services;
    return services.filter((service) => {
      return (
        service.name.toLowerCase().includes(keyword) ||
        service.serviceType.toLowerCase().includes(keyword) ||
        String(service.id).includes(keyword)
      );
    });
  }, [search, services]);

  const outlookServices = useMemo(() => {
    return filteredServices.filter((service) => service.serviceType === "outlook");
  }, [filteredServices]);

  const statsSnapshot = useMemo(() => {
    const enabledCount = stats?.enabledCount ?? services.filter((service) => service.enabled).length;
    const outlookCount = stats?.outlookCount ?? services.filter((service) => service.serviceType === "outlook").length;
    const tempMailCount =
      stats?.tempMailCount ??
      services.filter((service) => service.serviceType === "tempmail" || service.serviceType === "temp_mail").length;
    const customCount =
      stats?.customCount ?? services.filter((service) => service.serviceType === "custom_domain").length;
    const totalServices = outlookCount + customCount + tempMailCount;

    return {
      totalServices,
      enabledCount,
      disabledCount: Math.max(0, totalServices - enabledCount),
      outlookCount,
      customCount,
      tempMailCount,
      tempmailAvailable: stats?.tempmailAvailable ?? true,
    };
  }, [services, stats]);

  const allVisibleOutlookSelected =
    outlookServices.length > 0 &&
    outlookServices.every((service) => selectedOutlookIds.includes(service.id));

  const selectedType = serviceTypeMap.get(formState.serviceType);
  const isSubmittingForm = isCreating || isUpdating || isReadingFull || isOpeningEdit;

  useEffect(() => {
    if (!formOpen || formState.mode !== "create" || formState.serviceType || serviceTypes.length === 0) {
      return;
    }
    const nextType = serviceTypes[0];
    setFormState((current) => ({
      ...current,
      serviceType: nextType.value,
      config: buildFormConfig(nextType),
    }));
  }, [formOpen, formState.mode, formState.serviceType, serviceTypes]);

  useEffect(() => {
    setSelectedOutlookIds((current) =>
      current.filter((id) => outlookServices.some((service) => service.id === id))
    );
  }, [outlookServices]);

  const openCreateDialog = () => {
    const nextType = serviceTypes[0];
    setFormState(createFormState("create", nextType));
    setFormOpen(true);
  };

  const openEditDialog = async (serviceId: number) => {
    setIsOpeningEdit(true);
    try {
      const fullService = await readEmailServiceFull(serviceId);
      const typeMeta = serviceTypeMap.get(fullService.serviceType);
      setFormState(createFormState("edit", typeMeta, fullService));
      setFormOpen(true);
    } catch {
      // mutation 已统一 toast
    } finally {
      setIsOpeningEdit(false);
    }
  };

  const handleServiceTypeChange = (value: string | null) => {
    const nextValue = value || "";
    if (!nextValue) {
      return;
    }
    const nextType = serviceTypeMap.get(nextValue);
    setFormState((current) => ({
      ...current,
      serviceType: nextValue,
      config: buildFormConfig(nextType, current.mode === "edit" ? current.config : undefined),
    }));
  };

  const handleConfigChange = (fieldName: string, value: unknown) => {
    setFormState((current) => ({
      ...current,
      config: {
        ...current.config,
        [fieldName]: value,
      },
    }));
  };

  const handleFilterTypeChange = (value: string | null) => {
    setServiceTypeFilter(value || "all");
  };

  const handleSubmitForm = async () => {
    const serviceType = formState.serviceType.trim();
    const name = formState.name.trim();
    if (!serviceType) return;
    if (!name) {
      toast.error("请输入服务名称");
      return;
    }

    const missingRequiredField = (selectedType?.configFields || []).find((field) => {
      if (!field.required) return false;
      const value = formState.config[field.name];
      if (typeof value === "boolean") return false;
      return String(value ?? "").trim() === "";
    });
    if (missingRequiredField) {
      toast.error(`请填写必填项：${missingRequiredField.label}`);
      return;
    }

    const parsedPriority = Number(formState.priority || 0);
    const priority = Number.isFinite(parsedPriority) ? Math.max(0, Math.trunc(parsedPriority)) : 0;
    const config = buildConfigPayload(formState, selectedType);

    try {
      if (formState.mode === "create") {
        await createEmailService({
          serviceType,
          name,
          enabled: formState.enabled,
          priority,
          config,
        });
      } else if (formState.serviceId) {
        await updateEmailService({
          serviceId: formState.serviceId,
          name,
          enabled: formState.enabled,
          priority,
          config,
        });
      }
      setFormOpen(false);
      setFormState(EMPTY_FORM);
    } catch {
      // mutation 已统一 toast
    }
  };

  const handleToggleOutlookSelection = (serviceId: number, checked: boolean) => {
    setSelectedOutlookIds((current) => {
      if (checked) {
        return current.includes(serviceId) ? current : [...current, serviceId];
      }
      return current.filter((id) => id !== serviceId);
    });
  };

  const handleToggleAllVisibleOutlook = (checked: boolean) => {
    const visibleIds = outlookServices.map((service) => service.id);
    setSelectedOutlookIds((current) => {
      if (checked) {
        return Array.from(new Set([...current, ...visibleIds]));
      }
      return current.filter((id) => !visibleIds.includes(id));
    });
  };

  const handleDeleteConfirm = async () => {
    if (!deleteState) return;
    if (deleteState.kind === "single") {
      deleteEmailService(deleteState.service.id);
      return;
    }

    try {
      await batchDeleteOutlookServices(deleteState.ids);
      setSelectedOutlookIds((current) => current.filter((id) => !deleteState.ids.includes(id)));
    } catch {
      // mutation 已统一 toast
    }
  };

  const handleMovePriority = async (serviceId: number, direction: "up" | "down") => {
    const nextList = [...services];
    const currentIndex = nextList.findIndex((item) => item.id === serviceId);
    if (currentIndex < 0) return;

    const targetIndex = direction === "up" ? currentIndex - 1 : currentIndex + 1;
    if (targetIndex < 0 || targetIndex >= nextList.length) return;

    const [currentItem] = nextList.splice(currentIndex, 1);
    nextList.splice(targetIndex, 0, currentItem);

    try {
      await reorderEmailServices(nextList.map((item) => item.id));
    } catch {
      // mutation 已统一 toast
    }
  };

  const handleOutlookImport = async () => {
    const data = outlookImportData.trim();
    if (!data) {
      toast.error("请先粘贴 Outlook 账号内容");
      return;
    }
    const parsedPriority = Number(outlookImportPriority || 0);
    const priority = Number.isFinite(parsedPriority) ? Math.max(0, Math.trunc(parsedPriority)) : 0;

    try {
      const result = await importOutlookServices({
        data,
        enabled: outlookImportEnabled,
        priority,
      });
      setImportResultText(
        [
          `总行数: ${result.total}`,
          `成功: ${result.success}`,
          `失败: ${result.failed}`,
          result.errors.length ? "错误明细:" : "",
          ...result.errors,
        ].filter(Boolean).join("\n")
      );
      if (result.success > 0) {
        setOutlookImportData("");
      }
    } catch {
      // mutation 已统一 toast
    }
  };

  const handleTestTempmail = async () => {
    try {
      await testTempmailConnection(tempmailTestUrl.trim() || null);
      setTempmailTestOpen(false);
    } catch {
      // mutation 已统一 toast
    }
  };

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {[
          {
            title: "服务总数",
            value: statsSnapshot.totalServices,
            hint: "注册流程可调度的邮箱服务",
            icon: Mail,
          },
          {
            title: "启用中",
            value: statsSnapshot.enabledCount,
            hint: `已禁用 ${statsSnapshot.disabledCount}`,
            icon: ShieldCheck,
          },
          {
            title: "Outlook",
            value: statsSnapshot.outlookCount,
            hint: "支持批量导入邮箱账户",
            icon: Upload,
          },
          {
            title: "自定义/临时邮箱",
            value: statsSnapshot.customCount + statsSnapshot.tempMailCount,
            hint: statsSnapshot.tempmailAvailable ? "Tempmail.lol 当前可用" : "Tempmail.lol 当前不可用",
            icon: Wrench,
          },
        ].map((item) => (
          <Card key={item.title} className="glass-card border-none shadow-md">
            <CardContent className="flex items-start justify-between gap-4 pt-0">
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">{item.title}</p>
                <div className="text-3xl font-semibold tracking-tight">{item.value}</div>
                <p className="text-xs text-muted-foreground">{item.hint}</p>
              </div>
              <div className="rounded-2xl bg-primary/10 p-3 text-primary">
                <item.icon className="h-5 w-5" />
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card className="glass-card border-none shadow-md">
        <CardHeader className="border-b border-border/60">
          <CardTitle>邮箱服务管理</CardTitle>
          <CardDescription>
            管理自动注册可用的邮箱服务。支持单个配置、启停、联通性测试，以及 Outlook 批量导入。
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 pt-0 lg:grid-cols-[minmax(0,1fr)_180px_auto_auto] lg:items-center">
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="搜索名称 / 类型 / ID..."
            className="h-10 rounded-xl bg-card/60"
          />

          <Select value={serviceTypeFilter} onValueChange={handleFilterTypeChange}>
            <SelectTrigger className="h-10 w-full rounded-xl bg-card/60">
              <SelectValue placeholder="全部类型" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部类型</SelectItem>
              {serviceTypes.map((type) => (
                <SelectItem key={type.value} value={type.value}>
                  {type.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <div className="flex items-center gap-2 rounded-xl border border-border/60 bg-muted/30 px-3 py-2">
            <Switch checked={enabledOnly} onCheckedChange={setEnabledOnly} />
            <span className="text-sm">只看启用项</span>
          </div>

          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button
              variant="outline"
              className="h-10 rounded-xl"
              onClick={() => void refetchServices()}
            >
              <RefreshCw className={cn("h-4 w-4", isLoading && "animate-spin")} />
              刷新
            </Button>
            <Button
              variant="outline"
              className="h-10 rounded-xl"
              disabled={isStatsLoading || isTestingTempmail}
              onClick={() => setTempmailTestOpen(true)}
            >
              <Wrench className="h-4 w-4" />
              测试 Tempmail
            </Button>
            <Button
              variant="outline"
              className="h-10 rounded-xl"
              onClick={() => {
                setOutlookImportOpen(true);
                setImportResultText("");
              }}
            >
              <Upload className="h-4 w-4" />
              Outlook 批量导入
            </Button>
            <Button
              className="h-10 rounded-xl"
              disabled={isTypesLoading || serviceTypes.length === 0}
              onClick={openCreateDialog}
            >
              <Plus className="h-4 w-4" />
              新建服务
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card className="glass-card overflow-hidden border-none py-0 shadow-xl">
        <CardHeader className="border-b border-border/60">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <CardTitle>服务列表</CardTitle>
              <CardDescription>
                Outlook 账户支持批量选择删除；所有服务都支持启停、测试和编辑。
              </CardDescription>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                className="h-9 rounded-xl"
                disabled={selectedOutlookIds.length === 0 || isBatchDeletingOutlook}
                onClick={() =>
                  setDeleteState({
                    kind: "outlook-batch",
                    ids: [...selectedOutlookIds],
                    count: selectedOutlookIds.length,
                  })
                }
              >
                <Trash2 className="h-4 w-4" />
                {selectedOutlookIds.length > 0
                  ? `删除选中 Outlook (${selectedOutlookIds.length})`
                  : "批量删除 Outlook"}
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[52px] text-center">
                  <Checkbox
                    checked={allVisibleOutlookSelected}
                    onCheckedChange={(checked) => handleToggleAllVisibleOutlook(checked === true)}
                  />
                </TableHead>
                <TableHead className="w-[84px]">ID</TableHead>
                <TableHead className="min-w-[180px]">名称</TableHead>
                <TableHead className="w-[140px]">类型</TableHead>
                <TableHead className="w-[90px]">状态</TableHead>
                <TableHead className="w-[90px]">优先级</TableHead>
                <TableHead className="min-w-[280px]">配置概览</TableHead>
                <TableHead className="w-[160px]">最近使用</TableHead>
                <TableHead className="w-[72px] text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                Array.from({ length: 5 }).map((_, index) => (
                  <TableRow key={`loading-${index}`}>
                    <TableCell colSpan={9}>
                      <Skeleton className="h-9 w-full" />
                    </TableCell>
                  </TableRow>
                ))
              ) : filteredServices.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={9} className="py-12 text-center text-muted-foreground">
                    当前没有匹配的邮箱服务
                  </TableCell>
                </TableRow>
              ) : (
                filteredServices.map((service) => {
                  const typeMeta = serviceTypeMap.get(service.serviceType);
                  const isOutlook = service.serviceType === "outlook";
                  return (
                    <TableRow key={service.id} className="border-border/60">
                      <TableCell className="text-center">
                        {isOutlook ? (
                          <Checkbox
                            checked={selectedOutlookIds.includes(service.id)}
                            onCheckedChange={(checked) =>
                              handleToggleOutlookSelection(service.id, checked === true)
                            }
                          />
                        ) : null}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        #{service.id}
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col gap-1">
                          <span className="font-medium">{service.name}</span>
                          <span className="text-xs text-muted-foreground">
                            更新于 {formatTimestamp(service.updatedAt)}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>{formatServiceTypeLabel(typeMeta, service.serviceType)}</TableCell>
                      <TableCell>
                        <Badge variant={service.enabled ? "default" : "secondary"}>
                          {service.enabled ? "已启用" : "已禁用"}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <span>{service.priority}</span>
                          <div className="flex items-center gap-1">
                            <Button
                              variant="ghost"
                              size="icon-xs"
                              disabled={isReordering}
                              onClick={() => void handleMovePriority(service.id, "up")}
                            >
                              ↑
                            </Button>
                            <Button
                              variant="ghost"
                              size="icon-xs"
                              disabled={isReordering}
                              onClick={() => void handleMovePriority(service.id, "down")}
                            >
                              ↓
                            </Button>
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="whitespace-normal text-xs text-muted-foreground">
                        {summarizeConfig(service.config, typeMeta)}
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {formatTimestamp(service.lastUsed)}
                      </TableCell>
                      <TableCell className="text-right">
                        <DropdownMenu>
                          <DropdownMenuTrigger>
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              render={<span />}
                              nativeButton={false}
                            >
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end" className="w-44">
                            <DropdownMenuItem onClick={() => void openEditDialog(service.id)}>
                              <Wrench className="mr-2 h-4 w-4" />
                              编辑
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => void testEmailService(service.id)}>
                              <PlayCircle className="mr-2 h-4 w-4" />
                              测试连接
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onClick={() =>
                                setEmailServiceEnabled({
                                  serviceId: service.id,
                                  enabled: !service.enabled,
                                })
                              }
                            >
                              {service.enabled ? (
                                <XCircle className="mr-2 h-4 w-4" />
                              ) : (
                                <CheckCircle2 className="mr-2 h-4 w-4" />
                              )}
                              {service.enabled ? "禁用" : "启用"}
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onClick={() => setDeleteState({ kind: "single", service })}
                            >
                              <Trash2 className="mr-2 h-4 w-4" />
                              删除
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog
        open={formOpen}
        onOpenChange={(open) => {
          setFormOpen(open);
          if (!open) {
            setFormState(EMPTY_FORM);
          }
        }}
      >
        <DialogContent className="glass-card max-h-[88vh] overflow-y-auto border-none p-6 sm:max-w-[720px]">
          <DialogHeader>
            <DialogTitle>{formState.mode === "create" ? "新建邮箱服务" : "编辑邮箱服务"}</DialogTitle>
            <DialogDescription>
              {formState.mode === "create"
                ? "创建后即可在自动注册流程中直接使用。"
                : "这里读取的是完整配置，包含编辑时需要的敏感字段。"}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>服务类型</Label>
              <Select
                value={formState.serviceType}
                onValueChange={handleServiceTypeChange}
                disabled={formState.mode === "edit"}
              >
                <SelectTrigger className="h-10 w-full rounded-xl">
                  <SelectValue placeholder="选择邮箱服务类型" />
                </SelectTrigger>
                <SelectContent>
                  {serviceTypes.map((type) => (
                    <SelectItem key={type.value} value={type.value}>
                      {type.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {selectedType?.description ? (
                <p className="text-xs text-muted-foreground">{selectedType.description}</p>
              ) : null}
            </div>

            <div className="space-y-2">
              <Label>服务名称</Label>
              <Input
                value={formState.name}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, name: event.target.value }))
                }
                placeholder="例如：主力 Outlook 池"
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>优先级</Label>
              <Input
                type="number"
                min="0"
                value={formState.priority}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, priority: event.target.value }))
                }
                className="h-10 rounded-xl"
              />
            </div>

            <div className="space-y-2">
              <Label>启用状态</Label>
              <div className="flex h-10 items-center justify-between rounded-xl border border-border/60 px-3">
                <span className="text-sm text-muted-foreground">创建后立即参与调度</span>
                <Switch
                  checked={formState.enabled}
                  onCheckedChange={(checked) =>
                    setFormState((current) => ({ ...current, enabled: checked }))
                  }
                />
              </div>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            {(selectedType?.configFields || []).map((field) => {
              const fieldValue = formState.config[field.name];
              const isBoolean = isBooleanField(field, fieldValue);
              const isNumber = isNumberField(field, fieldValue);

              return (
                <div key={field.name} className="space-y-2">
                  <Label>
                    {field.label}
                    {field.required ? <span className="text-destructive">*</span> : null}
                  </Label>
                  {isBoolean ? (
                    <div className="flex h-10 items-center justify-between rounded-xl border border-border/60 px-3">
                      <span className="text-sm text-muted-foreground">
                        {field.placeholder || "启用后按该配置运行"}
                      </span>
                      <Switch
                        checked={Boolean(fieldValue)}
                        onCheckedChange={(checked) => handleConfigChange(field.name, checked)}
                      />
                    </div>
                  ) : (
                    <Input
                      type={isNumber ? "number" : field.secret ? "password" : "text"}
                      value={stringifyFieldValue(fieldValue)}
                      placeholder={field.placeholder || field.label}
                      className="h-10 rounded-xl"
                      onChange={(event) => handleConfigChange(field.name, event.target.value)}
                    />
                  )}
                </div>
              );
            })}
          </div>

          <DialogFooter className="gap-2 sm:gap-2">
            <Button variant="outline" onClick={() => setFormOpen(false)}>
              取消
            </Button>
            <Button
              disabled={isSubmittingForm || !formState.serviceType.trim() || !formState.name.trim()}
              onClick={() => void handleSubmitForm()}
            >
              {isSubmittingForm ? "提交中..." : formState.mode === "create" ? "创建" : "保存"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={outlookImportOpen} onOpenChange={setOutlookImportOpen}>
        <DialogContent className="glass-card max-h-[88vh] overflow-y-auto border-none p-6 sm:max-w-[760px]">
          <DialogHeader>
            <DialogTitle>Outlook 批量导入</DialogTitle>
            <DialogDescription>
              每行一个账号，格式支持 `邮箱----密码` 或 `邮箱----密码----client_id----refresh_token`。
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_220px]">
            <div className="space-y-2">
              <Label>账号内容</Label>
              <Textarea
                value={outlookImportData}
                onChange={(event) => setOutlookImportData(event.target.value)}
                placeholder={"user@example.com----password\nuser2@example.com----password----client_id----refresh_token"}
                className="min-h-[260px] rounded-xl"
              />
            </div>

            <div className="space-y-4">
              <div className="space-y-2">
                <Label>导入优先级</Label>
                <Input
                  type="number"
                  min="0"
                  value={outlookImportPriority}
                  onChange={(event) => setOutlookImportPriority(event.target.value)}
                  className="h-10 rounded-xl"
                />
              </div>
              <div className="space-y-2">
                <Label>导入后启用</Label>
                <div className="flex h-10 items-center justify-between rounded-xl border border-border/60 px-3">
                  <span className="text-sm text-muted-foreground">新导入账号立即可用</span>
                  <Switch
                    checked={outlookImportEnabled}
                    onCheckedChange={setOutlookImportEnabled}
                  />
                </div>
              </div>
              <div className="rounded-2xl border border-dashed border-border/70 bg-muted/30 p-4 text-xs leading-6 text-muted-foreground">
                <p>支持注释行和空行。</p>
                <p>重复邮箱会被判定失败，不会覆盖现有配置。</p>
                <p>批量导入完成后，服务列表会自动刷新。</p>
              </div>
            </div>
          </div>

          {importResultText ? (
            <div className="space-y-2">
              <Label>导入结果</Label>
              <Textarea value={importResultText} readOnly className="min-h-[180px] rounded-xl" />
            </div>
          ) : null}

          <DialogFooter className="gap-2 sm:gap-2">
            <Button variant="outline" onClick={() => setOutlookImportOpen(false)}>
              关闭
            </Button>
            <Button disabled={isImporting || !outlookImportData.trim()} onClick={() => void handleOutlookImport()}>
              {isImporting ? "导入中..." : "开始导入"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={tempmailTestOpen} onOpenChange={setTempmailTestOpen}>
        <DialogContent className="glass-card border-none p-6 sm:max-w-[560px]">
          <DialogHeader>
            <DialogTitle>测试 Tempmail 直连</DialogTitle>
            <DialogDescription>
              可选填写自定义 API 地址；留空时会按注册服务默认配置测试 `Tempmail.lol`。
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-2">
            <Label>Tempmail API 地址</Label>
            <Input
              value={tempmailTestUrl}
              onChange={(event) => setTempmailTestUrl(event.target.value)}
              placeholder="https://api.tempmail.lol/v2"
              className="h-10 rounded-xl"
            />
          </div>

          <DialogFooter className="gap-2 sm:gap-2">
            <Button variant="outline" onClick={() => setTempmailTestOpen(false)}>
              取消
            </Button>
            <Button disabled={isTestingTempmail} onClick={() => void handleTestTempmail()}>
              {isTestingTempmail ? "测试中..." : "开始测试"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={!!deleteState}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteState(null);
          }
        }}
        title="删除邮箱服务"
        description={
          deleteState?.kind === "single"
            ? `确认删除“${deleteState.service.name}”吗？删除后自动注册将不再使用该服务。`
            : deleteState?.kind === "outlook-batch"
              ? `确认批量删除选中的 ${deleteState.count} 个 Outlook 账户吗？`
              : ""
        }
        confirmText={isDeleting || isBatchDeletingOutlook ? "删除中..." : "删除"}
        confirmVariant="destructive"
        onConfirm={() => {
          void handleDeleteConfirm();
        }}
      />

      {(isTesting || isToggling || isReordering) && (
        <div className="fixed right-6 bottom-6 rounded-full border border-border/70 bg-background/90 px-3 py-2 text-xs text-muted-foreground shadow-lg backdrop-blur">
          正在执行邮箱服务操作...
        </div>
      )}
    </div>
  );
}
