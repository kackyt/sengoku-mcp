## Context

`static/master_data/` に2つのCSVファイルが存在します：

- **`kuni.csv`** (12行): `ID, 名前, 初期大名, 金, 兵, 米, 人口, 石高, 町, 忠誠`
  - `ID` は1〜12の整数（CSVスコープのみの識別子）
  - `初期大名` 列が大名名（蛎崎、伊達、上杉…）
- **`neighbor.csv`** (17行): `ID1, ID2`
  - `kuni.csv` の整数IDを参照する隣接ペアリスト（双方向）

既存のドメインモデルの状況：
- `Kuni` は `KuniId(Uuid)` / `DaimyoId(Uuid)` で識別される
- `Daimyo` は `DaimyoId(Uuid)` と `DaimyoName(String)` のみ
- `BattleUseCase` / `DomesticUseCase::transport` に隣接チェックは存在しない
- 隣接マップを保持するリポジトリや構造は一切未実装

## Goals / Non-Goals

**Goals:**
- `kuni.csv` を読み込み、12国分の `Daimyo` と `Kuni` ドメインオブジェクトを生成する
- `neighbor.csv` を読み込み、`KuniId` 同士の隣接マップ（`HashMap<KuniId, Vec<KuniId>>`）を生成する
- 隣接マップを参照するための `NeighborRepository` トレイトを `engine/domain/repository` に新設する
- `BattleUseCase` / `DomesticUseCase::transport` に隣接チェックを追加し、非隣接国への操作を `DomainError::NotAdjacent` で拒否する
- 読み込みエラー時は `MasterDataError`（`thiserror` 使用）で明確にFail-Fastする

**Non-Goals:**
- CSVのホットリロードや実行中の動的更新
- データベースへの永続化

## Decisions

### CSVの整数IDからUUID IDへのマッピング
**採用: ロード時にUUIDを採番し `HashMap<u32, KuniId>` でマッピング管理する**

`neighbor.csv` の解決はこのマップを通じて行います。シンプルさを優先し、UUID v5による決定論的生成は採用しません。

### 隣接マップの保持場所
**採用: `NeighborRepository` トレイトを `engine/domain/repository` に新設する**

```rust
// engine/src/domain/repository/neighbor_repository.rs
pub trait NeighborRepository: Send + Sync {
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId>;
    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool;
}
```

`BattleUseCase` と `DomesticUseCase` がこのトレイトを依存注入で受け取り、隣接チェックに利用します。

### バリデーションの場所
**採用: Usecase層でチェックし `DomainError::NotAdjacent` を返す**

「隣接かどうか」はゲームルール（ドメインルール）であるため、アプリケーション層（Usecase）でリポジトリを使って検証するのが適切な層となります。

## Risks / Trade-offs

- **[Risk] CSVカラム順・文字コード変更でパースエラー**  
  → Mitigation: 起動時にエラー行・エラー原因をログ出力してFail-Fast

- **[Risk] `初期大名` 列が名前文字列なので同名大名の重複の可能性**  
  → Mitigation: `HashMap<String, Daimyo>` で一意化して処理する
