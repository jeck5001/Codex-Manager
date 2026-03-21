"use client";

import { useMemo } from "react";
import { isTauriRuntime } from "@/lib/api/transport";
import {
  resolveRuntimeCapabilityView,
  type RuntimeCapabilityView,
} from "@/lib/runtime/runtime-capabilities";
import { useAppStore } from "@/lib/store/useAppStore";

export function useRuntimeCapabilities(): RuntimeCapabilityView {
  const runtimeCapabilities = useAppStore((state) => state.runtimeCapabilities);

  return useMemo(() => {
    return resolveRuntimeCapabilityView(runtimeCapabilities, isTauriRuntime());
  }, [runtimeCapabilities]);
}
