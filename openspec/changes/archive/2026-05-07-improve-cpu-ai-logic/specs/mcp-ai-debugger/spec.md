# mcp-ai-debugger Specification

## ADDED Requirements

### Requirement: AI思考プロセスの可視化
MUST: システムは、CPUがなぜその行動を選択したか（スコアリング結果および各アクションの勾配）の詳細な思考理由をMCP経由で提供しなければならない。

#### Scenario: 思考理由の閲覧
- **WHEN** ユーザーが `get_action_log` などのMCPツールを実行した時
- **THEN** CPUのアクションに対して「線形最適化により雇用を選択 (勾配: 1.5, 基準スコア: 500)」といった詳細な理由が返される

### Requirement: ターン自動進行
SHALL: デバッグを高速化するため、プレイヤーの入力を待たずに指定ターン数分、全CPUを自動進行させるMCPツールを提供しなければならない。

#### Scenario: 10ターン一括進行
- **WHEN** MCPツール `advance_turns` を引数 `count: 10` で実行した時
- **THEN** ゲームが10ターン分自動で進行し、最終的な各国の状態が返される
