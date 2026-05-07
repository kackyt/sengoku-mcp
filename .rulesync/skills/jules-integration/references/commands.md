# Jules コマンドリファレンス (MCP対応版)

Julesは、分離されたクラウド環境でタスクを実行する自律型AIコーディングエージェントです。Antigravityからは、高度な制御が可能な **MCP ツール** を優先して使用します。

## 1. セッション管理 (MCPツール優先)

### セッションの新規作成
Julesにタスクを依頼します。詳細なパラメータ指定が可能です。
- **ツール**: `mcp_jules_create_session`
- **主要パラメータ**:
    - `prompt`: タスクの詳細な指示。
    - `branch`: 開始ブランチ名（現在のブランチを `git rev-parse --abbrev-ref HEAD` 等で取得して指定）。
    - `autoPr`: `true` (完了時に自動的にプルリクエストを作成)。
    - `repo`: 対象リポジトリ名 (例: `kackyt/mahjong-ai-server`)。

### セッション状態の監視
- **ツール**: `mcp_jules_get_session_state`
    - セッションIDを指定して、現在のステータス（`busy`, `stable`, `failed`）や最新のアクティビティを確認します。
    - `lastAgentMessage` や `pendingPlan` がある場合、それに応じて返信（`mcp_jules_send_reply_to_session`）を行います。

### セッション一覧の表示
- **ツール**: `mcp_jules_list_sessions`
    - 最近のセッション一覧をリストアップします。

## 2. 作業結果のレビュー (MCPツール)
成果物をローカルに取り込む前に、必ず内容をレビューしてください。

- **ファイル一覧の確認**: `mcp_jules_get_code_review_context`
- **コード差分の精査**: `mcp_jules_show_code_diff`

## 3. 成果物の取り込み (CLI)
作業結果をローカルリポジトリに適用します。この操作は CLI を使用します。

### 方法A: Teleport (推奨)
既存のリポジトリにセッションのパッチを直接適用します。
```bash
pnpm jules teleport <SESSION_ID>
```

### 方法B: Remote Pull
```bash
pnpm jules remote pull --session <SESSION_ID> --apply
```

---

## 運用ルール (Antigravity用)
1. **ブランチ指定**: `mcp_jules_create_session` 呼び出し時には、必ず `--branch` パラメータに現在の作業ブランチを含めること。
2. **レビューフロー**: `Completed` 到着後、すぐに `teleport` せず、まず `mcp_jules_show_code_diff` で内容を精査し、ユーザーに要約を報告すること。
3. **対話**: Julesがプランを提示した場合は、 `mcp_jules_send_reply_to_session` で承認またはフィードバックを返し、自律的に作業を完結させること。
