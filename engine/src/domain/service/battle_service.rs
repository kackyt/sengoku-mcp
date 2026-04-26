use crate::domain::error::DomainError;
use crate::domain::model::battle::{BattleAdvantage, BattleSide, Tactic, WarStatus};
use crate::domain::model::value_objects::Amount;

/// 戦闘計算を行うドメインサービス
pub struct BattleService;

impl BattleService {
    // 戦闘バランス調整用の定数
    const DMG_NORMAL: u32 = 180;
    const DMG_SURPRISE_SUCCESS: u32 = 300;
    const DMG_SURPRISE_FAIL: u32 = 40;
    const DMG_DEFAULT: u32 = 60;
    const MORALE_CHANGE: u32 = 10;
    const FOOD_CONSUMPTION_RATE: u32 = 30;
    const FIRE_HEI_LOSS_RATE: u32 = 30;
    const FIRE_KOME_LOSS_RATE: u32 = 50;

    /// 1ターンの戦闘計算を行います
    pub fn calculate_turn(
        mut status: WarStatus,
        attacker_tactic: Tactic,
        defender_tactic: Tactic,
    ) -> Result<WarStatus, DomainError> {
        // --- 退却判定 ---
        if attacker_tactic == Tactic::Retreat {
            status.winner = Some(BattleSide::Defender);
            return Ok(status);
        }
        if defender_tactic == Tactic::Retreat {
            status.winner = Some(BattleSide::Attacker);
            return Ok(status);
        }

        // --- ダメージ計算と策の効果 ---
        let base_damage = status.attacker.hei;

        let damage = match (attacker_tactic, defender_tactic) {
            (Tactic::Normal, Tactic::Normal) => base_damage.mul_percent(Self::DMG_NORMAL),
            (Tactic::Surprise, Tactic::Normal) => {
                status.defender.modify_morale(-(Self::MORALE_CHANGE as i32));
                status.attacker.modify_morale(Self::MORALE_CHANGE as i32);
                base_damage.mul_percent(Self::DMG_SURPRISE_FAIL)
            }
            (Tactic::Surprise, Tactic::Surprise) => {
                status.attacker.modify_morale(-(Self::MORALE_CHANGE as i32));
                base_damage.mul_percent(Self::DMG_SURPRISE_SUCCESS)
            }
            (Tactic::Fire, Tactic::Fire) => {
                let loss = status.attacker.hei.mul_percent(Self::FIRE_HEI_LOSS_RATE);
                status.attacker.take_damage(loss);
                status.attacker.modify_morale(-(Self::MORALE_CHANGE as i32));
                base_damage.mul_percent(Self::DMG_DEFAULT)
            }
            (Tactic::Fire, _) => {
                let loss = status.defender.kome.mul_percent(Self::FIRE_KOME_LOSS_RATE);
                status.defender.lose_kome(loss);
                status.defender.modify_morale(-(Self::MORALE_CHANGE as i32));
                status.attacker.modify_morale(Self::MORALE_CHANGE as i32);
                base_damage.mul_percent(Self::DMG_DEFAULT)
            }
            (_, Tactic::Inspire) => {
                status.defender.modify_morale(15);
                Amount::zero()
            }
            _ => base_damage.mul_percent(Self::DMG_DEFAULT),
        };

        // ダメージ適用
        status.defender.take_damage(damage);

        // --- 兵糧消費 ---
        let food_cost = status.attacker.hei.mul_percent(Self::FOOD_CONSUMPTION_RATE);
        status.attacker.pay_maintenance(food_cost);

        // --- 勝敗判定 ---
        status.winner = if status.defender.is_destroyed() {
            Some(BattleSide::Attacker)
        } else if status.attacker.is_destroyed() {
            Some(BattleSide::Defender)
        } else {
            None
        };

        // --- 勝利時のリソース接収 ---
        if status.winner == Some(BattleSide::Attacker) {
            status.attacker.plunder(&status.defender);
        }

        // 優勢度計算
        status.advantage = Self::calculate_advantage(status.attacker.hei, status.defender.hei);

        Ok(status)
    }

    /// 敵の策を決定します
    pub fn decide_tactic() -> Tactic {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..4) {
            0 => Tactic::Normal,
            1 => Tactic::Surprise,
            2 => Tactic::Fire,
            3 => Tactic::Inspire,
            _ => Tactic::Normal,
        }
    }

    /// 戦況の優劣を判定します
    pub fn calculate_advantage(attacker_hei: Amount, defender_hei: Amount) -> BattleAdvantage {
        if attacker_hei > defender_hei.add(defender_hei) {
            BattleAdvantage::Advantage
        } else if defender_hei > attacker_hei.add(attacker_hei) {
            BattleAdvantage::Disadvantage
        } else {
            BattleAdvantage::Even
        }
    }
}
