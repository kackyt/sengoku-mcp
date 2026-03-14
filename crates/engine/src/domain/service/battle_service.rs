use crate::domain::error::DomainError;
use crate::domain::model::kuni::Kuni;

/// 戦闘時の策
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tactic {
    /// 通常
    Normal,
    /// 奇襲
    Surprise,
    /// 火計
    Fire,
    /// 鼓舞
    Inspire,
}

/// 戦闘結果
#[derive(Debug)]
pub struct BattleResult {
    /// 攻撃側の国（戦闘後の状態）
    pub attacker_kuni: Kuni,
    /// 防御側の国（戦闘後の状態）
    pub defender_kuni: Kuni,
    /// 勝者
    pub winner: Option<BattleSide>, // 決着がつかない場合は None
}

/// 戦闘の陣営
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleSide {
    /// 攻撃側
    Attacker,
    /// 防御側
    Defender,
}

/// 戦闘計算を行うドメインサービス
pub struct BattleService;

impl BattleService {
    // 戦闘バランス調整用の定数
    const DMG_NORMAL: u32 = 180;
    const DMG_SURPRISE_SUCCESS: u32 = 300;
    const DMG_SURPRISE_FAIL: u32 = 40;
    const DMG_DEFAULT: u32 = 60;
    const PERCENT_BASE: u32 = 100;
    const MORALE_CHANGE: i32 = 10;
    const FOOD_CONSUMPTION_RATE: u32 = 30;
    const FIRE_HEI_LOSS_RATE: u32 = 30;
    const FIRE_KOME_LOSS_RATE: u32 = 50;

    /// 1ターンの戦闘計算を行います
    pub fn calculate_turn(
        mut attacker: Kuni,
        mut defender: Kuni,
        attacker_tactic: Tactic,
        defender_tactic: Tactic,
        attacker_troops: u32,
    ) -> Result<BattleResult, DomainError> {
        // --- ダメージ計算と策の効果 ---
        let mut base_damage = attacker_troops;

        match (attacker_tactic, defender_tactic) {
            (Tactic::Normal, Tactic::Normal) => {
                base_damage = (base_damage * Self::DMG_NORMAL) / Self::PERCENT_BASE;
            }
            (Tactic::Surprise, Tactic::Normal) => {
                // 奇襲失敗（簡易的な判定）
                base_damage = (base_damage * Self::DMG_SURPRISE_FAIL) / Self::PERCENT_BASE;
                defender.modify_tyu(-Self::MORALE_CHANGE);
                attacker.modify_tyu(Self::MORALE_CHANGE);
            }
            (Tactic::Surprise, Tactic::Surprise) => {
                // 奇襲成功
                base_damage = (base_damage * Self::DMG_SURPRISE_SUCCESS) / Self::PERCENT_BASE;
                attacker.modify_tyu(-Self::MORALE_CHANGE);
            }
            (Tactic::Fire, Tactic::Fire) => {
                // 火計同士で自軍に被害
                let loss = (attacker.resource.hei.value() * Self::FIRE_HEI_LOSS_RATE) / Self::PERCENT_BASE;
                let _ = attacker.consume_resource(0, loss, 0);
                attacker.modify_tyu(-Self::MORALE_CHANGE);
            }
            (Tactic::Fire, _) => {
                // 火計成功
                let loss = (defender.resource.kome.value() * Self::FIRE_KOME_LOSS_RATE) / Self::PERCENT_BASE;
                let _ = defender.consume_resource(0, 0, loss);
                defender.modify_tyu(-Self::MORALE_CHANGE);
                attacker.modify_tyu(Self::MORALE_CHANGE);
            }
            (_, Tactic::Inspire) => {
                defender.modify_tyu(15);
            }
            _ => {
                base_damage = (base_damage * Self::DMG_DEFAULT) / Self::PERCENT_BASE;
            }
        }

        // ダメージ適用
        let _ = defender.consume_resource(0, base_damage, 0);

        // --- 兵糧消費 ---
        let food_cost = (attacker_troops * Self::FOOD_CONSUMPTION_RATE) / Self::PERCENT_BASE;
        if attacker.consume_resource(0, 0, food_cost).is_err() {
            attacker.modify_tyu(-40); // 兵糧切れによる士気激減
        }

        // --- 勝敗判定 ---
        let winner = if defender.resource.hei.value() == 0
            || defender.resource.kome.value() == 0
            || defender.stats.tyu.value() == 0
        {
            Some(BattleSide::Attacker)
        } else if attacker.resource.hei.value() == 0
            || attacker.resource.kome.value() == 0
            || attacker.stats.tyu.value() == 0
        {
            Some(BattleSide::Defender)
        } else {
            None
        };

        // --- 勝利時のリソース接収 ---
        if winner == Some(BattleSide::Attacker) {
            attacker.add_resource(
                0,
                defender.resource.hei.value(),
                defender.resource.kome.value(),
            );
        }

        Ok(BattleResult {
            attacker_kuni: attacker,
            defender_kuni: defender,
            winner,
        })
    }
}
