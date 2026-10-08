use chrono::{Duration, Utc};
use engine::domain::model::value_objects::SessionId;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{SessionData, SessionPersistenceManager, SessionStorage};
use mcp_server::application::SessionManager;
use mcp_server::presentation::handlers::{
    DomesticParams, McpHandlers, SelectDaimyoParams, SessionParams,
};
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_backward_compatibility_default_session() {
    let dir = tempdir().unwrap();
    let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(
        SessionStorage::from_backend(persistence),
        master_data,
    ));
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
    let session_manager = Arc::new(SessionManager::new(
        SessionStorage::from_backend(persistence),
        master_data,
    ));
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
        let session_manager = Arc::new(SessionManager::new(
            SessionStorage::from_backend(persistence),
            master_data,
        ));
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
        let session_manager = Arc::new(SessionManager::new(
            SessionStorage::from_backend(persistence),
            master_data,
        ));
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
    let session_manager = Arc::new(SessionManager::new(
        SessionStorage::from_backend(persistence.clone()),
        master_data,
    ));

    // 1. 8日前のセッションを作成してファイル保存
    let mut old_data = SessionData::new(
        SessionId::new("old_session"),
        None,
        None,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    old_data.last_accessed_at = Utc::now() - Duration::days(8);
    persistence.save(&old_data).unwrap();

    // 2. 現在のセッションを作成
    let new_data = SessionData::new(
        SessionId::new("new_session"),
        None,
        None,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    persistence.save(&new_data).unwrap();

    // 3. クリーンアップ実行 (7日経過)
    let cleaned = session_manager
        .cleanup_expired(Duration::days(7))
        .await
        .unwrap();
    assert_eq!(cleaned, 1);

    // 4. old_sessionは削除され、new_sessionは残る
    assert!(persistence
        .load(&SessionId::new("old_session"))
        .unwrap()
        .is_none());
    assert!(persistence
        .load(&SessionId::new("new_session"))
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_path_traversal_safety() {
    let dir = tempdir().unwrap();
    let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(
        SessionStorage::from_backend(persistence.clone()),
        master_data,
    ));
    let handlers = McpHandlers::new(session_manager);

    // パストラバーサル文字を含むセッションID
    let malicious_id = "../../etc/passwd_test";

    // 大名選択
    let res = handlers
        .select_daimyo(Parameters(SelectDaimyoParams {
            daimyo_id: 7,
            session_id: Some(malicious_id.to_string()),
        }))
        .await;
    assert!(res.is_ok());

    // 保存先ディレクトリ外にファイルが作られていないこと（storage_dir内にサニタイズされたファイル名で作成されること）
    // ※ 閲覧トークンの対応表は storage_dir 内の view_tokens/ サブディレクトリに保存される
    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_file())
        .collect();
    assert_eq!(entries.len(), 1);
    let filename = entries[0].file_name().into_string().unwrap();
    assert!(!filename.contains('/'));
    assert!(!filename.contains('\\'));
    assert!(filename.ends_with(".json"));
}

#[tokio::test]
async fn test_concurrent_sessions() {
    let dir = tempdir().unwrap();
    let persistence = Arc::new(SessionPersistenceManager::new(dir.path()));
    let master_data = Arc::new(MasterDataLoader);
    let session_manager = Arc::new(SessionManager::new(
        SessionStorage::from_backend(persistence),
        master_data,
    ));
    let handlers = Arc::new(McpHandlers::new(session_manager));

    let mut handles = Vec::new();

    // 5つの異なるセッションから同時に大名選択とステータス取得を実行
    for i in 1..=5 {
        let handlers_clone = handlers.clone();
        let handle = tokio::spawn(async move {
            let session_id = format!("concurrent_user_{}", i);
            // 大名選択（大名ID 1〜5）
            handlers_clone
                .select_daimyo(Parameters(SelectDaimyoParams {
                    daimyo_id: i,
                    session_id: Some(session_id.clone()),
                }))
                .await
                .unwrap();

            // ステータス取得
            let status = handlers_clone
                .get_my_status(Parameters(SessionParams {
                    session_id: Some(session_id),
                }))
                .await
                .unwrap();

            assert!(!status.is_empty());
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
