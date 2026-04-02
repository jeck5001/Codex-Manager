"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { useAppStore } from "@/lib/store/useAppStore";
import { normalizeRoutePath } from "@/lib/utils/static-routes";

/**
 * 函数 `usePageTransitionReady`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * - expectedPath: 参数 expectedPath
 * - isReady: 参数 isReady
 *
 * # 返回
 * 返回函数执行结果
 */
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
