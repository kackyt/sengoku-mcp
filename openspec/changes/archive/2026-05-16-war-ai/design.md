## Context

現在の `CpuActionDecisionService::decide()` は内政行動のみを対象としており、`CpuActionDecision::Battle` は型として存在するが候補リストに含まれていない。また `BattleUseCase` はプレイヤーが攻撃側になるユースケースのみ対応しており、CPUが攻撃してプレイヤーが防衛側になるシナリオは未実装である。

本変更では、CPU戦略AIを3層に分けて実装する：
1. **出兵判断** (`WarDecisionService` 新設) - 隣接国と比較して攻撃可否を決定
2. **自動決着** (`BattleService::auto_resolve`) - CPU同士の合戦を同ターン内に解決
3. **防衛フェーズ** (`BattleUseCase::execute_defense_turn`) - プレイヤーが防衛側として参加

## Goals / Non-Goals

**Goals:**
- CPUが内政と合戦を自律的に選択し、隣接国への出兵判断を行う
- CPU同士の合戦が最大10ターンで自動決着し、結果をKuniに反映する
- CPUがプレイヤーに攻め込んだ時、プレイヤーが防衛側として戦術を選択できる
- 攻撃側・防衛側ともに戦況を読んだ確率的な戦術選択AIを持つ
- `military_bias` によって大名の攻撃積極性と出兵量が変化する

**Non-Goals:**
- 複数の敵から同時に攻め込まれるシナリオへの対応（1ターン1合戦のみ）
- プレイヤー自身が複数の国を保有している場合の国選択UI
- 合戦結果の詳細アニメーション表示（ログ記録のみ）

## Decisions

### Decision 1: 出兵判断を独立した Domain Service に切り出す

`CpuActionDecisionService` は単一の `Kuni` 情報しか受け取らないが、出兵判断には隣接国リスト（`Vec<Kuni>`）が必要。既存サービスの引数を大幅変更するより、`WarDecisionService` を新設して責務を分離する。

### Decision 2: 出兵判断閾値は military_bias に連動させる

`attack_threshold(%) = clamp(80.0 / military_bias, 60.0, 95.0)`

- `military_bias=1.0` → 閾値80%（自国兵力の80%で勝てる相手を攻める）
- `military_bias=0.5` → 閾値95%（ほぼ確実でないと動かない）
- `military_bias=1.0` が最大想定値のため、下限は60%でほぼ到達しない

### Decision 3: ターゲット選択は期待値スコアで行う

複数の攻撃候補がある場合、以下のスコアで選択する：

```
win_prob = clamp(dispatched_hei / enemy.hei, 0.0, 1.0)
strategic_value = enemy.kokudaka + enemy.machi
score = win_prob ^ (1.0 / military_bias) * strategic_value
```

`military_bias` が高いほど `win_prob` のべき乗が小さくなり、価値の高い国を勝率が低くても狙いやすくなる。

### Decision 4: 出兵量は military_bias に比例した上限で決定

```
base_rate = 50 + (military_bias / 1.0) * 30  → clamp [50, 80]
rate = clamp(base_rate + rng(-10..=10), 50, 80)
dispatched_hei = my_hei * rate%
dispatched_kome = min(dispatched_hei, my_kome / 2)
```

兵糧は「兵力を超えない」かつ「在庫の半分以下」を上限とする。

### Decision 5: CPU自動決着は同ターン内で同期的に実行

CPU攻撃側が別CPUの国を攻める場合、`BattleService::auto_resolve()` を `execute_cpu_action()` 内で同期的に実行する。最大10ターン、決着なしは攻撃側退却（防衛成功）。自動決着の戦術はNormal/Surprise/Fireを1/3ずつランダムで選択する。

### Decision 6: 戦略的戦術AIは確率ベースで状況加重

ルールベースで確定させるとプレイヤーに読まれる。状況が重みに影響するが必ずノイズを加えた確率的抽選にする。

**攻撃側AI**:
```
base: Normal=40, Surprise=35, Fire=25
+ enemy.kome低い(<enemy.hei*0.5): Fire+=25
+ 自軍優勢(>1.2): Surprise+=military_bias*10
+ 自軍劣勢(<0.8): Surprise-=15, Normal+=15
+ 各重みにrng(-15..=15)のノイズ
→ 重み付き抽選
```

**防衛側AI**（アンチ予測型、退却不可）:
```
Fire脅威 = 20 + (1.0 - 自kome比率) * 40
Surprise脅威 = 20 + enemy_strength_ratio * 20
各脅威にrng(-20..=20)のノイズを加算
→ 予測した攻撃戦術のアンチを選択
  予測Fire     → 防衛Fire
  予測Surprise → 防衛Surprise
  予測Normal   → 防衛Normal
```

### Decision 7: execute_defense_turn() を BattleUseCase に追加

CPUが攻撃側の場合、プレイヤーは `find_by_defender()` で自分が防衛側の `WarStatus` を検索し、`execute_defense_turn(my_kuni_id, tactic)` を呼ぶ。内部でCPUの攻撃戦術を戦略AIで決定し、1ターン計算する。

### Decision 8: BattleParticipationService による判定の共通化

プレイヤーが攻撃側、防衛側、あるいは不参加であるかの判定ロジックが各所に散らばるのを防ぐため、`BattleParticipationService` に集約する。これにより、UIでの表示切り替えや侵攻検知のロジックが簡素化される。

### Decision 9: TacticValidationService による戦術選択の整合性確保

プレイヤーやAIが選択した戦術が、現在の兵力や資源状況に照らして有効かどうかを `TacticValidationService` で一括して検証する。

### Decision 10: GameLifecycleUseCase による初期化・リセットの統合

ゲームの起動時、リセット時、あるいはテスト時の初期データロード手順を `GameLifecycleUseCase` にカプセル化する。各リポジトリのクリア順序やマスターデータのロード順序を保証する。

### Decision 11: CPU出兵のログ記録とCLIでの状態遷移

- **ログ記録**: `execute_cpu_action` 内で出兵が決定された直後、内政ログ (`ActionLog`) に「[国名]が[対象国名]へ出陣しました！」というエントリを追加する。
- **CLIの状態遷移**: プレイヤーが防衛側となる `WarStatus` が存在する場合、CLIのメインループまたはターン更新処理においてそれを検知し、専用の「侵攻通知モーダル」を表示した上で、自動的に合戦画面 (Battle Mode) へ遷移させる。

## Risks / Trade-offs

- **リスク**: CLIでの自動遷移が予期せぬタイミングで発生し、プレイヤーの操作を妨げる → ターン更新（進める）ボタンを押した直後のタイミングで判定することで、操作中の割り込みを避ける。
- **リスク**: CPU自動決着で大量のターン計算がブロッキング実行される → 最大10ターンで上限があるため許容範囲内
- **トレードオフ**: 出兵量のランダム幅により期待より少ない兵力で出兵することがある → ゲーム性として許容
- **リスク**: 戦術AIのノイズが大きすぎると非合理な行動が増える → ノイズ幅を±15〜20に制限することで緩和

## Migration Plan

1. `engine` クレートの変更（Service/Repository/UseCase）をビルド・テスト
2. `infrastructure` クレートの `BattleRepository` 実装に `find_by_defender()` を追加
3. `mcp-server` の presentation レイヤーに防衛ツールを追加
4. 既存テスト（`tests.rs`, `turn_progression_usecase_test.rs`）が通ることを確認

## Open Questions

なし（探索フェーズで全論点を確認済み）
