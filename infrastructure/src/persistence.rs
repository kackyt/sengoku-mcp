use engine::domain::error::DomainError;
use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::event::GameEvent;
use engine::domain::model::game_state::GameState;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::value_objects::{DaimyoId, KuniId};
use engine::domain::repository::daimyo_repository::DaimyoRepository;
use engine::domain::repository::event_dispatcher::EventDispatcher;
use engine::domain::repository::game_state_repository::GameStateRepository;
use engine::domain::repository::kuni_repository::KuniRepository;
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
    adjacency_map: Arc<std::sync::RwLock<HashMap<KuniId, Vec<KuniId>>>>,
}

impl InMemoryNeighborRepository {
    /// 新しいインスタンスを作成する
    pub fn new() -> Self {
        Self {
            adjacency_map: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// データから初期化する
    pub fn init_with_data(&self, adjacency_map: HashMap<KuniId, Vec<KuniId>>) {
        let mut guard = self.adjacency_map.write().unwrap();
        *guard = adjacency_map;
    }
}

impl Default for InMemoryNeighborRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl NeighborRepository for InMemoryNeighborRepository {
    /// 指定された国の隣接国リストを取得する
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.adjacency_map.read().unwrap().get(kuni_id).cloned().unwrap_or_default()
    }

    /// 2つの国が隣接しているか判定する
    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool {
        self.adjacency_map
            .read()
            .unwrap()
            .get(a)
            .is_some_and(|neighbors| neighbors.contains(b))
    }
}

/// インメモリでの国リポジトリの仮実装
pub struct InMemoryKuniRepository {
    kunis: Arc<RwLock<HashMap<KuniId, Kuni>>>,
}

impl InMemoryKuniRepository {
    pub fn new() -> Self {
        Self {
            kunis: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn init_with_data(&self, kunis: Vec<Kuni>) {
        let mut guard = self.kunis.write().await;
        for kuni in kunis {
            guard.insert(kuni.id, kuni);
        }
    }
}

impl Default for InMemoryKuniRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl KuniRepository for InMemoryKuniRepository {
    async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError> {
        let guard = self.kunis.read().await;
        Ok(guard.get(id).cloned())
    }

    async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, DomainError> {
        let guard = self.kunis.read().await;
        Ok(guard
            .values()
            .filter(|k| &k.daimyo_id == daimyo_id)
            .cloned()
            .collect())
    }

    async fn save(&self, kuni: &Kuni) -> Result<(), DomainError> {
        let mut guard = self.kunis.write().await;
        guard.insert(kuni.id, kuni.clone());
        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Kuni>, DomainError> {
        let guard = self.kunis.read().await;
        Ok(guard.values().cloned().collect())
    }
}

/// インメモリでの大名リポジトリの仮実装
pub struct InMemoryDaimyoRepository {
    daimyos: Arc<RwLock<HashMap<DaimyoId, Daimyo>>>,
}

impl InMemoryDaimyoRepository {
    pub fn new() -> Self {
        Self {
            daimyos: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn init_with_data(&self, daimyos: Vec<Daimyo>) {
        let mut guard = self.daimyos.write().await;
        for daimyo in daimyos {
            guard.insert(daimyo.id, daimyo);
        }
    }
}

impl Default for InMemoryDaimyoRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DaimyoRepository for InMemoryDaimyoRepository {
    async fn find_by_id(&self, id: &DaimyoId) -> Result<Option<Daimyo>, DomainError> {
        let guard = self.daimyos.read().await;
        Ok(guard.get(id).cloned())
    }

    async fn save(&self, daimyo: &Daimyo) -> Result<(), DomainError> {
        let mut guard = self.daimyos.write().await;
        guard.insert(daimyo.id, daimyo.clone());
        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Daimyo>, DomainError> {
        let guard = self.daimyos.read().await;
        Ok(guard.values().cloned().collect())
    }
}
