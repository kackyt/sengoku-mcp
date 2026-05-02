use crate::domain::model::value_objects::{Amount, DaimyoId, EventMessage, KuniId, TurnNumber};

/// ゲーム進行に関するイベント
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    /// 新しいターンが開始された
    TurnStarted { turn: TurnNumber },
    /// 特定の大名の行動番になった
    DaimyoActionStarted { daimyo_id: DaimyoId },
    /// 大名が内政コマンドを実行した
    DomesticAction {
        daimyo_id: DaimyoId,
        action_name: EventMessage,
        details: EventMessage,
    },
    /// 大名が戦争を実行した
    BattleAction {
        attacker_id: DaimyoId,
        target_kuni_id: KuniId,
        result_message: EventMessage,
    },
    /// 全大名の行動が終了し、ターンの季節処理が完了した
    SeasonPassed { turn: TurnNumber },
    /// 行動する大名が誰も残っていない（スキップなど）
    TurnCompleted,
}

/// 季節イベントの種別を表す列挙型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeasonalEventType {
    /// 疫病（通年、1/40確率）
    Plague,
    /// 洪水（夏季限定、1/40確率）
    Flood,
    /// 反乱（忠誠度50未満、(50-忠誠度)%確率）
    Rebellion,
    /// 人口増加（春）
    PopulationGrowth,
    /// 金収入（春）
    GoldIncome,
    /// 米収入（秋）
    RiceIncome,
}

/// 季節イベント発生時の効果を保持する構造体
/// UI層への通知やログ記録に使用する
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonalEventEffect {
    /// 影響を受けた国のID
    pub kuni_id: KuniId,
    /// イベントの種別
    pub event_type: SeasonalEventType,
    /// 金の変化量（正=増加、負=減少はAmount型上は0になる）
    pub kin_diff: Amount,
    /// 米の変化量
    pub kome_diff: Amount,
    /// 兵力の変化量
    pub hei_diff: Amount,
    /// 人口の変化量
    pub jinko_diff: Amount,
    /// 忠誠度の変化量（正負あり）
    pub tyu_diff: i32,
    /// 石高の変化量
    pub kokudaka_diff: Amount,
    /// 町の変化量
    pub machi_diff: Amount,
}
