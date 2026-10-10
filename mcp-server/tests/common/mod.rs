//! MCPサーバーの機能テストで共通に使う組み立て処理（テスト用の Composition Root）

use engine::domain::repository::master_data_repository::MasterDataRepository;
use game_session::GameLobby;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionPersistenceManager;
use mcp_server::application::SessionManager;
use mcp_server::presentation::handlers::{JoinGameParams, McpHandlers};
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

/// ファイル保存のリポジトリを注入した GameLobby と SessionManager を組み立てます
///
/// SessionPersistenceManager はセッション・閲覧トークン・参加チケットの各リポジトリ trait を
/// 実装しているため、同じインスタンスをそれぞれの trait オブジェクトとして注入する。
/// ゲーム作成（参加コード発行）にも使えるよう、SessionManager と同じ GameLobby も返す。
#[allow(dead_code)]
pub fn new_lobby_and_manager(
    backend: Arc<SessionPersistenceManager>,
) -> (GameLobby, Arc<SessionManager>) {
    let master_data: Arc<dyn MasterDataRepository> = Arc::new(MasterDataLoader);
    let lobby = GameLobby::new(
        backend.clone(),
        backend.clone(),
        backend.clone(),
        master_data.clone(),
    );
    let manager = Arc::new(SessionManager::new(
        backend.clone(),
        backend,
        lobby.clone(),
        master_data,
    ));
    (lobby, manager)
}

/// ファイル保存のリポジトリを注入した SessionManager を組み立てます
pub fn new_session_manager(backend: Arc<SessionPersistenceManager>) -> Arc<SessionManager> {
    new_lobby_and_manager(backend).1
}

/// 新しいゲームを作成し、参加コードで指定セッションに参加させます
///
/// 未参加のセッションに対するツール呼び出しはエラーになるため、
/// テストでは大名選択などの前にこの関数でゲームを用意する。
#[allow(dead_code)]
pub async fn start_game(lobby: &GameLobby, handlers: &McpHandlers, session_id: &str) {
    let created = lobby.create_game().await.unwrap();
    handlers
        .join_game(Parameters(JoinGameParams {
            code: created.join_code.value().to_string(),
            session_id: Some(session_id.to_string()),
        }))
        .await
        .unwrap();
}
