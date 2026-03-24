"use client";

import { useEffect, useState } from "react";
import { 
  Dialog, 
  DialogContent, 
  DialogDescription, 
  DialogHeader, 
  DialogTitle,
  DialogFooter
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { 
  Select, 
  SelectContent, 
  SelectItem, 
  SelectTrigger, 
  SelectValue 
} from "@/components/ui/select";
import { accountClient } from "@/lib/api/account-client";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { toast } from "sonner";
import { useQueryClient, useQuery } from "@tanstack/react-query";
import { Key, Globe, Clipboard, ShieldCheck } from "lucide-react";
import { ApiKey } from "@/types";

const PROTOCOL_LABELS: Record<string, string> = {
  openai_compat: "OpenAI 兼容",
  azure_openai: "Azure OpenAI",
  anthropic_native: "Claude Code 兼容",
};

const REASONING_LABELS: Record<string, string> = {
  auto: "跟随请求",
  low: "低 (low)",
  medium: "中 (medium)",
  high: "高 (high)",
  xhigh: "极高 (xhigh)",
};

interface ApiKeyModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  apiKey?: ApiKey | null;
}

export function ApiKeyModal({ open, onOpenChange, apiKey }: ApiKeyModalProps) {
  const [name, setName] = useState("");
  const [protocolType, setProtocolType] = useState("openai_compat");
  const [modelSlug, setModelSlug] = useState("");
  const [reasoningEffort, setReasoningEffort] = useState("");
  const [upstreamBaseUrl, setUpstreamBaseUrl] = useState("");
  const [azureEndpoint, setAzureEndpoint] = useState("");
  const [azureApiKey, setAzureApiKey] = useState("");
  const [expiresAtInput, setExpiresAtInput] = useState("");
  const [rpmInput, setRpmInput] = useState("");
  const [tpmInput, setTpmInput] = useState("");
  const [dailyLimitInput, setDailyLimitInput] = useState("");
  const [allowedModelsSelection, setAllowedModelsSelection] = useState<string[]>([]);
  const [fallbackModelsText, setFallbackModelsText] = useState("");
  const [responseCacheEnabled, setResponseCacheEnabled] = useState(false);
  const [generatedKey, setGeneratedKey] = useState("");
  
  const [isLoading, setIsLoading] = useState(false);
  const queryClient = useQueryClient();

  const { data: models } = useQuery({
    queryKey: ["apikey-models"],
    queryFn: () => accountClient.listModels(false),
    enabled: open,
  });
  const { data: rateLimitConfig } = useQuery({
    queryKey: ["apikey-rate-limit", apiKey?.id],
    queryFn: () => accountClient.getApiKeyRateLimit(String(apiKey?.id || "")),
    enabled: open && Boolean(apiKey?.id),
    retry: 1,
  });
  const { data: modelFallbackConfig } = useQuery({
    queryKey: ["apikey-model-fallback", apiKey?.id],
    queryFn: () => accountClient.getApiKeyModelFallback(String(apiKey?.id || "")),
    enabled: open && Boolean(apiKey?.id),
    retry: 1,
  });
  const { data: allowedModelsConfig } = useQuery({
    queryKey: ["apikey-allowed-models", apiKey?.id],
    queryFn: () => accountClient.getApiKeyAllowedModels(String(apiKey?.id || "")),
    enabled: open && Boolean(apiKey?.id),
    retry: 1,
  });
  const { data: responseCacheConfig } = useQuery({
    queryKey: ["apikey-response-cache", apiKey?.id],
    queryFn: () => accountClient.getApiKeyResponseCache(String(apiKey?.id || "")),
    enabled: open && Boolean(apiKey?.id),
    retry: 1,
  });

  const modelLabelMap = Object.fromEntries(
    (models || []).map((model) => [model.slug, model.displayName])
  );
  const allowedModelOptions = mergeAllowedModelOptions(models || [], allowedModelsSelection);
  const effectiveAllowedModels = normalizeAllowedModelsSelection(
    allowedModelsSelection,
    modelSlug
  );

  useEffect(() => {
    if (!open) return;

    if (!apiKey) {
      setName("");
      setProtocolType("openai_compat");
      setModelSlug("");
      setReasoningEffort("");
      setUpstreamBaseUrl("");
      setAzureEndpoint("");
      setAzureApiKey("");
      setExpiresAtInput("");
      setRpmInput("");
      setTpmInput("");
      setDailyLimitInput("");
      setAllowedModelsSelection([]);
      setFallbackModelsText("");
      setResponseCacheEnabled(false);
      setGeneratedKey("");
      return;
    }

    setName(apiKey.name || "");
    setProtocolType(apiKey.protocol || "openai_compat");
    setModelSlug(apiKey.modelSlug || "");
    setReasoningEffort(apiKey.reasoningEffort || "");
    setGeneratedKey("");

    if (apiKey.protocol === "azure_openai") {
      setAzureEndpoint(apiKey.upstreamBaseUrl || "");
      try {
        const headers = apiKey.staticHeadersJson
          ? JSON.parse(apiKey.staticHeadersJson)
          : {};
        setAzureApiKey(typeof headers["api-key"] === "string" ? headers["api-key"] : "");
      } catch {
        setAzureApiKey("");
      }
      setUpstreamBaseUrl("");
    } else {
      setUpstreamBaseUrl(apiKey.upstreamBaseUrl || "");
      setAzureEndpoint("");
      setAzureApiKey("");
    }
    setExpiresAtInput(apiKey.expiresAt ? toDateTimeLocalValue(apiKey.expiresAt) : "");
    setAllowedModelsSelection(allowedModelsConfig?.allowedModels || []);
    setFallbackModelsText(
      formatModelChainInput(modelFallbackConfig?.modelChain || [])
    );
    setResponseCacheEnabled(responseCacheConfig?.enabled === true);
  }, [
    allowedModelsConfig?.allowedModels,
    apiKey,
    modelFallbackConfig?.modelChain,
    open,
    responseCacheConfig?.enabled,
  ]);

  useEffect(() => {
    if (!open || !apiKey?.id) return;
    setRpmInput(formatLimitInput(rateLimitConfig?.rpm));
    setTpmInput(formatLimitInput(rateLimitConfig?.tpm));
    setDailyLimitInput(formatLimitInput(rateLimitConfig?.dailyLimit));
  }, [apiKey?.id, open, rateLimitConfig]);

  const handleSave = async () => {
    setIsLoading(true);
    try {
      const staticHeaders: Record<string, string> = {};
      if (protocolType === "azure_openai" && azureApiKey) {
        staticHeaders["api-key"] = azureApiKey;
      }

      const params = {
        name: name || null,
        modelSlug: !modelSlug || modelSlug === "auto" ? null : modelSlug,
        reasoningEffort:
          !reasoningEffort || reasoningEffort === "auto" ? null : reasoningEffort,
        protocolType,
        upstreamBaseUrl: protocolType === "azure_openai" ? azureEndpoint : (upstreamBaseUrl || null),
        staticHeadersJson: Object.keys(staticHeaders).length > 0 ? JSON.stringify(staticHeaders) : null,
        expiresAt: expiresAtInput ? parseDateTimeLocalValue(expiresAtInput) : null,
      };
      const rateLimitParams = {
        rpm: parseLimitInput(rpmInput),
        tpm: parseLimitInput(tpmInput),
        dailyLimit: parseLimitInput(dailyLimitInput),
      };
      const modelFallbackParams = {
        modelChain: parseModelChainInput(fallbackModelsText),
      };
      const allowedModelsParams = {
        allowedModels: effectiveAllowedModels,
      };

      if (apiKey?.id) {
        await accountClient.updateApiKey(apiKey.id, params);
        await accountClient.setApiKeyRateLimit(apiKey.id, rateLimitParams);
        await accountClient.setApiKeyModelFallback(apiKey.id, modelFallbackParams);
        await accountClient.setApiKeyAllowedModels(apiKey.id, allowedModelsParams);
        await accountClient.setApiKeyResponseCache(apiKey.id, {
          enabled: responseCacheEnabled,
        });
        toast.success("密钥配置已更新");
      } else {
        const result = await accountClient.createApiKey(params);
        setGeneratedKey(result.key);
        await accountClient.setApiKeyRateLimit(result.id, rateLimitParams);
        await accountClient.setApiKeyModelFallback(result.id, modelFallbackParams);
        await accountClient.setApiKeyAllowedModels(result.id, allowedModelsParams);
        await accountClient.setApiKeyResponseCache(result.id, {
          enabled: responseCacheEnabled,
        });
        toast.success("平台密钥已创建");
      }
      
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["apikeys"] }),
        queryClient.invalidateQueries({ queryKey: ["apikey-rate-limit"] }),
        queryClient.invalidateQueries({ queryKey: ["apikey-model-fallback"] }),
        queryClient.invalidateQueries({ queryKey: ["apikey-allowed-models"] }),
        queryClient.invalidateQueries({ queryKey: ["apikey-response-cache"] }),
        queryClient.invalidateQueries({ queryKey: ["apikey-models"] }),
        queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
      ]);
      if (apiKey?.id) onOpenChange(false);
    } catch (err: unknown) {
      toast.error(`操作失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const copyKey = () => {
    copyTextToClipboard(generatedKey)
      .then(() => {
        toast.success("密钥已复制");
      })
      .catch((error: unknown) => {
        toast.error(error instanceof Error ? error.message : "复制失败");
      });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[550px] glass-card border-none">
        <DialogHeader>
          <div className="flex items-center gap-3 mb-2">
            <div className="p-2 rounded-full bg-primary/10">
              <Key className="h-5 w-5 text-primary" />
            </div>
            <DialogTitle>{apiKey?.id ? "编辑平台密钥" : "创建平台密钥"}</DialogTitle>
          </div>
          <DialogDescription>
            配置网关访问凭据，您可以绑定特定模型、推理等级或自定义上游。
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-5 py-4">
          <div className="grid gap-2">
            <Label htmlFor="name">密钥名称 (可选)</Label>
            <Input 
              id="name"
              placeholder="例如：主机房 / 测试"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label>协议类型</Label>
              <Select value={protocolType} onValueChange={(val) => val && setProtocolType(val)}>
                <SelectTrigger>
                  <SelectValue>
                    {(value) => PROTOCOL_LABELS[String(value || "")] || "OpenAI 兼容"}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="openai_compat">OpenAI 兼容</SelectItem>
                  <SelectItem value="azure_openai">Azure OpenAI</SelectItem>
                  <SelectItem value="anthropic_native">Claude Code 兼容</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-2">
              <Label>绑定模型 (可选)</Label>
              <Select value={modelSlug} onValueChange={(val) => val && setModelSlug(val)}>
                <SelectTrigger>
                  <SelectValue placeholder="跟随请求">
                    {(value) => {
                      const nextValue = String(value || "").trim();
                      if (!nextValue || nextValue === "auto") return "跟随请求";
                      return modelLabelMap[nextValue] || nextValue;
                    }}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="auto">跟随请求</SelectItem>
                  {models?.map((model) => (
                    <SelectItem key={model.slug} value={model.slug}>
                      {model.displayName}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-[11px] text-muted-foreground">
                选择“跟随请求”时，会使用请求体里的实际模型；请求日志展示的是最终生效模型。
              </p>
            </div>
          </div>

          <div className="grid gap-2">
            <Label>推理等级 (可选)</Label>
            <Select value={reasoningEffort} onValueChange={(val) => val && setReasoningEffort(val)}>
              <SelectTrigger>
                <SelectValue placeholder="跟随请求等级">
                  {(value) => {
                    const nextValue = String(value || "").trim();
                    if (!nextValue) return "跟随请求等级";
                    return REASONING_LABELS[nextValue] || nextValue;
                  }}
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="auto">跟随请求</SelectItem>
                <SelectItem value="low">低 (low)</SelectItem>
                <SelectItem value="medium">中 (medium)</SelectItem>
                <SelectItem value="high">高 (high)</SelectItem>
                <SelectItem value="xhigh">极高 (xhigh)</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {!apiKey?.id ? (
            <div className="grid gap-2">
              <Label htmlFor="expires-at">过期时间 (可选)</Label>
              <Input
                id="expires-at"
                type="datetime-local"
                value={expiresAtInput}
                onChange={(e) => setExpiresAtInput(e.target.value)}
              />
              <p className="text-[11px] text-muted-foreground">
                不填写则永不过期；到期后网关会返回 401，并可在列表页执行续期。
              </p>
            </div>
          ) : (
            <p className="text-[11px] text-muted-foreground">
              当前过期时间：
              {expiresAtInput ? " 已设置，续期请在列表页操作。" : " 永不过期。"}
            </p>
          )}

          <div className="grid gap-4 rounded-xl border border-primary/10 bg-accent/20 p-4">
            <div className="space-y-1">
              <Label>请求限流 (可选)</Label>
              <p className="text-[11px] text-muted-foreground">
                留空表示不限流；配置后会在鉴权通过后、路由前直接返回 429。
              </p>
            </div>
            <div className="grid grid-cols-3 gap-3">
              <div className="grid gap-2">
                <Label htmlFor="rpm-limit" className="text-xs">RPM</Label>
                <Input
                  id="rpm-limit"
                  type="number"
                  min="1"
                  placeholder="例如 10"
                  value={rpmInput}
                  onChange={(e) => setRpmInput(e.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="tpm-limit" className="text-xs">TPM</Label>
                <Input
                  id="tpm-limit"
                  type="number"
                  min="1"
                  placeholder="例如 1000"
                  value={tpmInput}
                  onChange={(e) => setTpmInput(e.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="daily-limit" className="text-xs">日上限</Label>
                <Input
                  id="daily-limit"
                  type="number"
                  min="1"
                  placeholder="例如 50"
                  value={dailyLimitInput}
                  onChange={(e) => setDailyLimitInput(e.target.value)}
                />
              </div>
            </div>
          </div>

          <div className="grid gap-4 rounded-xl border border-primary/10 bg-accent/20 p-4">
            <div className="space-y-1">
              <Label>模型白名单 (可选)</Label>
              <p className="text-[11px] text-muted-foreground">
                不选择表示允许所有模型；启用后，仅允许选中的模型，降级链中的未授权模型会被自动跳过。
              </p>
            </div>
            <div className="max-h-56 space-y-2 overflow-y-auto rounded-lg border border-primary/10 bg-background/60 p-3">
              {allowedModelOptions.length > 0 ? (
                allowedModelOptions.map((model) => {
                  const checked = effectiveAllowedModels.includes(model.slug);
                  return (
                    <label
                      key={model.slug}
                      className="flex cursor-pointer items-start gap-3 rounded-lg border border-transparent px-2 py-2 transition-colors hover:border-primary/10 hover:bg-accent/40"
                    >
                      <Checkbox
                        checked={checked}
                        onCheckedChange={(nextChecked) =>
                          setAllowedModelsSelection((current) =>
                            updateAllowedModelsSelection(
                              current,
                              model.slug,
                              nextChecked === true
                            )
                          )
                        }
                        aria-label={`切换模型白名单 ${model.displayName}`}
                      />
                      <div className="min-w-0 space-y-1">
                        <div className="text-sm font-medium leading-none">
                          {model.displayName}
                        </div>
                        <div className="font-mono text-[11px] text-muted-foreground">
                          {model.slug}
                        </div>
                      </div>
                    </label>
                  );
                })
              ) : (
                <p className="text-[11px] text-muted-foreground">
                  当前没有可选模型；请先在列表页刷新模型元数据后再配置白名单。
                </p>
              )}
            </div>
            <p className="text-[11px] text-muted-foreground">
              当前白名单共 {effectiveAllowedModels.length} 个模型。
              {modelSlug && modelSlug !== "auto"
                ? " 已绑定的默认模型会在保存时自动加入白名单。"
                : ""}
            </p>
          </div>

          <div className="grid gap-4 rounded-xl border border-primary/10 bg-accent/20 p-4">
            <div className="space-y-1">
              <Label htmlFor="model-fallback-chain">模型降级链 (可选)</Label>
              <p className="text-[11px] text-muted-foreground">
                按顺序逐行填写备选模型；当当前模型在所有账号上都失败后，会自动尝试下一项。
              </p>
            </div>
            <Textarea
              id="model-fallback-chain"
              placeholder={"o3\no4-mini\ngpt-4o"}
              value={fallbackModelsText}
              onChange={(e) => setFallbackModelsText(e.target.value)}
              className="min-h-[108px] font-mono text-xs"
            />
          </div>

          <div className="flex items-start justify-between gap-4 rounded-xl border border-primary/10 bg-accent/20 p-4">
            <div className="space-y-1">
              <Label htmlFor="response-cache-enabled">响应缓存</Label>
              <p className="text-[11px] text-muted-foreground">
                仅对当前 API Key 生效；启用后，非流式相同请求可命中网关响应缓存。
              </p>
            </div>
            <Switch
              checked={responseCacheEnabled}
              onCheckedChange={setResponseCacheEnabled}
              aria-label="切换 API Key 响应缓存"
            />
          </div>

          {protocolType === "azure_openai" ? (
            <div className="grid gap-4 p-4 rounded-xl bg-accent/20 border border-primary/10">
               <div className="grid gap-2">
                <Label className="text-xs">Azure 接入地址</Label>
                <Input 
                  placeholder="https://your-resource.openai.azure.com"
                  value={azureEndpoint}
                  onChange={(e) => setAzureEndpoint(e.target.value)}
                  className="h-9 font-mono text-xs"
                />
              </div>
              <div className="grid gap-2">
                <Label className="text-xs">Azure 接口密钥</Label>
                <Input 
                  type="password"
                  placeholder="your-azure-key"
                  value={azureApiKey}
                  onChange={(e) => setAzureApiKey(e.target.value)}
                  className="h-9 font-mono text-xs"
                />
              </div>
            </div>
          ) : (
            <div className="grid gap-2">
              <Label className="flex items-center gap-2">
                <Globe className="h-3.5 w-3.5" /> 自定义上游 Base URL (可选)
              </Label>
              <Input 
                placeholder="https://api.openai.com/v1"
                value={upstreamBaseUrl}
                onChange={(e) => setUpstreamBaseUrl(e.target.value)}
              />
            </div>
          )}

          {generatedKey && (
            <div className="space-y-2 pt-4 border-t">
              <Label className="text-xs text-primary flex items-center gap-1.5">
                <ShieldCheck className="h-3.5 w-3.5" /> 平台密钥已生成
              </Label>
              <div className="flex gap-2">
                <Input value={generatedKey} readOnly className="font-mono text-sm bg-primary/5" />
                <Button variant="outline" onClick={copyKey}>
                  <Clipboard className="h-4 w-4" />
                </Button>
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {generatedKey ? "关闭" : "取消"}
          </Button>
          {!generatedKey && (
            <Button onClick={handleSave} disabled={isLoading}>
              {isLoading ? "保存中..." : "完成"}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function parseDateTimeLocalValue(value: string): number | null {
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) {
    return null;
  }
  return Math.floor(timestamp / 1000);
}

function toDateTimeLocalValue(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return "";
  }
  const offset = date.getTimezoneOffset();
  const localDate = new Date(date.getTime() - offset * 60_000);
  return localDate.toISOString().slice(0, 16);
}

function parseLimitInput(value: string): number | null {
  const normalized = value.trim();
  if (!normalized) {
    return null;
  }
  const parsed = Number(normalized);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return null;
  }
  return Math.floor(parsed);
}

function formatLimitInput(value: number | null | undefined): string {
  return value && value > 0 ? String(Math.floor(value)) : "";
}

function parseModelChainInput(value: string): string[] {
  const seen = new Set<string>();
  const models: string[] = [];
  for (const item of value.split(/\r?\n|,/)) {
    const normalized = item.trim();
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    models.push(normalized);
  }
  return models;
}

function formatModelChainInput(value: string[]): string {
  return value.filter((item) => item.trim().length > 0).join("\n");
}

function updateAllowedModelsSelection(
  current: string[],
  modelSlug: string,
  checked: boolean
): string[] {
  if (checked) {
    return normalizeAllowedModelsSelection([...current, modelSlug]);
  }
  return current.filter((item) => item !== modelSlug);
}

function normalizeAllowedModelsSelection(
  value: string[],
  boundModelSlug?: string
): string[] {
  const seen = new Set<string>();
  const models: string[] = [];
  const candidates = [...value];
  const normalizedBoundModel = (boundModelSlug || "").trim();
  if (normalizedBoundModel && normalizedBoundModel !== "auto") {
    candidates.unshift(normalizedBoundModel);
  }
  for (const item of candidates) {
    const normalized = item.trim();
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    models.push(normalized);
  }
  return models;
}

function mergeAllowedModelOptions(
  models: { slug: string; displayName: string }[],
  selected: string[]
): { slug: string; displayName: string }[] {
  const merged = [...models];
  const existing = new Set(models.map((item) => item.slug));
  for (const slug of selected) {
    const normalized = slug.trim();
    if (!normalized || existing.has(normalized)) continue;
    existing.add(normalized);
    merged.push({
      slug: normalized,
      displayName: normalized,
    });
  }
  return merged;
}
