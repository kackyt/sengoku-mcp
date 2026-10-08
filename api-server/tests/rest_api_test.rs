//! REST API の機能テスト
//!
//! MCPハンドラーで進めたゲーム状態を、同じストレージを参照する REST API から
//! 読み取れること（状態の共有）を確認します。

use api_server::application::StatusQueryService;
use api_server::presentation::build_router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{
    ObjectStoreSessionRepository, SessionPersistenceManager, SessionStorage,
};
use mcp_server::application::SessionManager;
use mcp_server::presentation::handlers::{
    DomesticParams, McpHandlers, SelectDaimyoParams, SessionParams, ViewUrlParams,
};
use object_store::memory::InMemory;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;

/// 同一の保存先を共有する MCPハンドラーと REST ルーターを構築します
fn build_servers(storage: SessionStorage) -> (McpHandlers, Router) {
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(storage.clone(), master_data.clone()));
    let handlers = McpHandlers::new(session_manager);
    let router = build_router(Arc::new(StatusQueryService::new(storage, master_data)));
    (handlers, router)
}

/// テスト用のファイル保存先を構築します
fn file_storage(dir: &tempfile::TempDir) -> SessionStorage {
    SessionStorage::from_backend(Arc::new(SessionPersistenceManager::new(dir.path())))
}

/// MCPツールで閲覧URLを発行し、REST API のパス部分を返します
async fn issue_view_path(handlers: &McpHandlers, session_id: &str, regenerate: bool) -> String {
    let message = handlers
        .get_status_view_url(Parameters(ViewUrlParams {
            regenerate,
            session_id: Some(session_id.to_string()),
        }))
        .await
        .unwrap();
    let url = message.lines().last().unwrap();
    // URLにはセッションIDを含めない
    assert!(!url.contains(session_id), "url: {url}");
    url.strip_prefix("http://localhost:8080")
        .expect("デフォルトの閲覧URLテンプレートであること")
        .to_string()
}

/// GET リクエストを送り、ステータスコードとボディを返します
async fn get(router: &Router, uri: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

/// 尾張（ID: 7）の兵数を取得します
fn owari_hei(body: &Value) -> u64 {
    body["kunis"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["id"] == 7)
        .expect("尾張が自領に含まれていること")["hei"]
        .as_u64()
        .unwrap()
}

/// MCPで操作した結果が REST API に反映されることを検証する共通シナリオ
async fn assert_state_is_shared(storage: SessionStorage) {
    let (handlers, router) = build_servers(storage);
    let session_id = "shared_session".to_string();

    // MCP: 織田（ID: 7）を選択してプレイヤーの手番まで進める
    handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: Some(session_id.clone()),
        }))
        .await
        .unwrap();
    // 行動順はランダムなため、既にプレイヤーの手番であればエラーになるのは許容する
    if let Err(e) = handlers
        .progress_turn(Parameters(SessionParams {
            session_id: Some(session_id.clone()),
        }))
        .await
    {
        assert!(e.contains("あなたの手番"), "想定外のエラー: {e}");
    }

    // REST: 自国の状況が取得できる
    let uri = format!("/api/sessions/{}/status", session_id);
    let (status, before) = get(&router, &uri).await;
    assert_eq!(status, StatusCode::OK, "body: {before}");
    assert_eq!(before["daimyo"]["id"], 7);
    assert_eq!(before["daimyo"]["name"], "織田");
    assert_eq!(
        before["totals"]["kuni_count"].as_u64().unwrap() as usize,
        before["kunis"].as_array().unwrap().len()
    );
    assert!(before["game"]["turn"].as_u64().unwrap() >= 1);
    assert!(before["defense_alerts"].is_array());

    // MCP: 尾張で兵を徴募する
    handlers
        .domestic_recruit(Parameters(DomesticParams {
            kuni_id: 7,
            amount: 1,
            session_id: Some(session_id.clone()),
        }))
        .await
        .unwrap();

    // REST: 徴募後の兵数が反映されている
    let (status, after) = get(&router, &uri).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        owari_hei(&after) > owari_hei(&before),
        "徴募後に兵数が増えていること: before={}, after={}",
        owari_hei(&before),
        owari_hei(&after)
    );
}

#[tokio::test]
async fn test_state_is_shared_via_file_storage() {
    let dir = tempdir().unwrap();
    assert_state_is_shared(file_storage(&dir)).await;
}

#[tokio::test]
async fn test_state_is_shared_via_object_storage() {
    // GCS と同じ ObjectStore 実装経由で状態を共有できること（InMemory を GCS の代替として使用）
    let repository = ObjectStoreSessionRepository::new(Arc::new(InMemory::new()), "sessions");
    assert_state_is_shared(SessionStorage::from_backend(Arc::new(repository))).await;
}

#[tokio::test]
async fn test_default_session_endpoint() {
    let dir = tempdir().unwrap();
    let (handlers, router) = build_servers(file_storage(&dir));

    // MCP: session_id 省略（default セッション）で武田（ID: 4）を選択
    handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 4,
            session_id: None,
        }))
        .await
        .unwrap();

    let (status, body) = get(&router, "/api/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["daimyo"]["id"], 4);
}

#[tokio::test]
async fn test_unknown_session_returns_404() {
    let dir = tempdir().unwrap();
    let (_, router) = build_servers(file_storage(&dir));

    let (status, body) = get(&router, "/api/sessions/nobody/status").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "session_not_found");

    // REST API は読み取り専用で、存在しないセッションを作成しない
    let (status, _) = get(&router, "/api/sessions/nobody/status").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_session_without_daimyo_returns_409() {
    let dir = tempdir().unwrap();
    let (handlers, router) = build_servers(file_storage(&dir));

    // MCP: 大名一覧の取得だけ行い、大名は選択しない（セッションは作成される）
    handlers
        .list_daimyos(Parameters(SessionParams {
            session_id: Some("no_daimyo".to_string()),
        }))
        .await
        .unwrap();

    let (status, body) = get(&router, "/api/sessions/no_daimyo/status").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "daimyo_not_selected");
}

#[tokio::test]
async fn test_health() {
    let dir = tempdir().unwrap();
    let (_, router) = build_servers(file_storage(&dir));

    let response = router
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_view_url_flow() {
    let dir = tempdir().unwrap();
    let (handlers, router) = build_servers(file_storage(&dir));
    let session_id = "chat-12345";

    // MCP: 大名を選択すると、LLMがツールを選ばなくても結果に閲覧URLが自動で付く
    let message = handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: Some(session_id.to_string()),
        }))
        .await
        .unwrap();
    let url = message
        .split_whitespace()
        .find(|w| w.starts_with("http://"))
        .expect("大名選択の結果に閲覧URLが含まれること");
    let path = url
        .strip_prefix("http://localhost:8080")
        .unwrap()
        .to_string();
    assert!(path.starts_with("/api/views/") && path.ends_with("/status"));
    assert!(!url.contains(session_id));

    // 明示的な取得ツールでも同じURLが返る
    assert_eq!(issue_view_path(&handlers, session_id, false).await, path);

    // REST: セッションIDを知らなくてもトークンだけで自国の状況を取得できる
    let (status, body) = get(&router, &path).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["daimyo"]["name"], "織田");
    assert!(!body.to_string().contains(session_id));

    // MCP: 再発行すると旧URLは無効になり、新URLで取得できる
    let new_path = issue_view_path(&handlers, session_id, true).await;
    assert_ne!(new_path, path);
    let (status, body) = get(&router, &path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "view_not_found");
    let (status, _) = get(&router, &new_path).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_view_url_via_object_storage() {
    // GCS と同じ ObjectStore 実装でもトークン経由で取得できること
    let repository = ObjectStoreSessionRepository::new(Arc::new(InMemory::new()), "sessions");
    let (handlers, router) = build_servers(SessionStorage::from_backend(Arc::new(repository)));

    handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 4,
            session_id: Some("gcs_user".to_string()),
        }))
        .await
        .unwrap();
    let path = issue_view_path(&handlers, "gcs_user", false).await;

    let (status, body) = get(&router, &path).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["daimyo"]["id"], 4);
}

#[tokio::test]
async fn test_invalid_view_token_returns_404() {
    let dir = tempdir().unwrap();
    let (_, router) = build_servers(file_storage(&dir));

    // 形式外のトークンや未発行のトークンはいずれも 404
    for path in [
        "/api/views/not-a-token/status",
        "/api/views/..%2F..%2Fdefault/status",
        "/api/views/0123456789abcdef0123456789abcdef/status",
    ] {
        let (status, body) = get(&router, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path: {path}");
        assert_eq!(body["code"], "view_not_found");
    }
}
