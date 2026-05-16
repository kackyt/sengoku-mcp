## ADDED Requirements

### Requirement: 出兵判断
CPUは毎ターンの行動選択時、隣接国の兵力と自国兵力を比較して出兵可否を判断しなければならない (MUST)。

#### Scenario: 出兵候補の抽出
- **WHEN** CPUの行動番になった時
- **THEN** システムは各隣接国に対し `required_hei = enemy_hei * 1.25 / military_bias` を計算し、自国兵力がこれを上回る国を攻撃候補とする

#### Scenario: 攻撃候補なし
- **WHEN** 自国兵力が `required_hei` を上回る隣接敵国が存在しない時
- **THEN** CPUは攻撃を選択せず、内政行動の候補のみで判断を継続する

#### Scenario: 複数攻撃候補からのターゲット選択
- **WHEN** 攻撃候補が複数存在する時
- **THEN** システムは各候補について `score = win_prob^(1/military_bias) * (kokudaka + machi)` を計算し、スコア最大の国を攻撃対象とする

#### Scenario: 最終出兵判断
- **WHEN** 攻撃対象が確定した時
- **THEN** システムは `threshold = 100.0 - (final_chance * military_bias)` (final_chanceは70±10) を算出し、勝率がこれを上回る場合にのみ出兵を決定する

### Requirement: 出兵量の決定
CPUが攻撃を選択した場合、自国兵力の最大80%を出兵量として決定しなければならない (MUST)。

#### Scenario: 出兵兵力の算出
- **WHEN** 出兵が決定した時
- **THEN** 出兵兵力は `自国兵力 * 0.8` とし、兵糧（kome）の在庫を上限とする

#### Scenario: 兵糧の決定
- **WHEN** 出兵兵力が確定した時
- **THEN** 出兵兵糧は出兵兵力と同量とする
