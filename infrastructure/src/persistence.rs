use engine::domain::error::DomainError;
use engine::domain::model::event::GameEvent;
use engine::domain::model::game_state::GameState;
use engine::domain::model::value_objects::KuniId;
use engine::domain::repository::event_dispatcher::EventDispatcher;
use engine::domain::repository::game_state_repository::GameStateRepository;
use engine::domain::repository::neighbor_repository::NeighborRepository;
use std::collections::HashMap;
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

/// 隣接国情報をメモリ上に保持するリポジトリの実装
pub struct InMemoryNeighborRepository {
    adjacency_map: HashMap<KuniId, Vec<KuniId>>,
}

impl InMemoryNeighborRepository {
    /// 新しいインスタンスを作成する
    pub fn new(adjacency_map: HashMap<KuniId, Vec<KuniId>>) -> Self {
        Self { adjacency_map }
    }
}

impl NeighborRepository for InMemoryNeighborRepository {
    /// 指定された国の隣接国リストを取得する
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.adjacency_map.get(kuni_id).cloned().unwrap_or_default()
    }

    /// 2つの国が隣接しているか判定する
    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool {
        self.adjacency_map
            .get(a)
            .is_some_and(|neighbors| neighbors.contains(b))
    }
}
