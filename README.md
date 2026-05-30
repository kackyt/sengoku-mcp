# sengoku-mcp
MCPゲー(戦国シミュレーション)

## 開発環境のセットアップ

本プロジェクトはRustで開発されています。以下の手順で開発環境をセットアップしてください。

1. **Rustのインストール**
   [rust install](https://rust-lang.org/tools/install//) などを利用して、Rustおよび `cargo` をインストールします。

※ mac, linuxの場合
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. **Node.jsとパッケージマネージャーのインストール**
  
   `rulesync` などのツールを使用するため、Node.jsと `pnpm` をインストールし、依存関係をセットアップします。
   ```bash
   corepack enable
   pnpm install
   ```

3. **ビルドとテスト**
   Rustのプロジェクトをビルドし、テストを実行して正常に動作することを確認します。
   ```bash
   cargo build -p cli
   ```

## skillの追加方法 (rulesync)

エージェント（AIアシスタント）の挙動を定義するカスタムスキルは、プロジェクト内の `.agent/skills/` ディレクトリ等で管理されています。
スキルの変更や追加を行った場合、`rulesync` コマンドを使用してエージェントに設定を同期させます。

プロジェクトのルートディレクトリで以下のコマンドを実行してください。

```bash
pnpm exec rulesync generate
```

## MCPの追加方法

AIアシスタント（Antigravity, Claude Code, Cursorなど）から本ゲームを操作できるようにするためには、MCPサーバーの登録が必要です。

MCP設定ファイル（例: `mcp.json` や設定画面の `mcpServers` 設定）に、以下のような設定を追加してください。
※プロジェクトのパスは、お使いの環境に合わせて適宜書き換えてください。

```json
{
  "mcpServers": {
    "sengoku": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "/path/to/sengoku-mcp/Cargo.toml",
        "-p",
        "mcp-server"
      ]
    }
  }
}
```

> **Note**: 事前に `cargo build --release -p mcp-server` でビルドを行い、生成されたバイナリファイル（例: `target/release/mcp-server.exe`）の絶対パスを `command` に直接指定することも可能です。

## 遊びかた

MCPサーバーが正常にAIアシスタントへ追加されたら、チャットを通じてゲームをプレイできます。
本プロジェクトには `sengoku-play` スキルが用意されており、対話的に戦国シミュレーションを進行可能です。

AIアシスタントに以下のように話しかけてみてください。

- 「戦国ゲームを始めて」「プレイしたい」
- 「現在の状況を教えて」
- 「お任せで内政してほしい」
- 「ターンを進めて」
- 「隣国に合戦を仕掛けて」
- 「戦略をアドバイスして」

AIがMCPツール（`sengoku` サーバーの提供する各種ツール）を自律的に呼び出し、状況分析からコマンド実行、ターン進行までをサポートしながら一緒に全国統一を目指してくれます。
