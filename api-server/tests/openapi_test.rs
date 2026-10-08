//! OpenAPI 仕様のテスト
//!
//! リポジトリに保存した `api-server/openapi.json` がコードから生成される仕様と一致することを検証します。
//! ハンドラーや DTO（ドキュメントコメント含む）を変更した場合は、次のコマンドで更新してください。
//!
//! ```bash
//! UPDATE_OPENAPI=1 cargo test -p api-server --test openapi_test
//! ```

use api_server::application::{GameCreationService, StatusQueryService};
use api_server::presentation::{build_router, ApiDoc, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use game_session::GameLobby;
use http_body_util::BodyExt;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{SessionPersistenceManager, SessionStorage};
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;
use utoipa::OpenApi;

/// 保存済みの OpenAPI 仕様ファイルのパス
fn spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("openapi.json")
}

/// コードから生成した OpenAPI 仕様（末尾改行付き）
fn generated_spec() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap() + "\n"
}

#[test]
fn test_openapi_json_is_up_to_date() {
    let generated = generated_spec();
    if std::env::var("UPDATE_OPENAPI").is_ok() {
        std::fs::write(spec_path(), &generated).unwrap();
        return;
    }
    let saved = std::fs::read_to_string(spec_path()).unwrap_or_default();
    assert!(
        saved == generated,
        "api-server/openapi.json が古くなっています。\
         `UPDATE_OPENAPI=1 cargo test -p api-server --test openapi_test` で更新してください。"
    );
}

#[test]
fn test_openapi_contains_all_endpoints() {
    let spec: serde_json::Value = serde_json::from_str(&generated_spec()).unwrap();
    for path in [
        "/health",
        "/api/games",
        "/api/views/{token}/status",
        "/api/status",
        "/api/sessions/{session_id}/status",
    ] {
        assert!(
            spec["paths"][path].is_object(),
            "{path} が仕様に含まれること"
        );
    }
    // ドキュメントコメントが説明として出力されていること
    assert_eq!(
        spec["paths"]["/api/games"]["post"]["summary"],
        "新規ゲームを作成する"
    );
    // エラーコードが列挙値として定義されていること
    let codes = spec["components"]["schemas"]["ErrorCode"]["enum"]
        .as_array()
        .unwrap();
    assert!(codes.contains(&serde_json::json!("waiting_for_join")));
}

#[tokio::test]
async fn test_openapi_and_docs_are_served() {
    let dir = tempfile::tempdir().unwrap();
    let storage =
        SessionStorage::from_backend(Arc::new(SessionPersistenceManager::new(dir.path())));
    let master_data = Arc::new(MasterDataLoader);
    let router = build_router(AppState {
        status: Arc::new(StatusQueryService::new(
            storage.clone(),
            master_data.clone(),
        )),
        games: Arc::new(GameCreationService::new(GameLobby::new(
            storage,
            master_data,
        ))),
    });

    // GET /openapi.json はコードから生成した仕様を返す
    let response = router
        .clone()
        .oneshot(Request::get("/openapi.json").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let served: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let expected: serde_json::Value = serde_json::from_str(&generated_spec()).unwrap();
    assert_eq!(served, expected);

    // GET /docs は API ドキュメント画面（HTML）を返す
    let response = router
        .oneshot(Request::get("/docs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cors_layer() {
    use api_server::presentation::cors_layer;

    // 未指定・空なら CORS レイヤーなし
    assert!(cors_layer(None).is_none());
    assert!(cors_layer(Some("  ")).is_none());

    let dir = tempfile::tempdir().unwrap();
    let storage =
        SessionStorage::from_backend(Arc::new(SessionPersistenceManager::new(dir.path())));
    let master_data = Arc::new(MasterDataLoader);
    let router = build_router(AppState {
        status: Arc::new(StatusQueryService::new(
            storage.clone(),
            master_data.clone(),
        )),
        games: Arc::new(GameCreationService::new(GameLobby::new(
            storage,
            master_data,
        ))),
    })
    .layer(cors_layer(Some("http://localhost:5173, https://sengoku.example.com")).unwrap());

    // 許可したオリジンには Access-Control-Allow-Origin が付く
    let response = router
        .clone()
        .oneshot(
            Request::get("/health")
                .header("Origin", "https://sengoku.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.headers()["access-control-allow-origin"],
        "https://sengoku.example.com"
    );

    // 許可していないオリジンには付かない
    let response = router
        .oneshot(
            Request::get("/health")
                .header("Origin", "https://evil.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}
