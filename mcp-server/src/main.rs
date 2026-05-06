mod presentation;

extern crate rmcp;

use crate::presentation::handlers::McpHandlers;
use engine::application::usecase::battle_usecase::BattleUseCase;
use engine::application::usecase::domestic_usecase::DomesticUseCase;
use engine::application::usecase::info_usecase::InfoUseCase;
use engine::application::usecase::kuni_query_usecase::KuniQueryUseCase;
use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{
    InMemoryActionLogRepository, InMemoryBattleRepository, InMemoryDaimyoRepository,
    InMemoryEventDispatcher, InMemoryGameStateRepository, InMemoryKuniRepository,
    InMemoryNeighborRepository,
};
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // リポジトリの構築
    let kuni_repo = Arc::new(InMemoryKuniRepository::new());
    let daimyo_repo = Arc::new(InMemoryDaimyoRepository::new());
    let game_state_repo = Arc::new(InMemoryGameStateRepository::new());
    let event_dispatcher = Arc::new(InMemoryEventDispatcher::new());
    let neighbor_repo = Arc::new(InMemoryNeighborRepository::new());
    let battle_repo = Arc::new(InMemoryBattleRepository::new());
    let action_log_repo = Arc::new(InMemoryActionLogRepository::new());

    // マスターデータのパス解決
    let base_dir = if let Ok(env_path) = std::env::var("SENGOKU_MASTER_DATA") {
        std::path::PathBuf::from(env_path)
    } else {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dev_path = manifest_dir.join("../static/master_data");

        if dev_path.exists() {
            dev_path
        } else {
            // カレントディレクトリからの相対パス（プロジェクトルートで実行される想定）
            std::path::PathBuf::from("static/master_data")
        }
    };

    if base_dir.exists() {
        let bundle = MasterDataLoader::load(&base_dir)?;
        kuni_repo.init_with_data(bundle.kunis).await;
        daimyo_repo.init_with_data(bundle.daimyos).await;
        neighbor_repo.init_with_data(bundle.adjacency_map);
    } else {
        // マスターデータが見つからなくても起動はさせる（警告のみ）
        eprintln!("Warning: Master data not found at {:?}", base_dir);
    }

    // ユースケースの構築
    let turn_progression_usecase = Arc::new(TurnProgressionUseCase::new(
        kuni_repo.clone(),
        daimyo_repo.clone(),
        game_state_repo.clone(),
        event_dispatcher.clone(),
        action_log_repo.clone(),
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
        turn_progression_usecase.clone(),
    ));

    let kuni_query_usecase = Arc::new(KuniQueryUseCase::new(
        kuni_repo.clone(),
        daimyo_repo.clone(),
        game_state_repo.clone(),
        neighbor_repo.clone(),
        action_log_repo.clone(),
    ));

    let info_usecase = Arc::new(InfoUseCase::new(
        kuni_repo.clone(),
        daimyo_repo.clone(),
        game_state_repo.clone(),
        turn_progression_usecase.clone(),
    ));

    let handlers = McpHandlers::new(
        turn_progression_usecase,
        domestic_usecase,
        battle_usecase,
        kuni_query_usecase,
        info_usecase,
        daimyo_repo.clone(),
    );

    // Build the transport (stdio)
    let transport = (stdin(), stdout());

    // Initialize and start the server
    let server = handlers.serve(transport).await?;

    // Wait for the server to finish
    server.waiting().await?;

    Ok(())
}
