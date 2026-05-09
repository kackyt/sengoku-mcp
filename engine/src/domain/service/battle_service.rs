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
        let atk_food_cost = status.attacker.hei.mul_percent(Self::FOOD_CONSUMPTION_RATE);
        status.attacker.pay_maintenance(atk_food_cost);

        let def_food_cost = status.defender.hei.mul_percent(Self::FOOD_CONSUMPTION_RATE);
        status.defender.pay_maintenance(def_food_cost);

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

    /// 攻撃側の戦術を決定します
    pub fn decide_tactic_for_attacker<R: rand::Rng>(
        my: &crate::domain::model::battle::ArmyStatus,
        enemy: &crate::domain::model::battle::ArmyStatus,
        military_bias: f64,
        rng: &mut R,
    ) -> Tactic {
        let mut weights = std::collections::HashMap::new();
        weights.insert(Tactic::Normal, 40.0);
        weights.insert(Tactic::Surprise, 30.0);
        weights.insert(Tactic::Fire, 20.0);
        weights.insert(Tactic::Inspire, 10.0);

        // 兵力差による補正 (自分が圧倒的ならNormalを増やす)
        if my.hei > enemy.hei.mul_percent(150) {
            *weights.get_mut(&Tactic::Normal).unwrap() += 20.0;
        }

        // 兵糧状況による補正 (相手の兵糧が少ないならFireを増やす)
        if enemy.kome < enemy.hei.mul_percent(500) {
            *weights.get_mut(&Tactic::Fire).unwrap() += 20.0;
        }

        // military_biasによる補正 (高いならSurpriseを増やす)
        if military_bias > 1.2 {
            *weights.get_mut(&Tactic::Surprise).unwrap() += 15.0;
        }

        // ノイズの付与
        for weight in weights.values_mut() {
            *weight += rng.gen_range(-15.0..15.0);
            if *weight < 0.0 {
                *weight = 0.0;
            }
        }

        Self::weighted_sample(weights, rng)
    }

    /// 防衛側の戦術を決定します
    pub fn decide_tactic_for_defender<R: rand::Rng>(
        _my: &crate::domain::model::battle::ArmyStatus,
        _enemy: &crate::domain::model::battle::ArmyStatus,
        rng: &mut R,
    ) -> Tactic {
        // 防衛側は相手の策を読んでアンチを出す。
        // 攻撃側が何を選ぶか確率的に予測し、それに対するアンチを選ぶ。

        let mut threats = std::collections::HashMap::new();
        threats.insert(Tactic::Normal, 33.0);
        threats.insert(Tactic::Surprise, 33.0);
        threats.insert(Tactic::Fire, 33.0);

        // ノイズ付与 (読みのブレ)
        for val in threats.values_mut() {
            *val += rng.gen_range(-20.0..20.0);
        }

        // 最も可能性が高いと思われる戦術に対するアンチを選択
        let predicted = threats
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(t, _)| t)
            .unwrap_or(Tactic::Normal);

        match predicted {
            Tactic::Normal => Tactic::Surprise, // NormalにはSurpriseで対抗（設計上はSurprise Failだが、防衛側はNormalを崩したい）
            Tactic::Surprise => Tactic::Surprise, // SurpriseにはSurpriseで相打ち
            Tactic::Fire => Tactic::Fire,       // FireにはFireで相打ち
            _ => Tactic::Normal,
        }
    }

    /// 重み付き抽選
    fn weighted_sample<R: rand::Rng>(
        weights: std::collections::HashMap<Tactic, f64>,
        rng: &mut R,
    ) -> Tactic {
        let total: f64 = weights.values().sum();
        if total <= 0.0 {
            return Tactic::Normal;
        }
        let mut n = rng.gen_range(0.0..total);
        for (tactic, weight) in weights {
            if n < weight {
                return tactic;
            }
            n -= weight;
        }
        Tactic::Normal
    }

    /// CPU同士の自動決着を行います
    pub fn auto_resolve<R: rand::Rng>(
        mut status: WarStatus,
        rng: &mut R,
    ) -> Result<(WarStatus, u32), DomainError> {
        let mut turn = 1;
        while turn <= 10 && status.winner.is_none() {
            // 自動決着では Normal/Surprise/Fire を 1/3 ずつランダムで選択
            let atk_t = match rng.gen_range(0..3) {
                0 => Tactic::Normal,
                1 => Tactic::Surprise,
                _ => Tactic::Fire,
            };
            let def_t = match rng.gen_range(0..3) {
                0 => Tactic::Normal,
                1 => Tactic::Surprise,
                _ => Tactic::Fire,
            };

            status = Self::calculate_turn(status, atk_t, def_t)?;
            turn += 1;
        }

        // 10ターンで決着がつかなければ防衛勝利
        if status.winner.is_none() {
            status.winner = Some(BattleSide::Defender);
        }

        Ok((status, turn - 1))
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
