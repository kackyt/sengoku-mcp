use crate::domain::model::value_objects::{KuniId, Amount};
use serde::{Deserialize, Serialize};

/// 戦闘時の策
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tactic {
    /// 通常
    Normal,
    /// 奇襲
    Surprise,
    /// 火計
    Fire,
    /// 鼓舞
    Inspire,
    /// 退却
    Retreat,
}

/// 戦闘の陣営
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleSide {
    /// 攻撃側
    Attacker,
    /// 防御側
    Defender,
}

/// 戦況の優劣
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleAdvantage {
    /// 優勢
    Advantage,
    /// 拮抗
    Even,
    /// 劣勢
    Disadvantage,
}

/// 合戦中の軍勢ステータス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WarStatus {
    pub attacker_id: KuniId,
    pub defender_id: KuniId,

    // 攻撃側
    pub attacker_hei: Amount,
    pub attacker_kome: Amount,
    pub attacker_morale: u32,

    // 防御側
    pub defender_hei: Amount,
    pub defender_kome: Amount,
    pub defender_morale: u32,

    pub winner: Option<BattleSide>,
    pub advantage: BattleAdvantage,
}
