## ADDED Requirements

### Requirement: 攻撃側CPUの戦略的戦術選択
CPUが攻撃側として合戦に参加する場合、戦況に応じた確率的な戦術選択を行わなければならない (MUST)。

#### Scenario: 基本の重み付き抽選
- **WHEN** 攻撃側CPUが戦術を選択する時
- **THEN** システムは Normal=40, Surprise=30, Fire=20, Inspire=10 を基本重みとして抽選する

#### Scenario: 敵の兵糧が少ない場合
- **WHEN** 防衛側のkomeが防衛側のheiの50%未満の時
- **THEN** Fireの重みに20を加算する（兵糧とどめを狙う）

#### Scenario: 攻撃的な性格の場合
- **WHEN** military_bias が 1.2 を超える時
- **THEN** Surpriseの重みに 15 を加算する

### Requirement: 防衛側CPUの戦略的戦術選択（アンチ予測型）
CPUが防衛側として合戦に参加する場合、攻撃側の戦術を予測してカウンターを選択しなければならない (MUST)。防衛側はRetreatを選択できない。

#### Scenario: 予測に基づくアンチ選択
- **WHEN** 防衛側CPUが戦術を選択する時
- **THEN** システムは攻撃側が Normal/Surprise/Fire を使う確率（各33.3%初期値）を戦況から推定し、重み付き抽選で「最も警戒すべき戦術」を予測し、そのアンチ戦術（NormalにはNormal、SurpriseにはSurprise、FireにはFire）を選択する

#### Scenario: 自軍の兵糧が少ない場合の火計警戒
- **WHEN** 防衛側のkomeがheiより少ない時
- **THEN** Fireを警戒する重みに 20 を加算する

#### Scenario: 敵兵力が強い場合の奇襲警戒
- **WHEN** 攻撃側の兵力が防衛側の兵力を上回る時
- **THEN** Surpriseを警戒する重みに 20 を加算する
