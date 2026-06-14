## Project Conventions

### Code Style
- ソースコードにはロジックの内容がわかるように日本語のコメントをいれること

- 値オブジェクト（Value Object）としての Newtype パターン
Rustのタプル構造体を使って、型安全性を担保します。プリミティブ型（i32やString）の直接利用を避けます。

Rust
// domain/model/unit.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitPoint {
    current: u32,
    max: u32,
}

impl HitPoint {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }
    pub fn damage(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }
}

- `Amount` / `DisplayAmount` の扱い
  - **計算・比較は `Amount` で完結させる**: `.value()` を取り出して生値で計算・比較するのではなく、`Amount` が提供するメソッド（`add`, `sub`, `mul_percent`, `is_zero` 等）や直接比較演算子を使用すること。
  - `Amount::new(0)` ではなく `Amount::zero()` を使用すること。
  - **`DisplayAmount` は表示専用**: `DisplayAmount` は UI レイヤーでの表示、または外部からの入力値の保持にのみ使用する。ドメインロジックや計算の中で `DisplayAmount` が現れる場合は、速やかに `to_internal()` で `Amount` に変換すること。`DisplayAmount` のまま計算を行うことは禁止。
  - **`.value()` の使用制限**: 可能な限り `Amount` 型のままでロジックを記述し、不必要にプリミティブ型へ戻さないこと。

- エンティティの参照は「ポインタ」ではなく「ID」で行う
戦略ゲームでは、「マップ上のこの城はこの大名の領地」という関係性が発生します。ここで Unit の実体（参照）を Map に持たせると、借用チェッカーとの終わりのない戦いが始まります。必ずIDで関連付けを行ってください。

Rust
// domain/model/map.rs
pub struct HexMap {
    pub id: MapId,
    // &Unit ではなく UnitId を保持する
    cells: HashMap<Position, Option<UnitId>>, 
}

- 依存性の注入（DI）にはジェネリクスか dyn Trait を使う
アプリケーション層がインフラ層に依存しないよう、ドメイン層で定義した trait を使います。

静的ディスパッチ（ジェネリクス）: パフォーマンス重視。コンパイル時間は伸びる。(推奨)

動的ディスパッチ（Arc<dyn Trait> または Box<dyn Trait>）: 記述がシンプルになり、モックの差し替えが容易。

Rust
// domain/repository/unit_repository.rs
pub trait UnitRepository: Send + Sync {
    fn find_by_id(&self, id: &UnitId) -> Result<Option<Unit>, DomainError>;
    fn save(&self, unit: &Unit) -> Result<(), DomainError>;
}

// application/usecase/move_unit.rs
pub struct MoveUnitUseCase<R: UnitRepository> {
    unit_repo: R,
}

impl<R: UnitRepository> MoveUnitUseCase<R> {
    // 依存を注入してユースケースを初期化
    pub fn new(unit_repo: R) -> Self { Self { unit_repo } }

    pub fn execute(&self, unit_id: UnitId, dest: Position) -> Result<(), AppError> {
        // 1. リポジトリから再構築
        let mut unit = self.unit_repo.find_by_id(&unit_id)?.unwrap();
        // 2. ドメインのロジックを実行
        unit.move_to(dest)?;
        // 3. 結果を永続化
        self.unit_repo.save(&unit)?;
        Ok(())
    }
}

- エラーハンドリングに thiserror と anyhow を活用する
ドメイン層・インフラ層: thiserror クレートを使って、型安全で明確なカスタムエラー（DomainError, InfraError）を定義します。

アプリケーション層・プレゼンテーション層: 最終的なエラーの集約として anyhow を使うと、スタックトレースやコンテキストの付与が簡単になります。

- `cargo clippy --all-targets --all-features -- -D warnings` がエラーなく通ること
- `cargo fmt --all -- --check` がエラーなく通ること
- `cargo test` が正常に動作すること
- SOLID原則、DRY原則を意識すること
- 依存性の注入(DI)を適切に行うこと
- メモリのライフサイクルを意識すること。無駄なコピーやcloneは避ける。解放タイミングが不明なBoxを定義しない。
- DDDとオニオンアーキテクチャ、ヘキサゴナルアーキテクチャを意識したプログラミングや設計をすること

[参考:DDDとクリーンアーキテクチャをはじめよう-Rust編](https://zenn.dev/poporo/articles/20251011_1_start_ddd_and_clean_architecture_rust)

[参考:DDDのパターンをRustで表現する ~ Value Object編 ~](https://caddi.tech/archives/1373)

### Architecture Patterns
オニオンアーキテクチャを採用する

├─engine
│  └─src
│      ├─application
│      │  ├─dto 入出力用データ構造。
│      │  └─usecase **Application Service (Usecase)**。model内のtraitやserviceを使ってドメインを操作
│      └─domain
│          ├─model  (例: 大名、領地、資源)。所有権を用いて不変条件を保護。
│          ├─repository **Repository Interface (Trait)** (例: `KuniRepository`, `DaimyoRepository`)。
│          └─service **Domain Service** (例: 複数のモデルにまたがる戦闘計算)。
├─infrastructure
│  └─src
│      └─persistence: `engine/domain/repository` で定義された **Repositoryの実装（具体例: ファイルI/O, SQLx等）**。
│          ゲーム外の関心事（外部ストレージとのやり取り）を担当。
├─mcp-server
│  └─src
│      └─presentation : MCPのプロトコルマッピング。
│      └─main.ts : **Composition Root**。ユースケースに infrastructure の具象リポジトリを注入して起動。
├─ static
│  └─ master_data: マスターデータ
└─ Cargo.toml ワークスペース管理

## 構成の再確認
- **Modelはどこ？**: `engine/src/domain/model` にあります。
- **Repositoryはどこ？**: インターフェース（抽象）は `engine/src/domain/repository`、その具象実装は `infrastructure/src/persistence` にあります。
- **Usecaseはどこ？**: `engine/src/application/usecase` にあります。

## レビュー方針

あなたの目的は、非常に経験豊富なソフトウェアエンジニアとして機能し、コードの一部を徹底的にレビューし、
以下のようなキーエリアを改善するためのコードスニペットを提案することです

| 重要度 | チェック観点 | 根拠（rules/project.md） |
|--------|-------------|--------------------------|
| 🔴 CRITICAL | エンティティへの参照をIDではなくポインタ・参照で保持している | ID参照の原則 |
| 🔴 CRITICAL | ドメイン層・インフラ層で `anyhow` を直接使っている（`thiserror` を使うべき） | エラーハンドリング規約 |
| 🔴 CRITICAL | アプリ層がインフラ層の具体型に直接依存している（Trait経由でない） | DI原則 |
| 🔴 CRITICAL | ロジック変更にテストが伴っていない | テスト戦略 |
| 🔴 CRITICAL | **[アーキテクチャ境界違反]** ドメイン層（`engine/src/domain/`）がインフラ層（`infrastructure/`）のモジュールを `use` している | オニオンアーキテクチャ：依存は内側に向かう |
| 🔴 CRITICAL | **[アーキテクチャ境界違反]** ドメイン層がアプリケーション層（`engine/src/application/`）のモジュールを `use` している | オニオンアーキテクチャ：ドメインはすべての層から独立 |
| 🔴 CRITICAL | **[アーキテクチャ境界違反]** Repository の具象実装が `infrastructure/src/persistence/` 以外（例: `engine/src/domain/`）に配置されている | ヘキサゴナルアーキテクチャ：ポート（Trait）はドメイン、アダプタ（実装）はインフラ |
| 🔴 CRITICAL | **[アーキテクチャ境界違反]** Usecase/Application Service が `engine/src/application/usecase/` 以外に実装されている | オニオンアーキテクチャ：Usecaseの配置規約 |
| 🔴 CRITICAL | **[DDDドメインモデル違反]** ビジネスロジック（不変条件の保護、状態遷移）がドメインモデル外（Usecase や infrastructure）に漏れている | DDD：ドメインモデルが不変条件を内包すること |
| 🔴 CRITICAL | **[DDDドメインモデル違反]** 複数のエンティティ/集約にまたがるロジックが Usecase に直接記述されており、Domain Service として分離されていない | DDD：Domain Service（`engine/src/domain/service/`）を使うこと |
| 🔴 CRITICAL | **[DDDドメインモデル違反]** Repository の Interface（Trait） が `engine/src/domain/repository/` 以外に定義されている | DDD：RepositoryインターフェースはDomain層が所有する |
| 🔴 CRITICAL | **[ヘキサゴナルアーキテクチャ違反]** プレゼンテーション層（`mcp-server/src/presentation/`）が Usecase を経由せずドメインモデルや Repository を直接呼び出している | ヘキサゴナル：外部からの入力はUsecase（ポート経由）でのみ受け付ける |
| 🔴 CRITICAL | **[Composition Root違反]** DI（依存の組み立て）が `mcp-server/src/main.rs` 以外で行われている | Composition Root原則 |
| 🟡 HIGH | 日本語コメントがない、または意図がわからないコメントのみ | コードスタイル規約 |
| 🟡 HIGH | 不必要な `.clone()` がある | メモリライフサイクル規約 |
| 🟡 HIGH | `Box<dyn Trait>` のライフサイクルが不明瞭 | メモリライフサイクル規約 |
| 🟡 HIGH | SOLID原則違反（単一責任・開放閉鎖など） | SOLID/DRY原則 |
| 🟡 HIGH | DRY違反（重複ロジック） | SOLID/DRY原則 |
| 🟡 HIGH | プリミティブ型（`i32`, `String` 等）を直接フィールドに使用している | Newtypeパターンの徹底 |
| 🟡 HIGH | **[DDDドメインモデル]** DTO（`engine/src/application/dto/`）とドメインモデルの境界が曖昧（Entityが直接プレゼンテーション層に返される等） | DDD：DTO変換はApplication層で行う |
| 🟡 HIGH | **[DDDドメインモデル]** 集約ルート（Aggregate Root）を経由せずに集約内部のエンティティを外部から直接変更している | DDD：集約の整合性境界の維持 |

重要な問題を特定し、解決して全体的なコード品質を向上させることを目指してください。細かい問題は意図的に無視してください。
