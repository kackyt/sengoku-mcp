## 1. ドメインモデルとDTOの準備

- [x] 1.1 `SeasonalEvent` Enum と `SeasonalEventEffect` 構造体の定義（領地ID、イベント種別、被害・増加量を保持）。
- [x] 1.2 `Kuni` モデルに、災害やイベントによるパラメータ変更を一括適用するメソッドを追加。
- [x] 1.3 既存の `Amount` や `Rate` の操作が不足している場合、計算用のヘルパーを追加。

## 2. SeasonalEventService の実装

- [x] 2.1 `engine/src/domain/service/seasonal_event_service.rs` を新規作成。
- [x] 2.2 人口増加と「金」生成（春）のロジック実装。
- [x] 2.3 「米」生成（秋）のロジック実装。
- [x] 2.4 洪水 (Flood) の夏季限定発生ロジックの実装。
- [x] 2.5 疫病 (Plague) の通年発生ロジックの実装。
- [x] 2.6 反乱 (Rebellion) の忠誠度依存発生ロジックの実装。
- [x] 2.7 季節イベントの結果を詳細に保持する DTO の実装。

## 3. TurnService と UseCase の更新

- [x] 3.1 `TurnService` をリファクタリングし、`SeasonalEventService` を利用するように変更。
- [x] 3.2 `ProgressTurnUseCase` が季節イベント結果を受け取り、アプリケーション層に通知するための DTO を更新。
- [x] 3.3 関連する単体テストの作成と、既存テストの修正。

## 4. 仕上げと検証

- [x] 4.1 全体を通したインテグレーションテストを実行し、意図した確率・タイミングでイベントが発生することを確認。
- [x] 4.2 `cargo clippy --all-targets --all-features -- -D warnings` の実行。
- [x] 4.3 `cargo fmt --all` の実行。
