---
name: jules-integration
description: >-
  Jules AI（Googleの自律型コーディングエージェント）を MCP ツール（jules）を通じて操作し、タスクの提出、状態管理、成果物のレビューを自律的に行います。MCPによってブランチ指定や自動PR作成などの高度な制御が可能になります。
---
# Jules Integration (MCP Priority)

Jules AI（Googleの自律型コーディングエージェント）と連携し、複雑なコーディングタスクを効率的に委譲するためのスキルです。

> [!IMPORTANT]
> **MCP優先原則**: `jules` MCP サーバーが利用可能な環境では、CLI コマンドよりも **MCP ツール（`mcp_jules_create_session` 等）を最優先で使用してください。** これにより、ブランチの明示的な指定や自動 PR 作成などの高度な制御が確実に行えます。

## 基本的なワークフロー

### 1. セッションの作成 (MCP優先)
Julesに新しい作業を依頼します。現在のブランチを自動で付与するのがベストプラクティスです。
- **ツール**: `mcp_jules_create_session`
- **パラメータ例**:
  - `prompt`: "指示内容..."
  - `repo`: "kackyt/mahjong-ai-server"
  - `branch`: "現在のブランチ名" (例: `git rev-parse --abbrev-ref HEAD` の実行結果)
  - `autoPr`: `true` (完了時に自動的にPRを作成)

### 2. 状態の監視
セッションが完了したか、または承認待ちかを確認します。
- **ツール**: `mcp_jules_get_session_state`
- **活用**: Julesがプラン（計画）を提示している場合は、 `mcp_jules_send_reply_to_session` で `action='approve'` を送信して作業を継続させます。

### 3. 作業内容のレビュー
成果物を取り込む前に、MCPツールで変更内容を精査します。
- **ツール**: `mcp_jules_get_code_review_context` (ファイル一覧と変更概要)
- **ツール**: `mcp_jules_show_code_diff` (具体的なコード差分)

### 4. 成果物のローカルへの取り込み (CLI使用)
Julesが作成した変更をローカル環境に適用します。この操作はローカルファイルへの書き込みを伴うため、CLIを使用します。

```bash
# 方法A: Teleport (推奨)
pnpm jules teleport <SESSION_ID>

# 方法B: Remote Pull
pnpm jules remote pull --session <SESSION_ID> --apply
```

---

## 運用上の注意点

- **ブランチの同期**: 新しいセッションを開始する前に、必ず最新の `main` を反映（merge/rebase）させた作業ブランチから開始してください。
- **指示の具体性**: 具体的なファイルパスや型の定義（HECS, Newtypeパターン等の規約）をプロンプトに含めてください。
- **認証**: 初回やセッション切れの際は `pnpm jules login` を実行して、Googleアカウントでのログインを完了させておく必要があります。

## トラブルシューティング
- **CLIツールがエラーを返す**: `jules login` が切れている可能性があります。ターミナルでログイン状態を確認してください。
- **成果物が見つからない**: セッション状態が `Completed` になっていることを確認してください。
