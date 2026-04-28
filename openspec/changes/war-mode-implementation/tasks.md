## 1. Domain & Application Layer の拡張

- [x] 1.1 `BattleUseCase` に `start_war` メソッドを追加し、出陣元の国から指定された兵数と米を減算する処理を実装
- [x] 1.2 `BattleUseCase::execute_battle_turn` の引数に現在の軍勢の米（兵糧）と士気を追加し、戦闘後の更新値を返すように修正
- [x] 1.3 `BattleService::calculate_turn` の判定ロジックを確認し、兵糧切れや士気0による決着が正しく行われるか検証
- [x] 1.4 必要に応じて `Tactic::Retreat`（退却）を `Tactic` 列挙型に追加

## 2. CLI 画面状態（ScreenState）の定義更新

- [x] 2.1 `ScreenState::War` 構造体に、現在の攻撃軍のステータス（兵数、米、士気）を保持するフィールドを追加
- [x] 2.2 `WarSubState` に `InputHeihe`（兵数入力）と `InputKome`（米入力）を追加
- [x] 2.3 `DomesticSubState::SelectTargetKuni` から合戦開始時の入力状態への遷移を定義

## 3. CLI UI 表示（Rendering）の実装

- [x] 3.1 `cli/src/ui/mod.rs` の `render_war` を更新し、軍勢のステータス（兵・米・士気）をプログレスバーなどで視覚的に表示
- [x] 3.2 合戦開始時の兵数・米入力用のダイアログ表示を実装

## 4. CLI イベントハンドリング（Handler）の実装

- [x] 4.1 `handle_domestic` において、合戦対象国を選択した後に兵数・米の入力を受け付けるロジックを実装
- [x] 4.2 `handle_war` において、1ターン終了後に決着がついていない場合は策選択に戻るループ処理を実装
- [x] 4.3 「退却」コマンドの選択時に、合戦を終了して内政画面に戻る処理を実装
- [x] 4.4 勝利時に、占領した国の資源を吸収する（PRD 252行目）処理の呼び出しを確認

## 5. 追加の修正と品質向上

- [x] 5.1 `BattleUseCase::start_war` において、自領地（同じ大名が支配する国）への攻撃を禁止するバリデーションを実装
- [x] 5.2 `KuniQueryUseCase` に隣接国を所有者でフィルタリングする機能を追加し、UI上で自領地を攻撃対象から除外
- [x] 5.3 手番の同期を修正し、占領直後の領地でコマンドが出せない問題を解消
- [x] 5.4 `cargo clippy` の警告（collapsible_if, redundant_guards）を解消し、コード品質を維持
