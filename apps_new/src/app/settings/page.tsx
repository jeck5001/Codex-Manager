"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { appClient } from "@/lib/api/app-client";
import { useAppStore } from "@/lib/store/useAppStore";
import { AppSettings } from "@/types";
import { useTheme } from "next-themes";
import { 
  Tabs, 
  TabsContent, 
  TabsList, 
  TabsTrigger 
} from "@/components/ui/tabs";
import { 
  Card, 
  CardContent, 
  CardDescription, 
  CardHeader, 
  CardTitle 
} from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { 
  Select, 
  SelectContent, 
  SelectItem, 
  SelectTrigger, 
  SelectValue 
} from "@/components/ui/select";
import { 
  Settings as SettingsIcon, 
  Globe, 
  Cpu, 
  Variable,
  Save,
  Search,
  RotateCcw,
  Info,
  Palette,
  Check,
  AppWindow
} from "lucide-react";
import { toast } from "sonner";
import { useState, useMemo, useEffect } from "react";
import { cn } from "@/lib/utils";

// Descriptions for environment variables
const ENV_DESCRIPTION_MAP: Record<string, string> = {
  CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS: "控制单次上游请求允许持续的最长时间，单位毫秒；超过后会主动结束请求并返回超时错误。",
  CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS: "控制流式上游请求允许持续的最长时间，单位毫秒；填 0 可关闭流式超时上限，适合长时间持续输出的 SSE/流式连接。",
  CODEXMANAGER_SSE_KEEPALIVE_INTERVAL_MS: "控制向下游补发 SSE keep-alive 帧的间隔，单位毫秒；上游长时间安静时可避免客户端误判连接中断。",
  CODEXMANAGER_UPSTREAM_CONNECT_TIMEOUT_SECS: "控制连接上游服务器时的超时时间，单位秒；主要影响握手和网络建立阶段。",
  CODEXMANAGER_UPSTREAM_BASE_URL: "控制默认上游地址；修改后，网关会把请求转发到新的目标地址。",
};

const THEMES = [
  { id: "tech", name: "企业蓝", color: "#2563eb" },
  { id: "dark", name: "极夜黑", color: "#09090b" },
  { id: "dark-one", name: "深邃黑", color: "#282c34" },
  { id: "business", name: "事务金", color: "#c28100" },
  { id: "mint", name: "薄荷绿", color: "#059669" },
  { id: "sunset", name: "晚霞橙", color: "#ea580c" },
  { id: "grape", name: "葡萄灰紫", color: "#7c3aed" },
  { id: "ocean", name: "海湾青", color: "#0284c7" },
  { id: "forest", name: "松林绿", color: "#166534" },
  { id: "rose", name: "玫瑰粉", color: "#db2777" },
  { id: "slate", name: "石板灰", color: "#475569" },
  { id: "aurora", name: "极光青", color: "#0d9488" },
];

export default function SettingsPage() {
  const { setAppSettings: setStoreSettings } = useAppStore();
  const { theme, setTheme } = useTheme();
  const queryClient = useQueryClient();
  const [envSearch, setEnvSearch] = useState("");
  const [selectedEnvKey, setSelectedEnvKey] = useState<string | null>(null);
  const [envEditValue, setEnvEditValue] = useState("");

  // Fetches full snapshot
  const { data: snapshot, isLoading } = useQuery({
    queryKey: ["app-settings-snapshot"],
    queryFn: () => appClient.getSettings(),
  });

  // Mutations
  const updateSettings = useMutation({
    mutationFn: (patch: any) => appClient.setSettings(patch),
    onSuccess: (newSnapshot) => {
      const typedSnapshot = newSnapshot as AppSettings;
      queryClient.setQueryData(["app-settings-snapshot"], typedSnapshot);
      setStoreSettings(typedSnapshot);
      
      // Sync transparency class
      if (typedSnapshot.lowTransparency) {
        document.body.classList.add("low-transparency");
      } else {
        document.body.classList.remove("low-transparency");
      }
      
      toast.success("设置已更新");
    },
    onError: (err: any) => toast.error(`更新失败: ${err.message}`),
  });

  // Handle theme change with persistence
  const handleThemeChange = (newTheme: string) => {
    setTheme(newTheme);
    updateSettings.mutate({ theme: newTheme });
  };

  // Sync initial theme from snapshot if needed
  useEffect(() => {
    if (snapshot?.theme && snapshot.theme !== theme) {
      setTheme(snapshot.theme);
    }
  }, [snapshot?.theme]);

  // Derived Environment Overrides
  const envCatalog = useMemo(() => snapshot?.envOverrideCatalog || [], [snapshot]);
  const envOverrides = useMemo(() => snapshot?.envOverrides || {}, [snapshot]);
  
  const filteredEnvCatalog = useMemo(() => {
    if (!envSearch) return envCatalog;
    const s = envSearch.toLowerCase();
    return envCatalog.filter((item: any) => 
      item.key.toLowerCase().includes(s) || item.label.toLowerCase().includes(s)
    );
  }, [envCatalog, envSearch]);

  const selectedEnvItem = useMemo(() => 
    envCatalog.find((item: any) => item.key === selectedEnvKey),
    [envCatalog, selectedEnvKey]
  );

  useEffect(() => {
    if (selectedEnvKey) {
      setEnvEditValue(envOverrides[selectedEnvKey] ?? selectedEnvItem?.defaultValue ?? "");
    }
  }, [selectedEnvKey, envOverrides, selectedEnvItem]);

  const handleSaveEnv = () => {
    if (!selectedEnvKey) return;
    const newOverrides = { ...envOverrides, [selectedEnvKey]: envEditValue };
    updateSettings.mutate({ envOverrides: newOverrides });
  };

  const handleResetEnv = () => {
    if (!selectedEnvKey) return;
    const { [selectedEnvKey]: _, ...rest } = envOverrides;
    updateSettings.mutate({ envOverrides: rest });
  };

  if (isLoading) return <div className="flex h-64 items-center justify-center text-muted-foreground">加载配置中...</div>;

  return (
    <div className="space-y-6 animate-in fade-in duration-500">
      <div>
        <h2 className="text-xl font-bold tracking-tight">系统设置</h2>
        <p className="text-sm text-muted-foreground mt-1">管理应用行为、网关策略及后台任务</p>
      </div>

      <Tabs defaultValue="general" className="w-full">
        <TabsList className="bg-muted/50 p-1 rounded-xl h-11 w-full lg:w-fit overflow-x-auto no-scrollbar justify-start mb-6">
          <TabsTrigger value="general" className="gap-2 px-5 shrink-0">
            <SettingsIcon className="h-4 w-4" /> 通用
          </TabsTrigger>
          <TabsTrigger value="appearance" className="gap-2 px-5 shrink-0">
            <Palette className="h-4 w-4" /> 外观
          </TabsTrigger>
          <TabsTrigger value="gateway" className="gap-2 px-5 shrink-0">
            <Globe className="h-4 w-4" /> 网关
          </TabsTrigger>
          <TabsTrigger value="tasks" className="gap-2 px-5 shrink-0">
            <Cpu className="h-4 w-4" /> 任务
          </TabsTrigger>
          <TabsTrigger value="env" className="gap-2 px-5 shrink-0">
            <Variable className="h-4 w-4" /> 环境
          </TabsTrigger>
        </TabsList>

        <TabsContent value="general" className="space-y-6">
          <Card className="border-none bg-card/50 shadow-md backdrop-blur-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <AppWindow className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">基础设置</CardTitle>
              </div>
              <CardDescription>控制应用启动和窗口行为</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>自动检查更新</Label>
                  <p className="text-xs text-muted-foreground">启动时自动检测新版本</p>
                </div>
                <Switch 
                  checked={snapshot?.updateAutoCheck} 
                  onCheckedChange={(val) => updateSettings.mutate({ updateAutoCheck: val })}
                />
              </div>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>关闭时最小化到托盘</Label>
                  <p className="text-xs text-muted-foreground">点击关闭按钮不会直接退出程序</p>
                </div>
                <Switch 
                  checked={snapshot?.closeToTrayOnClose} 
                  onCheckedChange={(val) => updateSettings.mutate({ closeToTrayOnClose: val })}
                />
              </div>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>视觉性能模式</Label>
                  <p className="text-xs text-muted-foreground">关闭毛玻璃等特效以提升低配电脑性能</p>
                </div>
                <Switch 
                  checked={snapshot?.lowTransparency} 
                  onCheckedChange={(val) => updateSettings.mutate({ lowTransparency: val })}
                />
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="appearance" className="space-y-6">
          <Card className="border-none bg-card/50 shadow-md backdrop-blur-md">
            <CardHeader>
              <div className="flex items-center gap-2">
                <Palette className="h-4 w-4 text-primary" />
                <CardTitle className="text-base">界面主题</CardTitle>
              </div>
              <CardDescription>选择您喜爱的配色方案，适配不同工作心情</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-12 gap-4">
                {THEMES.map((t) => (
                  <button
                    key={t.id}
                    onClick={() => handleThemeChange(t.id)}
                    className={cn(
                      "group relative flex flex-col items-center gap-2.5 p-4 rounded-2xl border transition-all duration-300 hover:scale-105",
                      theme === t.id 
                        ? "border-primary bg-primary/10 shadow-lg ring-1 ring-primary" 
                        : "border-transparent bg-muted/20 hover:bg-accent/40"
                    )}
                  >
                    <div 
                      className="h-10 w-10 rounded-full shadow-md border-2 border-white/20" 
                      style={{ backgroundColor: t.color }}
                    />
                    <span className={cn(
                      "text-[10px] font-semibold whitespace-nowrap transition-colors",
                      theme === t.id ? "text-primary" : "text-muted-foreground group-hover:text-foreground"
                    )}>
                      {t.name}
                    </span>
                    {theme === t.id && (
                      <div className="absolute top-2 right-2 bg-primary text-primary-foreground rounded-full p-0.5 shadow-sm">
                        <Check className="h-2.5 w-2.5" />
                      </div>
                    )}
                  </button>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="gateway" className="space-y-4">
           <Card className="border-none bg-card/50 shadow-md backdrop-blur-md">
            <CardHeader>
              <CardTitle className="text-base">网关策略</CardTitle>
              <CardDescription>配置账号选路和请求头处理方式</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
               <div className="grid gap-2">
                <Label>账号选路策略</Label>
                <Select 
                  value={snapshot?.routeStrategy || "ordered"} 
                  onValueChange={(val) => updateSettings.mutate({ routeStrategy: val })}
                >
                  <SelectTrigger className="w-full md:w-[300px]">
                    <SelectValue placeholder="选择策略" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ordered">顺序优先 (Ordered)</SelectItem>
                    <SelectItem value="balanced">均衡轮询 (Balanced)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-[10px] text-muted-foreground">顺序优先：始终尝试第一个可用账号；均衡轮询：在可用账号间平均分配。</p>
              </div>

              <div className="flex items-center justify-between border-t pt-6">
                <div className="space-y-0.5">
                  <Label>请求头收敛策略</Label>
                  <p className="text-xs text-muted-foreground">移除高风险会话头，降低 Cloudflare 验证命中率</p>
                </div>
                <Switch 
                  checked={snapshot?.cpaNoCookieHeaderModeEnabled} 
                  onCheckedChange={(val) => updateSettings.mutate({ cpaNoCookieHeaderModeEnabled: val })}
                />
              </div>

              <div className="grid gap-2 pt-2">
                <Label>上游代理 (Proxy)</Label>
                <div className="flex gap-2">
                  <Input 
                    placeholder="http://127.0.0.1:7890" 
                    className="max-w-md font-mono h-10"
                    defaultValue={snapshot?.upstreamProxyUrl}
                    onBlur={(e) => {
                      if (snapshot && e.target.value !== snapshot.upstreamProxyUrl) {
                        updateSettings.mutate({ upstreamProxyUrl: e.target.value });
                      }
                    }}
                  />
                </div>
                <p className="text-[10px] text-muted-foreground">支持 http/https/socks5，留空表示直连。</p>
              </div>

              <div className="grid grid-cols-2 gap-4 border-t pt-6">
                <div className="grid gap-2">
                  <Label>SSE 保活间隔 (ms)</Label>
                  <Input 
                    type="number"
                    defaultValue={snapshot?.sseKeepaliveIntervalMs}
                    onBlur={(e) => updateSettings.mutate({ sseKeepaliveIntervalMs: parseInt(e.target.value) })}
                  />
                </div>
                <div className="grid gap-2">
                  <Label>上游流式超时 (ms)</Label>
                  <Input 
                    type="number"
                    defaultValue={snapshot?.upstreamStreamTimeoutMs}
                    onBlur={(e) => updateSettings.mutate({ upstreamStreamTimeoutMs: parseInt(e.target.value) })}
                  />
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="tasks" className="space-y-4">
           <Card className="border-none bg-card/50 shadow-md backdrop-blur-md">
            <CardHeader>
              <CardTitle className="text-base">后台任务线程</CardTitle>
              <CardDescription>管理自动轮询和保活任务</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {[
                { label: "用量轮询线程", enabledKey: "usagePollingEnabled", intervalKey: "usagePollIntervalSecs" },
                { label: "网关保活线程", enabledKey: "gatewayKeepaliveEnabled", intervalKey: "gatewayKeepaliveIntervalSecs" },
                { label: "令牌刷新轮询", enabledKey: "tokenRefreshPollingEnabled", intervalKey: "tokenRefreshPollIntervalSecs" },
              ].map((task) => (
                <div key={task.enabledKey} className="flex items-center justify-between gap-4 p-3 rounded-lg bg-accent/20">
                  <div className="flex items-center gap-3">
                    <Switch 
                      checked={snapshot?.backgroundTasks?.[task.enabledKey]} 
                      onCheckedChange={(val) => {
                        if (snapshot) {
                          updateSettings.mutate({ 
                            backgroundTasks: { ...snapshot.backgroundTasks, [task.enabledKey]: val } 
                          });
                        }
                      }}
                    />
                    <Label>{task.label}</Label>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground">间隔(秒)</span>
                    <Input 
                      className="w-20 h-8" 
                      type="number"
                      defaultValue={snapshot?.backgroundTasks?.[task.intervalKey]}
                      onBlur={(e) => {
                        if (snapshot) {
                          updateSettings.mutate({ 
                            backgroundTasks: { ...snapshot.backgroundTasks, [task.intervalKey]: parseInt(e.target.value) } 
                          });
                        }
                      }}
                    />
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card className="border-none bg-card/50 shadow-md backdrop-blur-md">
            <CardHeader>
              <CardTitle className="text-base">Worker 并发参数</CardTitle>
              <CardDescription>调整执行单元并发规模（重启后生效）</CardDescription>
            </CardHeader>
            <CardContent className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {[
                { label: "用量刷新并发", key: "usageRefreshWorkers" },
                { label: "HTTP 因子", key: "httpWorkerFactor" },
                { label: "HTTP 最小并发", key: "httpWorkerMin" },
                { label: "流式因子", key: "httpStreamWorkerFactor" },
                { label: "流式最小并发", key: "httpStreamWorkerMin" },
              ].map((worker) => (
                <div key={worker.key} className="grid gap-1.5">
                  <Label className="text-xs">{worker.label}</Label>
                  <Input 
                    type="number" 
                    className="h-9"
                    defaultValue={snapshot?.backgroundTasks?.[worker.key]}
                    onBlur={(e) => {
                      if (snapshot) {
                        updateSettings.mutate({ 
                          backgroundTasks: { ...snapshot.backgroundTasks, [worker.key]: parseInt(e.target.value) } 
                        });
                      }
                    }}
                  />
                </div>
              ))}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="env" className="space-y-4">
          <div className="grid md:grid-cols-[300px_1fr] gap-6">
            <Card className="border-none bg-card/50 shadow-md flex flex-col h-[500px]">
              <CardHeader className="pb-3">
                <div className="relative">
                  <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                  <Input 
                    placeholder="搜索变量..." 
                    className="pl-9 h-9" 
                    value={envSearch}
                    onChange={(e) => setEnvSearch(e.target.value)}
                  />
                </div>
              </CardHeader>
              <CardContent className="flex-1 overflow-y-auto p-2">
                <div className="space-y-1">
                  {filteredEnvCatalog.map((item: any) => (
                    <button
                      key={item.key}
                      onClick={() => setSelectedEnvKey(item.key)}
                      className={cn(
                        "w-full text-left px-3 py-2 rounded-md text-sm transition-colors",
                        selectedEnvKey === item.key ? "bg-primary text-primary-foreground" : "hover:bg-accent"
                      )}
                    >
                      <div className="font-medium truncate">{item.label}</div>
                      <code className="text-[10px] opacity-70 truncate block">{item.key}</code>
                    </button>
                  ))}
                </div>
              </CardContent>
            </Card>

            <Card className="border-none bg-card/50 shadow-md min-h-[500px]">
              {selectedEnvKey ? (
                <>
                  <CardHeader>
                    <div className="flex flex-col gap-1">
                      <CardTitle className="text-lg">{selectedEnvItem?.label}</CardTitle>
                      <code className="text-xs text-primary bg-primary/10 px-2 py-0.5 rounded w-fit">{selectedEnvKey}</code>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-6">
                    <div className="rounded-lg bg-accent/30 p-4 text-sm text-muted-foreground border leading-relaxed">
                      <Info className="h-4 w-4 inline-block mr-2 text-primary" />
                      {ENV_DESCRIPTION_MAP[selectedEnvKey] || `${selectedEnvItem?.label} 对应环境变量，修改后会应用到相关模块。`}
                    </div>

                    <div className="space-y-2">
                      <Label>当前值</Label>
                      <Input 
                        value={envEditValue}
                        onChange={(e) => setEnvEditValue(e.target.value)}
                        className="font-mono h-11"
                        placeholder="输入变量值"
                      />
                      <p className="text-[10px] text-muted-foreground">
                        默认值: <span className="font-mono italic">{selectedEnvItem?.defaultValue || "空"}</span>
                      </p>
                    </div>

                    <div className="flex gap-3 pt-4 border-t">
                      <Button onClick={handleSaveEnv} className="gap-2">
                        <Save className="h-4 w-4" /> 保存修改
                      </Button>
                      <Button variant="outline" onClick={handleResetEnv} className="gap-2">
                        <RotateCcw className="h-4 w-4" /> 恢复默认
                      </Button>
                    </div>
                  </CardContent>
                </>
              ) : (
                <CardContent className="h-full flex flex-col items-center justify-center text-muted-foreground gap-4">
                  <div className="p-4 rounded-full bg-accent/30">
                    <Variable className="h-12 w-12 opacity-20" />
                  </div>
                  <p>请从左侧列表选择一个环境变量进行配置</p>
                </CardContent>
              )}
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
