mod common;

use common::new_session_manager;
use engine::domain::model::value_objects::SessionId;
use infrastructure::persistence::{SessionData, SessionPersistenceManager};
use mcp_server::presentation::handlers::{McpHandlers, SessionParams};
use rmcp::handler::server::wrapper::Parameters;
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;

/// 指定ディレクトリを保存先とする MCPハンドラーを構築します
fn build_handlers(dir: &Path) -> McpHandlers {
    McpHandlers::new(new_session_manager(Arc::new(
        SessionPersistenceManager::new(dir),
    )))
}

#[tokio::test]
async fn test_legacy_session_gets_view_token_on_load() {
    let dir = tempdir().unwrap();
    let persistence = SessionPersistenceManager::new(dir.path());

    // 閲覧トークン導入前の形式（view_token なし）のセッションを用意する
    let session_id = SessionId::new("chat_42");
    let legacy = SessionData::new(
        session_id.clone(),
        None,
        None,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    persistence.save(&legacy).unwrap();
    assert!(persistence
        .load(&session_id)
        .unwrap()
        .unwrap()
        .view_token
        .is_none());

    // 読み込み時にトークンが発行され、保存先にも反映される
    let handlers = build_handlers(dir.path());
    handlers
        .list_daimyos(Parameters(SessionParams {
            session_id: Some("chat_42".to_string()),
        }))
        .await
        .unwrap();
    let saved = persistence.load(&session_id).unwrap().unwrap();
    let token = saved
        .view_token
        .expect("読み込み時に閲覧トークンが発行されること");
    assert_eq!(
        persistence.find_view_token(&token).unwrap(),
        Some(session_id)
    );
}
