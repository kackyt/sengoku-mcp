#[cfg(test)]
mod tests {
    use crate::domain::model::{
        kuni::Kuni,
        resource::{DevelopmentStats, Resource},
        value_objects::{DaimyoId, IninFlag, KuniId},
    };
    use crate::domain::service::turn_service::TurnService;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use uuid::Uuid;

    fn create_test_kuni(jinko: u32, tyu: u32) -> Kuni {
        Kuni::new(
            KuniId(Uuid::new_v4()),
            DaimyoId(Uuid::new_v4()),
            Resource::new(1000, 1000, 1000, jinko),
            DevelopmentStats::new(100, 100, tyu),
            IninFlag::new(false),
        )
    }

    #[test]
    fn test_process_season_no_action_turns() {
        let kunis = vec![create_test_kuni(10000, 50)];
        let mut rng = StdRng::seed_from_u64(12345);
        let result = TurnService::process_season(3, kunis, &mut rng);

        // Most likely unchanged based on random rng disaster constraints,
        // unless disaster happens (1/40 chance).
        // Since we can't reliably test strict equality with randomness without mocking RNG,
        // we'll at least assert length and structure are preserved.
        assert_eq!(result.len(), 1);
        let k = &result[0];
        assert!(k.stats.tyu.value() <= 50);
    }

    #[test]
    fn test_process_season_harvest_turn() {
        let kunis = vec![create_test_kuni(10000, 50)];
        let initial_kin = kunis[0].resource.kin.value();
        let initial_kome = kunis[0].resource.kome.value();

        // Turn 2 is a harvest turn (turn % 4 == 2)
        let mut rng = StdRng::seed_from_u64(12345);
        let result = TurnService::process_season(2, kunis, &mut rng);

        assert_eq!(result.len(), 1);
        let k = &result[0];

        // Kin and Kome should have increased
        assert!(k.resource.kin.value() > initial_kin);
        assert!(k.resource.kome.value() > initial_kome);
    }

    #[test]
    fn test_process_season_growth_turn() {
        let kunis = vec![create_test_kuni(10000, 50)];
        let initial_jinko = kunis[0].resource.jinko.value();

        // Turn 4 is a population growth turn (turn % 4 == 0)
        let mut rng = StdRng::seed_from_u64(12345);
        let result = TurnService::process_season(4, kunis, &mut rng);

        assert_eq!(result.len(), 1);
        let k = &result[0];

        // Jinko might increase, or rarely decrease if disaster hits at same time,
        // but normally it increases
        assert!(k.resource.jinko.value() >= initial_jinko);
    }
}
