use crate::domain::error::DomainError;
use crate::domain::repository::{
    action_log_repository::ActionLogRepository, battle_repository::BattleRepository,
    daimyo_repository::DaimyoRepository, event_dispatcher::EventDispatcher,
    game_state_repository::GameStateRepository, kuni_repository::KuniRepository,
    master_data_repository::MasterDataRepository, neighbor_repository::NeighborRepository,
};
use crate::domain::model::action_log::ActionLogCategory;
use std::sync::Arc;

/// ゲームのライフサイクル（初期化、リセットなど）を管理するユースケース
pub struct GameLifecycleUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    daimyo_repo: Arc<dyn DaimyoRepository>,
    game_state_repo: Arc<dyn GameStateRepository>,
    action_log_repo: Arc<dyn ActionLogRepository>,
    battle_repo: Arc<dyn BattleRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
    event_dispatcher: Arc<dyn EventDispatcher>,
    master_data_repo: Arc<dyn MasterDataRepository>,
}

impl GameLifecycleUseCase {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        daimyo_repo: Arc<dyn DaimyoRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
        action_log_repo: Arc<dyn ActionLogRepository>,
        battle_repo: Arc<dyn BattleRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
        event_dispatcher: Arc<dyn EventDispatcher>,
        master_data_repo: Arc<dyn MasterDataRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            action_log_repo,
            battle_repo,
            neighbor_repo,
            event_dispatcher,
            master_data_repo,
        }
    }

    /// ゲーム状態を完全にリセットし、初期データで初期化します
    pub async fn reset_game(&self) -> Result<(), DomainError> {
        // 1. 各リポジトリのクリア
        self.game_state_repo.clear().await?;
        self.event_dispatcher.clear().await?;
        self.battle_repo.clear().await?;
        self.action_log_repo.clear(ActionLogCategory::Domestic)?;
        self.action_log_repo.clear(ActionLogCategory::War)?;
        self.kuni_repo.clear().await?;
        self.daimyo_repo.clear().await?;

        // 2. マスターデータのロード
        let bundle = self.master_data_repo.load()?;

        // 3. リポジトリへの初期データの保存
        for kuni in bundle.kunis {
            self.kuni_repo.save(&kuni).await?;
        }
        for daimyo in bundle.daimyos {
            self.daimyo_repo.save(&daimyo).await?;
        }
        self.neighbor_repo.reset(bundle.adjacency_map)?;

        Ok(())
    }
}
