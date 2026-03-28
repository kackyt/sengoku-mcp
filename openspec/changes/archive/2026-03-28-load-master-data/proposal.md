## Why

ゲームルール上、戦争と輸送は**隣接する国に対してのみ**実行可能です。
現在の `BattleUseCase` および `DomesticUseCase::transport` には隣接チェックが存在しないため、非隣接国への戦争・輸送が誤って可能になっています。

`neighbor.csv` から生成した隣接マップをシステム全体で参照可能にすることで、この制約を実装します。

## What Changes

- `static/master_data/` のCSVを読み込むローダー機能の追加
  - `kuni.csv`: 12国の `Kuni`・`Daimyo` オブジェクトを生成
  - `neighbor.csv`: 隣接マップ（`HashMap<KuniId, Vec<KuniId>>`）を構築
- 隣接マップを保持・参照するためのリポジトリトレイト `NeighborRepository` を `engine/domain` に追加
- `BattleUseCase` に隣接チェックを追加（非隣接国への戦争を禁止）
- `DomesticUseCase::transport` に隣接チェックを追加（非隣接国への輸送を禁止）

## Capabilities

### New Capabilities

- `load-master-data`: `static/master_data/` のCSVを読み込み、`Daimyo`・`Kuni`・隣接マップをオンメモリに展開するローダー機能
- `adjacency-constraint`: 戦争・輸送を隣接国に限定するドメインルールの適用

### Modified Capabilities

- `execute-battle`: 非隣接国への攻撃開始が禁止になる（新しいバリデーションの追加）
- `manage-domestic`: 非隣接国への輸送が禁止になる（新しいバリデーションの追加）

## Impact

- `infrastructure/src/`：`MasterDataLoader` を追加
- `engine/src/domain/repository/`：`NeighborRepository` トレイトを新設
- `engine/src/domain/error.rs`：`DomainError::NotAdjacent` バリアントを追加
- `engine/src/application/usecase/battle_usecase.rs`：隣接チェックを追加
- `engine/src/application/usecase/domestic_usecase.rs`：輸送の隣接チェックを追加
