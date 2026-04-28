## Why

内政コマンド実行後の数値変化（特に忠誠度の増減）が、PRDの想定やゲームバランスと比較して過大になっている。
現状のコードでは、兵力の徴募（雇用）を行うと忠誠度が急激に低下し、逆に施しを行うと一気に最大まで回復してしまう問題がある。これは `INTERNAL_SCALE` (BIAS) の扱いが、`Amount` 型（金・米・兵など）と `Rate` 型（忠誠度）で混同されていることが原因である。

## What Changes

- `Kuni` ドメインモデルにおける各内政コマンドの計算式を修正し、PRDの意図に沿ったバランスに調整する。
- **[Refactor] 内部計算単位と表示単位の分離**:
    - 内部計算用の `Amount` と表示用の `DisplayAmount` を明確に分離し、型安全性を向上させる。
    - `INTERNAL_SCALE` (BIAS) を 10 から 100 に変更し、計算精度を向上させる。
- 特に、忠誠度（0-100の範囲）の増減が、投入量に対して適切（1/10 程度のスケール）になるように修正する。
- `recruit_troops`, `dismiss_troops`, `give_charity` のロジックを修正。
- `DomesticUseCase` および `BattleUseCase` のシグネチャを `DisplayAmount` を受け取るように変更。

## Capabilities

### Modified Capabilities
- `manage-domestic`: 忠誠度の増減値を適切なスケール（表示単位に基づいた値）に修正。

## Impact

- `engine/src/domain/model/kuni.rs`: 内政コマンドの計算ロジックの修正。
- `engine/src/application/usecase/tests.rs`: 期待値の検証を追加・修正。
- `engine/src/domain/model/tests.rs` (存在する場合): ドメインモデルの単体テスト修正。
