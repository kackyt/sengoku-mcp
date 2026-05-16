## ADDED Requirements

### Requirement: プレイヤー防衛フェーズへの移行
CPUがプレイヤーの国に攻め込んだ場合、WarStatusを保存してプレイヤーが防衛側として戦術を選択できる合戦フェーズに移行しなければならない (MUST)。

#### Scenario: CPUからプレイヤーへの攻撃
- **WHEN** CPUが行動選択で攻撃を選択し、対象がプレイヤーの国である時
- **THEN** システムはCPUの出兵兵力・兵糧を消費してWarStatusを保存し、CPUのターンを終了する

#### Scenario: プレイヤーへの侵攻通知とCLI遷移
- **WHEN** プレイヤーが防衛側となる WarStatus が作成された時
- **THEN** アクションログに「○○が□□に攻め込んできた」と記録する
- **AND** CLIは「侵攻モーダル」を表示して合戦開始をプレイヤーに通知し、自動的に合戦画面へ遷移しなければならない

### Requirement: プレイヤーの防衛戦術選択
プレイヤーは防衛側として戦術を選択し、合戦ターンを進行させなければならない (MUST)。

#### Scenario: execute_defense_turn の呼び出し
- **WHEN** プレイヤーが防衛戦術を選択してexecute_defense_turn(my_kuni_id, tactic)を呼ぶ時
- **THEN** システムは find_by_defender() でWarStatusを取得し、攻撃側CPUの戦術を戦略AIで決定した上で1ターンの合戦計算を行い、結果を返す

#### Scenario: 防衛フェーズでの勝敗確定
- **WHEN** 合戦ターンで勝者が確定した時
- **THEN** 攻撃側勝利の場合はプレイヤーの国がCPUに占領され、防衛成功の場合はWarStatusを削除してプレイヤーの国に残存兵力が残る
