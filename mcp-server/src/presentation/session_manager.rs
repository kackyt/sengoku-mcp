use chrono::{DateTime, Duration, Utc};
use engine::application::usecase::battle_usecase::BattleUseCase;
use engine::application::usecase::daimyo_query_usecase::DaimyoQueryUseCase;
use engine::application::usecase::domestic_usecase::DomesticUseCase;
use engine::application::usecase::game_lifecycle_usecase::GameLifecycleUseCase;
use engine::application::usecase::info_usecase::InfoUseCase;
use engine::application::usecase::kuni_query_usecase::KuniQueryUseCase;
use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use engine::domain::model::action_log::ActionLogCategory;
use engine::domain::model::value_objects::DaimyoId;
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
    InMemoryNeighborRepository, SessionData, SessionPersistenceManager,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// 1つのセッションに紐づくゲーム実行コンテキスト
pub struct GameContext {
    pub kuni_repo: Arc<InMemoryKuniRepository>,
    pub daimyo_repo: Arc<InMemoryDaimyoRepository>,
    pub game_state_repo: Arc<InMemoryGameStateRepository>,
    #[allow(dead_code)]
    pub event_dispatcher: Arc<InMemoryEventDispatcher>,
    #[allow(dead_code)]
    pub neighbor_repo: Arc<InMemoryNeighborRepository>,
    pub battle_repo: Arc<InMemoryBattleRepository>,
    pub action_log_repo: Arc<InMemoryActionLogRepository>,

    pub turn_progression_usecase: Arc<TurnProgressionUseCase>,
    pub game_lifecycle_usecase: Arc<GameLifecycleUseCase>,
    pub domestic_usecase: Arc<DomesticUseCase>,
    pub battle_usecase: Arc<BattleUseCase>,
    pub kuni_query_usecase: Arc<KuniQueryUseCase>,
    pub info_usecase: Arc<InfoUseCase>,
    pub daimyo_query_usecase: Arc<DaimyoQueryUseCase>,

    pub selected_daimyo_id: Arc<Mutex<Option<DaimyoId>>>,
    pub last_accessed_at: Arc<Mutex<DateTime<Utc>>>,
}

impl GameContext {
    /// 新規ゲーム状態として初期化します
    pub async fn new_initial(master_data: Arc<MasterDataLoader>) -> Result<Self, anyhow::Error> {
        let kuni_repo = Arc::new(InMemoryKuniRepository::new());
        let daimyo_repo = Arc::new(InMemoryDaimyoRepository::new());
        let game_state_repo = Arc::new(InMemoryGameStateRepository::new());
        let event_dispatcher = Arc::new(InMemoryEventDispatcher::new());
        let neighbor_repo = Arc::new(InMemoryNeighborRepository::new());
        let battle_repo = Arc::new(InMemoryBattleRepository::new());
        let action_log_repo = Arc::new(InMemoryActionLogRepository::new());

        let turn_progression_usecase = Arc::new(TurnProgressionUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            event_dispatcher.clone(),
            action_log_repo.clone(),
            battle_repo.clone(),
            neighbor_repo.clone(),
        ));

        let domestic_usecase = Arc::new(DomesticUseCase::new(
            kuni_repo.clone(),
            neighbor_repo.clone(),
            action_log_repo.clone(),
            game_state_repo.clone(),
            turn_progression_usecase.clone(),
        ));

        let battle_usecase = Arc::new(BattleUseCase::new(
            kuni_repo.clone(),
            neighbor_repo.clone(),
            battle_repo.clone(),
            action_log_repo.clone(),
            game_state_repo.clone(),
            daimyo_repo.clone(),
            turn_progression_usecase.clone(),
        ));

        let kuni_query_usecase = Arc::new(KuniQueryUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            neighbor_repo.clone(),
            action_log_repo.clone(),
            battle_repo.clone(),
        ));

        let info_usecase = Arc::new(InfoUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            turn_progression_usecase.clone(),
        ));

        let daimyo_query_usecase = Arc::new(DaimyoQueryUseCase::new(daimyo_repo.clone()));

        let game_lifecycle_usecase = Arc::new(GameLifecycleUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            action_log_repo.clone(),
            battle_repo.clone(),
            neighbor_repo.clone(),
            event_dispatcher.clone(),
            master_data,
        ));

        // マスターデータから初期化
        game_lifecycle_usecase.reset_game().await?;

        Ok(Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            event_dispatcher,
            neighbor_repo,
            battle_repo,
            action_log_repo,
            turn_progression_usecase,
            game_lifecycle_usecase,
            domestic_usecase,
            battle_usecase,
            kuni_query_usecase,
            info_usecase,
            daimyo_query_usecase,
            selected_daimyo_id: Arc::new(Mutex::new(None)),
            last_accessed_at: Arc::new(Mutex::new(Utc::now())),
        })
    }

    /// 保存データから復元します
    pub async fn from_session_data(
        data: SessionData,
        master_data: Arc<MasterDataLoader>,
    ) -> Result<Self, anyhow::Error> {
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
        // 隣接データはマスターデータからロード
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

        let turn_progression_usecase = Arc::new(TurnProgressionUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            event_dispatcher.clone(),
            action_log_repo.clone(),
            battle_repo.clone(),
            neighbor_repo.clone(),
        ));

        let domestic_usecase = Arc::new(DomesticUseCase::new(
            kuni_repo.clone(),
            neighbor_repo.clone(),
            action_log_repo.clone(),
            game_state_repo.clone(),
            turn_progression_usecase.clone(),
        ));

        let battle_usecase = Arc::new(BattleUseCase::new(
            kuni_repo.clone(),
            neighbor_repo.clone(),
            battle_repo.clone(),
            action_log_repo.clone(),
            game_state_repo.clone(),
            daimyo_repo.clone(),
            turn_progression_usecase.clone(),
        ));

        let kuni_query_usecase = Arc::new(KuniQueryUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            neighbor_repo.clone(),
            action_log_repo.clone(),
            battle_repo.clone(),
        ));

        let info_usecase = Arc::new(InfoUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            turn_progression_usecase.clone(),
        ));

        let daimyo_query_usecase = Arc::new(DaimyoQueryUseCase::new(daimyo_repo.clone()));

        let game_lifecycle_usecase = Arc::new(GameLifecycleUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            action_log_repo.clone(),
            battle_repo.clone(),
            neighbor_repo.clone(),
            event_dispatcher.clone(),
            master_data,
        ));

        Ok(Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            event_dispatcher,
            neighbor_repo,
            battle_repo,
            action_log_repo,
            turn_progression_usecase,
            game_lifecycle_usecase,
            domestic_usecase,
            battle_usecase,
            kuni_query_usecase,
            info_usecase,
            daimyo_query_usecase,
            selected_daimyo_id: Arc::new(Mutex::new(data.selected_daimyo_id)),
            last_accessed_at: Arc::new(Mutex::new(data.last_accessed_at)),
        })
    }

    /// 現在の状態を `SessionData` としてエクスポートします
    pub async fn export_session_data(
        &self,
        session_id: &str,
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
            session_id,
            selected_daimyo_id,
            game_state,
            kunis,
            daimyos,
            battles,
            action_logs,
        );
        data.last_accessed_at = *self.last_accessed_at.lock().await;
        Ok(data)
    }

    /// アクセス日時を更新します
    pub async fn touch(&self) {
        let mut lock = self.last_accessed_at.lock().await;
        *lock = Utc::now();
    }
}

/// 複数セッションを管理するセッションマネージャー
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<GameContext>>>>,
    persistence: Arc<SessionPersistenceManager>,
    master_data: Arc<MasterDataLoader>,
}

impl SessionManager {
    /// 新規セッションマネージャーを初期化します
    pub fn new(
        persistence: Arc<SessionPersistenceManager>,
        master_data: Arc<MasterDataLoader>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            persistence,
            master_data,
        }
    }

    /// セッションIDに対応する GameContext を取得または作成します
    /// 1. メモリ内に存在すればそれを返す
    /// 2. ファイルに存在すれば復元してキャッシュして返す
    /// 3. なければ新規初期化して保存・キャッシュして返す
    pub async fn get_or_create(&self, session_id: &str) -> Result<Arc<GameContext>, anyhow::Error> {
        // 1. メモリ内キャッシュ確認
        {
            let guard = self.sessions.read().await;
            if let Some(ctx) = guard.get(session_id) {
                ctx.touch().await;
                return Ok(ctx.clone());
            }
        }

        // 2. ファイルからの復元確認
        let mut write_guard = self.sessions.write().await;
        // Double check
        if let Some(ctx) = write_guard.get(session_id) {
            ctx.touch().await;
            return Ok(ctx.clone());
        }

        let ctx = if let Some(data) = self.persistence.load(session_id)? {
            let ctx =
                Arc::new(GameContext::from_session_data(data, self.master_data.clone()).await?);
            ctx.touch().await;
            ctx
        } else {
            // 3. 新規作成
            let ctx = Arc::new(GameContext::new_initial(self.master_data.clone()).await?);
            let session_data = ctx.export_session_data(session_id).await?;
            self.persistence.save(&session_data)?;
            ctx
        };

        write_guard.insert(session_id.to_string(), ctx.clone());
        Ok(ctx)
    }

    /// セッションの現在状態をファイルに保存します
    pub async fn save_session(&self, session_id: &str) -> Result<(), anyhow::Error> {
        let ctx = {
            let guard = self.sessions.read().await;
            guard.get(session_id).cloned()
        };

        if let Some(ctx) = ctx {
            let data = ctx.export_session_data(session_id).await?;
            self.persistence.save(&data)?;
        }
        Ok(())
    }

    /// 期限切れのセッションをメモリおよびファイルから削除します
    pub async fn cleanup_expired(&self, ttl: Duration) -> Result<usize, anyhow::Error> {
        let now = Utc::now();
        let mut expired_keys = Vec::new();

        // メモリ内チェック
        {
            let guard = self.sessions.read().await;
            for (id, ctx) in guard.iter() {
                let last_access = *ctx.last_accessed_at.lock().await;
                if (now - last_access) > ttl {
                    expired_keys.push(id.clone());
                }
            }
        }

        if !expired_keys.is_empty() {
            let mut write_guard = self.sessions.write().await;
            for key in &expired_keys {
                write_guard.remove(key);
            }
        }

        // ファイルクリーンアップ
        let file_deleted = self.persistence.cleanup_expired(ttl)?;
        Ok(file_deleted.max(expired_keys.len()))
    }

    /// バックグラウンドで定期的にクリーンアップを実行するタスクを開始します
    pub fn start_cleanup_task(self: Arc<Self>, interval: std::time::Duration, ttl: Duration) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                if let Err(e) = self.cleanup_expired(ttl).await {
                    eprintln!("[SessionManager] Cleanup error: {:?}", e);
                }
            }
        });
    }
}
