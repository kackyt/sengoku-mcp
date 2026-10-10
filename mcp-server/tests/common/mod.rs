//! MCPサーバーの機能テストで共通に使う組み立て処理（テスト用の Composition Root）

use engine::domain::repository::master_data_repository::MasterDataRepository;
use game_session::GameLobby;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionPersistenceManager;
use mcp_server::application::SessionManager;
use std::sync::Arc;

/// ファイル保存のリポジトリを注入した SessionManager を組み立てます
///
/// SessionPersistenceManager はセッション・閲覧トークン・参加チケットの各リポジトリ trait を
/// 実装しているため、同じインスタンスをそれぞれの trait オブジェクトとして注入する。
pub fn new_session_manager(backend: Arc<SessionPersistenceManager>) -> Arc<SessionManager> {
    let master_data: Arc<dyn MasterDataRepository> = Arc::new(MasterDataLoader);
    let lobby = GameLobby::new(
        backend.clone(),
        backend.clone(),
        backend.clone(),
        master_data.clone(),
    );
    Arc::new(SessionManager::new(
        backend.clone(),
        backend,
        lobby,
        master_data,
    ))
}
