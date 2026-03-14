import type { Metadata } from "next";
import { Fira_Code, Fira_Sans } from "next/font/google";
import { ThemeProvider as NextThemesProvider } from "next-themes";
import { CodexAppProvider } from "@/app/codex-app-provider";
import { themeOptions } from "@/lib/codex/ui-config";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import "./globals.css";

const appSans = Fira_Sans({
  subsets: ["latin"],
  weight: ["300", "400", "500", "600", "700"],
  variable: "--app-font-sans",
  display: "swap",
});

const appMono = Fira_Code({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  variable: "--app-font-mono",
  display: "swap",
});

export const metadata: Metadata = {
  applicationName: "CodexManager",
  title: "CodexManager",
  description: "CodexManager 全新前端，基于 Next.js、TypeScript 与 shadcn/ui 重构。",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <body className={`cm-root antialiased ${appSans.variable} ${appMono.variable}`}>
        <NextThemesProvider
          attribute="data-theme"
          defaultTheme="tech"
          enableSystem={false}
          themes={themeOptions.map((item) => item.id)}
          disableTransitionOnChange
        >
          <CodexAppProvider>
            <TooltipProvider delay={120}>
              {children}
              <Toaster richColors />
            </TooltipProvider>
          </CodexAppProvider>
        </NextThemesProvider>
      </body>
    </html>
  );
}
