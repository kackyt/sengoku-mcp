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
6. [TUI版で遊ぶ（おまけ）](#tui版で遊ぶおまけ)
7. [開発者向け：チェックコマンド](#開発者向けチェックコマンド)
8. [トラブルシューティング](#トラブルシューティング)

---

## アーキテクチャ概要

オニオンアーキテクチャを採用した Cargo ワークスペースです。

```
sengoku-mcp/
├─ engine/          ドメイン層・アプリケーション層（ゲームロジック本体）
├─ infrastructure/  リポジトリ実装・マスターデータのロード
├─ mcp-server/      MCPプロトコルのマッピング（LLMから操作する入口）
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
| **Rust / cargo** | エンジン・MCPサーバーのビルドと実行 | 1.85 以上（`cli` が edition 2024 を使用） |
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
rustc --version   # 1.85 以上であることを確認
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
「戦国ゲームを始めて」「ターンを進めて」「合戦を仕掛けて」「お任せで」などと話しかけると、
スキルが上記ツール群を適切な順序で呼び出して対話的に進行します。

※`sengoku-play-ikuo` スキルを使えば某EMがアシスタントとして
上記ツール群を適切な順序で呼び出して対話的に進行します。
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

- **`edition2024` 関連のビルドエラー** — Rust が古い可能性があります。`rustup update` で
  1.85 以上に更新してください。
- **`pnpm: command not found`** — `corepack enable` を実行したか確認してください。
- **MCPクライアントがサーバーに接続できない** — `.mcp.json` の `--manifest-path` が
  クローン先の絶対パスを指しているか確認してください。初回はビルドに時間がかかるため、
  事前に `cargo build --release -p mcp-server` を済ませておくと安定します。
- **「大名が選択されていません」と表示される** — 最初に `select_daimyo` を実行する必要があります。
