use crate::application::usecase::battle_usecase::BattleUseCase;
use crate::application::usecase::domestic_usecase::DomesticUseCase;
use crate::domain::error::DomainError;
use crate::domain::model::action_log::{ActionLogCategory, ActionLogEntry};
use crate::domain::model::battle::{Tactic, WarStatus};
use crate::domain::model::daimyo::Daimyo;
use crate::domain::model::game_state::GameState;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::resource::{DevelopmentStats, Resource};
use crate::domain::model::value_objects::{DaimyoId, DisplayAmount, IninFlag, KuniId};
use crate::domain::repository::action_log_repository::ActionLogRepository;
use crate::domain::repository::battle_repository::BattleRepository;
use crate::domain::repository::daimyo_repository::DaimyoRepository;
use crate::domain::repository::game_state_repository::GameStateRepository;
use crate::domain::repository::kuni_repository::KuniRepository;
use crate::domain::repository::neighbor_repository::NeighborRepository;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// --- モックリポジトリ ---

struct MockKuniRepository {
    kunis: Mutex<HashMap<KuniId, Kuni>>,
}

impl MockKuniRepository {
    fn new() -> Self {
        Self {
            kunis: Mutex::new(HashMap::new()),
        }
    }

    fn setup(&self, kuni: Kuni) {
        self.kunis.lock().unwrap().insert(kuni.id, kuni);
    }
}

#[async_trait]
impl KuniRepository for MockKuniRepository {
    async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError> {
        Ok(self.kunis.lock().unwrap().get(id).cloned())
    }

    async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, DomainError> {
        Ok(self
            .kunis
            .lock()
            .unwrap()
            .values()
            .filter(|k| k.daimyo_id == *daimyo_id)
            .cloned()
            .collect())
    }

    async fn save(&self, kuni: &Kuni) -> Result<(), DomainError> {
        self.kunis.lock().unwrap().insert(kuni.id, kuni.clone());
        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Kuni>, DomainError> {
        Ok(self.kunis.lock().unwrap().values().cloned().collect())
    }

    async fn clear(&self) -> Result<(), DomainError> {
        self.kunis.lock().unwrap().clear();
        Ok(())
    }
}

struct MockNeighborRepository {
    adjacency_map: HashMap<KuniId, Vec<KuniId>>,
}

impl MockNeighborRepository {
    fn new() -> Self {
        Self {
            adjacency_map: HashMap::new(),
        }
    }

    fn add_neighbor(&mut self, a: KuniId, b: KuniId) {
        self.adjacency_map.entry(a).or_default().push(b);
        self.adjacency_map.entry(b).or_default().push(a);
    }
}

impl NeighborRepository for MockNeighborRepository {
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.adjacency_map.get(kuni_id).cloned().unwrap_or_default()
    }

    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool {
        self.adjacency_map
            .get(a)
            .is_some_and(|neighbors| neighbors.contains(b))
    }

    fn reset(&self, _adjacency_map: HashMap<KuniId, Vec<KuniId>>) -> Result<(), DomainError> {
        // MockNeighborRepository doesn't use Mutex for simplicity in initial implementation,
        // but for reset to work in a real use case it would need it.
        // For tests, we'll just ignore it or assume it's not called concurrently.
        // Actually, adjacency_map in struct is not Mutex.
        // I should probably change the struct to use Mutex if I want to support reset properly.
        // But let's just do a no-op or panic if it's not supposed to be called in these tests.
        Ok(())
    }
}

struct MockBattleRepository {
    wars: Mutex<HashMap<KuniId, WarStatus>>,
}

impl MockBattleRepository {
    fn new() -> Self {
        Self {
            wars: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl BattleRepository for MockBattleRepository {
    async fn find_by_attacker(
        &self,
        attacker_id: &KuniId,
    ) -> Result<Option<WarStatus>, DomainError> {
        Ok(self.wars.lock().unwrap().get(attacker_id).cloned())
    }

    async fn save(&self, status: &WarStatus) -> Result<(), DomainError> {
        self.wars
            .lock()
            .unwrap()
            .insert(status.attacker.kuni_id, status.clone());
        Ok(())
    }

    async fn find_by_defender(
        &self,
        defender_id: &KuniId,
    ) -> Result<Option<WarStatus>, DomainError> {
        Ok(self
            .wars
            .lock()
            .unwrap()
            .values()
            .find(|w| &w.defender.kuni_id == defender_id)
            .cloned())
    }

    async fn find_all(&self) -> Result<Vec<WarStatus>, DomainError> {
        Ok(self.wars.lock().unwrap().values().cloned().collect())
    }

    async fn delete_by_attacker(&self, attacker_id: &KuniId) -> Result<(), DomainError> {
        self.wars.lock().unwrap().remove(attacker_id);
        Ok(())
    }

    async fn clear(&self) -> Result<(), DomainError> {
        self.wars.lock().unwrap().clear();
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
    fn find_all(&self, _category: ActionLogCategory) -> Result<Vec<ActionLogEntry>, DomainError> {
        Ok(vec![])
    }
    fn clear(&self, _category: ActionLogCategory) -> Result<(), DomainError> {
        Ok(())
    }
}

struct MockDaimyoRepository {
    daimyos: Mutex<HashMap<DaimyoId, Daimyo>>,
}

impl MockDaimyoRepository {
    fn new() -> Self {
        Self {
            daimyos: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl DaimyoRepository for MockDaimyoRepository {
    async fn find_by_id(&self, id: &DaimyoId) -> Result<Option<Daimyo>, DomainError> {
        Ok(self.daimyos.lock().unwrap().get(id).cloned())
    }

    async fn save(&self, daimyo: &Daimyo) -> Result<(), DomainError> {
        self.daimyos
            .lock()
            .unwrap()
            .insert(daimyo.id, daimyo.clone());
        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Daimyo>, DomainError> {
        Ok(self.daimyos.lock().unwrap().values().cloned().collect())
    }

    async fn clear(&self) -> Result<(), DomainError> {
        self.daimyos.lock().unwrap().clear();
        Ok(())
    }
}

struct MockGameStateRepository {
    state: Mutex<GameState>,
}
impl MockGameStateRepository {
    fn new() -> Self {
        Self {
            state: Mutex::new(
                GameState::new(
                    crate::domain::model::value_objects::TurnNumber::new(1),
                    vec![],
                    crate::domain::model::value_objects::ActionOrderIndex::new(0),
                )
                .expect("valid turn"),
            ),
        }
    }
}
#[async_trait]
impl GameStateRepository for MockGameStateRepository {
    async fn get(&self) -> Result<Option<GameState>, DomainError> {
        Ok(Some(self.state.lock().unwrap().clone()))
    }
    async fn save(&self, state: &GameState) -> Result<(), DomainError> {
        let mut current = self.state.lock().unwrap();
        *current = state.clone();
        Ok(())
    }
    async fn clear(&self) -> Result<(), DomainError> {
        // Dummy implementation for mock
        Ok(())
    }
}

struct MockEventDispatcher;
#[async_trait]
impl crate::domain::repository::event_dispatcher::EventDispatcher for MockEventDispatcher {
    async fn dispatch(
        &self,
        _event: crate::domain::model::event::GameEvent,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn clear(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

// --- テストデータ作成ヘルパー ---

fn create_test_kuni(id: u32) -> Kuni {
    let daimyo_id = DaimyoId(id);
    Kuni::new(
        KuniId(id),
        format!("TestKuni-{}", id),
        daimyo_id,
        Resource::new(1000 * 100, 100 * 100, 1000 * 100, 1000 * 100),
        DevelopmentStats::new(100 * 100, 100 * 100, 60),
        IninFlag(false),
    )
}

// --- DomesticUseCase テスト ---

#[tokio::test]
async fn test_domestic_sell_rice() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new());
    let kuni = create_test_kuni(1);
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let state_repo = Arc::new(MockGameStateRepository::new());
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![kuni_id, crate::domain::model::value_objects::KuniId(999)],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            Arc::new(MockDaimyoRepository::new()),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = DomesticUseCase::new(
        repo.clone(),
        neighbor_repo.clone(),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        turn_progression,
    );
    usecase
        .sell_rice(None, kuni_id, DisplayAmount::new(1))
        .await
        .expect("売却成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    // 100000 - 100 = 99900
    assert_eq!(updated.resource.kome.value(), 99900);
    // 金が増えているはず (100000 + alpha)
    assert!(updated.resource.kin.value() > 100000);
}

#[tokio::test]
async fn test_domestic_buy_rice() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new());
    let kuni = create_test_kuni(1);
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let state_repo = Arc::new(MockGameStateRepository::new());
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![kuni_id, crate::domain::model::value_objects::KuniId(999)],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            Arc::new(MockDaimyoRepository::new()),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = DomesticUseCase::new(
        repo.clone(),
        neighbor_repo.clone(),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        turn_progression,
    );
    usecase
        .buy_rice(None, kuni_id, DisplayAmount::new(1))
        .await
        .expect("購入成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    // 100000 + (70 ~ 100)
    assert!(updated.resource.kome.value() >= 100070 && updated.resource.kome.value() <= 100100);
    // 金が減っているはず
    assert!(updated.resource.kin.value() < 100000);
}

#[tokio::test]
async fn test_domestic_recruit() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new());
    let kuni = create_test_kuni(1);
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let state_repo = Arc::new(MockGameStateRepository::new());
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![kuni_id, crate::domain::model::value_objects::KuniId(999)],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            Arc::new(MockDaimyoRepository::new()),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = DomesticUseCase::new(
        repo.clone(),
        neighbor_repo.clone(),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        turn_progression,
    );
    usecase
        .recruit(None, kuni_id, DisplayAmount::new(1))
        .await
        .expect("徴募成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    assert_eq!(updated.resource.hei.value(), 10100);
    assert_eq!(updated.resource.jinko.value(), 99900); // 100000 - 100
}

#[tokio::test]
async fn test_domestic_transport_success_when_adjacent() {
    let repo = Arc::new(MockKuniRepository::new());
    let mut mock_neighbor = MockNeighborRepository::new();
    let from_kuni = create_test_kuni(1);
    let to_kuni = create_test_kuni(2);
    let from_id = from_kuni.id;
    let to_id = to_kuni.id;

    repo.setup(from_kuni);
    repo.setup(to_kuni);
    mock_neighbor.add_neighbor(from_id, to_id);

    let neighbor_repo = Arc::new(mock_neighbor);
    let state_repo = Arc::new(MockGameStateRepository::new());
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![from_id, crate::domain::model::value_objects::KuniId(999)],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            Arc::new(MockDaimyoRepository::new()),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = DomesticUseCase::new(
        repo.clone(),
        neighbor_repo.clone(),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        turn_progression,
    );

    let res = usecase
        .transport(
            None,
            from_id,
            to_id,
            DisplayAmount::new(1),
            DisplayAmount::new(0),
            DisplayAmount::new(0),
        )
        .await;
    assert!(res.is_ok());

    let updated_from = repo.find_by_id(&from_id).await.unwrap().unwrap();
    let updated_to = repo.find_by_id(&to_id).await.unwrap().unwrap();
    assert_eq!(updated_from.resource.kin.value(), 99900);
    assert_eq!(updated_to.resource.kin.value(), 100100);
}

#[tokio::test]
async fn test_domestic_transport_fails_when_not_adjacent() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new()); // No adjacency
    let from_kuni = create_test_kuni(1);
    let to_kuni = create_test_kuni(2);
    let from_id = from_kuni.id;
    let to_id = to_kuni.id;

    repo.setup(from_kuni);
    repo.setup(to_kuni);

    let state_repo = Arc::new(MockGameStateRepository::new());
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![from_id, crate::domain::model::value_objects::KuniId(999)],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            Arc::new(MockDaimyoRepository::new()),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = DomesticUseCase::new(
        repo.clone(),
        neighbor_repo.clone(),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        turn_progression,
    );
    let res = usecase
        .transport(
            None,
            from_id,
            to_id,
            DisplayAmount::new(1),
            DisplayAmount::new(0),
            DisplayAmount::new(0),
        )
        .await;
    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(err_str.contains("隣接していません"));
}

// --- BattleUseCase テスト ---

#[tokio::test]
async fn test_battle_execution_success_when_adjacent() {
    let repo = Arc::new(MockKuniRepository::new());
    let mut mock_neighbor = MockNeighborRepository::new();
    let attacker = create_test_kuni(1);
    let defender = Kuni::new(
        KuniId(2),
        "Defender".to_string(),
        DaimyoId(2), // Different daimyo
        Resource::new(1000 * 100, 1000 * 100, 1000 * 100, 1000 * 100),
        DevelopmentStats::new(100 * 100, 100 * 100, 60),
        IninFlag(false),
    );
    let attacker_id = attacker.id;
    let defender_id = defender.id;

    repo.setup(attacker);
    repo.setup(defender);
    mock_neighbor.add_neighbor(attacker_id, defender_id);
    let neighbor_repo = Arc::new(mock_neighbor);
    let battle_repo = Arc::new(MockBattleRepository::new());
    let daimyo_repo = Arc::new(MockDaimyoRepository::new());

    // 大名の登録（性格パラメータが必要になったため）
    let personality = crate::domain::model::daimyo_personality::DaimyoPersonality::default();
    daimyo_repo
        .save(&Daimyo::new(
            DaimyoId(1),
            "AttackerDaimyo",
            personality.clone(),
        ))
        .await
        .unwrap();
    daimyo_repo
        .save(&Daimyo::new(DaimyoId(2), "DefenderDaimyo", personality))
        .await
        .unwrap();

    let state_repo = Arc::new(MockGameStateRepository::new());
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![
                    attacker_id,
                    crate::domain::model::value_objects::KuniId(999),
                ],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            daimyo_repo.clone(),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = BattleUseCase::new(
        repo.clone(),
        neighbor_repo.clone(),
        battle_repo.clone(),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        daimyo_repo.clone(),
        turn_progression,
    );
    let _initial_status = usecase
        .start_war(
            None,
            attacker_id,
            defender_id,
            DisplayAmount::new(5),
            DisplayAmount::new(10),
        )
        .await
        .expect("合戦開始成功");

    // start_war で手番が進むため、テスト用に手番を戻す
    let mut state = state_repo.get().await.unwrap().unwrap();
    let current_phase = state.phase();
    state = GameState::with_all_fields(
        state.current_turn(),
        state.action_order().to_vec(),
        crate::domain::model::value_objects::ActionOrderIndex::new(0),
        false, // action_performed
        current_phase,
        state.winner(),
    );
    state_repo.save(&state).await.unwrap();

    let result = usecase
        .execute_battle_turn(None, attacker_id, Tactic::Normal)
        .await
        .expect("合戦成功");

    // 状態が保存されているか確認
    let updated_attacker = repo.find_by_id(&attacker_id).await.unwrap().unwrap();

    // 出陣した分、本国の兵力が減っていることを確認 (1000 - 5 = 995)
    assert_eq!(updated_attacker.resource.hei.to_display().value(), 95);
    // 戦場の兵力は 500 以下（ダメージを受けている可能性があるため）
    assert!(result.attacker.hei.value() <= 500); // 500 = 5 * INTERNAL_SCALE
                                                 // 防御側の状態が変化していることを確認（兵力減少、または鼓舞による士気向上）
    assert!(result.defender.hei.value() < 100000 || result.defender.morale.value() > 60);
}

#[tokio::test]
async fn test_battle_execution_fails_when_not_adjacent() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new()); // No adjacency
    let attacker = create_test_kuni(1);
    let defender = create_test_kuni(1);
    let attacker_id = attacker.id;
    let defender_id = defender.id;

    repo.setup(attacker);
    repo.setup(defender);

    let battle_repo = Arc::new(MockBattleRepository::new());
    let state_repo = Arc::new(MockGameStateRepository::new());
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![
                    attacker_id,
                    crate::domain::model::value_objects::KuniId(999),
                ],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            Arc::new(MockDaimyoRepository::new()),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = BattleUseCase::new(
        repo.clone(),
        neighbor_repo.clone(),
        battle_repo.clone(),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        Arc::new(MockDaimyoRepository::new()),
        turn_progression,
    );
    let result = usecase
        .start_war(
            None,
            attacker_id,
            defender_id,
            DisplayAmount::new(5),
            DisplayAmount::new(10),
        )
        .await;

    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("隣接していません"));
}

#[tokio::test]
async fn test_turn_validation_fails_on_wrong_turn() {
    let repo = Arc::new(MockKuniRepository::new());
    let kuni1 = create_test_kuni(1);
    let kuni2 = create_test_kuni(2);
    repo.setup(kuni1.clone());
    repo.setup(kuni2.clone());

    let state_repo = Arc::new(MockGameStateRepository::new());
    // 手番を国2に設定
    state_repo
        .save(
            &GameState::new(
                crate::domain::model::value_objects::TurnNumber::new(1),
                vec![kuni2.id, kuni1.id],
                crate::domain::model::value_objects::ActionOrderIndex::new(0),
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let turn_progression = Arc::new(
        crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase::new(
            repo.clone(),
            Arc::new(MockDaimyoRepository::new()),
            state_repo.clone(),
            Arc::new(MockEventDispatcher),
            Arc::new(MockActionLogRepository),
            Arc::new(MockBattleRepository::new()),
            Arc::new(MockNeighborRepository::new()),
        ),
    );

    let usecase = DomesticUseCase::new(
        repo.clone(),
        Arc::new(MockNeighborRepository::new()),
        Arc::new(MockActionLogRepository),
        state_repo.clone(),
        turn_progression,
    );

    // 国1が行動しようとするとエラーになるはず
    let res = usecase
        .sell_rice(None, kuni1.id, DisplayAmount::new(1))
        .await;
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("現在の手番ではありません"));
}
