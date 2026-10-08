import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// 開発時は /api を api-server（既定: http://localhost:8080）へプロキシする
const apiTarget = process.env.SENGOKU_API_URL ?? "http://localhost:8080";

export default defineConfig({
  plugins: [react()],
  server: {
    // 隣接情報としてリポジトリ直下の static/master_data/neighbor.csv を読み込むため、親ディレクトリを許可する
    fs: { allow: [".."] },
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
