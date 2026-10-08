use engine::domain::model::value_objects::SessionId;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{SessionData, SessionPersistenceManager, SessionStorage};
use mcp_server::application::SessionManager;
use mcp_server::presentation::handlers::{
    McpHandlers, SelectDaimyoParams, SessionParams, ViewUrlParams,
};
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

/// ツール結果から閲覧URLを抽出します
fn extract_url(message: &str) -> Option<&str> {
    message
        .split_whitespace()
        .find(|w| w.starts_with("http://") || w.starts_with("https://"))
}

#[tokio::test]
async fn test_view_url_is_attached_automatically() {
    let dir = tempdir().unwrap();
    let handlers = build_handlers(dir.path());

    // 大名選択（ゲーム開始）の結果に閲覧URLと、プレイヤーへ伝える指示が付く
    let message = handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: Some("chat_42".to_string()),
        }))
        .await
        .unwrap();
    let url = extract_url(&message).expect("閲覧URLが付与されること");
    assert!(url.starts_with("http://localhost:8080/api/views/"));
    assert!(message.contains("プレイヤーにそのまま伝えてください"));

    // 自国の状況の結果にも同じURLが付く
    let status = handlers
        .get_my_status(Parameters(SessionParams {
            session_id: Some("chat_42".to_string()),
        }))
        .await
        .unwrap();
    assert!(status.contains("尾張"));
    assert_eq!(extract_url(&status), Some(url));

    // 明示的な取得ツールとも一致する
    assert_eq!(view_url(&handlers, false).await, url);
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
    let url = view_url(&handlers, false).await;
    let saved = persistence.load(&session_id).unwrap().unwrap();
    let token = saved
        .view_token
        .expect("読み込み時に閲覧トークンが発行されること");
    assert!(url.contains(token.value()));
    assert_eq!(
        persistence.find_view_token(&token).unwrap(),
        Some(session_id)
    );
}
