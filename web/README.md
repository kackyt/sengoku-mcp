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
2. 「チャット用の文をコピー」で「参加コード XXXXXX でゲームに参加して」をコピーし、LLM とのチャットに貼ります（MCP ツール `join_game`）。
3. LLM が大名を選ぶと、ターン数・自領の状況・勢力図が表示されます。

ページの URL（`?token=...`）を開けば、別のブラウザからも同じゲームを表示できます。
MCP ツールの結果に付く閲覧 URL をこのアプリに向けるには、MCP サーバーに次を設定します。

```bash
SENGOKU_VIEW_URL_TEMPLATE='http://localhost:5173/?token={token}'
```

## 画面の方針

説明文は置かず、**色と数字**で状況を示します。表示する文字は国名・大名名（凡例）・数値・参加コードのみです。

### 大名ごとの色分け

国は支配大名の色で塗り、自領は太い黒の輪郭で示します。凡例（地図の下）は自国を先頭に、領地数の多い順に並びます。
色は大名ごとに固定（`src/daimyoColors.ts`）で、ゲーム開始時に隣接する大名同士の色差が最大になるよう、
通常の色覚と色覚多様性のシミュレーションの両方で割り当てを探索して決めています。
攻略が進んで近い色の大名が隣り合った場合でも、国にホバー（タップ）すれば支配大名が表示されます。

### 隣接（行き来できる国）

マスターデータ（`static/master_data/neighbor.csv`）の隣接情報を、国の中心点を結ぶ**接続線**で表示します。
国にホバー（タップ・フォーカス）すると、その国の接続線と隣接国、同じ大名の領地を強調します。

### 小さな画面・低解像度での表示

- 文字・点の大きさと線の太さは**画面上のピクセルで固定**しているため、地図を縮小しても細く・小さくなりすぎません。
- 日本地図の表示幅が 460px 未満になる場合（スマートフォンや高さの低い画面）は、既定で**接続図**
  （国を箱、隣接を線で表した模式図。海路は破線）に切り替えます。「地図 / 接続」ボタンでいつでも切り替えられます。

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

`src/map/japanMap.json`（国の形・中心点・接続線）は [Natural Earth](https://www.naturalearthdata.com/)（パブリックドメイン）の
1:10m Admin-1（都道府県）境界を、ゲームの12国（`static/master_data/kuni.csv`）に割り当てて結合・簡略化し、
メルカトル図法で投影したものです。都道府県と国の対応は `scripts/build-japan-map.mjs` を参照してください
（例: 三河＝静岡県、尾張＝岐阜県・愛知県。ゲーム上の勢力圏を都道府県単位で近似しています）。
沖縄・奄美・小笠原などゲームの12国に含まれない南方の島は描画していません。
接続線は `static/master_data/neighbor.csv` から生成し、別の国の中心点の近くを通る線は曲線にして避けています
（地図データとマスターデータの隣接情報が一致することはテストで確認しています）。
