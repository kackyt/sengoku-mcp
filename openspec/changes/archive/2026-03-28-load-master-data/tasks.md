## 1. エラー型の定義

- [x] 1.1 `infrastructure` クレートに `MasterDataError`（`thiserror` 使用）を追加する
  - バリアント: `FileNotFound`, `ParseError { line: usize, field: String, reason: String }`, `InvalidReference { id: u32 }`
- [x] 1.2 `engine/src/domain/error.rs` に `DomainError::NotAdjacent` バリアントを追加する

## 2. NeighborRepositoryトレイトの定義

- [x] 2.1 `engine/src/domain/repository/neighbor_repository.rs` を新規作成し、以下のトレイトを定義する
  ```rust
  pub trait NeighborRepository: Send + Sync {
      fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId>;
      fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool;
  }
  ```
- [x] 2.2 `engine/src/domain/repository/mod.rs` に `neighbor_repository` を追加する

## 3. kuni.csvローダーの実装

- [x] 3.1 `infrastructure/src/master_data.rs` を新規作成し、`MasterDataLoader` 構造体を定義する
- [x] 3.2 `kuni.csv` の各行を `KuniRecord { id: u32, name: String, initial_daimyo: String, kin: u32, hei: u32, kome: u32, jinko: u32, kokudaka: u32, machi: u32, tyu: u32 }` にパースする処理を実装する
- [x] 3.3 `KuniRecord` から `Daimyo`・`Kuni` ドメインオブジェクトを生成する処理を実装する
  - 同名大名は `HashMap<String, Daimyo>` で一意化する
  - CSVの整数ID → `KuniId(Uuid)` のマッピングを `HashMap<u32, KuniId>` で保持する

## 4. neighbor.csvローダーの実装

- [x] 4.1 `neighbor.csv` の各行を `(u32, u32)` ペアにパースする処理を実装する
- [x] 4.2 CSVのID → `KuniId` マッピングを使って `HashMap<KuniId, Vec<KuniId>>` 隣接マップを構築する（双方向展開）
- [x] 4.3 kuni.csv未登録のIDが含まれる場合は `MasterDataError::InvalidReference` を返す

## 5. InMemoryNeighborRepositoryの実装

- [x] 5.1 `infrastructure/src/persistence.rs`（または新規ファイル）に `InMemoryNeighborRepository` を追加し、`NeighborRepository` トレイトを実装する

## 6. ローダーの統合インターフェース

- [x] 6.1 `MasterDataLoader::load(base_dir: &Path)` 関数を実装し、両CSVを読み込んで以下を返す: `Vec<Daimyo>`, `Vec<Kuni>`, `InMemoryNeighborRepository`

## 7. 隣接チェックのUsecase適用

- [x] 7.1 `BattleUseCase` に `NeighborRepository` を依存注入し、`execute_battle_turn` の冒頭で隣接チェックを追加する（非隣接なら `DomainError::NotAdjacent` を返す）
- [x] 7.2 `DomesticUseCase` に `NeighborRepository` を依存注入し、`transport` の冒頭で隣接チェックを追加する（非隣接なら `DomainError::NotAdjacent` を返す）

## 8. テスト

- [x] 8.1 正常な `kuni.csv` を読み込んで12件の `Kuni` と対応 `Daimyo` が生成されることを検証する単体テスト
- [x] 8.2 不正フォーマットの `kuni.csv` で `MasterDataError::ParseError` が返ることを検証する単体テスト
- [x] 8.3 正常な `neighbor.csv` を読み込んで双方向の隣接マップが構築されることを検証する単体テスト
- [x] 8.4 `neighbor.csv` に未登録IDが含まれる場合に `MasterDataError::InvalidReference` が返ることを検証する単体テスト
- [x] 8.5 隣接国への戦争が成功し、非隣接国への戦争が `NotAdjacent` エラーになることを検証する単体テスト
- [x] 8.6 隣接国への輸送が成功し、非隣接国への輸送が `NotAdjacent` エラーになることを検証する単体テスト
