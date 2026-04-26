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
                kuni_id: KuniId::new(),
                hei: Amount::new(a_hei),
                kome: Amount::new(a_kome),
                morale: Rate::new(50),
            },
            defender: ArmyStatus {
                kuni_id: KuniId::new(),
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
        let status = create_test_status(500, 1000, 1000, 1000);

        let result = BattleService::calculate_turn(status, Tactic::Normal, Tactic::Normal).unwrap();

        // 500 * 1.8 = 900 damage expected on defender
        assert_eq!(result.defender.hei.value(), 1000 - 900);

        // Attacker food cost: 500 * 0.3 = 150
        assert_eq!(result.attacker.kome.value(), 1000 - 150);

        assert_eq!(result.winner, None);
    }

    #[test]
    fn test_surprise_tactic_advantage() {
        let status = create_test_status(500, 1000, 1000, 1000);

        let result =
            BattleService::calculate_turn(status, Tactic::Surprise, Tactic::Normal).unwrap();

        // 500 * 0.4 = 200 damage on defender
        assert_eq!(result.defender.hei.value(), 1000 - 200);

        // Attacker food cost: 500 * 0.3 = 150
        assert_eq!(result.attacker.kome.value(), 1000 - 150);

        // Morale changes: attacker +10, defender -10
        assert_eq!(result.attacker.morale.value(), 60);
        assert_eq!(result.defender.morale.value(), 40);
    }

    #[test]
    fn test_fire_tactic_defender_food_loss() {
        let status = create_test_status(500, 1000, 1000, 1000);

        let result = BattleService::calculate_turn(status, Tactic::Fire, Tactic::Normal).unwrap();

        // Defender loses 50% food
        assert_eq!(result.defender.kome.value(), 500);

        // Morale changes: attacker +10, defender -10
        assert_eq!(result.attacker.morale.value(), 60);
        assert_eq!(result.defender.morale.value(), 40);

        assert_eq!(result.winner, None);
    }

    #[test]
    fn test_battle_victory_condition() {
        let status = create_test_status(1000, 1000, 1000, 1000);
        // 攻撃側の兵1000人で攻撃。1.8倍ダメージで1800ダメージ。
        // 防御側の兵は1000人なので、0になるはず。
        let result = BattleService::calculate_turn(status, Tactic::Normal, Tactic::Normal).unwrap();

        assert_eq!(result.defender.hei.value(), 0);
        assert_eq!(result.winner, Some(BattleSide::Attacker));
    }
}
