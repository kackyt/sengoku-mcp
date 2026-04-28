## 1. 調査と問題の再現 (Red)

- [x] 1.1 `engine/src/application/usecase/tests.rs` に、雇用・施し時の忠誠度変化を検証するテストケースを追加する
- [x] 1.2 `cargo test` を実行し、忠誠度の変化が過大であることを確認する

## 2. 実装 (Green)

- [x] 2.1 `engine/src/domain/model/kuni.rs` の `recruit_troops` を修正し、忠誠度の減少を表示単位ベースにする
- [x] 2.2 `engine/src/domain/model/kuni.rs` の `dismiss_troops` を修正し、忠誠度の上昇を表示単位ベースにする
- [x] 2.3 `engine/src/domain/model/kuni.rs` の `give_charity` を修正し、忠誠度の上昇値を適切なスケール（表示単位ベース）に調整する

## 3. リファクタリング (Refactor)

- [x] 3.1 `DisplayAmount` 型を導入し、`Amount` と分離する
- [x] 3.2 `INTERNAL_SCALE` を 100 に変更し、計算式をパーセンテージベースに更新する
- [x] 3.3 `DomesticUseCase`, `BattleUseCase` の引数を `DisplayAmount` に変更する
- [x] 3.4 CLI ハンドラおよび UI を `DisplayAmount` に対応させる

## 4. 検証

- [x] 4.1 `cargo test` を実行し、すべてのテストがパスすることを確認する
- [x] 4.2 `cargo clippy --all-targets --all-features -- -D warnings` を実行し、警告がないことを確認する
- [x] 4.3 `cargo fmt --all -- --check` を実行し、フォーマットを確認する
