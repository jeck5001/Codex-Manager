import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // 暂时禁用 Beta 版编译器以确保稳定性
  reactCompiler: false,
  // Tauri 开发态通过 127.0.0.1 加载 Next 资源，显式放行避免 dev 跨源告警。
  allowedDevOrigins: ["127.0.0.1", "[::1]"],
  output: 'export',
  images: {
    unoptimized: true,
  },
};

export default nextConfig;
