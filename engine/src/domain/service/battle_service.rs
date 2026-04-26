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
    const MORALE_CHANGE: i32 = 10;
    const FOOD_CONSUMPTION_RATE: u32 = 30;
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

        let atk_hei = status.attacker.hei;
        let def_hei = status.defender.hei;
        let def_kome = status.defender.kome;

        // 組み合わせによる一括計算
        let (atk_to_def_dmg, def_to_atk_dmg, atk_m_mod, def_m_mod, def_kome_loss, atk_kome_loss) =
            match (attacker_tactic, defender_tactic) {
                // --- Normal vs ---
                (Tactic::Normal, Tactic::Normal) => (
                    atk_hei.mul_percent(Self::DMG_NORMAL),
                    def_hei.mul_percent(Self::DMG_NORMAL),
                    0,
                    0,
                    Amount::zero(),
                    Amount::zero(),
                ),
                (Tactic::Normal, _) => (
                    atk_hei.mul_percent(Self::DMG_DEFAULT),
                    def_hei.mul_percent(Self::DMG_SURPRISE_FAIL),
                    0,
                    0,
                    Amount::zero(),
                    Amount::zero(),
                ),
                // --- Surprise vs ---
                (Tactic::Surprise, Tactic::Surprise) => (
                    atk_hei.mul_percent(Self::DMG_SURPRISE_FAIL),
                    def_hei.mul_percent(Self::DMG_SURPRISE_SUCCESS),
                    -Self::MORALE_CHANGE,
                    Self::MORALE_CHANGE,
                    Amount::zero(),
                    Amount::zero(),
                ),
                (Tactic::Surprise, _) => (
                    atk_hei.mul_percent(Self::DMG_SURPRISE_SUCCESS),
                    def_hei.mul_percent(Self::DMG_SURPRISE_FAIL),
                    Self::MORALE_CHANGE,
                    -Self::MORALE_CHANGE,
                    Amount::zero(),
                    Amount::zero(),
                ),
                // --- Fire vs ---
                (Tactic::Fire, Tactic::Fire) => (
                    atk_hei.mul_percent(Self::DMG_SURPRISE_FAIL),
                    def_hei.mul_percent(Self::DMG_DEFAULT),
                    -Self::MORALE_CHANGE,
                    Self::MORALE_CHANGE,
                    Amount::zero(),
                    Amount::zero(),
                ),
                (Tactic::Fire, _) => (
                    atk_hei.mul_percent(Self::DMG_DEFAULT),
                    def_hei.mul_percent(Self::DMG_DEFAULT),
                    Self::MORALE_CHANGE,
                    -Self::MORALE_CHANGE,
                    def_kome.mul_percent(Self::FIRE_KOME_LOSS_RATE),
                    Amount::zero(),
                ),
                // --- Inspire vs ---
                (Tactic::Inspire, Tactic::Inspire) => (
                    Amount::zero(),
                    Amount::zero(),
                    15,
                    15,
                    Amount::zero(),
                    Amount::zero(),
                ),
                (Tactic::Inspire, _) => (
                    Amount::zero(),
                    def_hei.mul_percent(Self::DMG_DEFAULT),
                    15,
                    0,
                    Amount::zero(),
                    Amount::zero(),
                ),
                // --- その他 ---
                (_, Tactic::Inspire) => (
                    atk_hei.mul_percent(Self::DMG_DEFAULT),
                    Amount::zero(),
                    0,
                    15,
                    Amount::zero(),
                    Amount::zero(),
                ),
                _ => (
                    atk_hei.mul_percent(Self::DMG_DEFAULT),
                    def_hei.mul_percent(Self::DMG_DEFAULT),
                    0,
                    0,
                    Amount::zero(),
                    Amount::zero(),
                ),
            };

        // --- 同時解決 ---
        status.defender.take_damage(atk_to_def_dmg);
        status.attacker.take_damage(def_to_atk_dmg);

        status.attacker.modify_morale(atk_m_mod);
        status.defender.modify_morale(def_m_mod);

        status.defender.lose_kome(def_kome_loss);
        status.attacker.lose_kome(atk_kome_loss);

        // --- 兵糧消費 (維持費) ---
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

        if status.winner == Some(BattleSide::Attacker) {
            status.attacker.plunder(&status.defender);
        }

        status.advantage = Self::calculate_advantage(atk_to_def_dmg, def_to_atk_dmg);

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
    pub fn calculate_advantage(dmg_to_def: Amount, dmg_to_atk: Amount) -> BattleAdvantage {
        if dmg_to_def > dmg_to_atk {
            BattleAdvantage::Advantage
        } else if dmg_to_atk > dmg_to_def {
            BattleAdvantage::Disadvantage
        } else {
            BattleAdvantage::Even
        }
    }
}
