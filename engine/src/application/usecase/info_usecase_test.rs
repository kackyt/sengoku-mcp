#[cfg(test)]
mod tests {
    use crate::application::usecase::info_usecase::InfoUseCase;
    use crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
    use crate::domain::error::DomainError;
    use crate::domain::model::action_log::{ActionLogCategory, ActionLogEntry};
    use crate::domain::model::daimyo::Daimyo;
    use crate::domain::model::daimyo_personality::DaimyoPersonality;
    use crate::domain::model::event::GameEvent;
    use crate::domain::model::game_state::GameState;
    use crate::domain::model::kuni::Kuni;
    use crate::domain::model::resource::{DevelopmentStats, Resource};
    use crate::domain::model::value_objects::{
        ActionOrderIndex, DaimyoId, DisplayAmount, IninFlag, KuniId, TurnNumber, INTERNAL_SCALE,
    };
    use crate::domain::repository::action_log_repository::ActionLogRepository;
    use crate::domain::repository::battle_repository::BattleRepository;
    use crate::domain::repository::daimyo_repository::DaimyoRepository;
    use crate::domain::repository::event_dispatcher::EventDispatcher;
    use crate::domain::repository::game_state_repository::GameStateRepository;
    use crate::domain::repository::kuni_repository::KuniRepository;
    use crate::domain::repository::neighbor_repository::NeighborRepository;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // --- Mocks ---

    struct MockKuniRepository {
        kunis: RwLock<HashMap<KuniId, Kuni>>,
    }
    #[async_trait]
    impl KuniRepository for MockKuniRepository {
        async fn find_all(&self) -> Result<Vec<Kuni>, DomainError> {
            Ok(self.kunis.read().await.values().cloned().collect())
        }
        async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError> {
            Ok(self.kunis.read().await.get(id).cloned())
        }
        async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, DomainError> {
            let mut kunis: Vec<_> = self
                .kunis
                .read()
                .await
                .values()
                .filter(|k| &k.daimyo_id == daimyo_id)
                .cloned()
                .collect();
            kunis.sort_by_key(|k| k.id);
            Ok(kunis)
        }
        async fn save(&self, kuni: &Kuni) -> Result<(), DomainError> {
            self.kunis.write().await.insert(kuni.id, kuni.clone());
            Ok(())
        }
    }

    struct MockDaimyoRepository {
        daimyos: RwLock<HashMap<DaimyoId, Daimyo>>,
    }
    #[async_trait]
    impl DaimyoRepository for MockDaimyoRepository {
        async fn find_all(&self) -> Result<Vec<Daimyo>, DomainError> {
            let mut daimyos: Vec<_> = self.daimyos.read().await.values().cloned().collect();
            daimyos.sort_by_key(|d| d.id);
            Ok(daimyos)
        }
        async fn find_by_id(&self, id: &DaimyoId) -> Result<Option<Daimyo>, DomainError> {
            Ok(self.daimyos.read().await.get(id).cloned())
        }
        async fn save(&self, daimyo: &Daimyo) -> Result<(), DomainError> {
            self.daimyos.write().await.insert(daimyo.id, daimyo.clone());
            Ok(())
        }
    }

    struct MockGameStateRepository {
        state: RwLock<Option<GameState>>,
    }
    #[async_trait]
    impl GameStateRepository for MockGameStateRepository {
        async fn get(&self) -> Result<Option<GameState>, DomainError> {
            Ok(self.state.read().await.clone())
        }
        async fn save(&self, state: &GameState) -> Result<(), DomainError> {
            *self.state.write().await = Some(state.clone());
            Ok(())
        }
    }

    struct MockEventDispatcher;
    #[async_trait]
    impl EventDispatcher for MockEventDispatcher {
        async fn dispatch(&self, _event: GameEvent) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct MockActionLogRepository;
    impl ActionLogRepository for MockActionLogRepository {
        fn save(&self, _entry: ActionLogEntry) -> Result<(), DomainError> {
            Ok(())
        }
        fn find_visible(
            &self,
            _category: ActionLogCategory,
            _limit: usize,
        ) -> Result<Vec<ActionLogEntry>, DomainError> {
            Ok(vec![])
        }
        fn find_all(
            &self,
            _category: ActionLogCategory,
        ) -> Result<Vec<ActionLogEntry>, DomainError> {
            Ok(vec![])
        }
        fn clear(&self, _category: ActionLogCategory) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct MockBattleRepository;
    #[async_trait]
    impl BattleRepository for MockBattleRepository {
        async fn save(
            &self,
            _status: &crate::domain::model::battle::WarStatus,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn find_by_attacker(
            &self,
            _attacker_id: &KuniId,
        ) -> Result<Option<crate::domain::model::battle::WarStatus>, DomainError> {
            Ok(None)
        }
        async fn find_by_defender(
            &self,
            _defender_id: &KuniId,
        ) -> Result<Option<crate::domain::model::battle::WarStatus>, DomainError> {
            Ok(None)
        }
        async fn find_all(
            &self,
        ) -> Result<Vec<crate::domain::model::battle::WarStatus>, DomainError> {
            Ok(vec![])
        }
        async fn delete_by_attacker(&self, _attacker_id: &KuniId) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct MockNeighborRepository;
    impl NeighborRepository for MockNeighborRepository {
        fn get_neighbors(&self, _kuni_id: &KuniId) -> Vec<KuniId> {
            vec![]
        }
        fn are_adjacent(&self, _a: &KuniId, _b: &KuniId) -> bool {
            false
        }
    }

    // --- Helper ---
    fn to_internal(val: u32) -> u32 {
        val * INTERNAL_SCALE
    }

    // --- Tests ---

    #[tokio::test]
    async fn test_get_other_countries_info() {
        let kuni_repo = Arc::new(MockKuniRepository {
            kunis: RwLock::new(HashMap::new()),
        });
        let daimyo_repo = Arc::new(MockDaimyoRepository {
            daimyos: RwLock::new(HashMap::new()),
        });
        let game_state_repo = Arc::new(MockGameStateRepository {
            state: RwLock::new(None),
        });
        let event_dispatcher = Arc::new(MockEventDispatcher);
        let action_log_repo = Arc::new(MockActionLogRepository);

        let player_id = DaimyoId(1);
        let enemy_id = DaimyoId(2);

        daimyo_repo
            .save(&Daimyo::new(
                player_id,
                "プレイヤー",
                DaimyoPersonality::default(),
            ))
            .await
            .unwrap();
        daimyo_repo
            .save(&Daimyo::new(
                enemy_id,
                "敵大名",
                DaimyoPersonality::default(),
            ))
            .await
            .unwrap();

        let player_kuni_id = KuniId(1);
        kuni_repo
            .save(&Kuni::new(
                player_kuni_id,
                "プレイヤー国",
                player_id,
                Resource::new(
                    to_internal(100),
                    to_internal(10),
                    to_internal(100),
                    to_internal(500),
                ),
                DevelopmentStats::new(to_internal(10), to_internal(10), 100), // tyu is Rate, not scaled by INTERNAL_SCALE?
                IninFlag(false),
            ))
            .await
            .unwrap();

        let enemy_kuni_id = KuniId(10);
        kuni_repo
            .save(&Kuni::new(
                enemy_kuni_id,
                "敵国",
                enemy_id,
                Resource::new(
                    to_internal(1000),
                    to_internal(500),
                    to_internal(1000),
                    to_internal(2000),
                ),
                DevelopmentStats::new(to_internal(200), to_internal(150), 80),
                IninFlag(false),
            ))
            .await
            .unwrap();

        let game_state = GameState::new(
            TurnNumber::new(1),
            vec![player_kuni_id, enemy_kuni_id],
            ActionOrderIndex::new(0),
        )
        .unwrap();
        game_state_repo.save(&game_state).await.unwrap();

        let turn_progression_usecase = Arc::new(TurnProgressionUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            event_dispatcher.clone(),
            action_log_repo.clone(),
            Arc::new(MockBattleRepository),
            Arc::new(MockNeighborRepository),
        ));

        let info_usecase = InfoUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            turn_progression_usecase.clone(),
        );

        let result = info_usecase
            .get_other_countries_info(player_id)
            .await
            .unwrap();

        assert_eq!(result.countries.len(), 1);
        assert_eq!(result.countries[0].kuni_id, 10);
        assert_eq!(result.countries[0].kuni_name, "敵国");
        assert_eq!(result.countries[0].daimyo_name, "敵大名");
        assert_eq!(result.countries[0].kin, DisplayAmount::new(1000));
        assert_eq!(result.countries[0].kome, DisplayAmount::new(1000));
        assert_eq!(result.countries[0].hei, DisplayAmount::new(500));
        assert_eq!(result.countries[0].kokudaka, DisplayAmount::new(200));
        assert_eq!(result.countries[0].towns, DisplayAmount::new(150));
        assert_eq!(result.countries[0].tyu, 80);

        let updated_state = game_state_repo.get().await.unwrap().unwrap();
        assert_eq!(updated_state.current_action_index().value(), 1);
    }
}
