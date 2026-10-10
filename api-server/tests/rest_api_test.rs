//! REST API の機能テスト
//!
//! MCPハンドラーで進めたゲーム状態を、同じストレージを参照する REST API から
//! 読み取れること（状態の共有）を確認します。

use api_server::application::{GameCreationService, StatusQueryService};
use api_server::presentation::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use engine::domain::repository::master_data_repository::MasterDataRepository;
use game_session::GameLobby;
use http_body_util::BodyExt;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{
    ObjectStoreSessionRepository, SessionPersistenceManager, SessionStorage,
};
use mcp_server::application::SessionManager;
use mcp_server::presentation::handlers::{
    DomesticParams, JoinGameParams, McpHandlers, SelectDaimyoParams, SessionParams, ViewUrlParams,
};
use object_store::memory::InMemory;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;

/// 同一の保存先を共有する MCPハンドラーと REST ルーターを構築します（テスト用の Composition Root）
///
/// 保存先の各リポジトリ（trait オブジェクト）を、MCP 側と REST 側のサービスへそれぞれ注入する。
fn build_servers(storage: SessionStorage) -> (McpHandlers, Router) {
    let SessionStorage {
        sessions,
        view_tokens,
        join_tickets,
    } = storage;
    let master_data: Arc<dyn MasterDataRepository> = Arc::new(MasterDataLoader);
    let lobby = GameLobby::new(
        sessions.clone(),
        view_tokens.clone(),
        join_tickets,
        master_data.clone(),
    );
    let session_manager = Arc::new(SessionManager::new(
        sessions.clone(),
        view_tokens.clone(),
        lobby.clone(),
        master_data.clone(),
    ));
    let state = AppState {
        status: Arc::new(StatusQueryService::new(sessions, view_tokens, master_data)),
        games: Arc::new(GameCreationService::new(lobby)),
    };
    (McpHandlers::new(session_manager), build_router(state))
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
    send(router, Request::get(uri).body(Body::empty()).unwrap()).await
}

/// POST リクエストを送り、ステータスコードとボディを返します
async fn post(router: &Router, uri: &str) -> (StatusCode, Value) {
    send(router, Request::post(uri).body(Body::empty()).unwrap()).await
}

/// リクエストを送り、ステータスコードと JSON ボディを返します
async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

/// 尾張（ID: 7）の兵数を取得します
fn owari_hei(body: &Value) -> u64 {
    body["my_kunis"]
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
    assert!(before["turn"].as_u64().unwrap() >= 1);
    // 返すのは「自国の状況」「ターン数」「他国の支配大名」のみ
    let mut keys: Vec<_> = before.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["daimyo", "my_kunis", "other_kunis", "turn"]);
    // 自領と他国で全12国を網羅し、他国は支配大名のみ（資源は含まない）
    let my_count = before["my_kunis"].as_array().unwrap().len();
    let others = before["other_kunis"].as_array().unwrap();
    assert_eq!(my_count + others.len(), 12);
    let mikawa = others
        .iter()
        .find(|k| k["id"] == 6)
        .expect("三河が他国に含まれること");
    assert_eq!(mikawa["name"], "三河");
    assert_eq!(mikawa["daimyo"]["name"], "徳川");
    let mut other_keys: Vec<_> = mikawa.as_object().unwrap().keys().cloned().collect();
    other_keys.sort();
    assert_eq!(other_keys, ["daimyo", "id", "name"]);
    assert!(others.iter().all(|k| k["daimyo"]["id"] != 7));

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

/// MCPツールで参加コードを使ってゲームに参加します
async fn join(handlers: &McpHandlers, code: &str, session_id: &str) -> Result<String, String> {
    handlers
        .join_game(Parameters(JoinGameParams {
            code: code.to_string(),
            session_id: Some(session_id.to_string()),
        }))
        .await
}

/// Webでゲームを作成し、(閲覧パス, 参加コード) を返します
async fn create_game(router: &Router) -> (String, String) {
    let (status, body) = post(router, "/api/games").await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["join_message"]
        .as_str()
        .unwrap()
        .contains(body["join_code"].as_str().unwrap()));
    assert!(body["join_code_expires_at"].is_string());
    (
        body["status_url"].as_str().unwrap().to_string(),
        body["join_code"].as_str().unwrap().to_string(),
    )
}

/// Webでゲームを作成 → チャットで参加 → 大名選択 の一連の流れを検証する共通シナリオ
async fn assert_web_created_game_flow(storage: SessionStorage) {
    let (handlers, router) = build_servers(storage);
    let chat_id = "discord_98765";

    // 1. Web: 新規ゲームを作成すると、閲覧URLと参加コードが返る
    let (path, code) = create_game(&router).await;

    // 2. Web: チャット側が参加するまでは「参加待ち」
    let (status, body) = get(&router, &path).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "waiting_for_join");

    // 3. MCP: プレイヤーが伝えた参加コード（小文字・区切り入り）で参加する
    let typed = format!("{}-{}", &code[..3], &code[3..]).to_ascii_lowercase();
    let message = join(&handlers, &typed, chat_id).await.unwrap();
    assert!(message.contains("select_daimyo"));

    // 4. Web: 参加後、大名選択までは「大名未選択」
    let (status, body) = get(&router, &path).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "daimyo_not_selected");
    // エラーメッセージにもセッションIDは含めない
    assert!(!body.to_string().contains(chat_id), "body: {body}");

    // 5. MCP: 大名を選択すると、Webの同じURLで状況が見られる
    let message = handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: Some(chat_id.to_string()),
        }))
        .await
        .unwrap();
    assert!(
        message.contains(&path),
        "同じ閲覧URLが付与されること: {message}"
    );
    let (status, body) = get(&router, &path).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["daimyo"]["name"], "織田");
    assert!(!body.to_string().contains(chat_id));
    assert!(!body.to_string().contains("web_"));

    // 6. MCP: 参加コードは1回限り
    let err = join(&handlers, &code, chat_id).await.unwrap_err();
    assert!(err.contains("見つかりません"), "err: {err}");
}

#[tokio::test]
async fn test_web_created_game_flow_via_file_storage() {
    let dir = tempdir().unwrap();
    assert_web_created_game_flow(file_storage(&dir)).await;
}

#[tokio::test]
async fn test_web_created_game_flow_via_object_storage() {
    let repository = ObjectStoreSessionRepository::new(Arc::new(InMemory::new()), "sessions");
    assert_web_created_game_flow(SessionStorage::from_backend(Arc::new(repository))).await;
}

#[tokio::test]
async fn test_join_replaces_existing_chat_game() {
    let dir = tempdir().unwrap();
    let (handlers, router) = build_servers(file_storage(&dir));
    let chat_id = "discord_1";

    // 1つ目のゲームに参加して大名を選択
    let (first_path, first_code) = create_game(&router).await;
    join(&handlers, &first_code, chat_id).await.unwrap();
    handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: Some(chat_id.to_string()),
        }))
        .await
        .unwrap();
    assert_eq!(get(&router, &first_path).await.0, StatusCode::OK);

    // 同じチャットで2つ目のゲームに参加すると、新しいゲームに置き換わり旧URLは無効になる
    let (second_path, second_code) = create_game(&router).await;
    join(&handlers, &second_code, chat_id).await.unwrap();
    let (status, body) = get(&router, &first_path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "view_not_found");
    let (status, body) = get(&router, &second_path).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "daimyo_not_selected");

    // 参加待ちだったセッションのファイルは残らない（チャットのセッション1件のみ）
    let session_files = std::fs::read_dir(dir.path())
        .unwrap()
        .filter(|e| e.as_ref().unwrap().file_type().unwrap().is_file())
        .count();
    assert_eq!(session_files, 1);
}

#[tokio::test]
async fn test_join_with_invalid_code() {
    let dir = tempdir().unwrap();
    let (handlers, _) = build_servers(file_storage(&dir));

    let err = join(&handlers, "hello", "discord_1").await.unwrap_err();
    assert!(err.contains("形式"), "err: {err}");
    let err = join(&handlers, "ABCDEF", "discord_1").await.unwrap_err();
    assert!(err.contains("見つかりません"), "err: {err}");
}
