## 1. Domain Model & Repository Interface

- [x] 1.1 `engine/src/domain/model/action_log.rs` を作成し、`ActionLogEntry`, `ActionLogCategory`, `ActionLogVisibility` を定義する。
- [x] 1.2 `engine/src/domain/repository/action_log_repository.rs` に `ActionLogRepository` trait を定義する（`save`, `find_visible`, `find_all`, `clear`）。

## 2. Infrastructure Implementation

- [x] 2.1 `infrastructure/src/persistence/in_memory_action_log_repository.rs` を作成し、`VecDeque` によるカテゴリ別リングバッファ（Domestic:200件、War:100件）で実装する。

## 3. Application: 内政コマンドへのログ記録

- [x] 3.1 `DomesticUseCase` の各メソッド（`sell_rice`, `buy_rice`, `develop_land`, `build_town`, `recruit`, `dismiss`, `give_charity`, `transport`, `set_delegation`）の完了後に、`Domestic / Player` ログを記録する。
- [x] 3.2 `TurnProgressionUseCase::execute_cpu_action` の完了後に、`Domestic / Internal` ログを記録する。

## 4. Application: ターン進行・季節イベントへのログ記録

- [x] 4.1 `TurnProgressionUseCase::finish_turn` 内の `TurnService::process_season` 呼び出し前後に、ターン開始・季節イベント（人口増加・資源生成・洪水・疫病・反乱）の `Domestic / Public` ログを記録する。
- [x] 4.2 ターン開始ログに季節名（例：「第1ターン（春）」）を表示する。
- [x] 4.3 季節イベント（収穫、資金増加など）を種類別に集約してログ出力する。
- [x] 4.4 特定の国で発生する災害（洪水、疫病、反乱）については、発生場所を国名で列挙して表示する。

## 5. Application: 合戦へのログ記録

- [x] 5.1 `BattleUseCase::start_war` の先頭で `ActionLogRepository::clear(War)` を呼び出し、`War / Public` の合戦開始ログを記録する。また、`Domestic / Public` にも侵攻開始を記録する。
- [x] 5.2 `BattleUseCase::execute_battle_turn` の各ターン計算後に、プレイヤー側の与ダメージ・被ダメージを `War / Player` として記録し、CPUの策を `War / Internal` として記録する。
- [x] 5.3 被害（兵力の減少）は表示用単位（`DisplayAmount`）で表示し、使用した戦術名をログに含める。
- [x] 5.4 合戦決着（占領/防衛成功）時に `War / Public` および `Domestic / Public` に結果ログを記録する。

## 6. CLI Implementation

- [x] 6.1 `cli/src/ui.rs` に `render_action_log` 関数を追加し、`find_visible` の結果をリスト表示する。
- [x] 6.2 内政画面・合戦画面のレイアウトを修正し、下部にログ表示領域（高さ12行程度）を確保する。
- [x] 6.3 ターミナルの高さが20行以下の場合、ログ領域を非表示にするレスポンシブ制御を実装する。
- [x] 6.4 アクションログがコマンドメニューや戦術選択などのポップアップの背面に描画されるように表示順序を修正する。

## 7. Testing & Verification

- [x] 7.1 `ActionLogRepository`（`InMemory`実装）の単体テストを実装する（save, find_visible, clear の動作確認）。
- [x] 7.2 `DomesticUseCase` のテストを修正し、ログが記録されることを確認する。
- [x] 7.3 `BattleUseCase::start_war` のテストで、合戦開始時に `War` ログがクリアされることを確認する。
- [x] 7.4 `cargo clippy` および `cargo test` が正常終了することを確認する。

## 8. Refactoring: 型安全なログ記録（ActionLogEventの導入）

- [x] 8.1 `ActionLogEntry` から String 型の `message`, `detail` を削除し、`ActionLogEvent` (Discriminated Union) を保持するように変更。
- [x] 8.2 `DomesticUseCase` および `BattleUseCase` のログ出力箇所を、各イベントバリアントを生成するように修正。
- [x] 8.3 CLI 側の表示ロジックを `ActionLogRenderer` として抽出し、イベント型から表示用文字列を生成するように分離。
- [x] 8.4 リファクタリング後の全テスト（23個）が正常にパスすることを確認。
