# sengoku-mcp

ターン制戦国シミュレーションを実現するMCP（Model Context Protocol）サーバーです。
Rustで実装されたゲームエンジンをMCPサーバーとして公開し、Claude などの LLM クライアントと
連携して「全国統一」を目指してプレイします。TUI（端末GUI）版のクライアントも同梱しています。

外部仕様の詳細は [PRD.md](PRD.md) を参照してください。

---

## 目次

1. [アーキテクチャ概要](#アーキテクチャ概要)
2. [前提ツール](#前提ツール)
3. [セットアップ](#セットアップ)
4. [ビルドと動作確認](#ビルドと動作確認)
5. [MCPサーバーを使った戦国シミュレーション](#mcpサーバーを使った戦国シミュレーション)
6. [セッションの保存先（ファイル / Google Cloud Storage）](#セッションの保存先ファイル--google-cloud-storage)
7. [REST APIで自国の状況を取得する](#rest-apiで自国の状況を取得する)
8. [TUI版で遊ぶ（おまけ）](#tui版で遊ぶおまけ)
9. [開発者向け：チェックコマンド](#開発者向けチェックコマンド)
10. [トラブルシューティング](#トラブルシューティング)

---

## アーキテクチャ概要

オニオンアーキテクチャを採用した Cargo ワークスペースです。

```
sengoku-mcp/
├─ engine/          ドメイン層・アプリケーション層（ゲームロジック本体）
├─ infrastructure/  リポジトリ実装・マスターデータのロード
├─ game-session/    セッション（GameContext）の構築・管理（MCP / REST で共有）
├─ mcp-server/      MCPプロトコルのマッピング（LLMから操作する入口）
├─ api-server/      自国の状況を返す REST API
├─ web/             状況と勢力図（日本地図）を表示するブラウザアプリ（React）
├─ cli/             TUI（ratatui/crossterm）クライアント
├─ static/master_data/  マスターデータ（daimyo.csv / kuni.csv / neighbor.csv）
├─ .rulesync/       AIツール設定のソース（rulesyncで各ツール向けに展開）
└─ Cargo.toml       ワークスペース定義
```

マスターデータは `include_str!` でバイナリに埋め込まれるため、実行時にCSVのパス設定は不要です。

---

## 前提ツール

| ツール | 用途 | 推奨バージョン |
| --- | --- | --- |
| **Rust / cargo** | エンジン・MCPサーバーのビルドと実行 | 1.88 以上（`api-server` の utoipa が 1.88 を要求、`cli` が edition 2024 を使用） |
| **Node.js** | pnpm の実行環境 | 20 以上 |
| **pnpm** | `rulesync` / `openspec` などの開発ツール管理 | 10.x（`package.json` の `packageManager` 参照） |
| **rulesync** | `.rulesync/` から各AIツール設定を生成 | devDependency で導入 |
| **MCPクライアント** | サーバーへ接続してプレイ（Claude Code / Cursor / Antigravity 等） | 任意 |

> Windows / macOS / Linux いずれでも動作します。以下のコマンド例は `bash` 想定ですが、
> PowerShell でも同様に実行できます。

### Rust のインストール

```bash
# rustup 経由でインストール（既に入っていればスキップ）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version   # 1.88 以上であることを確認
```

### pnpm のインストール

```bash
# corepack を使うのが簡単（Node.js 20+ に同梱）
corepack enable
corepack prepare pnpm@10.19.0 --activate
pnpm --version
```

---

## セットアップ

### 1. リポジトリの取得

```bash
git clone <このリポジトリのURL> sengoku-mcp
cd sengoku-mcp
```

### 2. 開発ツールのインストール（pnpm）

`rulesync` と `openspec` を devDependency として導入します。

```bash
pnpm install
```

### 3. AIツール設定の生成（rulesync）

`.rulesync/` 配下のルール・スキル・MCP定義を、各AIツール（Claude Code / Cursor /
Antigravity）向けの設定ファイルに展開します。生成ターゲットは [rulesync.jsonc](rulesync.jsonc)
で定義されています。

```bash
pnpm exec rulesync generate
```

これにより、`.claude/` や `.cursor/` などにスキルやMCPサーバー定義が出力されます。
`sengoku-play` スキルなど、対話プレイ用のスキルもここで配置されます。

---

## ビルドと動作確認

ワークスペース全体をリリースビルドします（初回は依存クレートのコンパイルに数分かかります）。

```bash
# 全クレートをビルド
cargo build --release

# テストが通ることを確認
cargo test
```

MCPサーバー単体をリリースビルドしておくと、後段のクライアント接続が高速になります。

```bash
cargo build --release -p mcp-server
```

---

## MCPサーバーを使った戦国シミュレーション

MCPサーバーは **stdio トランスポート** で動作します。MCPクライアントが
`cargo run -p mcp-server` をサブプロセスとして起動し、標準入出力でやり取りします。

### MCPサーバーの登録

リポジトリ同梱の [.mcp.json](.mcp.json) を参考に、クライアントへサーバーを登録します。

```jsonc
{
  "mcpServers": {
    "sengoku-mcp": {
      "type": "stdio",
      "command": "cargo",
      "args": [
        "run",
        "--release",
        "--manifest-path",
        "/path/to/sengoku-mcp/Cargo.toml",  // ← clone した場所の絶対パスに変更
        "-p",
        "mcp-server"
      ],
      "env": {}
    }
  }
}
```

> `--manifest-path` はクローンした場所に合わせて書き換えてください。
> `.mcp.json` / `.rulesync/mcp.json` では `$HOME/sengoku-mcp/Cargo.toml` を例示しています。

> **Note**: 事前に `cargo build --release -p mcp-server` でビルドしておき、生成されたバイナリ
> （例: `target/release/mcp-server` / Windows では `target/release/mcp-server.exe`）の絶対パスを
> `command` に直接指定することも可能です。

#### Claude Code から使う場合

プロジェクトルートに `.mcp.json` があれば自動で読み込まれます。手動で追加する場合は次の通りです。

```bash
claude mcp add sengoku-mcp -- cargo run --release --manifest-path /path/to/sengoku-mcp/Cargo.toml -p mcp-server
```

### サーバーの動作を手元で確認する

クライアントなしでも、サーバーが起動するか確認できます（stdio待ち受け状態になります。Ctrl+C で終了）。

```bash
cargo run --release -p mcp-server
```

起動時にマスターデータでゲームが初期化され、ツール呼び出しを待ち受けます。

### 提供される主なMCPツール

| カテゴリ | ツール | 説明 |
| --- | --- | --- |
| 準備 | `list_daimyos` | 選択可能な大名の一覧を取得 |
| 準備 | `select_daimyo` | 操作する大名を選び、ゲームを初期状態から開始 |
| 状況把握 | `get_my_status` | 自分の領地・資源・手番・侵攻アラートを取得 |
| 状況把握 | `get_game_status` | フェーズ・ターン・季節・勝者を取得 |
| 状況把握 | `get_other_countries_info` | 他国の情報を取得（コマンド権を1消費） |
| 状況把握 | `get_neighbor_info` | 指定国の隣接国（攻撃・輸送先候補）を取得 |
| 準備 | `join_game` | ブラウザで作成したゲームに参加コードで参加（プレイヤーからコードを伝えられたら最初に呼ぶ） |
| 状況把握 | `get_status_view_url` | 閲覧URLを取得（通常は `select_daimyo` / `get_my_status` の結果に自動で付く）。`regenerate=true` で再発行（旧URLは無効化） |
| 内政 | `domestic_rice_sell` / `domestic_rice_buy` | 米売り / 米買い |
| 内政 | `domestic_recruit` | 兵の徴募 |
| 内政 | `domestic_develop_land` | 開墾（石高アップ） |
| 内政 | `domestic_build_town` | 町作り（収入アップ） |
| 内政 | `domestic_give_charity` | 施し（忠誠度アップ） |
| 内政 | `domestic_transport` | 隣接自領への資源輸送 |
| 合戦 | `battle_start_war` | 隣接他国へ出陣 |
| 合戦 | `battle_execute_turn` | 攻撃側として戦術を選び1ターン進行（1:通常 2:奇襲 3:火計 4:鼓舞 5:退却） |
| 合戦 | `battle_execute_defense_turn` | 防御側として戦術を選び1ターン進行 |
| 合戦 | `get_battle_status` | 進行中の合戦の兵数・士気・優劣を取得 |
| 進行 | `progress_turn` | 自分の手番が来る／ターンが終わるまでゲームを進める |
| 進行 | `domestic_auto_action` | 手番の国をAIに自動行動させる（お任せ） |
| ログ | `get_recent_logs` | 直近の行動ログを取得 |

### 基本的なプレイの流れ

LLMクライアントから、おおむね次の順でツールを呼び出してプレイします。

1. **大名を選ぶ** — `list_daimyos` で一覧を見て、`select_daimyo`（例: `daimyo_id: 1`）で開始。
2. **状況を把握する** — `get_my_status` で資源と手番、`get_neighbor_info` で隣接国を確認。
3. **内政で国力を上げる** — `domestic_recruit`（兵）や `domestic_develop_land`（石高）などを実行。
4. **合戦を仕掛ける** — `battle_start_war` で出陣し、`battle_execute_turn` で戦術を選んで決着まで進める。
5. **ターンを進める** — `progress_turn` で次の自分の手番まで進行。CPU大名が侵攻してきたら
   `get_my_status` の侵攻アラートを見て `battle_execute_defense_turn` で防衛。
6. 上記を繰り返し、**全大名を滅ぼして全国統一**を目指します（`get_game_status` の勝者を確認）。

### スキルを使った対話プレイ

`pnpm exec rulesync generate` 済みのClaude Code環境では、`sengoku-play` スキルが利用できます。
AIアシスタントに以下のように話しかけてみてください。

- 「戦国ゲームを始めて」「プレイしたい」
- 「現在の状況を教えて」
- 「お任せで内政してほしい」
- 「ターンを進めて」
- 「隣国に合戦を仕掛けて」
- 「戦略をアドバイスして」

AIがMCPツールを自律的に呼び出し、状況分析からコマンド実行・ターン進行までをサポートしながら
一緒に全国統一を目指してくれます。

※`sengoku-play-ikuo` スキルを使えば某EMがアシスタントとして上記ツール群を適切な順序で呼び出して
対話的に進行します。

---

## セッションの保存先（ファイル / Google Cloud Storage）

ゲーム状態はセッションごとに JSON として保存されます。MCPサーバーは操作のたびに保存し、
REST APIサーバーは同じ保存先を読み込むことで状態を共有します。保存先は環境変数で切り替えます。

| 環境変数 | 説明 | デフォルト |
| --- | --- | --- |
| `SENGOKU_STORAGE` | `file` または `gcs` | `file` |
| `SENGOKU_SESSIONS_DIR` | `file` の保存先ディレクトリ | `data/sessions` |
| `SENGOKU_GCS_BUCKET` | `gcs` のバケット名（`gcs` では必須） | なし |
| `SENGOKU_GCS_PREFIX` | `gcs` のオブジェクトキーのプレフィックス | `sessions` |

`gcs` の場合、`gs://<バケット>/<プレフィックス>/<セッションID>.json` に保存されます。
閲覧トークンの対応表は同じ保存先の `view_tokens/<トークン>.json` に保存されます（期限切れクリーンアップの対象外）。
認証情報は次の順で解決されます。

- `GOOGLE_SERVICE_ACCOUNT`（サービスアカウントキーのファイルパス）または
  `GOOGLE_SERVICE_ACCOUNT_KEY`（キーのJSON文字列）
- Application Default Credentials（`gcloud auth application-default login` の結果、
  Cloud Run / GCE のメタデータサーバー）

サービスアカウントには対象バケットのオブジェクト読み書き権限（例: `roles/storage.objectUser`）を付与してください。

```bash
# 例: MCPサーバーを GCS 保存で起動
SENGOKU_STORAGE=gcs SENGOKU_GCS_BUCKET=my-sengoku-bucket cargo run --release -p mcp-server
```

MCPクライアントから起動する場合は、`.mcp.json` の `env` に同じ変数を設定します。

> **Note**: MCPサーバーはセッションをメモリにキャッシュするため、同じ保存先に対して
> 複数のMCPサーバーを同時に書き込ませる構成は想定していません（REST APIサーバーは読み取り専用なので何台でも可）。

---

## REST APIで自国の状況を取得する

`api-server` は MCPサーバーと同じ保存先を読み込み、自国の状況を JSON で返す読み取り専用の
HTTPサーバーです。リクエストのたびに保存先から最新の状態を読み込むため、MCPで進めた内容が即座に反映されます。

### Webアプリとチャットの連携（ブラウザでゲームを作成 → 参加コードで参加）

セッションIDはMCP内部で扱う値（Chat ID 等）なので、Webアプリは知りません。そこで、
**ブラウザ側でゲームを作成し、短い参加コードをチャットに伝える**ことで両者を結び付けます。
LLM が扱うのは参加コードを1回渡すことだけで、URLの中継やセッションIDの受け渡しは不要です。

```text
[ブラウザ]  POST /api/games
            → 閲覧トークン（status_url）と参加コード（例: KX7P2Q、30分有効・1回限り）を受け取る
            → status_url をポーリング（参加前は 409 waiting_for_join）
[プレイヤー] チャットで「参加コード KX7P2Q でゲームに参加して」と伝える
[LLM]       join_game(code="KX7P2Q") を1回呼ぶ
            → ゲームがチャットのセッションIDへ移り、閲覧トークンもそのゲームを指すようになる
[LLM]       list_daimyos → select_daimyo（以降は通常どおり、チャットのセッションIDで操作）
[ブラウザ]  同じ status_url で自国の状況が取得できる（200）
```

- 参加コードは大文字・小文字やハイフン・空白の有無を問いません（`kx7-p2q` でも可）。
- 同じチャットで別のゲームに参加すると、進行中のゲームは新しいゲームに置き換わり、旧URLは 404 になります。
- 参加されなかったゲームは、通常のセッションと同様に7日で削除されます。期限切れの参加コードは定期クリーンアップで削除されます。

#### チャットで開始したゲームの場合（補助的な経路）

ブラウザを使わずチャットだけで開始したゲームにも、閲覧トークンが自動で発行されます。
`select_daimyo`（ゲーム開始）と `get_my_status` の結果の末尾に、サーバーが
`📺 ブラウザで自国の状況を見る: <URL>` を付けるので、LLM がそれを伝えればブラウザでも閲覧できます。

トークンはセッションごとに1つで、セッションと一緒に永続化されます（MCPサーバーを再起動しても同じURL）。
トークン導入前に保存されたセッションには、次回読み込み時に発行されます。
URLが漏れた場合は `get_status_view_url` を `regenerate=true` で呼ぶと再発行され、以前のURLは 404 になります。

MCPツールが返すURLは `SENGOKU_VIEW_URL_TEMPLATE`（MCPサーバー側の環境変数）で変更できます。
`{token}` がトークンに置換されます。

```bash
# デフォルト: http://localhost:8080/api/views/{token}/status
# 例: Webアプリのページを返す（Webアプリは token クエリを使って REST API を呼ぶ）
SENGOKU_VIEW_URL_TEMPLATE='https://sengoku.example.com/status?token={token}'
```

### エンドポイント

```bash
# MCPサーバーと同じ保存先設定で起動（待ち受け: SENGOKU_API_ADDR > PORT > 0.0.0.0:8080）
SENGOKU_STORAGE=gcs SENGOKU_GCS_BUCKET=my-sengoku-bucket cargo run --release -p api-server
```

| メソッド / パス | 説明 |
| --- | --- |
| `GET /health` | ヘルスチェック（`ok`） |
| `POST /api/games` | 新規ゲームを作成し、閲覧トークン（`status_url`）と参加コードを発行（201） |
| `GET /api/views/{token}/status` | 閲覧トークンに対応するセッションの自国の状況（Webアプリ向け） |
| `GET /api/status` | `default` セッション（MCPで `session_id` 省略時）の自国の状況 |
| `GET /api/sessions/{session_id}/status` | 指定セッションの自国の状況（セッションIDを知っているクライアント・デバッグ向け） |
| `GET /openapi.json` | OpenAPI 3.1 仕様（JSON） |
| `GET /docs` | APIドキュメント画面（Scalar。画面のJSは CDN から読み込み） |

### OpenAPI 仕様

OpenAPI 仕様は [utoipa](https://github.com/juhaku/utoipa) でコードから生成しています。
ハンドラー（`api-server/src/presentation/routes.rs`）と DTO（`api-server/src/application/dto.rs`）の
ドキュメントコメントが、そのままエンドポイント・スキーマの説明になります。

- 生成結果は [api-server/openapi.json](api-server/openapi.json) にコミットしており、クライアントコード生成などに使えます。
- コードと食い違うと `cargo test` が失敗します。API を変更したら次のコマンドで更新してください。

```bash
UPDATE_OPENAPI=1 cargo test -p api-server --test openapi_test
```

`POST /api/games` のレスポンス例:

```json
{
  "view_token": "777fc6d170d94495b49babeba2230e7d",
  "status_url": "/api/views/777fc6d170d94495b49babeba2230e7d/status",
  "join_code": "VQ4X7K",
  "join_code_expires_at": "2026-10-08T04:20:32Z",
  "join_message": "チャットで「参加コード VQ4X7K でゲームに参加して」と伝えてください"
}
```

状況取得（`GET .../status`）のレスポンス例。返すのは **自国の状況・ターン数・他国の支配大名のみ** で、
他国の資源（兵・金など）は含みません。

```json
{
  "turn": 3,
  "daimyo": { "id": 7, "name": "織田" },
  "my_kunis": [
    { "id": 7, "name": "尾張", "kin": 200, "kome": 120, "hei": 80,
      "jinko": 350, "kokudaka": 60, "machi": 100, "tyu": 80 }
  ],
  "other_kunis": [
    { "id": 6, "name": "三河", "daimyo": { "id": 6, "name": "徳川" } },
    { "id": 8, "name": "山城", "daimyo": { "id": 8, "name": "足利" } }
  ]
}
```

| ステータス | `code` | 意味 |
| --- | --- | --- |
| 409 | `waiting_for_join` | Webで作成したゲームに、チャット側がまだ参加していない |
| 404 | `view_not_found` | 閲覧トークンが不正・未発行・再発行で失効済み、またはセッションが期限切れ |
| 404 | `session_not_found` | セッションが保存先に存在しない |
| 409 | `daimyo_not_selected` | セッションはあるが大名が未選択 |
| 500 | `internal_error` | 保存先へのアクセス失敗など（詳細はサーバーログ） |

Web アプリを API と別オリジンで配信する場合は、`SENGOKU_CORS_ALLOW_ORIGINS` に許可するオリジンを
カンマ区切りで指定してください（例: `https://sengoku.example.com`。`*` で全許可）。

### ブラウザアプリ（勢力図）

[web/](web/README.md) に、この API を使って自国の状況・ターン数・勢力図（日本地図）を表示する React アプリがあります。

```bash
cargo run -p api-server        # API（:8080）
cd web && pnpm install && pnpm dev   # Web（:5173、/api を :8080 へプロキシ）
```

MCP ツールの結果に付く閲覧URLを Web アプリに向けるには、MCP サーバーに
`SENGOKU_VIEW_URL_TEMPLATE='http://localhost:5173/?token={token}'` を設定します。

> **Note**: `POST /api/games` は認証なしでセッションを作成できるため、公開する場合は
> Cloud Run の IAM 認証・API Gateway のレート制限などで乱用を防いでください。
>
> **Note**: 閲覧トークンのURLは「URLを知っている人なら誰でも見られる」共有リンクです。
> `/api/status` と `/api/sessions/{session_id}/status` はセッションIDだけで参照できるため、
> インターネットに公開する場合はこれらを Cloud Run の IAM 認証やリバースプロキシで制限してください。

---

## TUI版で遊ぶ（おまけ）

LLMを使わず、端末上のGUIで直接プレイすることもできます。

```bash
cargo run --release -p cli
```

方向キーでメニュー選択、Enterで決定です。MCPサーバーと同じエンジン・マスターデータを使用します。

---

## 開発者向け：チェックコマンド

コミット前に以下がすべて通ることを確認してください（[CLAUDE.md](CLAUDE.md) の規約）。

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test
```

---

## トラブルシューティング

- **`edition2024` や `rust-version` 関連のビルドエラー** — Rust が古い可能性があります。`rustup update` で
  1.88 以上に更新してください。
- **`pnpm: command not found`** — `corepack enable` を実行したか確認してください。
- **MCPクライアントがサーバーに接続できない** — `.mcp.json` の `--manifest-path` が
  クローン先の絶対パスを指しているか確認してください。初回はビルドに時間がかかるため、
  事前に `cargo build --release -p mcp-server` を済ませておくと安定します。
- **「大名が選択されていません」と表示される** — 最初に `select_daimyo` を実行する必要があります。
