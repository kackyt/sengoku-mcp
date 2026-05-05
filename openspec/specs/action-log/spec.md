# action-log Specification

## Purpose
TBD - created by archiving change action-log-display. Update Purpose after archive.
## Requirements
### Requirement: domestic-action-logging
MUST: システムは、プレイヤーが実行した内政コマンドを `Domestic / Player` として記録しなければならない。

#### Scenario: log-sell-rice
- **WHEN** プレイヤーが売米コマンドを実行したとき
- **THEN** 米売却の詳細情報（国名、得られた金、売却量、残量）を含む `DomesticLogEvent::RiceSold` データが `Domestic / Player` として保存される

#### Scenario: log-develop-land
- **WHEN** プレイヤーが開墾コマンドを実行したとき
- **THEN** 開墾の詳細情報（国名、石高増加量、投資額、新忠誠度）を含む `DomesticLogEvent::LandReclaimed` データが `Domestic / Player` として保存される

#### Scenario: log-build-town
- **WHEN** プレイヤーが町作りコマンドを実行したとき
- **THEN** 町作りの詳細情報（国名、町ランク増加量、投資額、新忠誠度）を含む `DomesticLogEvent::TownDeveloped` データが `Domestic / Player` として保存される

#### Scenario: log-recruit-troops
- **WHEN** プレイヤーが徴募コマンドを実行したとき
- **THEN** 徴募の詳細情報（国名、徴募量、兵数残量、人口残量、新忠誠度）を含む `DomesticLogEvent::TroopsDrafted` データが `Domestic / Player` として保存される

#### Scenario: log-give-charity
- **WHEN** プレイヤーが施しコマンドを実行したとき
- **THEN** 「{国名}：施しを行い、忠誠度が{gain}上昇しました」というメッセージとともに `Domestic / Player` ログが保存される

#### Scenario: log-transport
- **WHEN** プレイヤーが輸送コマンドを実行したとき
- **THEN** 輸送の詳細情報（出発国、到着国、金・兵・米の各輸送量）を含む `DomesticLogEvent::ResourcesTransported` データが `Domestic / Player` として保存される

### Requirement: seasonal-event-logging
MUST: システムは、ターン進行時に発生する季節イベントを `Domestic / Public` として記録しなければならない。

#### Scenario: log-turn-start
- **WHEN** 新しいターンが開始されたとき
- **THEN** 「第nターン（季節名）が始まりました」というメッセージが記録される

#### Scenario: log-summarized-events
- **WHEN** 各地の収穫や人口増加などの全土的なイベントが発生したとき
- **THEN** 「各地で{イベント名}が発生しました」のように、種類ごとに集約された1行のログが保存される

#### Scenario: log-disaster-with-locations
- **WHEN** 洪水、疫病、反乱などの特定の国で発生する災害が発生したとき
- **THEN** 「【災害名】{国名1}, {国名2} で発生しました」のように、発生場所を列挙したログが保存される

### Requirement: cpu-action-internal-logging
SHALL: システムは、CPUの内政・判断をチート防止のため `Internal` として記録しなければならない。

#### Scenario: log-cpu-action-internally
- **WHEN** CPUが開墾・町作り・休息いずれかの行動を実行したとき
- **THEN** `Domestic / Internal` として行動の種類と国ID、変化量が記録されるが、CLIには表示されない

### Requirement: war-action-logging
MUST: システムは、合戦フェーズで発生したイベントを `War` カテゴリとして記録しなければならない。また、侵攻開始と合戦の結末は `Domestic` カテゴリにも記録しなければならない。

#### Scenario: log-war-start
- **WHEN** 合戦が開始されたとき
- **THEN** 「{攻撃国} が {防御国} へ侵攻しました」というログが `War / Public` および `Domestic / Public` の両方に保存される
- **AND** `War` カテゴリのログがリセットされる

#### Scenario: log-battle-turn-player
- **WHEN** 合戦の各ターンが計算されたとき
- **THEN** 両軍の戦術と被害数を含む `WarLogEvent::Damage` データが `War / Player` として保存される
- **AND** 被害の数値は表示用単位（DisplayAmount）でなければならない

#### Scenario: log-battle-result
- **WHEN** 合戦の勝敗が決定したとき（占領または防衛成功）
- **THEN** 合戦の結末を示すメッセージが `War / Public` および `Domestic / Public` の両方に保存される

### Requirement: war-log-reset
SHALL: 戦争ログは、新しい合戦が開始されるたびに消去されなければならない。

#### Scenario: reset-war-log-on-battle-start
- **WHEN** 新しい合戦が開始されたとき
- **THEN** 以前の合戦のログが `War` カテゴリからすべて削除される

### Requirement: domestic-log-persistence
SHALL: 内政ログは、上限件数まで継続して保持されなければならない。

#### Scenario: persistent-domestic-log
- **WHEN** 合戦が終了して内政フェーズに戻ったとき
- **THEN** 合戦前に記録されていた内政ログが引き続き参照できる

