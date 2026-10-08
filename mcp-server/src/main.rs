mod application;
mod presentation;

extern crate rmcp;

use crate::application::SessionManager;
use crate::presentation::handlers::McpHandlers;
use chrono::Duration;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionStorageConfig;
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // マスターデータのローダー初期化
    let master_data = Arc::new(MasterDataLoader);

    // セッション永続化先の初期化（環境変数 SENGOKU_STORAGE で file / gcs を切り替え）
    let storage_config = SessionStorageConfig::from_env()?;
    eprintln!(
        "[Sengoku-MCP] Session storage: {}",
        storage_config.describe()
    );
    let storage = storage_config.build()?;

    // 起動時に7日以上経過した期限切れセッションをクリーンアップ
    let expired_ttl = Duration::days(7);
    match storage.sessions.cleanup_expired(expired_ttl).await {
        Ok(cleaned) if cleaned > 0 => {
            eprintln!("[Sengoku-MCP] Cleaned up {} expired session(s)", cleaned);
        }
        Ok(_) => {}
        Err(e) => eprintln!("[Sengoku-MCP] Cleanup error: {}", e),
    }

    // セッションマネージャーの構築
    let session_manager = Arc::new(SessionManager::new(storage, master_data));

    // バックグラウンドで定期クリーンアップタスク（1時間間隔、7日経過で削除）を開始
    session_manager
        .clone()
        .start_cleanup_task(std::time::Duration::from_secs(3600), expired_ttl);

    // MCPハンドラーの初期化
    // 閲覧URLのテンプレートは SENGOKU_VIEW_URL_TEMPLATE で上書きできる（例: Webアプリのページ）
    let mut handlers = McpHandlers::new(session_manager);
    if let Ok(template) = std::env::var("SENGOKU_VIEW_URL_TEMPLATE") {
        handlers = handlers.with_view_url_template(template);
    }

    // Build the transport (stdio)
    let transport = (stdin(), stdout());

    // Initialize and start the server
    let server = handlers.serve(transport).await?;

    // Wait for the server to finish
    server.waiting().await?;

    Ok(())
}
