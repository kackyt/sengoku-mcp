use crate::domain::model::value_objects::DaimyoId;

/// ゲーム進行に関するイベント
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    /// 新しいターンが開始された
    TurnStarted { turn: u32 },
    /// 特定の大名の行動番になった
    DaimyoActionStarted { daimyo_id: DaimyoId },
    /// 大名が内政コマンドを実行した
    DomesticAction {
        daimyo_id: DaimyoId,
        action_name: String,
        details: String,
    },
    /// 大名が戦争を実行した
    BattleAction {
        attacker_id: DaimyoId,
        target_kuni_id: crate::domain::model::value_objects::KuniId,
        result_message: String,
    },
    /// 全大名の行動が終了し、ターンの季節処理が完了した
    SeasonPassed { turn: u32 },
    /// 行動する大名が誰も残っていない（スキップなど）
    TurnCompleted,
}
