## MODIFIED Requirements

### Requirement: 資源の減算と保持
SHALL: 合戦に投入された資源は、出撃元の国の在庫から即座に差し引かれ、合戦中の軍勢ステータスとして管理されなければならない。CPUが攻撃側として自動出兵する場合も、同様に資源を消費してWarStatusを初期化しなければならない。

#### Scenario: 国の資源変動
- **WHEN** 資源入力を確定して合戦を開始したとき
- **THEN** 出撃元の国の「兵」と「米」が入力分だけ減少する
- **AND** 合戦画面にその資源量が「軍勢のステータス」として表示される

#### Scenario: CPU自動出兵時の資源消費
- **WHEN** CPUが出兵判断により攻撃を選択した時
- **THEN** 攻撃側CPUの国から dispatched_hei と dispatched_kome が即座に差し引かれ、WarStatusの attacker.hei と attacker.kome として設定される
