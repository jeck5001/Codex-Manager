"use client";

import { useEffect, useMemo, useState } from "react";
import {
  MoreVertical,
  PencilLine,
  Plus,
  RefreshCw,
  Search,
  Trash2,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
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
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import { ModelCatalogModal } from "@/components/modals/model-catalog-modal";
import { useManagedModels } from "@/hooks/useManagedModels";
import { findBestMatchingModel } from "@/lib/api/model-catalog";
import { useAppStore } from "@/lib/store/useAppStore";
import { formatTsFromSeconds } from "@/lib/utils/usage";

type ModelFilter = "all" | "api" | "custom" | "edited";

function MiniStatBadge({ label, value }: { label: string; value: string }) {
  return (
    <div className="inline-flex items-center gap-2 rounded-full border border-border/60 bg-background/45 px-3 py-1.5 text-xs text-muted-foreground">
      <span>{label}</span>
      <span className="font-semibold text-foreground">{value}</span>
    </div>
  );
}

function t(message: string, params?: Record<string, string | number>) {
  if (!params) return message;
  return message.replace(/\{(\w+)\}/g, (_, key) => String(params[key] ?? ""));
}

export default function ModelsPage() {
  const { serviceStatus } = useAppStore();
  const {
    models,
    isLoading,
    isServiceReady,
    refreshRemote,
    saveModel,
    deleteModel,
    deleteModels,
    isRefreshing,
    isSaving,
    isDeleting,
  } = useManagedModels();

  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<ModelFilter>("all");
  const [modalOpen, setModalOpen] = useState(false);
  const [editingSlug, setEditingSlug] = useState<string | null>(null);
  const [selectedSlugs, setSelectedSlugs] = useState<string[]>([]);
  const [deleteSlugs, setDeleteSlugs] = useState<string[]>([]);

  useEffect(() => {
    const availableSlugs = new Set(models.map((item) => item.slug));
    setSelectedSlugs((current) => current.filter((slug) => availableSlugs.has(slug)));
    setDeleteSlugs((current) => current.filter((slug) => availableSlugs.has(slug)));
  }, [models]);

  const editingModel = useMemo(
    () => findBestMatchingModel(models, editingSlug || ""),
    [editingSlug, models]
  );
  const nextSortIndex = useMemo(
    () => models.reduce((maxValue, item) => Math.max(maxValue, item.sortIndex), -1) + 1,
    [models]
  );
  const stats = useMemo(
    () => ({
      total: models.length,
      apiEnabled: models.filter((item) => item.supportedInApi).length,
      custom: models.filter((item) => item.sourceKind === "custom").length,
      edited: models.filter((item) => item.userEdited).length,
    }),
    [models]
  );

  const filteredModels = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    return models.filter((model) => {
      const matchesKeyword =
        !keyword ||
        model.slug.toLowerCase().includes(keyword) ||
        model.displayName.toLowerCase().includes(keyword) ||
        String(model.description || "").toLowerCase().includes(keyword);
      if (!matchesKeyword) return false;
      switch (filter) {
        case "api":
          return model.supportedInApi;
        case "custom":
          return model.sourceKind === "custom";
        case "edited":
          return model.userEdited;
        default:
          return true;
      }
    });
  }, [filter, models, search]);

  const visibleSelectedSlugs = useMemo(
    () =>
      filteredModels.map((model) => model.slug).filter((slug) => selectedSlugs.includes(slug)),
    [filteredModels, selectedSlugs]
  );

  const allVisibleSelected =
    filteredModels.length > 0 && visibleSelectedSlugs.length === filteredModels.length;
  const deleteTargetCount = deleteSlugs.length;
  const singleDeleteSlug = deleteTargetCount === 1 ? deleteSlugs[0] : null;

  const toggleSelectSlug = (slug: string) => {
    setSelectedSlugs((current) =>
      current.includes(slug) ? current.filter((item) => item !== slug) : [...current, slug]
    );
  };

  const toggleSelectAllVisible = () => {
    const visibleSlugs = filteredModels.map((model) => model.slug);
    setSelectedSlugs((current) => {
      if (visibleSlugs.length > 0 && visibleSlugs.every((slug) => current.includes(slug))) {
        return current.filter((slug) => !visibleSlugs.includes(slug));
      }
      return Array.from(new Set([...current, ...visibleSlugs]));
    });
  };

  const openSingleDeleteDialog = (slug: string) => setDeleteSlugs([slug]);
  const openBatchDeleteDialog = () => setDeleteSlugs(selectedSlugs);

  return (
    <>
      <div className="space-y-3 animate-in fade-in duration-500">
        <div className="space-y-2">
          <Badge className="w-fit rounded-full bg-primary/10 px-3 py-1 text-primary">
            {t("模型目录")}
          </Badge>
          <div className="space-y-1">
            <h1 className="text-3xl font-semibold tracking-tight">{t("模型管理")}</h1>
            <p className="max-w-4xl text-sm leading-6 text-muted-foreground">
              {t("这里维护本地结构化模型目录，支持远端模型并入与本地自定义覆写。")}
            </p>
          </div>
        </div>

        <Card className="glass-card border-none shadow-md backdrop-blur-md">
          <CardHeader className="pb-3">
            <div className="flex flex-col gap-3">
              <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                <div>
                  <CardTitle>{t("模型目录明细")}</CardTitle>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {t("按 slug、显示名称或描述定位，并结合来源与覆写状态查看当前目录。")}
                  </p>
                </div>
                <div className="flex flex-wrap gap-2 lg:justify-end">
                  <Button
                    variant="outline"
                    onClick={() => void refreshRemote()}
                    disabled={isRefreshing || !serviceStatus.connected}
                  >
                    <RefreshCw className={`mr-2 h-4 w-4 ${isRefreshing ? "animate-spin" : ""}`} />
                    {t("远端并入")}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={openBatchDeleteDialog}
                    disabled={selectedSlugs.length === 0 || isDeleting}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t("批量删除模型")}
                  </Button>
                  <Button
                    onClick={() => {
                      setEditingSlug(null);
                      setModalOpen(true);
                    }}
                  >
                    <Plus className="mr-2 h-4 w-4" />
                    {t("新增自定义模型")}
                  </Button>
                </div>
              </div>
              <div className="flex flex-wrap gap-2 text-xs">
                <MiniStatBadge label={t("模型总数")} value={`${stats.total}`} />
                <MiniStatBadge label={t("API 可用")} value={`${stats.apiEnabled}`} />
                <MiniStatBadge label={t("自定义模型")} value={`${stats.custom}`} />
                <MiniStatBadge label={t("本地覆写")} value={`${stats.edited}`} />
                {selectedSlugs.length > 0 ? (
                  <Badge variant="secondary" className="rounded-full px-3 py-1">
                    {t("已选 {count} 项", { count: selectedSlugs.length })}
                  </Badge>
                ) : null}
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <div className="flex h-10 items-center gap-2 rounded-xl border border-border/60 bg-background/35 px-3">
                  <Search className="h-4 w-4 text-muted-foreground" />
                  <Input
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                    placeholder={t("搜索 slug、显示名称或描述")}
                    className="h-full border-none bg-transparent px-0 shadow-none focus-visible:ring-0"
                  />
                </div>
                <Select value={filter} onValueChange={(value) => setFilter(value as ModelFilter)}>
                  <SelectTrigger className="h-10 w-full rounded-xl px-3">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t("全部模型")}</SelectItem>
                    <SelectItem value="api">{t("仅 API 可用")}</SelectItem>
                    <SelectItem value="custom">{t("仅自定义")}</SelectItem>
                    <SelectItem value="edited">{t("仅本地覆写")}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            {!isServiceReady ? (
              <div className="rounded-2xl border border-dashed border-border/70 bg-background/35 px-6 py-10 text-sm text-muted-foreground">
                {t("服务未连接，当前无法读取模型目录。")}
              </div>
            ) : isLoading ? (
              <div className="space-y-3">
                {Array.from({ length: 6 }).map((_, index) => (
                  <Skeleton key={`models-skeleton-${index}`} className="h-12 w-full rounded-xl" />
                ))}
              </div>
            ) : filteredModels.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-border/70 bg-background/35 px-6 py-10 text-sm text-muted-foreground">
                {t("没有匹配的模型。你可以调整筛选条件，或直接新增一个自定义模型。")}
              </div>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="w-12 text-center">
                        <Checkbox checked={allVisibleSelected} onCheckedChange={toggleSelectAllVisible} />
                      </TableHead>
                      <TableHead>Slug</TableHead>
                      <TableHead>{t("显示名称")}</TableHead>
                      <TableHead>{t("来源")}</TableHead>
                      <TableHead>{t("状态")}</TableHead>
                      <TableHead>{t("更新时间")}</TableHead>
                      <TableHead className="w-16 text-right">{t("操作")}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredModels.map((model) => (
                      <TableRow key={model.slug}>
                        <TableCell className="text-center">
                          <Checkbox
                            checked={selectedSlugs.includes(model.slug)}
                            onCheckedChange={() => toggleSelectSlug(model.slug)}
                          />
                        </TableCell>
                        <TableCell className="font-medium">{model.slug}</TableCell>
                        <TableCell>
                          <div className="space-y-1">
                            <div>{model.displayName}</div>
                            {model.description ? (
                              <div className="text-xs text-muted-foreground">{model.description}</div>
                            ) : null}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary">
                            {model.sourceKind === "custom" ? t("自定义") : t("远端")}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap gap-2">
                            {model.supportedInApi ? <Badge>{t("API")}</Badge> : null}
                            {model.userEdited ? (
                              <Badge variant="secondary">{t("已覆写")}</Badge>
                            ) : null}
                            {model.visibility ? (
                              <Badge variant="outline">{model.visibility}</Badge>
                            ) : null}
                          </div>
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          {model.updatedAt > 0 ? formatTsFromSeconds(model.updatedAt) : "-"}
                        </TableCell>
                        <TableCell className="text-right">
                          <DropdownMenu>
                            <DropdownMenuTrigger>
                              <Button variant="ghost" size="icon">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuItem
                                onClick={() => {
                                  setEditingSlug(model.slug);
                                  setModalOpen(true);
                                }}
                              >
                                <PencilLine className="mr-2 h-4 w-4" />
                                {t("编辑")}
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => openSingleDeleteDialog(model.slug)}>
                                <Trash2 className="mr-2 h-4 w-4" />
                                {t("删除")}
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <ModelCatalogModal
        open={modalOpen}
        onOpenChange={setModalOpen}
        model={editingModel}
        nextSortIndex={nextSortIndex}
        isSaving={isSaving}
        onSave={saveModel}
      />

      <ConfirmDialog
        open={deleteTargetCount > 0}
        onOpenChange={(open) => {
          if (!open) setDeleteSlugs([]);
        }}
        title={singleDeleteSlug ? t("删除模型") : t("批量删除模型")}
        description={
          singleDeleteSlug
            ? t("确定删除模型 `{slug}` 吗？", { slug: singleDeleteSlug })
            : t("确定删除已选中的 {count} 个模型吗？", { count: deleteTargetCount })
        }
        confirmText={t("删除")}
        confirmVariant="destructive"
        onConfirm={() => {
          if (deleteTargetCount <= 1 && singleDeleteSlug) {
            void deleteModel(singleDeleteSlug);
            setSelectedSlugs((current) => current.filter((slug) => slug !== singleDeleteSlug));
            return;
          }
          void deleteModels(deleteSlugs);
          setSelectedSlugs((current) => current.filter((slug) => !deleteSlugs.includes(slug)));
        }}
      />
    </>
  );
}
