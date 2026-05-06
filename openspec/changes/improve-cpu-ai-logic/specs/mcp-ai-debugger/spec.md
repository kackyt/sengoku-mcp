# mcp-ai-debugger Specification

## Requirements

### Requirement: AI思考プロセスの可視化
システムは、CPUがなぜその行動を選択したか（スコアリング結果）をMCP経由で提供しなければならない。

#### Scenario: 思考理由の閲覧
- **WHEN** ユーザーが `get_action_log` などのMCPツールを実行した時
- **THEN** CPUのアクションに対して「隣接敵国の兵力：XXXのため雇用を選択」といった詳細な理由が返される

### Requirement: シナリオ再現ツールの提供
開発者は、AIの挙動をテストするために各国の資源状態を強制的に変更できなければならない。

#### Scenario: 資源の強制変更
- **WHEN** MCPツール `set_kuni_resource` を特定の国IDと数値で実行した時
- **THEN** その国の金・兵・米が指定された値に更新される

### Requirement: ターン自動進行
デバッグを高速化するため、プレイヤーの入力を待たずに指定ターン数分、全CPUを自動進行できなければならない。

#### Scenario: 10ターン一括進行
- **WHEN** MCPツール `advance_turns` を引数 `count: 10` で実行した時
- **THEN** ゲームが10ターン分自動で進行し、最終的な各国の状態が返される
