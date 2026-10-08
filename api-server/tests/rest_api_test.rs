//! REST API の機能テスト
//!
//! MCPハンドラーで進めたゲーム状態を、同じストレージを参照する REST API から
//! 読み取れること（状態の共有）を確認します。

use api_server::application::StatusQueryService;
use api_server::presentation::build_router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use engine::domain::repository::session_repository::SessionRepository;
use http_body_util::BodyExt;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{ObjectStoreSessionRepository, SessionPersistenceManager};
use mcp_server::application::SessionManager;
use mcp_server::presentation::handlers::{
    DomesticParams, McpHandlers, SelectDaimyoParams, SessionParams,
};
use object_store::memory::InMemory;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;

/// 同一リポジトリを共有する MCPハンドラーと REST ルーターを構築します
fn build_servers(repository: Arc<dyn SessionRepository>) -> (McpHandlers, Router) {
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(repository.clone(), master_data.clone()));
    let handlers = McpHandlers::new(session_manager);
    let router = build_router(Arc::new(StatusQueryService::new(repository, master_data)));
    (handlers, router)
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
async fn assert_state_is_shared(repository: Arc<dyn SessionRepository>) {
    let (handlers, router) = build_servers(repository);
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
    assert_eq!(before["session_id"], "shared_session");
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
    assert_state_is_shared(Arc::new(SessionPersistenceManager::new(dir.path()))).await;
}

#[tokio::test]
async fn test_state_is_shared_via_object_storage() {
    // GCS と同じ ObjectStore 実装経由で状態を共有できること（InMemory を GCS の代替として使用）
    let repository = ObjectStoreSessionRepository::new(Arc::new(InMemory::new()), "sessions");
    assert_state_is_shared(Arc::new(repository)).await;
}

#[tokio::test]
async fn test_default_session_endpoint() {
    let dir = tempdir().unwrap();
    let (handlers, router) = build_servers(Arc::new(SessionPersistenceManager::new(dir.path())));

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
    assert_eq!(body["session_id"], "default");
    assert_eq!(body["daimyo"]["id"], 4);
}

#[tokio::test]
async fn test_unknown_session_returns_404() {
    let dir = tempdir().unwrap();
    let (_, router) = build_servers(Arc::new(SessionPersistenceManager::new(dir.path())));

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
    let (handlers, router) = build_servers(Arc::new(SessionPersistenceManager::new(dir.path())));

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
    let (_, router) = build_servers(Arc::new(SessionPersistenceManager::new(dir.path())));

    let response = router
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
