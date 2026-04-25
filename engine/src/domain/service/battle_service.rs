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
    const PERCENT_BASE: u32 = 100;
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
        let mut base_damage = status.attacker_hei.to_internal();

        let damage = match (attacker_tactic, defender_tactic) {
            (Tactic::Normal, Tactic::Normal) => base_damage.mul_percent(Self::DMG_NORMAL),
            (Tactic::Surprise, Tactic::Normal) => {
                // 奇襲失敗（簡易的な判定）
                status.defender_morale = status.defender_morale.saturating_sub(Self::MORALE_CHANGE);
                status.attacker_morale = status.attacker_morale.saturating_add(Self::MORALE_CHANGE);
                base_damage.mul_percent(Self::DMG_SURPRISE_FAIL)
            }
            (Tactic::Surprise, Tactic::Surprise) => {
                // 奇襲成功
                status.attacker_morale = status.attacker_morale.saturating_sub(Self::MORALE_CHANGE);
                base_damage.mul_percent(Self::DMG_SURPRISE_SUCCESS)
            }
            (Tactic::Fire, Tactic::Fire) => {
                // 火計同士で自軍に被害
                let loss = (status.attacker_hei.to_internal().value() * Self::FIRE_HEI_LOSS_RATE)
                    / Self::PERCENT_BASE;
                status.attacker_hei = status
                    .attacker_hei
                    .to_internal()
                    .sub(Amount::new(loss))
                    .to_display();
                status.attacker_morale = status.attacker_morale.saturating_sub(Self::MORALE_CHANGE);
                base_damage.mul_percent(Self::DMG_DEFAULT)
            }
            (Tactic::Fire, _) => {
                // 火計成功
                let loss = (status.defender_kome.to_internal().value() * Self::FIRE_KOME_LOSS_RATE)
                    / Self::PERCENT_BASE;
                status.defender_kome = status
                    .defender_kome
                    .to_internal()
                    .sub(Amount::new(loss))
                    .to_display();
                status.defender_morale = status
                    .defender_morale
                    .saturating_sub(Self::MORALE_CHANGE as u32);
                status.attacker_morale = status
                    .attacker_morale
                    .saturating_add(Self::MORALE_CHANGE as u32);
                base_damage.mul_percent(Self::DMG_DEFAULT)
            }
            (_, Tactic::Inspire) => {
                status.defender_morale = status.defender_morale.saturating_add(15);
                Amount::new(0)
            }
            _ => base_damage.mul_percent(Self::DMG_DEFAULT),
        };

        // ダメージ適用
        status.defender_hei = status.defender_hei.to_internal().sub(damage).to_display();

        // --- 兵糧消費 ---
        let food_cost = (status.attacker_hei.to_internal().value() * Self::FOOD_CONSUMPTION_RATE)
            / Self::PERCENT_BASE;
        let a_kome_internal = status.attacker_kome.to_internal();
        if a_kome_internal.value() < food_cost {
            status.attacker_kome = Amount::new(0).to_display();
            status.attacker_morale = status.attacker_morale.saturating_sub(40);
        } else {
            status.attacker_kome = a_kome_internal.sub(Amount::new(food_cost)).to_display();
        }

        // --- 勝敗判定 ---
        status.winner = if status.defender_hei.to_internal().value() == 0
            || status.defender_kome.to_internal().value() == 0
            || status.defender_morale == 0
        {
            Some(BattleSide::Attacker)
        } else if status.attacker_hei.to_internal().value() == 0
            || status.attacker_kome.to_internal().value() == 0
            || status.attacker_morale == 0
        {
            Some(BattleSide::Defender)
        } else {
            None
        };

        // --- 勝利時のリソース接収 ---
        if status.winner == Some(BattleSide::Attacker) {
            let a_hei = status
                .attacker_hei
                .to_internal()
                .add(status.defender_hei.to_internal());
            let a_kome = status
                .attacker_kome
                .to_internal()
                .add(status.defender_kome.to_internal());
            status.attacker_hei = a_hei.to_display();
            status.attacker_kome = a_kome.to_display();
        }

        // 優勢度計算
        status.advantage = Self::calculate_advantage(
            status.attacker_hei.to_internal().value(),
            status.defender_hei.to_internal().value(),
        );

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
    pub fn calculate_advantage(attacker_hei: u32, defender_hei: u32) -> BattleAdvantage {
        if attacker_hei > defender_hei * 2 {
            BattleAdvantage::Advantage
        } else if defender_hei > attacker_hei * 2 {
            BattleAdvantage::Disadvantage
        } else {
            BattleAdvantage::Even
        }
    }
}
