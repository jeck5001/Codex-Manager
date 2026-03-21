"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { useAppStore } from "@/lib/store/useAppStore";
import { normalizeRoutePath } from "@/lib/utils/static-routes";

export function usePageTransitionReady(isReady: boolean) {
  const pathname = normalizeRoutePath(usePathname());
  const pendingRoutePath = useAppStore((state) => state.pendingRoutePath);
  const setPendingRoutePath = useAppStore((state) => state.setPendingRoutePath);

  useEffect(() => {
    if (!isReady || !pendingRoutePath) {
      return;
    }
    if (pendingRoutePath !== pathname) {
      return;
    }
    setPendingRoutePath("");
  }, [isReady, pathname, pendingRoutePath, setPendingRoutePath]);
}
