# mcp-ai-debugger Specification

## Requirements

### Requirement: AI思考プロセスの可視化
システムは、CPUがなぜその行動を選択したか（スコアリング結果）をMCP経由で提供しなければならない。

#### Scenario: 思考理由の閲覧
- **WHEN** ユーザーが `get_action_log` などのMCPツールを実行した時
- **THEN** CPUのアクションに対して「隣接敵国の兵力：XXXのため雇用を選択」といった詳細な理由が返される

### Requirement: ターン自動進行
デバッグを高速化するため、プレイヤーの入力を待たずに指定ターン数分、全CPUを自動進行できなければならない。

#### Scenario: 10ターン一括進行
- **WHEN** MCPツール `advance_turns` を引数 `count: 10` で実行した時
- **THEN** ゲームが10ターン分自動で進行し、最終的な各国の状態が返される
