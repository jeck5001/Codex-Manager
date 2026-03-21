"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { useAppStore } from "@/lib/store/useAppStore";
import { normalizeRoutePath } from "@/lib/utils/static-routes";

export function usePageTransitionReady(expectedPath: string, isReady: boolean) {
  const pathname = normalizeRoutePath(usePathname());
  const normalizedExpectedPath = normalizeRoutePath(expectedPath);
  const pendingRoutePath = useAppStore((state) => state.pendingRoutePath);
  const setPendingRoutePath = useAppStore((state) => state.setPendingRoutePath);

  useEffect(() => {
    if (!isReady || !pendingRoutePath) {
      return;
    }
    if (normalizedExpectedPath !== pathname) {
      return;
    }
    if (normalizeRoutePath(pendingRoutePath) !== normalizedExpectedPath) {
      return;
    }
    setPendingRoutePath("");
  }, [
    isReady,
    normalizedExpectedPath,
    pathname,
    pendingRoutePath,
    setPendingRoutePath,
  ]);
}
