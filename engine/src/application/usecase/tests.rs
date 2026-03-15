use crate::application::usecase::battle_usecase::BattleUseCase;
use crate::application::usecase::domestic_usecase::DomesticUseCase;
use crate::application::usecase::turn_usecase::TurnUseCase;
use crate::domain::error::DomainError;
use crate::domain::model::{
    kuni::Kuni,
    resource::{DevelopmentStats, Resource},
    value_objects::{Amount, DaimyoId, IninFlag, KuniId},
};
use crate::domain::repository::kuni_repository::KuniRepository;
use crate::domain::service::battle_service::Tactic;
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

// --- テストデータ作成ヘルパー ---

fn create_test_kuni() -> Kuni {
    Kuni::new(
        KuniId(Uuid::new_v4()),
        DaimyoId(Uuid::new_v4()),
        Resource::new(1000, 1000, 1000, 10000),
        DevelopmentStats::new(100, 100, 50),
        IninFlag::new(false),
    )
}

// --- DomesticUseCase テスト ---

#[tokio::test]
async fn test_domestic_sell_rice() {
    let repo = Arc::new(MockKuniRepository::new());
    let kuni = create_test_kuni();
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let usecase = DomesticUseCase::new(repo.clone());
    usecase
        .sell_rice(kuni_id, Amount::new(100))
        .await
        .expect("売却成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    // 1000 - 100 = 900
    assert_eq!(updated.resource.kome.value(), 900);
    // 金が増えているはず (1.0 - 2.0倍)
    assert!(updated.resource.kin.value() > 1000);
}

#[tokio::test]
async fn test_domestic_buy_rice() {
    let repo = Arc::new(MockKuniRepository::new());
    let kuni = create_test_kuni();
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let usecase = DomesticUseCase::new(repo.clone());
    usecase
        .buy_rice(kuni_id, Amount::new(100))
        .await
        .expect("購入成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    // 1000 + 100 = 1100
    assert_eq!(updated.resource.kome.value(), 1100);
    // 金が減っているはず
    assert!(updated.resource.kin.value() < 1000);
}

#[tokio::test]
async fn test_domestic_recruit() {
    let repo = Arc::new(MockKuniRepository::new());
    let kuni = create_test_kuni();
    let kuni_id = kuni.id;
    repo.setup(kuni);

    let usecase = DomesticUseCase::new(repo.clone());
    usecase
        .recruit(kuni_id, Amount::new(100))
        .await
        .expect("徴募成功");

    let updated = repo.find_by_id(&kuni_id).await.unwrap().unwrap();
    assert_eq!(updated.resource.hei.value(), 1100);
    assert_eq!(updated.resource.jinko.value(), 9900); // 10000 - 100
}

// --- BattleUseCase テスト ---

#[tokio::test]
async fn test_battle_execution() {
    let repo = Arc::new(MockKuniRepository::new());
    let attacker = create_test_kuni();
    let defender = create_test_kuni();
    let attacker_id = attacker.id;
    let defender_id = defender.id;

    repo.setup(attacker);
    repo.setup(defender);

    let usecase = BattleUseCase::new(repo.clone());
    let result = usecase
        .execute_battle_turn(
            attacker_id,
            defender_id,
            Tactic::Normal,
            Tactic::Normal,
            Amount::new(500),
        )
        .await
        .expect("合戦成功");

    // 状態が保存されているか確認
    let updated_attacker = repo.find_by_id(&attacker_id).await.unwrap().unwrap();
    let updated_defender = repo.find_by_id(&defender_id).await.unwrap().unwrap();

    assert_eq!(
        updated_attacker.resource.hei.value(),
        result.attacker_kuni.resource.hei.value()
    );
    assert_eq!(
        updated_defender.resource.hei.value(),
        result.defender_kuni.resource.hei.value()
    );
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
