use chrono::{DateTime, Utc};
use engine::application::usecase::battle_usecase::BattleUseCase;
use engine::application::usecase::daimyo_query_usecase::DaimyoQueryUseCase;
use engine::application::usecase::domestic_usecase::DomesticUseCase;
use engine::application::usecase::game_lifecycle_usecase::GameLifecycleUseCase;
use engine::application::usecase::info_usecase::InfoUseCase;
use engine::application::usecase::kuni_query_usecase::KuniQueryUseCase;
use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use engine::domain::model::action_log::ActionLogCategory;
use engine::domain::model::session::SessionData;
use engine::domain::model::value_objects::{DaimyoId, SessionId, ViewToken};
use engine::domain::repository::action_log_repository::ActionLogRepository;
use engine::domain::repository::battle_repository::BattleRepository;
use engine::domain::repository::daimyo_repository::DaimyoRepository;
use engine::domain::repository::game_state_repository::GameStateRepository;
use engine::domain::repository::kuni_repository::KuniRepository;
use engine::domain::repository::master_data_repository::MasterDataRepository;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{
    InMemoryActionLogRepository, InMemoryBattleRepository, InMemoryDaimyoRepository,
    InMemoryEventDispatcher, InMemoryGameStateRepository, InMemoryKuniRepository,
    InMemoryNeighborRepository,
};
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;

/// リポジトリのバンドル構造体
struct Repositories {
    kuni_repo: Arc<InMemoryKuniRepository>,
    daimyo_repo: Arc<InMemoryDaimyoRepository>,
    game_state_repo: Arc<InMemoryGameStateRepository>,
    event_dispatcher: Arc<InMemoryEventDispatcher>,
    neighbor_repo: Arc<InMemoryNeighborRepository>,
    battle_repo: Arc<InMemoryBattleRepository>,
    action_log_repo: Arc<InMemoryActionLogRepository>,
}

/// ユースケースのバンドル構造体
pub struct UseCases {
    pub turn_progression: Arc<TurnProgressionUseCase>,
    pub game_lifecycle: Arc<GameLifecycleUseCase>,
    pub domestic: Arc<DomesticUseCase>,
    pub battle: Arc<BattleUseCase>,
    pub kuni_query: Arc<KuniQueryUseCase>,
    pub info: Arc<InfoUseCase>,
    pub daimyo_query: Arc<DaimyoQueryUseCase>,
}

/// 1つのセッションに紐づくゲーム実行コンテキスト
pub struct GameContext {
    // リポジトリ群は内部カプセル化（プレゼンテーション層へ直接公開しない）
    kuni_repo: Arc<InMemoryKuniRepository>,
    daimyo_repo: Arc<InMemoryDaimyoRepository>,
    game_state_repo: Arc<InMemoryGameStateRepository>,
    battle_repo: Arc<InMemoryBattleRepository>,
    action_log_repo: Arc<InMemoryActionLogRepository>,

    // プレゼンテーション層へ公開するユースケース群
    pub turn_progression_usecase: Arc<TurnProgressionUseCase>,
    pub game_lifecycle_usecase: Arc<GameLifecycleUseCase>,
    pub domestic_usecase: Arc<DomesticUseCase>,
    pub battle_usecase: Arc<BattleUseCase>,
    pub kuni_query_usecase: Arc<KuniQueryUseCase>,
    pub info_usecase: Arc<InfoUseCase>,
    pub daimyo_query_usecase: Arc<DaimyoQueryUseCase>,

    pub selected_daimyo_id: Arc<Mutex<Option<DaimyoId>>>,
    pub last_accessed_at: Arc<Mutex<DateTime<Utc>>>,
    /// 外部クライアント向けに発行済みの閲覧トークン
    pub view_token: Arc<Mutex<Option<ViewToken>>>,
}

impl fmt::Debug for GameContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameContext")
            .field("selected_daimyo_id", &"<Mutex<Option<DaimyoId>>>")
            .field("last_accessed_at", &"<Mutex<DateTime<Utc>>>")
            .field("view_token", &"<Mutex<Option<ViewToken>>>")
            .finish()
    }
}

impl GameContext {
    /// アクセス日時を現在時刻に更新します
    pub async fn touch(&self) {
        let mut lock = self.last_accessed_at.lock().await;
        *lock = Utc::now();
    }

    /// 現在の状態を `SessionData` としてエクスポートします
    pub async fn export_session_data(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionData, anyhow::Error> {
        let kunis = self.kuni_repo.find_all().await?;
        let daimyos = self.daimyo_repo.find_all().await?;
        let game_state = self.game_state_repo.get().await?;
        let battles = self.battle_repo.find_all().await?;
        let mut action_logs = self.action_log_repo.find_all(ActionLogCategory::Domestic)?;
        let war_logs = self.action_log_repo.find_all(ActionLogCategory::War)?;
        action_logs.extend(war_logs);
        let selected_daimyo_id = *self.selected_daimyo_id.lock().await;

        let mut data = SessionData::new(
            session_id.clone(),
            selected_daimyo_id,
            game_state,
            kunis,
            daimyos,
            battles,
            action_logs,
        );
        data.last_accessed_at = *self.last_accessed_at.lock().await;
        data.view_token = self.view_token.lock().await.clone();
        Ok(data)
    }
}

/// リポジトリ群とユースケース群を組み立てるファクトリ
pub struct GameContextFactory;

impl GameContextFactory {
    /// リポジトリ群からユースケース群を組み立てる共通ビルダー関数 (DRY原則・引数と戻り値の構造体化)
    fn build_usecases(repos: &Repositories, master_data: Arc<MasterDataLoader>) -> UseCases {
        let turn_progression = Arc::new(TurnProgressionUseCase::new(
            repos.kuni_repo.clone(),
            repos.daimyo_repo.clone(),
            repos.game_state_repo.clone(),
            repos.event_dispatcher.clone(),
            repos.action_log_repo.clone(),
            repos.battle_repo.clone(),
            repos.neighbor_repo.clone(),
        ));

        let domestic = Arc::new(DomesticUseCase::new(
            repos.kuni_repo.clone(),
            repos.neighbor_repo.clone(),
            repos.action_log_repo.clone(),
            repos.game_state_repo.clone(),
            turn_progression.clone(),
        ));

        let battle = Arc::new(BattleUseCase::new(
            repos.kuni_repo.clone(),
            repos.neighbor_repo.clone(),
            repos.battle_repo.clone(),
            repos.action_log_repo.clone(),
            repos.game_state_repo.clone(),
            repos.daimyo_repo.clone(),
            turn_progression.clone(),
        ));

        let kuni_query = Arc::new(KuniQueryUseCase::new(
            repos.kuni_repo.clone(),
            repos.daimyo_repo.clone(),
            repos.game_state_repo.clone(),
            repos.neighbor_repo.clone(),
            repos.action_log_repo.clone(),
            repos.battle_repo.clone(),
        ));

        let info = Arc::new(InfoUseCase::new(
            repos.kuni_repo.clone(),
            repos.daimyo_repo.clone(),
            repos.game_state_repo.clone(),
            turn_progression.clone(),
        ));

        let daimyo_query = Arc::new(DaimyoQueryUseCase::new(repos.daimyo_repo.clone()));

        let game_lifecycle = Arc::new(GameLifecycleUseCase::new(
            repos.kuni_repo.clone(),
            repos.daimyo_repo.clone(),
            repos.game_state_repo.clone(),
            repos.action_log_repo.clone(),
            repos.battle_repo.clone(),
            repos.neighbor_repo.clone(),
            repos.event_dispatcher.clone(),
            master_data,
        ));

        UseCases {
            turn_progression,
            game_lifecycle,
            domestic,
            battle,
            kuni_query,
            info,
            daimyo_query,
        }
    }

    /// 新規ゲーム状態として初期化します
    pub async fn create_initial(
        master_data: Arc<MasterDataLoader>,
    ) -> Result<GameContext, anyhow::Error> {
        let repos = Repositories {
            kuni_repo: Arc::new(InMemoryKuniRepository::new()),
            daimyo_repo: Arc::new(InMemoryDaimyoRepository::new()),
            game_state_repo: Arc::new(InMemoryGameStateRepository::new()),
            event_dispatcher: Arc::new(InMemoryEventDispatcher::new()),
            neighbor_repo: Arc::new(InMemoryNeighborRepository::new()),
            battle_repo: Arc::new(InMemoryBattleRepository::new()),
            action_log_repo: Arc::new(InMemoryActionLogRepository::new()),
        };

        let usecases = Self::build_usecases(&repos, master_data);

        // マスターデータから初期化
        usecases.game_lifecycle.reset_game().await?;

        Ok(GameContext {
            kuni_repo: repos.kuni_repo,
            daimyo_repo: repos.daimyo_repo,
            game_state_repo: repos.game_state_repo,
            battle_repo: repos.battle_repo,
            action_log_repo: repos.action_log_repo,
            turn_progression_usecase: usecases.turn_progression,
            game_lifecycle_usecase: usecases.game_lifecycle,
            domestic_usecase: usecases.domestic,
            battle_usecase: usecases.battle,
            kuni_query_usecase: usecases.kuni_query,
            info_usecase: usecases.info,
            daimyo_query_usecase: usecases.daimyo_query,
            selected_daimyo_id: Arc::new(Mutex::new(None)),
            last_accessed_at: Arc::new(Mutex::new(Utc::now())),
            view_token: Arc::new(Mutex::new(None)),
        })
    }

    /// 保存データから復元します
    pub async fn create_from_data(
        data: SessionData,
        master_data: Arc<MasterDataLoader>,
    ) -> Result<GameContext, anyhow::Error> {
        let kuni_repo = Arc::new(InMemoryKuniRepository::new());
        kuni_repo.init_with_data(data.kunis).await;

        let daimyo_repo = Arc::new(InMemoryDaimyoRepository::new());
        daimyo_repo.init_with_data(data.daimyos).await;

        let game_state_repo = Arc::new(InMemoryGameStateRepository::new());
        if let Some(ref state) = data.game_state {
            game_state_repo.save(state).await?;
        }

        let event_dispatcher = Arc::new(InMemoryEventDispatcher::new());

        let neighbor_repo = Arc::new(InMemoryNeighborRepository::new());
        let bundle = master_data.load().map_err(|e| anyhow::anyhow!("{}", e))?;
        neighbor_repo.init_with_data(bundle.adjacency_map);

        let battle_repo = Arc::new(InMemoryBattleRepository::new());
        for war_status in data.battles {
            battle_repo.save(&war_status).await?;
        }

        let action_log_repo = Arc::new(InMemoryActionLogRepository::new());
        for log in data.action_logs {
            action_log_repo.save(log)?;
        }

        let repos = Repositories {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            event_dispatcher,
            neighbor_repo,
            battle_repo,
            action_log_repo,
        };

        let usecases = Self::build_usecases(&repos, master_data);

        Ok(GameContext {
            kuni_repo: repos.kuni_repo,
            daimyo_repo: repos.daimyo_repo,
            game_state_repo: repos.game_state_repo,
            battle_repo: repos.battle_repo,
            action_log_repo: repos.action_log_repo,
            turn_progression_usecase: usecases.turn_progression,
            game_lifecycle_usecase: usecases.game_lifecycle,
            domestic_usecase: usecases.domestic,
            battle_usecase: usecases.battle,
            kuni_query_usecase: usecases.kuni_query,
            info_usecase: usecases.info,
            daimyo_query_usecase: usecases.daimyo_query,
            selected_daimyo_id: Arc::new(Mutex::new(data.selected_daimyo_id)),
            last_accessed_at: Arc::new(Mutex::new(data.last_accessed_at)),
            view_token: Arc::new(Mutex::new(data.view_token)),
        })
    }
}
