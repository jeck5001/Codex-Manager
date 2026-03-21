"use client";

import { useEffect, useState } from "react";
import { isTauriRuntime } from "@/lib/api/transport";

export function useDeferredDesktopActivation(enabled: boolean): boolean {
  const shouldDefer = isTauriRuntime();
  const [isActivated, setIsActivated] = useState(
    () => enabled && !shouldDefer,
  );

  useEffect(() => {
    if (!enabled) {
      setIsActivated(false);
      return;
    }

    if (!shouldDefer || typeof window === "undefined") {
      setIsActivated(true);
      return;
    }

    setIsActivated(false);
    let cancelled = false;
    let secondFrameId: number | null = null;
    const firstFrameId = window.requestAnimationFrame(() => {
      secondFrameId = window.requestAnimationFrame(() => {
        if (!cancelled) {
          setIsActivated(true);
        }
      });
    });

    return () => {
      cancelled = true;
      window.cancelAnimationFrame(firstFrameId);
      if (secondFrameId !== null) {
        window.cancelAnimationFrame(secondFrameId);
      }
    };
  }, [enabled, shouldDefer]);

  return isActivated;
}
