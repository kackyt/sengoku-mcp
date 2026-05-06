mod presentation;

use crate::presentation::handlers::McpHandlers;
use engine::application::usecase::info_usecase::InfoUseCase;
use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use infrastructure::persistence::{
    InMemoryDaimyoRepository, InMemoryEventDispatcher, InMemoryGameStateRepository,
    InMemoryKuniRepository, InMemoryNeighborRepository,
    in_memory_action_log_repository::InMemoryActionLogRepository,
};
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let kuni_repo = Arc::new(InMemoryKuniRepository::new());
    let daimyo_repo = Arc::new(InMemoryDaimyoRepository::new());
    let game_state_repo = Arc::new(InMemoryGameStateRepository::new());
    let event_dispatcher = Arc::new(InMemoryEventDispatcher::new());
    let _neighbor_repo = Arc::new(InMemoryNeighborRepository::new());
    let action_log_repo = Arc::new(InMemoryActionLogRepository::new());

    let turn_progression_usecase = Arc::new(TurnProgressionUseCase::new(
        kuni_repo.clone(),
        game_state_repo.clone(),
        event_dispatcher.clone(),
        action_log_repo.clone(),
    ));

    let info_usecase = Arc::new(InfoUseCase::new(
        kuni_repo.clone(),
        daimyo_repo.clone(),
        game_state_repo.clone(),
        turn_progression_usecase.clone(),
    ));

    let handlers = McpHandlers::new(
        turn_progression_usecase.clone(),
        info_usecase.clone(),
    );

    // Build the transport (stdio)
    let transport = (stdin(), stdout());

    // Initialize and start the server
    let server = handlers.serve(transport).await?;

    // Wait for the server to finish
    server.waiting().await?;

    Ok(())
}
