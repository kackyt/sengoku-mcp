#[cfg(test)]
mod tests {
    use crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
    use crate::domain::error::DomainError;
    use crate::domain::model::{
        daimyo::Daimyo,
        event::GameEvent,
        game_state::GameState,
        kuni::Kuni,
        resource::{DevelopmentStats, Resource},
        value_objects::{DaimyoId, IninFlag, KuniId},
    };
    use crate::domain::repository::{
        daimyo_repository::DaimyoRepository, event_dispatcher::EventDispatcher,
        game_state_repository::GameStateRepository, kuni_repository::KuniRepository,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use uuid::Uuid;

    // --- Mocks ---

    struct MockKuniRepository {
        kunis: Arc<RwLock<HashMap<KuniId, Kuni>>>,
    }
    impl MockKuniRepository {
        fn new() -> Self {
            Self {
                kunis: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        async fn setup(&self, kuni: Kuni) {
            self.kunis.write().await.insert(kuni.id, kuni);
        }
    }
    #[async_trait]
    impl KuniRepository for MockKuniRepository {
        async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError> {
            Ok(self.kunis.read().await.get(id).cloned())
        }
        async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, DomainError> {
            Ok(self
                .kunis
                .read()
                .await
                .values()
                .filter(|k| k.daimyo_id == *daimyo_id)
                .cloned()
                .collect())
        }
        async fn save(&self, kuni: &Kuni) -> Result<(), DomainError> {
            self.kunis.write().await.insert(kuni.id, kuni.clone());
            Ok(())
        }
        async fn find_all(&self) -> Result<Vec<Kuni>, DomainError> {
            Ok(self.kunis.read().await.values().cloned().collect())
        }
    }

    struct MockDaimyoRepository {
        daimyos: Arc<RwLock<HashMap<DaimyoId, Daimyo>>>,
    }
    impl MockDaimyoRepository {
        fn new() -> Self {
            Self {
                daimyos: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        async fn setup(&self, daimyo: Daimyo) {
            self.daimyos.write().await.insert(daimyo.id, daimyo);
        }
    }
    #[async_trait]
    impl DaimyoRepository for MockDaimyoRepository {
        async fn find_by_id(&self, id: &DaimyoId) -> Result<Option<Daimyo>, DomainError> {
            Ok(self.daimyos.read().await.get(id).cloned())
        }
        async fn save(&self, daimyo: &Daimyo) -> Result<(), DomainError> {
            self.daimyos.write().await.insert(daimyo.id, daimyo.clone());
            Ok(())
        }
        async fn find_all(&self) -> Result<Vec<Daimyo>, DomainError> {
            Ok(self.daimyos.read().await.values().cloned().collect())
        }
    }

    struct MockGameStateRepository {
        state: Arc<RwLock<Option<GameState>>>,
    }
    impl MockGameStateRepository {
        fn new() -> Self {
            Self {
                state: Arc::new(RwLock::new(None)),
            }
        }
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

    struct MockEventDispatcher {
        events: Arc<RwLock<Vec<GameEvent>>>,
    }
    impl MockEventDispatcher {
        fn new() -> Self {
            Self {
                events: Arc::new(RwLock::new(Vec::new())),
            }
        }
        async fn get_events(&self) -> Vec<GameEvent> {
            self.events.read().await.clone()
        }
    }
    #[async_trait]
    impl EventDispatcher for MockEventDispatcher {
        async fn dispatch(&self, event: GameEvent) -> Result<(), DomainError> {
            self.events.write().await.push(event);
            Ok(())
        }
    }

    // --- Helpers ---

    fn create_test_daimyo(name: &str) -> Daimyo {
        Daimyo::new(DaimyoId(Uuid::new_v4()), name)
    }

    fn create_test_kuni(daimyo_id: DaimyoId) -> Kuni {
        Kuni::new(
            KuniId::new(),
            "TestKuni",
            daimyo_id,
            Resource::new(1000 * 10, 1000 * 10, 1000 * 10, 10000 * 10),
            DevelopmentStats::new(100 * 10, 100 * 10, 50),
            IninFlag(false),
        )
    }

    // --- Tests ---

    #[tokio::test]
    async fn test_full_turn_progression() {
        let kuni_repo = Arc::new(MockKuniRepository::new());
        let daimyo_repo = Arc::new(MockDaimyoRepository::new());
        let state_repo = Arc::new(MockGameStateRepository::new());
        let event_dispatcher = Arc::new(MockEventDispatcher::new());

        let daimyo1 = create_test_daimyo("織田信長");
        let daimyo2 = create_test_daimyo("武田信玄");

        let kuni1 = create_test_kuni(daimyo1.id);
        let kuni2 = create_test_kuni(daimyo2.id);

        daimyo_repo.setup(daimyo1.clone()).await;
        daimyo_repo.setup(daimyo2.clone()).await;
        kuni_repo.setup(kuni1.clone()).await;
        kuni_repo.setup(kuni2.clone()).await;

        let usecase = TurnProgressionUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            state_repo.clone(),
            event_dispatcher.clone(),
        );

        // 1. 初回進行: ターン1の開始をセットアップし、最初の大名が行動
        usecase.progress().await.expect("進行成功");

        let state = state_repo.get().await.unwrap().unwrap();
        assert_eq!(state.current_turn().value(), 1);
        assert_eq!(state.action_order().len(), 2);
        assert!(state.current_daimyo().is_some());

        // 一人目の行動イベントが発火していることを確認
        let events = event_dispatcher.get_events().await;
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::TurnStarted { turn } if turn.value() == 1)));
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::DaimyoActionStarted { .. })));

        // 2. 2番目の大名が行動
        usecase.progress().await.expect("進行成功");

        let state2 = state_repo.get().await.unwrap().unwrap();
        assert!(state2.is_turn_completed());

        // 3. ターンの終了処理（季節処理と次ターンへの移行）
        usecase.progress().await.expect("進行成功");

        let state3 = state_repo.get().await.unwrap().unwrap();
        assert_eq!(state3.current_turn().value(), 2); // ターンが進んでいる
        assert!(state3.current_daimyo().is_some());

        let final_events = event_dispatcher.get_events().await;
        assert!(final_events
            .iter()
            .any(|e| matches!(e, GameEvent::SeasonPassed { turn } if turn.value() == 1)));
        assert!(final_events
            .iter()
            .any(|e| matches!(e, GameEvent::TurnStarted { turn } if turn.value() == 2)));
    }
}
