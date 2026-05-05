#[cfg(test)]
mod tests {
    use crate::domain::model::{
        event::SeasonalEventType,
        kuni::Kuni,
        resource::{DevelopmentStats, Resource},
        value_objects::{DaimyoId, IninFlag, KuniId, TurnNumber},
    };
    use crate::domain::service::turn_service::TurnService;

    fn create_test_kuni(jinko: u32, tyu: u32) -> Kuni {
        Kuni::new(
            KuniId::new(1),
            "TestKuni".to_string(),
            DaimyoId(1),
            Resource::new(1000, 1000, 1000, jinko),
            DevelopmentStats::new(100, 100, tyu),
            IninFlag(false),
        )
    }

    #[test]
    fn test_process_end_turn_autumn_rice_income() {
        let mut kunis = vec![create_test_kuni(10000, 50)];
        let initial_kome = kunis[0].resource.kome.value();

        // ターン3は秋（(3-1)%4 == 2）→ 米収入
        let effects = TurnService::process_end_turn_events(TurnNumber::new(3), &mut kunis);

        assert_eq!(kunis.len(), 1);
        // 米が増えているか確認
        assert!(kunis[0].resource.kome.value() > initial_kome);
        // RiceIncomeイベントが発生しているか確認
        assert!(effects
            .iter()
            .any(|e| e.event_type == SeasonalEventType::RiceIncome));
    }

    #[test]
    fn test_process_end_turn_spring_population_and_gold() {
        let mut kunis = vec![create_test_kuni(10000, 50)];
        let initial_kin = kunis[0].resource.kin.value();
        let initial_jinko = kunis[0].resource.jinko.value();

        // ターン5は春（(5-1)%4 == 0）→ 人口増加と金収入
        // ただし1ターン目はスキップされるため5ターン目でテスト
        let effects = TurnService::process_end_turn_events(TurnNumber::new(5), &mut kunis);

        assert_eq!(kunis.len(), 1);
        // 金が増えているか確認
        assert!(kunis[0].resource.kin.value() > initial_kin);
        // 人口が増えているか確認
        assert!(kunis[0].resource.jinko.value() > initial_jinko);
        // GoldIncomeとPopulationGrowthイベントが発生しているか確認
        assert!(effects
            .iter()
            .any(|e| e.event_type == SeasonalEventType::GoldIncome));
        assert!(effects
            .iter()
            .any(|e| e.event_type == SeasonalEventType::PopulationGrowth));
    }

    #[test]
    fn test_process_start_turn_action_order() {
        let kuni1 = create_test_kuni(10000, 80);
        let kuni2 = Kuni::new(
            KuniId::new(2),
            "TestKuni2",
            DaimyoId(2),
            Resource::new(1000, 1000, 1000, 5000),
            DevelopmentStats::new(100, 100, 70),
            IninFlag(false),
        );
        let kunis = vec![kuni1, kuni2];
        let mut rng = rand::thread_rng();

        // 行動順の決定が正しく機能することを確認
        let order = TurnService::determine_action_order(&kunis, &mut rng);
        assert_eq!(order.len(), 2);
        assert!(order.contains(&KuniId::new(1)));
        assert!(order.contains(&KuniId::new(2)));
    }

    #[test]
    fn test_process_start_turn_rebellion_with_low_loyalty() {
        // 忠誠度10 → 反乱確率40%
        let kunis = vec![create_test_kuni(10000, 10)];

        // 100回試行して少なくとも1回反乱が発生することを確認
        let mut rebellion_count = 0;
        for _ in 0..100 {
            let mut test_kunis = kunis.clone();
            let effects =
                TurnService::process_start_turn_events(TurnNumber::new(2), &mut test_kunis);
            if effects
                .iter()
                .any(|e| e.event_type == SeasonalEventType::Rebellion)
            {
                rebellion_count += 1;
            }
        }

        // 反乱確率40%なので100回中に少なくとも1回は発生するはず
        assert!(rebellion_count > 0);
    }
}
