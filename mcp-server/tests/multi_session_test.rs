use chrono::{Duration, Utc};
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{SessionData, SessionPersistenceManager};
use mcp_server::presentation::handlers::{
    DomesticParams, McpHandlers, SelectDaimyoParams, SessionParams,
};
use mcp_server::presentation::session_manager::SessionManager;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_backward_compatibility_default_session() {
    let dir = tempdir().unwrap();
    let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(persistence, master_data));
    let handlers = McpHandlers::new(session_manager);

    // 1. session_id: None で大名一覧取得
    let list_res = handlers
        .list_daimyos(Parameters(SessionParams { session_id: None }))
        .await
        .unwrap();
    assert!(list_res.contains("織田"));

    // 2. session_id: None で大名選択 (織田 ID: 7)
    let select_res = handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: None,
        }))
        .await
        .unwrap();
    assert!(select_res.contains("織田"));

    // 3. session_id: None でステータス取得
    let status_res = handlers
        .get_my_status(Parameters(SessionParams { session_id: None }))
        .await
        .unwrap();
    assert!(status_res.contains("尾張"));
}

#[tokio::test]
async fn test_multi_session_isolation() {
    let dir = tempdir().unwrap();
    let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(persistence, master_data));
    let handlers = McpHandlers::new(session_manager);

    // セッションA (織田 ID: 7)
    handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: Some("session_oda".to_string()),
        }))
        .await
        .unwrap();

    // セッションB (武田 ID: 4)
    handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 4,
            session_id: Some("session_takeda".to_string()),
        }))
        .await
        .unwrap();

    // セッションAの確認: 尾張（ID: 7）があり、甲信（ID: 4）はない
    let status_a = handlers
        .get_my_status(Parameters(SessionParams {
            session_id: Some("session_oda".to_string()),
        }))
        .await
        .unwrap();
    assert!(status_a.contains("尾張"));
    assert!(!status_a.contains("甲信"));

    // セッションBの確認: 甲信（ID: 4）があり、尾張（ID: 7）はない
    let status_b = handlers
        .get_my_status(Parameters(SessionParams {
            session_id: Some("session_takeda".to_string()),
        }))
        .await
        .unwrap();
    assert!(status_b.contains("甲信"));
    assert!(!status_b.contains("尾張"));

    // セッションBで尾張 (ID: 7) の内政を実行しようとすると、自領ではないためエラーになること
    let err_res = handlers
        .domestic_develop_land(Parameters(DomesticParams {
            kuni_id: 7,
            amount: 10,
            session_id: Some("session_takeda".to_string()),
        }))
        .await;
    assert!(err_res.is_err());
    assert!(err_res.unwrap_err().contains("あなたの領地ではありません"));
}

#[tokio::test]
async fn test_session_persistence_and_restoration() {
    let dir = tempdir().unwrap();

    // 1. セッションを作成してプレイヤー手番まで進めて内政を実行
    {
        let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
        let master_data = Arc::new(MasterDataLoader);
        let session_manager = Arc::new(SessionManager::new(persistence, master_data));
        let handlers = McpHandlers::new(session_manager);

        handlers
            .select_daimyo(Parameters(SelectDaimyoParams {
                daimyo_id: 7,
                session_id: Some("persistent_session".to_string()),
            }))
            .await
            .unwrap();

        // プレイヤーの手番まで進行
        handlers
            .progress_turn(Parameters(SessionParams {
                session_id: Some("persistent_session".to_string()),
            }))
            .await
            .unwrap();

        // 米を売却して金を増やす (尾張 ID: 7)
        handlers
            .domestic_rice_sell(Parameters(DomesticParams {
                kuni_id: 7,
                amount: 10,
                session_id: Some("persistent_session".to_string()),
            }))
            .await
            .unwrap();
    }

    // 2. メモリがリセットされた新しい SessionManager で同じセッションをロード
    {
        let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
        let master_data = Arc::new(MasterDataLoader);
        let session_manager = Arc::new(SessionManager::new(persistence, master_data));
        let handlers = McpHandlers::new(session_manager);

        // 状態が復元され、織田が選択された状態であること
        let status = handlers
            .get_my_status(Parameters(SessionParams {
                session_id: Some("persistent_session".to_string()),
            }))
            .await
            .unwrap();
        assert!(status.contains("尾張"));
    }
}

#[tokio::test]
async fn test_cleanup_expired_sessions() {
    let dir = tempdir().unwrap();
    let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(persistence.clone(), master_data));

    // 1. 8日前のセッションを作成してファイル保存
    let mut old_data = SessionData::new("old_session", None, None, vec![], vec![], vec![], vec![]);
    old_data.last_accessed_at = Utc::now() - Duration::days(8);
    persistence.save(&old_data).unwrap();

    // 2. 現在のセッションを作成
    let new_data = SessionData::new("new_session", None, None, vec![], vec![], vec![], vec![]);
    persistence.save(&new_data).unwrap();

    // 3. クリーンアップ実行 (7日経過)
    let cleaned = session_manager
        .cleanup_expired(Duration::days(7))
        .await
        .unwrap();
    assert_eq!(cleaned, 1);

    // 4. old_sessionは削除され、new_sessionは残る
    assert!(persistence.load("old_session").unwrap().is_none());
    assert!(persistence.load("new_session").unwrap().is_some());
}
