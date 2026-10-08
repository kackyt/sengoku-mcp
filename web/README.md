# sengoku-web

sengoku-mcp の「自国の状況・ターン数・勢力図」を日本地図で表示するブラウザアプリです（React + Vite + TypeScript）。
データは `api-server` の REST API から取得し、5秒ごとに自動更新します。

## 使い方

```bash
# 1. API サーバーを起動（MCP サーバーと同じ保存先を指定する）
cargo run -p api-server

# 2. Web アプリを起動（/api は http://localhost:8080 へプロキシ）
cd web
pnpm install
pnpm dev   # http://localhost:5173
```

1. 「新しいゲームを始める」を押すと参加コードが表示されます。
2. LLM とのチャットで「参加コード XXXXXX でゲームに参加して」と伝えます（MCP ツール `join_game`）。
3. LLM が大名を選ぶと、ターン数・自領の状況・勢力図が表示されます。

ページの URL（`?token=...`）を開けば、別のブラウザからも同じゲームを表示できます。
MCP ツールの結果に付く閲覧 URL をこのアプリに向けるには、MCP サーバーに次を設定します。

```bash
SENGOKU_VIEW_URL_TEMPLATE='http://localhost:5173/?token={token}'
```

## 地図の表現

12家の大名を色で塗り分けると判別できないため、**自領のみをアクセント色**、他家の領地は無彩色で塗ります。
どの大名の領地かは、国名の下のラベル・ホバー（同じ大名の領地をまとめて強調）・「他国の支配大名」の表で示します。

## 開発

| コマンド | 内容 |
| --- | --- |
| `pnpm dev` | 開発サーバー（`SENGOKU_API_URL` でプロキシ先を変更可） |
| `pnpm test` | ユニットテスト（Vitest + Testing Library） |
| `pnpm typecheck` | 型チェック |
| `pnpm build` | 本番ビルド（`dist/`）。API が別オリジンなら `VITE_API_BASE_URL` を指定 |
| `pnpm gen:api` | `api-server/openapi.json` から API の型（`src/api/schema.d.ts`）を再生成 |
| `pnpm gen:map -- <geojson>` | 日本地図データ（`src/map/japanMap.json`）を再生成 |

API と別オリジンで配信する場合は、`api-server` 側で `SENGOKU_CORS_ALLOW_ORIGINS`（カンマ区切り）を指定してください。

### 地図データ

`src/map/japanMap.json` は [Natural Earth](https://www.naturalearthdata.com/)（パブリックドメイン）の
1:10m Admin-1（都道府県）境界を、ゲームの12国（`static/master_data/kuni.csv`）に割り当てて結合・簡略化し、
メルカトル図法で投影したものです。都道府県と国の対応は `scripts/build-japan-map.mjs` を参照してください
（例: 三河＝静岡県、尾張＝岐阜県・愛知県。ゲーム上の勢力圏を都道府県単位で近似しています）。
沖縄・奄美・小笠原などゲームの12国に含まれない南方の島は描画していません。
