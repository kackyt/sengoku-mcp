#[cfg(test)]
mod tests {
    use crate::domain::model::{
        event::SeasonalEventType,
        kuni::Kuni,
        resource::{DevelopmentStats, Resource},
        value_objects::{DaimyoId, IninFlag, KuniId, TurnNumber},
    };
    use crate::domain::service::seasonal_event_service::SeasonalEventService;

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
    fn test_gold_income_spring() {
        let mut kuni = create_test_kuni(10000, 80);
        let service = SeasonalEventService::new();
        
        // 1ターン目はスキップされるため、2回目の春である5ターン目でテストする
        let effects = service.process_end_turn_events(TurnNumber::new(5), &mut kuni);
        
        // 春(5)終了時は人口増加と金収入が発生するはず
        assert!(effects.iter().any(|e| e.event_type == SeasonalEventType::PopulationGrowth));
        assert!(effects.iter().any(|e| e.event_type == SeasonalEventType::GoldIncome));
        
        // 金が増えているか確認
        assert!(kuni.resource.kin.value() > 1000);
    }

    #[test]
    fn test_rice_income_autumn() {
        let mut kuni = create_test_kuni(10000, 80);
        let service = SeasonalEventService::new();

        let effects = service.process_end_turn_events(TurnNumber::new(3), &mut kuni);

        // 秋(3)終了時は米収入が発生するはず
        assert!(effects
            .iter()
            .any(|e| e.event_type == SeasonalEventType::RiceIncome));

        // 米が増えているか確認
        assert!(kuni.resource.kome.value() > 1000);
    }

    #[test]
    fn test_rebellion_probability() {
        let kuni = create_test_kuni(10000, 10); // 忠誠度 10 -> 反乱確率 40%
        let service = SeasonalEventService::new();

        // 100回試行して、何度か反乱が発生することを確認（確率はランダムだが40%ならほぼ確実に数回は出る）
        let mut rebellion_count = 0;
        for _ in 0..100 {
            let mut test_kuni = kuni.clone();
            let effects = service.process_start_turn_events(TurnNumber::new(2), &mut test_kuni);
            if effects
                .iter()
                .any(|e| e.event_type == SeasonalEventType::Rebellion)
            {
                rebellion_count += 1;
            }
        }

        assert!(rebellion_count > 0);
    }
}
