"use client";

import { useEffect, useState, useCallback } from "react";
import { useAppStore } from "@/lib/store/useAppStore";
import { serviceClient } from "@/lib/api/service-client";
import { appClient } from "@/lib/api/app-client";
import { isTauriRuntime } from "@/lib/api/transport";
import { Button } from "@/components/ui/button";
import { RefreshCw, AlertCircle, Play } from "lucide-react";
import { toast } from "sonner";

export function AppBootstrap({ children }: { children: React.ReactNode }) {
  const { setServiceStatus, setAppSettings, serviceStatus } = useAppStore();
  const [isInitializing, setIsInitializing] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const init = useCallback(async () => {
    if (!isTauriRuntime()) {
      setIsInitializing(false);
      return;
    }

    setIsInitializing(true);
    setError(null);

    try {
      // 1. Load app settings first (No network dependency)
      const settings = await appClient.getSettings();
      if (settings) {
        setAppSettings(settings);
        if (settings.lowTransparency) {
          document.body.classList.add("low-transparency");
        }
      }

      // 2. Get startup snapshot to see if service is running
      // This is a native command, should not timeout easily
      const snapshot = await serviceClient.getStartupSnapshot();
      let isRunning = false;
      let addr = "localhost:48760";

      if (snapshot) {
         isRunning = snapshot.is_running ?? false;
         addr = snapshot.listen_addr || addr;
         setServiceStatus({ 
           connected: isRunning, 
           version: snapshot.version,
           addr: addr
         });
      }

      // 3. Only try to initialize if service is actually running
      if (isRunning) {
        try {
          await serviceClient.initialize();
        } catch (e) {
          console.warn("Service running but initialize failed:", e);
          // Don't crash the whole app if initialize fails but service is "running"
        }
      }
      
      setIsInitializing(false);
    } catch (err: any) {
      console.error("Failed to initialize app:", err);
      setError(err.message || String(err));
    }
  }, [setServiceStatus, setAppSettings]);

  const handleForceStart = async () => {
    setIsInitializing(true);
    setError(null);
    try {
      await serviceClient.start(serviceStatus.addr || "localhost:48760");
      setServiceStatus({ connected: true });
      toast.success("服务已强制启动");
      await init();
    } catch (err: any) {
      setError(`强制启动失败: ${err.message}`);
      setIsInitializing(false);
    }
  };

  useEffect(() => {
    init();
  }, [init]);

  if (isInitializing || error) {
    return (
      <div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-6 p-10 rounded-3xl glass-card shadow-2xl animate-in fade-in zoom-in duration-500 max-w-md w-full">
          {!error ? (
            <>
              <div className="h-14 w-14 rounded-full border-4 border-primary border-t-transparent animate-spin" />
              <div className="flex flex-col items-center gap-2">
                <h2 className="text-2xl font-bold tracking-tight">正在准备环境</h2>
                <p className="text-sm text-muted-foreground text-center px-4">正在同步本地配置，请稍候...</p>
              </div>
            </>
          ) : (
            <>
              <div className="h-14 w-14 rounded-full bg-destructive/10 flex items-center justify-center">
                <AlertCircle className="h-8 w-8 text-destructive" />
              </div>
              <div className="flex flex-col items-center gap-2 text-center">
                <h2 className="text-xl font-bold tracking-tight text-destructive">无法同步核心服务状态</h2>
                <p className="text-[10px] text-muted-foreground font-mono bg-muted/50 p-3 rounded-lg break-all max-h-32 overflow-y-auto">
                  {error}
                </p>
              </div>
              <div className="grid grid-cols-2 gap-3 w-full">
                <Button variant="outline" onClick={() => init()} className="gap-2 h-11">
                  <RefreshCw className="h-4 w-4" /> 重试
                </Button>
                <Button onClick={handleForceStart} className="gap-2 h-11 bg-primary">
                  <Play className="h-4 w-4" /> 强制启动
                </Button>
              </div>
              <p className="text-[10px] text-muted-foreground text-center">
                如果服务未运行，请点击“强制启动”。
              </p>
            </>
          )}
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
