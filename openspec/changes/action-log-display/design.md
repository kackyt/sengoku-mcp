## Context

現在、ゲーム内でのアクション（内政コマンド、合戦操作、季節処理）は実行後に状態変化としてのみ現れ、何がいつ起きたかを遡って確認する手段がありません。特に「内政フェーズ」と「合戦フェーズ」ではログの関心事が大きく異なるため、これらを分離して管理・表示する仕組みを導入します。

## Goals / Non-Goals

**Goals:**
- ログを `Domestic`（内政）と `War`（合戦）の2つのカテゴリに分けて管理する。
- ログに `Visibility`（公開範囲）を持たせ、チート防止のためCPUの詳細行動はCLIに表示しない。
- 内政ログは継続保持し、戦争ログは合戦開始時にリセットする。
- 今後実装予定の季節イベント（seasonal-events）もログ対象として設計に含める。

**Non-Goals:**
- 内政ログのファイル永続化（インメモリ保持とする）。
- 戦争ログのアーカイブ（過去の合戦ログを後から見る機能）。

## Decisions

### データ構造

```rust
pub enum ActionLogCategory {
    Domestic, // 内政フェーズのイベント
    War,      // 合戦フェーズのイベント
}

pub enum ActionLogVisibility {
    Public,   // 全ての情報をCLIに表示する（季節イベント、合戦決着等）
    Player,   // 操作プレイヤーに係るイベントのみCLIに表示する
    Internal, // 詳細に記録するがCLIには表示しない（CPU行動、詳細計算値等）
}

// ドメイン層で定義される構造化されたイベント
pub enum ActionLogEvent {
    Domestic(DomesticLogEvent),
    War(WarLogEvent),
}

pub enum DomesticLogEvent {
    RiceSold { kuni_name: KuniName, gain: Amount, amount: DisplayAmount, rem_kin: Amount, rem_kome: Amount },
    RiceBought { kuni_name: KuniName, cost: Amount, amount: DisplayAmount, rem_kin: Amount, rem_kome: Amount },
    LandReclaimed { kuni_name: KuniName, gain: Amount, cost: Amount, new_tyu: Rate },
    TownDeveloped { kuni_name: KuniName, gain: Amount, cost: Amount, new_tyu: Rate },
    TroopsDrafted { kuni_name: KuniName, amount: DisplayAmount, rem_hei: Amount, rem_jinko: Amount, new_tyu: Rate },
    ResourcesTransported { from_kuni: KuniName, to_kuni: KuniName, kin: Amount, hei: Amount, kome: Amount },
    CpuAction { daimyo_id: DaimyoId, action_msg: String },
    TurnStart { turn: TurnNumber, season: u32 },
    SeasonalEvent { event_type: SeasonalEventType, kuni_names: Vec<KuniName> },
    // ...その他、合戦の開始・占領・防衛成功など
}

pub enum WarLogEvent {
    Damage { attacker_tactic: Tactic, defender_tactic: Tactic, attacker_damage: u32, defender_damage: u32 },
    AttackerVictory { home_name: KuniName, attacker_id: DaimyoId, occupied_name: KuniName, defender_id: DaimyoId },
    WarStarted { attacker_name: KuniName, defender_name: KuniName, attacker_id: DaimyoId, defender_id: DaimyoId },
    // ...その他
}

pub struct ActionLogEntry {
    pub visibility: ActionLogVisibility,
    pub turn:       TurnNumber,
    pub event:      ActionLogEvent, // 文字列ではなく型安全なイベントを保持
}
```

---

### 具体的なログ内容・Visibility・表示メッセージ設計

#### 【内政カテゴリ (`Domestic`)】

| イベント | Visibility | CLIに表示するメッセージ例 | `detail`に記録する内容の例 |
| :--- | :--- | :--- | :--- |
| **ターン開始** | `Public` | 「第{turn}ターン（{季節}）が始まりました」 | turn=N |
| **集約イベント** | `Public` | 「各地で{収穫/資金増加/人口増加}が発生しました」 | 国ごとの増加詳細 |
| **個別災害** | `Public` | 「【{洪水/疫病/反乱}】{国名1}, {国名2} で発生しました」 | 被害詳細（国ごと） |
| **合戦開始** | `Public` | 「{攻撃国名} が {防御国名} へ侵攻しました」 | 攻撃国ID、防御国ID |
| **合戦決着** | `Public` | 「{攻撃国名} が {防御国名} を占領しました」 | 勝敗結果詳細 |
| **プレイヤー：売米** | `Player` | 「{国名}：米を売却し、金{amount}を得ました」 | 売却量、売却後の金/米の残量 |
| **プレイヤー：開墾** | `Player` | 「{国名}：開墾し、石高が{gain}上昇しました」 | 投資額、増加量、開墾後の石高 |
| **プレイヤー：町作り** | `Player` | 「{国名}：町を整備し、町が{gain}上昇しました」 | 投資額、増加量、整備後の町の値 |
| **プレイヤー：徴募** | `Player` | 「{国名}：兵を{amount}徴募しました」 | 徴募後の兵力・人口・忠誠度 |
| **プレイヤー：施し** | `Player` | 「{国名}：施しを行い、忠誠度が{gain}上昇しました」 | 消費量、増加量、施し後の忠誠度 |
| **プレイヤー：輸送** | `Player` | 「{from国名}→{to国名}：資源を輸送しました（金:{kin} 兵:{hei} 米:{kome}）」 | 各資源の輸送量 |
| **CPU：内政（開墾/町作り）** | `Internal` | （非表示） | CPU大名ID、実行したコマンド、変化量 |

#### 【合戦カテゴリ (`War`)】

| イベント | Visibility | CLIに表示するメッセージ例 | `detail`に記録する内容の例 |
| :--- | :--- | :--- | :--- |
| **合戦開始** | `Public` | 「{攻撃国名} が {防御国名} へ侵攻しました」 | 攻撃国ID、防御国ID |
| **戦闘計算（ターン）** | `Player` | 「自軍({自軍戦術})の被害: {x}、敵軍({敵軍戦術})の被害: {y}」 | 両軍の兵力変動（内部値）、士気変化 |
| **攻撃側勝利（占領）** | `Public` | 「【占領】{攻撃国名} が {防御国名} を占領しました！」 | 占領後の資源状況 |
| **防御側勝利（防衛成功）** | `Public` | 「【防衛成功】{防御国名} は攻撃を退けました」 | 防衛後の兵力・忠誠度 |
| **CPU：策の選択** | `Internal` | （非表示） | CPUが選択した策 |

---

### リポジトリインターフェース

```rust
pub trait ActionLogRepository: Send + Sync {
    fn save(&self, entry: ActionLogEntry);
    /// 表示用：Public + Player のみを最新N件返す
    fn find_visible(&self, category: ActionLogCategory, limit: usize) -> Vec<ActionLogEntry>;
    /// デバッグ用：全件（Internalを含む）を返す
    fn find_all(&self, category: ActionLogCategory) -> Vec<ActionLogEntry>;
    /// カテゴリを指定してログをクリアする（合戦開始時にWarをクリア）
    fn clear(&self, category: ActionLogCategory);
}
```

### リセットのタイミング

- `BattleUseCase::start_war` 内で `ActionLogRepository::clear(War)` を呼び出し、合戦開始のたびにログをリセットする。

### CLI表示ロジック

- 内政画面: `find_visible(Domestic, 10)` を呼び出し最新10件を表示。
- 合戦画面: `find_visible(War, 10)` を呼び出し最新10件を表示。
- どちらも画面下部（高さ12行程度）に表示領域を確保する。ターミナルの高さが20行以下の場合は非表示。

## Risks / Trade-offs

- **CPUの行動が全く見えない問題**: CPU大名が複数の国を持つ場合、内政処理が終わっているのかどうかがUIから判断しにくい。ターン進行時に「{大名名}の手番です」などの通知（Public）は表示することで対処する。
- **季節イベントの国ごとのログ量**: 疫病・洪水・反乱が全国同時発生した場合、1ターンで大量のPublicログが出る。表示は10件に絞るが、scroll操作を将来的に検討する。
- **メモリ管理**: 内政ログ（Domestic）は上限200件、合戦ログ（War）はリセットのため無制限でも実用上問題ないが念のため100件を上限とする。`VecDeque` を用いたリングバッファで実装する。
