# sengoku-web

sengoku-mcp の「自国の状況・年と季節・勢力図」を表示するブラウザアプリです（React + Vite + TypeScript）。
データは `api-server` の REST API から取得し、5秒ごとに自動更新します。

## 使い方

mcp-server・api-server との連携手順と環境変数の一覧は [docs/integration.md](../docs/integration.md) を参照してください。

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

### レイアウト（サイドバー想定）

ブラウザの横半分ほどのサイドバーで使う想定です。

- 上部の1行に、大名・年と季節・自領の合計（領地・兵・金・米）。年と季節は第1ターン＝1560年春として、1ターン＝1季節で進みます（`src/gameDate.ts`）。
- 勢力図は**接続図**（国を箱、隣接を線で表した模式図。海路は破線）。凡例は右下の空き領域に重ねます。
- 自領は国ごとのカード。表示幅が 600px 以上なら接続図の右に、狭ければ下に、**折り返して全件**表示します（切れたり横スクロールになったりしません）。

### 大名ごとの色分け

国の箱は支配大名の色で塗り、自領は太い黒の輪郭で示します。凡例は自国を先頭に、領地数の多い順に並びます。
色は大名ごとに固定（`src/daimyoColors.ts`）で、ゲーム開始時に隣接する大名同士の色差が最大になるよう、
通常の色覚と色覚多様性のシミュレーションの両方で割り当てを探索して決めています。
近い色の大名が隣り合った場合でも、国を選べば支配大名が表示されます。

### 隣接（行き来できる国）

マスターデータ（`static/master_data/neighbor.csv`）をビルド時に読み込み、接続線で表示します。
国にホバー（タッチ操作ではタップ）すると、その国の接続線と隣接国、同じ大名の領地を強調します。

## 開発

| コマンド | 内容 |
| --- | --- |
| `pnpm dev` | 開発サーバー（`SENGOKU_API_URL` でプロキシ先を変更可） |
| `pnpm test` | ユニットテスト（Vitest + Testing Library） |
| `pnpm typecheck` | 型チェック |
| `pnpm build` | 本番ビルド（`dist/`）。API が別オリジンなら `VITE_API_BASE_URL` を指定 |
| `pnpm gen:api` | `api-server/openapi.json` から API クライアント（`src/api/generated/`）を再生成 |

### API クライアント

`src/api/generated/` は [@hey-api/openapi-ts](https://heyapi.dev/) が `api-server/openapi.json` から生成する
fetch ベースのクライアント・型・SDK 関数です（設定は `openapi-ts.config.ts`、手で編集しない）。
API 仕様を変えたら `pnpm gen:api` で再生成してコミットしてください（CI で差分を検査します）。

画面からは `src/api/client.ts` 経由で呼び出します。通信部分は [ky](https://github.com/sindresorhus/ky) に差し替えており、
GET は通信失敗と一時的なエラー（408 / 429 / 500 / 502 / 503 / 504）を最大2回まで再試行します。
ゲーム作成（POST）は二重作成を避けるため再試行しません。

API と別オリジンで配信する場合は、`api-server` 側で `SENGOKU_CORS_ALLOW_ORIGINS`（カンマ区切り）を指定してください。
