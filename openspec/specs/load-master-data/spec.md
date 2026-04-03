## Purpose
本システムが外部のCSVファイル（kuni.csv, neighbor.csv）からマスターデータを読み込み、ドメインモデルおよび初期状態を構築するための仕様を定義する。

## Requirements

### Requirement: kuni.csvからの国・大名データ読み込み
MUST: システムは起動時に `static/master_data/kuni.csv` を読み込み、全12国の `Kuni` ドメインオブジェクトおよびそれに対応する `Daimyo` ドメインオブジェクトを生成しなければならない。

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

### Requirement: neighbor.csvからの隣接マップ構築
MUST: システムは `static/master_data/neighbor.csv` を読み込み、`KuniId` 同士の隣接関係マップを `NeighborRepository` 経由で参照可能にしなければならない。

CSVの各行は `ID1, ID2` の形式で、双方向の隣接関係を意味する。

#### Scenario: 正常なneighbor.csvの読み込み
- **WHEN** `static/master_data/neighbor.csv` が正しいフォーマットで存在する状態で起動したとき
- **THEN** neighbor.csvの各ペアが双方向に展開された隣接マップが構築される（ID1→ID2 かつ ID2→ID1）
- **THEN** `NeighborRepository::are_adjacent` で隣接判定ができる状態になる

#### Scenario: neighbor.csvにkuni.csv未登録のIDが含まれる場合
- **WHEN** `neighbor.csv` に `kuni.csv` に存在しないIDが記載されている状態で起動したとき
- **THEN** システムは `MasterDataError::InvalidReference` を返し、起動を中止する
