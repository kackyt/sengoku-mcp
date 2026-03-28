## ADDED Requirements

### Requirement: kuni.csvからの国・大名データ読み込み
システムは起動時に `static/master_data/kuni.csv` を読み込み、全12国の `Kuni` ドメインオブジェクトおよびそれに対応する `Daimyo` ドメインオブジェクトを生成しなければならない。

CSVの各列: `ID, 名前, 初期大名, 金, 兵, 米, 人口, 石高, 町, 忠誠`

#### Scenario: 正常なkuni.csvの読み込み
- **WHEN** `static/master_data/kuni.csv` が正しいフォーマットで存在する状態でアプリケーションが起動したとき
- **THEN** 12件の `Kuni` オブジェクトと対応する `Daimyo` オブジェクトが生成される
- **THEN** 各 `Kuni` は `名前, 金, 兵, 米, 人口, 石高, 町, 忠誠` が正しくセットされている
- **THEN** 各 `Kuni.daimyo_id` は、`初期大名` 列の名前で生成された `Daimyo` のIDを参照している

#### Scenario: kuni.csvが不正なフォーマットの場合
- **WHEN** `kuni.csv` に数値以外の値が数値フィールド（金など）に含まれている状態で起動したとき
- **THEN** システムは `MasterDataError::ParseError` を返し、起動を中止する
- **THEN** エラーメッセージは該当の行番号とフィールド名を含む

#### Scenario: kuni.csvが存在しない場合
- **WHEN** `static/master_data/kuni.csv` が存在しない状態で起動したとき
- **THEN** システムは `MasterDataError::FileNotFound` を返し、起動を中止する

---

### Requirement: neighbor.csvからの隣接マップ構築
システムは `static/master_data/neighbor.csv` を読み込み、`KuniId` 同士の隣接関係マップを `NeighborRepository` 経由で参照可能にしなければならない。

CSVの各行は `ID1, ID2` の形式で、双方向の隣接関係を意味する。

#### Scenario: 正常なneighbor.csvの読み込み
- **WHEN** `static/master_data/neighbor.csv` が正しいフォーマットで存在する状態で起動したとき
- **THEN** neighbor.csvの各ペアが双方向に展開された隣接マップが構築される（ID1→ID2 かつ ID2→ID1）
- **THEN** `NeighborRepository::are_adjacent` で隣接判定ができる状態になる

#### Scenario: neighbor.csvにkuni.csv未登録のIDが含まれる場合
- **WHEN** `neighbor.csv` に `kuni.csv` に存在しないIDが記載されている状態で起動したとき
- **THEN** システムは `MasterDataError::InvalidReference` を返し、起動を中止する

## MODIFIED Requirements

### Requirement: 内政コマンドによる資源管理
システムは、領土と資源を管理するためのすべての内政コマンドを提供しなければならない (MUST)。

#### Scenario: 米売り
- **WHEN** プレイヤーが指定した米の量で米売りコマンドを実行した時
- **THEN** 米が減少し、金がランダムな割合で増加する

#### Scenario: 米買い
- **WHEN** プレイヤーが指定した金の量で米買いコマンドを実行した時
- **THEN** 金がランダムな割合で減少し、米が増加する

#### Scenario: 開墾
- **WHEN** プレイヤーが指定した金の量で開墾コマンドを実行した時
- **THEN** 金が減少し、石高（領地の価値）が増加する

#### Scenario: 町造り
- **WHEN** プレイヤーが指定した金の量で町造りコマンドを実行した時
- **THEN** 金が減少し、町（都市数）が増加する

#### Scenario: 雇用
- **WHEN** プレイヤーが雇用コマンドを実行した時
- **THEN** 金、人口、忠誠度が減少し、兵力が増加する

#### Scenario: 解雇
- **WHEN** プレイヤーが解雇コマンドを実行した時
- **THEN** 兵力が減少し、忠誠度と人口が増加する

#### Scenario: 施し
- **WHEN** プレイヤーが施しコマンドを実行した時
- **THEN** 米が減少し、忠誠度が上昇する

#### Scenario: 輸送（隣接チェックあり）
- **WHEN** プレイヤーが隣接する自国領地へ資源（兵・金・米）の輸送を指定した時
- **THEN** 指定した資源が元の領地から減少し、対象の領地へ増加する

#### Scenario: 非隣接国への輸送の拒否
- **WHEN** プレイヤーが隣接していない領地への輸送を指定した時
- **THEN** システムは `DomainError::NotAdjacent` を返し、輸送を実行しない

#### Scenario: 委任と解任
- **WHEN** プレイヤーが自国領地を委任、または委任解除した時
- **THEN** その領地の管理がCPUによる自動管理、またはプレイヤーの直接管理に切り替わる

#### Scenario: 情報
- **WHEN** プレイヤーが他国の情報コマンドを実行した時
- **THEN** 他国のステータス情報が表示される

### Requirement: 戦闘処理
システムは、選択された戦術、軍の規模、および士気に基づいて戦闘ターンの消費と結果を計算しなければならない (MUST)。

#### Scenario: 非隣接国への攻撃の拒否
- **WHEN** 攻撃側が隣接していない国を攻撃対象として指定した時
- **THEN** システムは `DomainError::NotAdjacent` を返し、戦闘を開始しない

#### Scenario: 通常戦術
- **WHEN** 攻撃側が「通常」を選択した時
- **THEN** 防御側の戦術に応じてダメージが変動する（通常防御に対しては標準ダメージ、その他に対しては低ダメージ）

#### Scenario: 奇襲戦術
- **WHEN** 攻撃側が「奇襲」を選択した時
- **THEN** 奇襲防御以外に対しては高い効果を発揮し敵の士気を下げるが、奇襲防御に対しては効果が低く自軍の士気が低下する

#### Scenario: 火計戦術
- **WHEN** 攻撃側が「火計」を選択した時
- **THEN** 火計防御以外に対しては敵の食料を大幅に減らすが、火計防御に対しては自軍の兵力が減少し士気が低下する

#### Scenario: 鼓舞戦術
- **WHEN** 防御側が「鼓舞」を選択した時
- **THEN** 自軍の士気が上昇する

#### Scenario: 戦闘ターンの消費
- **WHEN** 戦闘で1ターンが経過する時
- **THEN** 投入した兵力の30%の食料を消費し、食料が不足した場合は士気が大幅に低下する

#### Scenario: 戦闘の決着
- **WHEN** 軍の兵力、食料、または士気のいずれかが0以下になった場合
- **THEN** 戦闘が終了し、攻撃側が勝利した場合は領土を獲得し残存資源を吸収、敗北した場合は投入資源を失う
