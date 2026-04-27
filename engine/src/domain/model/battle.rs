use crate::domain::model::value_objects::{Amount, KuniId, Rate};
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

/// 1つの軍勢のステータス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArmyStatus {
    pub kuni_id: KuniId,
    pub hei: Amount,
    pub kome: Amount,
    pub morale: Rate,
}

impl ArmyStatus {
    /// 兵力ダメージを受ける
    pub fn take_damage(&mut self, damage: Amount) {
        self.hei -= damage;
    }

    /// 兵糧を喪失する（火計などによる純粋な減少）
    pub fn lose_kome(&mut self, loss: Amount) {
        self.kome -= loss;
    }

    /// 士気を上昇させる
    pub fn add_morale(&mut self, amount: u32) {
        self.morale += Rate::new(amount);
    }

    /// 士気を低下させる
    pub fn sub_morale(&mut self, amount: u32) {
        self.morale -= Rate::new(amount);
    }

    /// 士気を変動させる（負の数なら低下）
    pub fn modify_morale(&mut self, delta: i32) {
        if delta > 0 {
            self.add_morale(delta as u32);
        } else {
            self.sub_morale(delta.unsigned_abs());
        }
    }

    /// ターンの維持費（兵糧）を支払う。足りなければ士気が激減する
    pub fn pay_maintenance(&mut self, cost: Amount) {
        if self.kome < cost {
            use rand::Rng;
            self.kome = Amount::zero();
            let drop = rand::thread_rng().gen_range(30..=50);
            self.morale -= Rate::new(drop); // 飢えによる士気低下
        } else {
            self.kome -= cost;
        }
    }

    /// 敵軍の資源を接収する（勝利時の合算処理）
    pub fn plunder(&mut self, enemy: &ArmyStatus) {
        self.hei += enemy.hei;
        self.kome += enemy.kome;
    }

    /// 壊滅判定
    pub fn is_destroyed(&self) -> bool {
        self.hei.is_zero() || self.kome.is_zero() || self.morale.value() == 0
    }
}

/// 合戦全体の状態
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WarStatus {
    pub attacker: ArmyStatus,
    pub defender: ArmyStatus,
    pub winner: Option<BattleSide>,
    pub advantage: BattleAdvantage,
}

impl WarStatus {
    pub fn attacker_id(&self) -> KuniId {
        self.attacker.kuni_id
    }
    pub fn defender_id(&self) -> KuniId {
        self.defender.kuni_id
    }
}
