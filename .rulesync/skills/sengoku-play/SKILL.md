---
name: sengoku-play
description: >-
  sengoku-mcp ツール群を使って、ユーザーと対話的に戦国シミュレーションを進めるスキルです。
  ターン進行・内政コマンド実行・合戦モード・お任せレコメンドを含む。 「プレイしたい」「戦国ゲームを始めて」「ターンを進めて」「合戦を仕掛けて」「お任せで」
  「内政してほしい」「戦略をアドバイスして」などと要求されたときにトリガーします。
---
# Sengoku Play（戦国プレイ）スキル

プレイヤーと対話しながら sengoku-mcp の MCP ツールを呼び出し、ターン制戦国シミュレーションを進行させるスキルです。

## ゲームの基本フロー

```
1. 大名選択     → list_daimyos → select_daimyo
2. 作戦確認     → プレイヤーの作戦タイプを確認（初回のみ）
3. ターン開始   → get_my_status で状況確認 → progress_turn でCPUを動かす (progress_turnはプレーヤーに聞くことなく自動で実行する)
4. 自分の手番   → 内政/合戦コマンドを国ごとに実行（手動 or お任せ）
5. 手番終了     → progress_turn で次のCPUターンへ
6. ターン終了時イベントを報告 → 3へ戻る
```

> **重要: コマンドは「国ごと」に実行します**
> 大名ではなく支配している「国」の単位でコマンドを実行します。
> 領地が1国なら1回、合戦に勝利して3国になれば3国それぞれにコマンドを実行します。ただし、その国のターン出ない場合は実行できません。手番となっている国のコマンドを実行してください
> 合戦後に領地が増えた場合は、その国の資源状況を確認してコマンドを追加してください。

## 作戦タイプの確認（セッション開始時）

プレイヤーに作戦タイプを質問し、以降の全ターンで使用します。

```
「どのような作戦で進めますか？
  1. 武力重視（軍備・征服を優先）
  2. 内政重視（石高・金収入を優先）
  3. バランス（どちらもバランスよく）」
```

| 回答 | strategy 値 |
|---|---|
| 武力・征服・軍 | `military` |
| 内政・開発・経済 | `domestic` |
| バランス・普通 | `balanced` |

作戦タイプはセッション中に「方針を変えたい」と言われたら更新します。

## 1ターンの報告フォーマット

毎ターン、以下の情報をユーザーに報告します。

```
=== 第N ターン（季節） ===
【イベント】
- 季節イベント（金収入・米収穫・洪水・疫病など）

【あなたの領地】（国ごとに表示）
- 国名 (ID): 金=X, 米=X, 兵=X, 石高=X, 町=X, 忠誠=X

【隣接する脅威/機会】
- 攻撃可能な隣国 or 侵攻されているリスク

【推奨アクション】（お任せ時のみ）
- recommend.py の出力を使用（国ごとにレコメンド）
```

## コマンド対応表

### 内政フェーズ（Domestic Phase）

コマンドは **国（kuni_id）** 単位で実行します。領地が複数ある場合はそれぞれの国に対して実行が必要です。

| ユーザー指示 | MCPツール | パラメータ |
|---|---|---|
| 米を売る | `domestic_rice_sell` | kuni_id, amount |
| 米を買う | `domestic_rice_buy` | kuni_id, amount |
| 兵を雇う/徴募 | `domestic_recruit` | kuni_id, amount |
| 開墾する | `domestic_develop_land` | kuni_id, amount |
| 町を建てる | `domestic_build_town` | kuni_id, amount |
| 施しをする | `domestic_give_charity` | kuni_id, amount |
| 資源を輸送 | `domestic_transport` | from/to kuni_id, kin/hei/kome |
| 他国情報を見る | `get_other_countries_info` | なし（ターン消費） |
| ターンを進める | `progress_turn` | なし |
| お任せで行動 | 後述の「お任せモード」 | - |

### 合戦フェーズ（Battle Phase）

合戦モードへの入り方：
- プレイヤーが `battle_start_war` で宣戦布告したとき
- CPUがプレイヤー領地へ侵攻してきたとき（`get_my_status` の警告）

**合戦も「国ごと」です。** 攻撃する国（attacker_kuni_id）を指定します。

| ユーザー指示 | MCPツール | 戦術番号 |
|---|---|---|
| 合戦を仕掛ける | `battle_start_war` | hei, kome を指定 |
| 攻める（通常） | `battle_execute_turn` | tactic=1 |
| 奇襲する | `battle_execute_turn` | tactic=2 |
| 火計を使う | `battle_execute_turn` | tactic=3 |
| 鼓舞する | `battle_execute_turn` | tactic=4 |
| 退却する | `battle_execute_turn` | tactic=5 |
| 防衛（通常） | `battle_execute_defense_turn` | tactic=1 |
| 防衛（奇襲） | `battle_execute_defense_turn` | tactic=2 |
| 防衛（火計） | `battle_execute_defense_turn` | tactic=3 |
| 防衛（鼓舞） | `battle_execute_defense_turn` | tactic=4 |

#### 合戦ターンの進行

```
loop:
  1. battle_execute_turn または battle_execute_defense_turn を実行
  2. 残兵数と状況を報告
  3. winner が出たら合戦終了を報告 → progress_turn で再開
  4. 次の戦術をユーザーに確認（お任せなら自動選択）
```

## お任せモード

「お任せで」「全部やって」と言われた場合、以下の手順を実行します。

### ステップ1: 状況把握

```
get_my_status → 自国の状態取得
```

**他国情報の収集（任意）:**
手番が残っていれば `get_other_countries_info` を実行して敵国情報を取得します。
この情報はレコメンドの精度向上に使用します（公開情報のみ・チートなし）。

> 注意: `get_other_countries_info` はターンを1消費します。
> 敵情報なしでもレコメンドは動作しますが精度は下がります。

### ステップ2: recommend.py を実行

[scripts/recommend.py](./scripts/recommend.py) を使ってレコメンドを生成します。

> [!IMPORTANT]
> **入力JSONは毎ターン、必ず最新のステータスから新規作成してください。**
> `scratch/recommend_input.json` などの既存ファイルを再利用すると、古いデータに基づいた誤ったレコメンドが生成されます。

入力JSONを組み立てる際の注意点：
1. `get_my_status` の結果から、`kin`, `kome`, `hei`, `kokudaka`, `machi`, `tyu`, **`jinko`** を正確に抽出します。
2. **`jinko` (人口) は必須です。** 省略すると推定値が使われ、正確な徴募レコメンドができなくなります。
3. 全ての数値は `get_my_status` で表示された値（Display Amount）をそのまま使用します。

入力 JSON の形式（[strategy.md](./references/strategy.md) を参照）:
```json
{
  "strategy": "military" | "domestic" | "balanced",
  "season": 0-3,
  "turn": <int>,
  "my_countries": [ ... ],
  "enemy_countries": [ ... ],
  "neighbor_map": { ... }
}
```

実行コマンド:
```powershell
python <recommend_path> <input_json_file>
```

### ステップ3: 結果を提示して実行

recommend.py の出力を以下の形式でユーザーに提示してから実行します:

```
【国ごとの推奨アクション】（作戦: XX、季節: XX）

〇〇国 (ID:X):
  優先度1: [コマンド] 量:X → 理由
  優先度2: [コマンド] 量:X → 理由

△△国 (ID:X):
  優先度1: [コマンド] 量:X → 理由
```

お任せ時でも必ず「何をするか」を宣言してから実行すること。

## 合戦「お任せ」での戦術選択

recommend.py の出力に攻撃推奨が含まれている場合：
- 兵力比 1.5 倍以上かつ兵糧十分 → 攻撃を推奨
- 武力重視（military）なら追加ボーナス

合戦中の戦術選択（お任せ）:
- 兵差が大きく優勢 → 通常（1）か鼓舞（4）
- 米が少ない敵 → 火計（3）で兵糧を焼く
- 自軍が劣勢 → 奇襲（2）で逆転を狙うか退却（5）
- 防衛かつ敵が多い → 鼓舞（4）で士気維持 → 次ターン奇襲

## トラブルシューティング

- **推奨される徴募数が異常に多い/少ない**: 
  - `recommend_input.json` の `jinko`（人口）と `hei`（兵数）が最新の `get_my_status` と一致しているか確認してください。
  - 特に人口が兵士数に対して極端に多い古いデータが残っていると、過剰な徴募が推奨されます。
- **実行中にmcpの不具合を見つけた場合**: プレイヤーに報告してMCPの再起動をお願いしてください。
- **`select_daimyo` が未実行**: ほぼすべてのコマンドがエラーになります。
- **合戦フェーズ中**: 内政コマンドは使用できません。
- **領地が増えた場合**: `recommend.py` は全領地を一括評価しますが、Agentは各国に対して個別にコマンドを発行する必要があります。
