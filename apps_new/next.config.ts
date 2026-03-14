import type { NextConfig } from "next";

const isDesktopExport = process.env.NEXT_DESKTOP_EXPORT === "1";

const nextConfig: NextConfig = {
  allowedDevOrigins: ["127.0.0.1"],
  distDir: isDesktopExport ? "out" : undefined,
  output: isDesktopExport ? "export" : undefined,
};

export default nextConfig;
