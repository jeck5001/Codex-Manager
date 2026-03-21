"use client";

import { LoaderCircle } from "lucide-react";
import { useRuntimeCapabilities } from "@/hooks/useRuntimeCapabilities";
import { useAppStore } from "@/lib/store/useAppStore";

export function RouteTransitionOverlay() {
  const pendingRoutePath = useAppStore((state) => state.pendingRoutePath);
  const { isDesktopRuntime } = useRuntimeCapabilities();

  if (!isDesktopRuntime || !pendingRoutePath) {
    return null;
  }

  return (
    <div className="absolute inset-0 z-20 flex items-center justify-center bg-background/16 backdrop-blur-[1.5px]">
      <div className="glass-card flex min-w-[220px] flex-col items-center gap-3 rounded-3xl border-none bg-background/70 px-8 py-7 text-center shadow-2xl">
        <LoaderCircle className="h-8 w-8 animate-spin text-primary" />
        <div className="space-y-1">
          <div className="text-sm font-semibold text-foreground/90">正在切换页面</div>
          <div className="text-xs text-muted-foreground">组件已加载，正在同步页面数据...</div>
        </div>
      </div>
    </div>
  );
}
