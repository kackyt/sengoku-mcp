use crate::application::usecase::battle_usecase::BattleUseCase;
use crate::application::usecase::domestic_usecase::DomesticUseCase;
use crate::application::usecase::turn_usecase::TurnUseCase;
use crate::domain::error::DomainError;
use crate::domain::model::battle::{Tactic, WarStatus};
use crate::domain::model::kuni::Kuni;
use crate::domain::model::resource::{DevelopmentStats, Resource};
use crate::domain::model::value_objects::{DaimyoId, DisplayAmount, IninFlag, KuniId};
use crate::domain::repository::battle_repository::BattleRepository;
use crate::domain::repository::kuni_repository::KuniRepository;
use crate::domain::repository::neighbor_repository::NeighborRepository;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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
    async fn find_by_attacker(&self, attacker_id: &KuniId) -> anyhow::Result<Option<WarStatus>> {
        Ok(self.wars.lock().unwrap().get(attacker_id).cloned())
    }

    async fn save(&self, status: &WarStatus) -> anyhow::Result<()> {
        self.wars
            .lock()
            .unwrap()
            .insert(status.attacker.kuni_id, status.clone());
        Ok(())
    }

    async fn delete_by_attacker(&self, attacker_id: &KuniId) -> anyhow::Result<()> {
        self.wars.lock().unwrap().remove(attacker_id);
        Ok(())
    }
}

// --- テストデータ作成ヘルパー ---

fn create_test_kuni() -> Kuni {
    let daimyo_id = DaimyoId(Uuid::new_v4());
    Kuni::new(
        KuniId(Uuid::new_v4()),
        "TestKuni".to_string(),
        daimyo_id,
        Resource::new(1000 * 10, 1000 * 10, 1000 * 10, 1000 * 10),
        DevelopmentStats::new(100 * 100, 100 * 100, 60), // 石高と町は Amount なので 100 * INTERNAL_SCALE
        IninFlag(false),
    )
}

// --- DomesticUseCase テスト ---

#[tokio::test]
async fn test_domestic_sell_rice() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new());
    let kuni = create_test_kuni();
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let usecase = DomesticUseCase::new(repo.clone(), neighbor_repo.clone());
    usecase
        .sell_rice(kuni_id, DisplayAmount::new(1))
        .await
        .expect("売却成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    // 10000 - 100 = 9900
    assert_eq!(updated.resource.kome.value(), 9900);
    // 金が増えているはず (10000 + alpha)
    assert!(updated.resource.kin.value() > 10000);
}

#[tokio::test]
async fn test_domestic_buy_rice() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new());
    let kuni = create_test_kuni();
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let usecase = DomesticUseCase::new(repo.clone(), neighbor_repo.clone());
    usecase
        .buy_rice(kuni_id, DisplayAmount::new(1))
        .await
        .expect("購入成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    // 10000 + 100 = 10100
    assert_eq!(updated.resource.kome.value(), 10100);
    // 金が減っているはず
    assert!(updated.resource.kin.value() < 10000);
}

#[tokio::test]
async fn test_domestic_recruit() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new());
    let kuni = create_test_kuni();
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let usecase = DomesticUseCase::new(repo.clone(), neighbor_repo.clone());
    usecase
        .recruit(kuni_id, DisplayAmount::new(1))
        .await
        .expect("徴募成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    assert_eq!(updated.resource.hei.value(), 10100);
    assert_eq!(updated.resource.jinko.value(), 9900); // 10000 - 100
}

#[tokio::test]
async fn test_domestic_transport_success_when_adjacent() {
    let repo = Arc::new(MockKuniRepository::new());
    let mut mock_neighbor = MockNeighborRepository::new();
    let from_kuni = create_test_kuni();
    let to_kuni = create_test_kuni();
    let from_id = from_kuni.id;
    let to_id = to_kuni.id;

    repo.setup(from_kuni);
    repo.setup(to_kuni);
    mock_neighbor.add_neighbor(from_id, to_id);

    let neighbor_repo = Arc::new(mock_neighbor);
    let usecase = DomesticUseCase::new(repo.clone(), neighbor_repo.clone());

    let res = usecase
        .transport(
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
    assert_eq!(updated_from.resource.kin.value(), 9900);
    assert_eq!(updated_to.resource.kin.value(), 10100);
}

#[tokio::test]
async fn test_domestic_transport_fails_when_not_adjacent() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new()); // No adjacency
    let from_kuni = create_test_kuni();
    let to_kuni = create_test_kuni();
    let from_id = from_kuni.id;
    let to_id = to_kuni.id;

    repo.setup(from_kuni);
    repo.setup(to_kuni);

    let usecase = DomesticUseCase::new(repo.clone(), neighbor_repo.clone());
    let res = usecase
        .transport(
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
    let attacker = create_test_kuni();
    let defender = create_test_kuni();
    let attacker_id = attacker.id;
    let defender_id = defender.id;

    repo.setup(attacker);
    repo.setup(defender);
    mock_neighbor.add_neighbor(attacker_id, defender_id);
    let neighbor_repo = Arc::new(mock_neighbor);
    let battle_repo = Arc::new(MockBattleRepository::new());

    let usecase = BattleUseCase::new(repo.clone(), neighbor_repo.clone(), battle_repo.clone());
    let initial_status = usecase
        .start_war(
            attacker_id,
            defender_id,
            DisplayAmount::new(5),
            DisplayAmount::new(10),
        )
        .await
        .expect("合戦開始成功");

    let result = usecase
        .execute_battle_turn(initial_status, Tactic::Normal)
        .await
        .expect("合戦成功");

    // 状態が保存されているか確認
    let updated_attacker = repo.find_by_id(&attacker_id).await.unwrap().unwrap();

    // 出陣した分、本国の兵力が減っていることを確認 (100 - 5 = 95)
    assert_eq!(updated_attacker.resource.hei.to_display().value(), 95);
    // 戦場の兵力は 500 以下（ダメージを受けている可能性があるため）
    assert!(result.attacker.hei.value() <= 500); // 500 = 5 * INTERNAL_SCALE
                                                 // 防御側の状態が変化していることを確認（兵力減少、または鼓舞による士気向上）
    assert!(result.defender.hei.value() < 10000 || result.defender.morale.value() > 60);
}

#[tokio::test]
async fn test_battle_execution_fails_when_not_adjacent() {
    let repo = Arc::new(MockKuniRepository::new());
    let neighbor_repo = Arc::new(MockNeighborRepository::new()); // No adjacency
    let attacker = create_test_kuni();
    let defender = create_test_kuni();
    let attacker_id = attacker.id;
    let defender_id = defender.id;

    repo.setup(attacker);
    repo.setup(defender);

    let battle_repo = Arc::new(MockBattleRepository::new());
    let usecase = BattleUseCase::new(repo.clone(), neighbor_repo.clone(), battle_repo.clone());
    let result = usecase
        .start_war(
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

// --- TurnUseCase テスト ---

#[tokio::test]
async fn test_turn_progress() {
    let repo = Arc::new(MockKuniRepository::new());
    let kuni = create_test_kuni();
    let _kuni_id = kuni.id;
    repo.setup(kuni);

    let _usecase = TurnUseCase::new(repo.clone());
    // TurnUseCase::progress_turn は private なので、
    // 将来的にパブリックな口ができたらテストする。
    // 現状は BattleUseCase 等のテストで十分。
}
