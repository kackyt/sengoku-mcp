#[cfg(test)]
mod tests {
    use crate::domain::model::battle::{
        ArmyStatus, BattleAdvantage, BattleSide, Tactic, WarStatus,
    };
    use crate::domain::model::value_objects::{Amount, KuniId, Rate};
    use crate::domain::service::battle_service::BattleService;

    fn create_test_status(a_hei: u32, a_kome: u32, d_hei: u32, d_kome: u32) -> WarStatus {
        WarStatus {
            attacker: ArmyStatus {
                kuni_id: KuniId::new(1),
                hei: Amount::new(a_hei),
                kome: Amount::new(a_kome),
                morale: Rate::new(50),
            },
            defender: ArmyStatus {
                kuni_id: KuniId::new(2),
                hei: Amount::new(d_hei),
                kome: Amount::new(d_kome),
                morale: Rate::new(50),
            },
            winner: None,
            advantage: BattleAdvantage::Even,
        }
    }

    #[test]
    fn test_normal_tactic_damage() {
        // 攻撃側 2000, 防御側 1000
        let status = create_test_status(2000, 1000, 1000, 1000);

        let result = BattleService::calculate_turn(status, Tactic::Normal, Tactic::Normal).unwrap();

        // 攻撃側からのダメージ: 2000 * 1.8 = 3600 -> 防御側は 0
        assert_eq!(result.defender.hei.value(), 0);

        // 防御側からの反撃ダメージ (Normal vs Normal):
        // 1000 * 1.8 = 1800 -> 攻撃側(2000)は 200
        assert_eq!(result.attacker.hei.value(), 200);

        // 防御側が全滅したため攻撃側の勝利
        assert_eq!(result.winner, Some(BattleSide::Attacker));
    }

    #[test]
    fn test_surprise_tactic_counter_damage() {
        let status = create_test_status(1000, 1000, 500, 1000);

        // 攻撃側: Surprise, 防御側: Normal (不一致 -> Surprise Success 300%)
        let result =
            BattleService::calculate_turn(status, Tactic::Surprise, Tactic::Normal).unwrap();

        // 攻撃側からのダメージ: 1000 * 3.0 = 3000 -> 防御側(500)は 0
        assert_eq!(result.defender.hei.value(), 0);

        // 防御側からの反撃ダメージ (Normal vs Surprise -> Surprise Fail 40%):
        // 500 * 0.4 = 200 -> 攻撃側(1000)は 800
        assert_eq!(result.attacker.hei.value(), 800);

        // Morale changes:
        // Atk(Surprise) vs Def(Normal): Atk +10, Def -10
        assert_eq!(result.attacker.morale.value(), 60);
        assert_eq!(result.defender.morale.value(), 40);

        // 防御側全滅
        assert_eq!(result.winner, Some(BattleSide::Attacker));
    }

    #[test]
    fn test_fire_tactic_counter_effect() {
        let status = create_test_status(1000, 1000, 1000, 1000);

        // 両軍 火計 (Fire vs Fire -> Atk: Fail 40%, Def: Default 60%)
        let result = BattleService::calculate_turn(status, Tactic::Fire, Tactic::Fire).unwrap();

        // 攻撃側の火計ダメージ: 1000 * 0.4 = 400 -> 防御側 600
        // 防御側の火計反撃: 1000 * 0.6 = 600 -> 攻撃側 400
        assert_eq!(result.defender.hei.value(), 600);
        assert_eq!(result.attacker.hei.value(), 400);
    }

    #[test]
    fn test_battle_victory_condition() {
        let status = create_test_status(1000, 1000, 100, 1000);
        // 攻撃側 1000人, 防御側 100人
        let result = BattleService::calculate_turn(status, Tactic::Normal, Tactic::Normal).unwrap();

        assert_eq!(result.defender.hei.value(), 0);
        // 反撃: 100 * 1.8 = 180 -> 攻撃側 1000 - 180 = 820
        assert_eq!(result.attacker.hei.value(), 820);
        assert_eq!(result.winner, Some(BattleSide::Attacker));
    }
}
