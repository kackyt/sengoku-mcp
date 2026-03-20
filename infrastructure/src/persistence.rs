use engine::domain::error::DomainError;
use engine::domain::model::event::GameEvent;
use engine::domain::model::game_state::GameState;
use engine::domain::repository::event_dispatcher::EventDispatcher;
use engine::domain::repository::game_state_repository::GameStateRepository;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PersistenceManager {}

/// インメモリでのゲーム状態リポジトリの仮実装
pub struct InMemoryGameStateRepository {
    state: Arc<RwLock<Option<GameState>>>,
}

impl InMemoryGameStateRepository {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(None::<GameState>)),
        }
    }
}

impl Default for InMemoryGameStateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl GameStateRepository for InMemoryGameStateRepository {
    async fn get(&self) -> Result<Option<GameState>, DomainError> {
        let guard = self.state.read().await;
        Ok(guard.clone())
    }

    async fn save(&self, state: &GameState) -> Result<(), DomainError> {
        let mut guard = self.state.write().await;
        *guard = Some(state.clone());
        Ok(())
    }
}

/// インメモリイベントディスパッチャの仮実装
pub struct InMemoryEventDispatcher {
    events: Arc<RwLock<Vec<GameEvent>>>,
}

impl InMemoryEventDispatcher {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_events(&self) -> Vec<GameEvent> {
        self.events.read().await.clone()
    }

    pub async fn clear_events(&self) {
        self.events.write().await.clear();
    }
}

impl Default for InMemoryEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EventDispatcher for InMemoryEventDispatcher {
    async fn dispatch(&self, event: GameEvent) -> Result<(), DomainError> {
        self.events.write().await.push(event);
        Ok(())
    }
}
