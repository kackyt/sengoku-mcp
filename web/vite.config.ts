import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// 開発時は /api を api-server（既定: http://localhost:8080）へプロキシする
const apiTarget = process.env.SENGOKU_API_URL ?? "http://localhost:8080";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": { target: apiTarget, changeOrigin: true },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
  },
});
