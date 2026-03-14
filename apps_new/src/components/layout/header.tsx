"use client";

import { useAppStore } from "@/lib/store/useAppStore";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { 
  Settings as SettingsIcon
} from "lucide-react";
import { Input } from "@/components/ui/input";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { WebPasswordModal } from "../modals/web-password-modal";
import { serviceClient } from "@/lib/api/service-client";
import { toast } from "sonner";

export function Header() {
  const { serviceStatus, setServiceStatus } = useAppStore();
  const pathname = usePathname();
  const [webPasswordModalOpen, setWebPasswordModalOpen] = useState(false);
  const [isToggling, setIsToggling] = useState(false);

  const getPageTitle = () => {
    switch (pathname) {
      case "/": return "仪表盘";
      case "/accounts": return "账号管理";
      case "/apikeys": return "平台密钥";
      case "/logs": return "请求日志";
      case "/settings": return "应用设置";
      case "/appearance": return "外观设置";
      default: return "CodexManager";
    }
  };

  const handleToggleService = async (val: boolean) => {
    setIsToggling(true);
    try {
      if (val) {
        // Start Service
        await serviceClient.start(serviceStatus.addr);
        setServiceStatus({ connected: true });
        toast.success("服务已启动");
      } else {
        // Stop Service
        await serviceClient.stop();
        setServiceStatus({ connected: false });
        toast.info("服务已停止");
      }
    } catch (err: any) {
      toast.error(`操作失败: ${err.message}`);
    } finally {
      setIsToggling(false);
    }
  };

  return (
    <>
      <header className="flex h-16 items-center justify-between glass-header px-6 sticky top-0 z-30">
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-semibold">{getPageTitle()}</h1>
          <Badge variant={serviceStatus.connected ? "default" : "secondary"} className="h-5">
            {serviceStatus.connected ? "服务已连接" : "服务未连接"}
          </Badge>
        </div>

        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 rounded-lg border bg-card/30 px-3 py-1.5 shadow-sm">
            <span className="text-xs text-muted-foreground font-medium">监听端口</span>
            <Input 
              className="h-7 w-16 border-none bg-transparent p-0 text-xs font-mono focus-visible:ring-0" 
              placeholder="48760" 
              value={serviceStatus.addr?.split(":")[1] || "48760"}
              onChange={(e) => {
                const port = e.target.value;
                setServiceStatus({ addr: `localhost:${port}` });
              }}
            />
            <div className="h-4 w-px bg-border mx-1" />
            <Switch 
              checked={serviceStatus.connected} 
              disabled={isToggling}
              onCheckedChange={handleToggleService}
              className="scale-90"
            />
          </div>

          <Button 
            variant="outline" 
            size="sm" 
            className="gap-2 h-9 px-3"
            onClick={() => setWebPasswordModalOpen(true)}
          >
            <SettingsIcon className="h-3.5 w-3.5" />
            <span className="text-xs">Web 密码</span>
          </Button>
        </div>
      </header>

      <WebPasswordModal 
        open={webPasswordModalOpen} 
        onOpenChange={setWebPasswordModalOpen} 
      />
    </>
  );
}
