use api_server::application::{GameCreationService, StatusQueryService};
use api_server::presentation::{build_router, AppState};
use game_session::GameLobby;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionStorageConfig;
use std::sync::Arc;

/// 待ち受けアドレスを解決します
///
/// `SENGOKU_API_ADDR` が指定されていればそれを使い、なければ `PORT`（Cloud Run 等が設定）を
/// `0.0.0.0` で待ち受けます。どちらもなければ `0.0.0.0:8080` を使います。
fn resolve_listen_addr() -> String {
    if let Ok(addr) = std::env::var("SENGOKU_API_ADDR") {
        return addr;
    }
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    format!("0.0.0.0:{}", port)
}

/// Composition Root: MCPサーバーと同じストレージ設定でリポジトリを構築し、REST API を起動する
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // MCPサーバーと共通の環境変数（SENGOKU_STORAGE 等）から保存先を解決する
    let storage_config = SessionStorageConfig::from_env()?;
    let storage = storage_config.build()?;
    let master_data = Arc::new(MasterDataLoader);
    let state = AppState {
        status: Arc::new(StatusQueryService::new(
            storage.clone(),
            master_data.clone(),
        )),
        games: Arc::new(GameCreationService::new(GameLobby::new(
            storage,
            master_data,
        ))),
    };

    let addr = resolve_listen_addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!(
        "[Sengoku-API] Listening on http://{} (session storage: {})",
        listener.local_addr()?,
        storage_config.describe()
    );

    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(async {
            // Ctrl+C で穏やかに停止する
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
