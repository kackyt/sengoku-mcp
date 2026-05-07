## Why

プレイヤーが戦略を立てるために、他国の資源量、兵力、開発度、忠誠度を把握する必要があるためです。
これまでは情報を得る手段が限られていたか、あるいは全くなかったため、これをコマンドとして実装します。
コマンド実行権を消費することで、情報の入手が戦略的なリソース管理の一環となります。

## What Changes

- 全ての他国の情報（米、金、兵、石高、町、忠誠度）を一覧表示する「情報コマンド」を実装します。
- このコマンドを実行すると、現在のターンのコマンド実行権（行動力）が1消費されます。
- MCP経由でプレイヤーがこの情報を取得できるようにします。

## Capabilities

### New Capabilities
- `info-command`: 他国の詳細情報を一覧表示し、コマンド実行権を消費する機能。

### Modified Capabilities
- `domestic-commands`: 国内コマンドの体系に「情報」を追加。
- `turn-management`: コマンド実行時の行動力消費ロジックの適用。

## Impact

- `engine/src/application/usecase`: 新しいユースケース `InfoUseCase` (または既存の DomesticUseCase への追加)
- `engine/src/domain/model`: 大名や領地の情報取得ロジック
- `mcp-server/src/presentation`: MCPツールとしての「情報」コマンドの定義
- 行動力（アクションポイント）の消費処理
