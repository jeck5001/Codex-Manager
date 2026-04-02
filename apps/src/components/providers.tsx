"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Toaster } from "@/components/ui/sonner";
import { ThemeProvider } from "next-themes";
import { useState } from "react";

/**
 * 函数 `Providers`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * - params: 参数 params
 *
 * # 返回
 * 返回函数执行结果
 */
export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 5000,
        refetchOnWindowFocus: false,
      },
    },
  }));

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider 
        attribute="data-theme" 
        defaultTheme="tech" 
        enableSystem={false}
        disableTransitionOnChange
        themes={["tech", "dark", "dark-one", "business", "mint", "sunset", "grape", "ocean", "forest", "rose", "slate", "aurora"]}
      >
        <TooltipProvider>
          {children}
          <Toaster 
            position="top-right" 
            richColors 
            expand={false} 
            visibleToasts={3}
            closeButton
            duration={4000}
            theme="system"
            toastOptions={{
              closeButton: true,
            }}
          />
        </TooltipProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}
