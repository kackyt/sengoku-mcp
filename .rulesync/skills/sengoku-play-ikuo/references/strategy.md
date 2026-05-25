# お任せ戦略レコメンドガイド

recommend.py の入力フォーマット詳細と、手動レコメンド時の判断基準を記載します。

## recommend.py 入力 JSON の組み立て方

`get_my_status` と `get_other_countries_info`（任意）の結果から JSON を組み立てます。

### 完全な入力例

```json
{
  "strategy": "military",
  "season": 2,
  "turn": 5,
  "my_countries": [
    {
      "id": 1,
      "name": "蝦夷",
      "kin": 300,
      "kome": 200,
      "hei": 100,
      "jinko": 420,
      "kokudaka": 150,
      "machi": 5,
      "tyu": 45
    },
    {
      "id": 2,
      "name": "陸奥",
      "kin": 500,
      "kome": 800,
      "hei": 300,
      "jinko": 850,
      "kokudaka": 400,
      "machi": 20,
      "tyu": 80
    }
  ],
  "enemy_countries": [
    {
      "kuni_id": 3,
      "kuni_name": "出羽",
      "daimyo_name": "最上",
      "kin": 100,
      "kome": 50,
      "hei": 60,
      "kokudaka": 200,
      "towns": 8,
      "tyu": 40
    }
  ],
  "neighbor_map": {
    "1": [3],
    "2": [3, 4]
  }
}
```

### フィールド説明

| フィールド | 型 | 説明 |
|---|---|---|
| strategy | string | `military` / `domestic` / `balanced` |
| season | int | 0=春, 1=夏, 2=秋, 3=冬（TurnNumber.season()の値） |
| turn | int | 現在のターン番号 |
| my_countries | array | 自国情報（`get_my_status` から取得。**`jinko`** は必須） |
| my_countries[].jinko | int | 人口。徴募上限計算に必須 |
| enemy_countries | array | 敵国情報（`get_other_countries_info` から取得。省略可） |
| neighbor_map | object | 自国IDをキーに隣接国IDの配列（`get_neighbor_info` から取得。省略可） |

### neighbor_map の取得方法

各自国に対して `get_neighbor_info(kuni_id=X)` を呼び出し、結果から ID を抽出します。
省略した場合、攻撃推奨はスキップされます。

## recommend.py のスコアリングロジック概要

各国・各アクションに対してスコアを計算し、上位3つを推奨します。

### 作戦タイプ別の傾向

| 作戦 | 徴募 | 開墾 | 町作り | 攻撃推奨 |
|---|---|---|---|---|
| military | +40 | -10 | -10 | 推奨（+30ボーナス） |
| domestic | -15 | +35 | +30 | スキップ |
| balanced | ±0 | ±0 | ±0 | 条件次第 |

### 攻撃判断の条件（チートなし・公開情報のみ使用）

- `get_other_countries_info` で得た敵の **公開兵数** を参照
- **自軍兵数 ≥ 敵軍兵数 × 1.5** かつ **米備蓄 ≥ 出陣兵 × 3**
- 敵の忠誠度が低い（< 60）と占領後の維持が楽 → ボーナス加点
- `neighbor_map` で隣接していない敵は対象外

### 季節別の推奨傾向

| 季節 | 推奨行動 |
|---|---|
| 春（0） | 開墾・町作り優先（収入強化）。米売却で金確保。 |
| 夏（1） | 疫病・洪水に備えて徴募で兵力維持。忠誠度チェック。 |
| 秋（2） | 米収穫後で余剰米多 → 売却チャンス。軍備拡張に好機。 |
| 冬（3） | 内政縮小。翌春に備えて兵糧・金を温存。 |

## 手動レコメンド時の基準（recommend.py を使わない場合）

### リソース優先度チェックリスト

```text
1. 忠誠度（tyu）が **50 未満** → 施し（give_charity）を最優先（50以上は主君指示時のみ）
2. 金が石高の2倍未満      → 米売却（rice_sell）または施し控え
3. 兵が目標値未満         → 徴募（recruit）
4. 春かつ金に余裕あり     → 開墾（develop_land）
5. 春かつ金に余裕あり     → 町作り（build_town）
```

### 目標兵数の計算

```text
balanced: 石高 ÷ 2
military: 石高 × 0.75
domestic: 石高 × 0.25
```

## 複数国を抱えた場合の管理指針

領地が増えると各国の状況が異なります。recommend.py は全国を一括評価します。

```text
国数が多い場合の典型的な分担例:
  本拠地（最も石高が高い国）: 主力軍の徴募・兵站管理
  前線国（敵に近い国）      : 攻撃拠点、兵糧集積
  後方国（敵から遠い国）    : 内政専念（開墾・町作り）
```

国間の資源移動には `domestic_transport`（隣接国のみ）を活用してください。
