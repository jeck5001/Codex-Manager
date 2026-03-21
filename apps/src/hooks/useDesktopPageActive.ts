"use client";

import { useMemo } from "react";
import { usePathname } from "next/navigation";
import { useAppStore } from "@/lib/store/useAppStore";
import { normalizeRoutePath } from "@/lib/utils/static-routes";

export function useDesktopPageActive(expectedPath: string): boolean {
  const pathname = normalizeRoutePath(usePathname());
  const pendingRoutePath = useAppStore((state) => state.pendingRoutePath);
  const runtimeCapabilities = useAppStore((state) => state.runtimeCapabilities);
  const normalizedExpectedPath = useMemo(
    () => normalizeRoutePath(expectedPath),
    [expectedPath],
  );
  const normalizedPendingRoutePath = useMemo(
    () => (pendingRoutePath ? normalizeRoutePath(pendingRoutePath) : ""),
    [pendingRoutePath],
  );
  const isDesktopRuntime = runtimeCapabilities?.mode === "desktop-tauri";

  if (isDesktopRuntime && normalizedPendingRoutePath) {
    return normalizedPendingRoutePath === normalizedExpectedPath;
  }

  return pathname === normalizedExpectedPath;
}
