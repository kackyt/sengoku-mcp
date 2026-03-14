#[cfg(test)]
mod tests {
    use crate::domain::model::{
        kuni::Kuni,
        resource::{DevelopmentStats, Resource},
        value_objects::{DaimyoId, KuniId},
    };
    use crate::domain::service::battle_service::{BattleService, BattleSide, Tactic};
    use uuid::Uuid;

    fn create_test_kuni(hei: u32, kome: u32, tyu: u32) -> Kuni {
        Kuni::new(
            KuniId(Uuid::new_v4()),
            DaimyoId(Uuid::new_v4()),
            Resource::new(1000, hei, kome, 10000),
            DevelopmentStats::new(100, 100, tyu),
            false,
        )
    }

    #[test]
    fn test_normal_tactic_damage() {
        let attacker = create_test_kuni(1000, 1000, 50);
        let defender = create_test_kuni(1000, 1000, 50);

        let result = BattleService::calculate_turn(
            attacker,
            defender,
            Tactic::Normal,
            Tactic::Normal,
            500, // Attacker sends 500 troops
        )
        .unwrap();

        // 500 * 1.8 = 900 damage expected on defender
        assert_eq!(result.defender_kuni.resource.hei.value(), 1000 - 900);

        // Attacker food cost: 500 * 0.3 = 150
        assert_eq!(result.attacker_kuni.resource.kome.value(), 1000 - 150);

        assert_eq!(result.winner, None);
    }

    #[test]
    fn test_surprise_tactic_advantage() {
        let attacker = create_test_kuni(1000, 1000, 50);
        let defender = create_test_kuni(1000, 1000, 50);

        let result = BattleService::calculate_turn(
            attacker,
            defender,
            Tactic::Surprise,
            Tactic::Normal,
            500,
        )
        .unwrap();

        // 500 * 0.4 = 200 damage on defender
        assert_eq!(result.defender_kuni.resource.hei.value(), 1000 - 200);

        // Attacker food cost: 500 * 0.3 = 150
        assert_eq!(result.attacker_kuni.resource.kome.value(), 1000 - 150);

        // Morale changes: attacker +10, defender -10
        assert_eq!(result.attacker_kuni.stats.tyu.value(), 60);
        assert_eq!(result.defender_kuni.stats.tyu.value(), 40);
    }

    #[test]
    fn test_fire_tactic_defender_food_loss() {
        let attacker = create_test_kuni(1000, 1000, 50);
        let defender = create_test_kuni(1000, 1000, 50);

        let result =
            BattleService::calculate_turn(attacker, defender, Tactic::Fire, Tactic::Normal, 500)
                .unwrap();

        // Defender loses 50% food
        assert_eq!(result.defender_kuni.resource.kome.value(), 500);

        // Morale changes: attacker +10, defender -10
        assert_eq!(result.attacker_kuni.stats.tyu.value(), 60);
        assert_eq!(result.defender_kuni.stats.tyu.value(), 40);

        assert_eq!(result.winner, None);
    }

    #[test]
    fn test_battle_victory_condition() {
        let attacker = create_test_kuni(1000, 1000, 50);
        // Weak defender about to run out of troops
        let defender = create_test_kuni(1000, 1000, 50);

        // Attacker deals 1800 damage, defender has 1000 troops, so it goes to 0
        let result =
            BattleService::calculate_turn(attacker, defender, Tactic::Normal, Tactic::Normal, 1000)
                .unwrap();

        assert_eq!(result.defender_kuni.resource.hei.value(), 0);
        assert_eq!(result.winner, Some(BattleSide::Attacker));
    }
}
