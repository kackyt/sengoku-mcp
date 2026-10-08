import { defineConfig } from "@hey-api/openapi-ts";

/**
 * REST API クライアントの生成設定（pnpm gen:api）
 *
 * api-server/openapi.json から、fetch ベースのクライアント・型・SDK 関数を
 * src/api/generated に生成する。生成物はコミットし、CI で仕様との差分がないことを確認する。
 */
export default defineConfig({
  input: "../api-server/openapi.json",
  output: { path: "src/api/generated", postProcess: [] },
  plugins: ["@hey-api/client-fetch", "@hey-api/typescript", "@hey-api/sdk"],
});
