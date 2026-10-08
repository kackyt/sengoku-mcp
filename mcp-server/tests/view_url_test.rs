use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{SessionPersistenceManager, SessionStorage};
use mcp_server::application::SessionManager;
use mcp_server::presentation::handlers::{McpHandlers, SelectDaimyoParams, ViewUrlParams};
use rmcp::handler::server::wrapper::Parameters;
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;

/// 指定ディレクトリを保存先とする MCPハンドラーを構築します
fn build_handlers(dir: &Path) -> McpHandlers {
    let storage = SessionStorage::from_backend(Arc::new(SessionPersistenceManager::new(dir)));
    let session_manager = Arc::new(SessionManager::new(storage, Arc::new(MasterDataLoader)));
    McpHandlers::new(session_manager)
}

/// 閲覧URLを発行し、メッセージ末尾のURLを返します
async fn view_url(handlers: &McpHandlers, regenerate: bool) -> String {
    handlers
        .get_status_view_url(Parameters(ViewUrlParams {
            regenerate,
            session_id: Some("chat_42".to_string()),
        }))
        .await
        .unwrap()
        .lines()
        .last()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn test_view_url_survives_restart() {
    let dir = tempdir().unwrap();

    // 1. 大名を選択して閲覧URLを発行
    let url = {
        let handlers = build_handlers(dir.path());
        handlers
            .select_daimyo(Parameters(SelectDaimyoParams {
                daimyo_id: 7,
                session_id: Some("chat_42".to_string()),
            }))
            .await
            .unwrap();
        view_url(&handlers, false).await
    };
    assert!(url.starts_with("http://localhost:8080/api/views/"));
    assert!(!url.contains("chat_42"));

    // 2. メモリをリセットした新しいサーバーでも同じURLが返る（セッションと一緒に永続化される）
    let handlers = build_handlers(dir.path());
    assert_eq!(view_url(&handlers, false).await, url);

    // 3. 再発行するとURLが変わり、以降はその新しいURLが返る
    let new_url = view_url(&handlers, true).await;
    assert_ne!(new_url, url);
    assert_eq!(view_url(&handlers, false).await, new_url);
}

#[tokio::test]
async fn test_view_url_template() {
    let dir = tempdir().unwrap();
    let handlers = build_handlers(dir.path())
        .with_view_url_template("https://sengoku.example.com/view?token={token}");

    let url = view_url(&handlers, false).await;
    let token = url
        .strip_prefix("https://sengoku.example.com/view?token=")
        .expect("テンプレートに沿ったURLであること");
    assert_eq!(token.len(), 32);
}
