use api_server::application::{GameCreationService, StatusQueryService};
use api_server::presentation::{build_router, cors_layer, AppState, ENV_CORS_ALLOW_ORIGINS};
use engine::domain::repository::master_data_repository::MasterDataRepository;
use game_session::GameLobby;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{SessionStorage, SessionStorageConfig};
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
    let SessionStorage {
        sessions,
        view_tokens,
        join_tickets,
    } = storage_config.build()?;
    let master_data: Arc<dyn MasterDataRepository> = Arc::new(MasterDataLoader);

    // アプリケーションサービスへ具象リポジトリ（trait オブジェクト）を注入する
    let lobby = GameLobby::new(
        sessions.clone(),
        view_tokens.clone(),
        join_tickets,
        master_data.clone(),
    );
    let state = AppState {
        status: Arc::new(StatusQueryService::new(sessions, view_tokens, master_data)),
        games: Arc::new(GameCreationService::new(lobby)),
    };

    let addr = resolve_listen_addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!(
        "[Sengoku-API] Listening on http://{} (session storage: {})",
        listener.local_addr()?,
        storage_config.describe()
    );

    // Webアプリを別オリジンで配信する場合は SENGOKU_CORS_ALLOW_ORIGINS で許可オリジンを指定する
    let mut router = build_router(state);
    if let Some(layer) = cors_layer(std::env::var(ENV_CORS_ALLOW_ORIGINS).ok().as_deref()) {
        router = router.layer(layer);
    }

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            // Ctrl+C で穏やかに停止する
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
