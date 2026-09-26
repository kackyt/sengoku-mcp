mod application;
mod presentation;

extern crate rmcp;

use crate::application::SessionManager;
use crate::presentation::handlers::McpHandlers;
use chrono::Duration;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionPersistenceManager;
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // マスターデータのローダー初期化
    let master_data = Arc::new(MasterDataLoader);

    // セッション永続化マネージャー初期化（デフォルト: data/sessions/ または環境変数）
    let persistence = Arc::new(SessionPersistenceManager::default());

    // 起動時に7日以上経過した期限切れセッションをクリーンアップ
    let expired_ttl = Duration::days(7);
    if let Ok(cleaned) = persistence.cleanup_expired(expired_ttl) {
        if cleaned > 0 {
            eprintln!("[Sengoku-MCP] Cleaned up {} expired session(s)", cleaned);
        }
    }

    // セッションマネージャーの構築
    let session_manager = Arc::new(SessionManager::new(persistence, master_data));

    // バックグラウンドで定期クリーンアップタスク（1時間間隔、7日経過で削除）を開始
    session_manager
        .clone()
        .start_cleanup_task(std::time::Duration::from_secs(3600), expired_ttl);

    // MCPハンドラーの初期化
    let handlers = McpHandlers::new(session_manager);

    // Build the transport (stdio)
    let transport = (stdin(), stdout());

    // Initialize and start the server
    let server = handlers.serve(transport).await?;

    // Wait for the server to finish
    server.waiting().await?;

    Ok(())
}
