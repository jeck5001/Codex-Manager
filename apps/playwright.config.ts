import { defineConfig } from "@playwright/test";

const PORT = 3200;

export default defineConfig({
  testDir: "./tests",
  timeout: 30_000,
  fullyParallel: false,
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: "on-first-retry",
    video: "retain-on-failure",
  },
  webServer: {
    command: "node tests/support/static-server.mjs",
    url: `http://127.0.0.1:${PORT}`,
    reuseExistingServer: true,
    timeout: 120_000,
    env: {
      PORT: String(PORT),
    },
  },
});
