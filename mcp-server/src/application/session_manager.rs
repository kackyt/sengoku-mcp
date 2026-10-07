use crate::application::game_context::{GameContext, GameContextFactory};
use chrono::{Duration, Utc};
use engine::domain::model::value_objects::SessionId;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionPersistenceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 複数セッションを管理するセッションマネージャー
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<GameContext>>>>,
    persistence: Arc<SessionPersistenceManager>,
    master_data: Arc<MasterDataLoader>,
}

impl SessionManager {
    /// 新規セッションマネージャーを初期化します
    pub fn new(
        persistence: Arc<SessionPersistenceManager>,
        master_data: Arc<MasterDataLoader>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            persistence,
            master_data,
        }
    }

    /// セッションIDに対応する GameContext を取得または作成します
    /// 1. メモリ内に存在すればそれを返す
    /// 2. ファイルに存在すれば復元してキャッシュして返す
    /// 3. なければ新規初期化して保存・キャッシュして返す
    pub async fn get_or_create(
        &self,
        session_id: &SessionId,
    ) -> Result<Arc<GameContext>, anyhow::Error> {
        // 1. メモリ内キャッシュ確認
        {
            let guard = self.sessions.read().await;
            if let Some(ctx) = guard.get(session_id) {
                ctx.touch().await;
                return Ok(ctx.clone());
            }
        }

        // 2. ファイルからの復元確認
        let mut write_guard = self.sessions.write().await;
        if let Some(ctx) = write_guard.get(session_id) {
            ctx.touch().await;
            return Ok(ctx.clone());
        }

        let ctx = if let Some(data) = self.persistence.load(session_id)? {
            let ctx = Arc::new(
                GameContextFactory::create_from_data(data, self.master_data.clone()).await?,
            );
            ctx.touch().await;
            ctx
        } else {
            // 3. 新規作成
            let ctx = Arc::new(GameContextFactory::create_initial(self.master_data.clone()).await?);
            let session_data = ctx.export_session_data(session_id).await?;
            self.persistence.save(&session_data)?;
            ctx
        };

        write_guard.insert(session_id.clone(), ctx.clone());
        Ok(ctx)
    }

    /// セッションの現在状態をファイルに保存します
    pub async fn save_session(&self, session_id: &SessionId) -> Result<(), anyhow::Error> {
        let ctx = {
            let guard = self.sessions.read().await;
            guard.get(session_id).cloned()
        };

        if let Some(ctx) = ctx {
            let data = ctx.export_session_data(session_id).await?;
            self.persistence.save(&data)?;
        }
        Ok(())
    }

    /// 期限切れのセッションをメモリおよびファイルから削除します
    pub async fn cleanup_expired(&self, ttl: Duration) -> Result<usize, anyhow::Error> {
        let now = Utc::now();
        let mut expired_keys = Vec::new();

        // メモリ内チェック
        {
            let guard = self.sessions.read().await;
            for (id, ctx) in guard.iter() {
                let last_access = *ctx.last_accessed_at.lock().await;
                if (now - last_access) > ttl {
                    expired_keys.push(id.clone());
                }
            }
        }

        if !expired_keys.is_empty() {
            let mut write_guard = self.sessions.write().await;
            for key in &expired_keys {
                write_guard.remove(key);
            }
        }

        // ファイルクリーンアップ
        let file_deleted = self.persistence.cleanup_expired(ttl)?;
        Ok(file_deleted.max(expired_keys.len()))
    }

    /// バックグラウンドで定期的にクリーンアップを実行するタスクを開始します
    pub fn start_cleanup_task(self: Arc<Self>, interval: std::time::Duration, ttl: Duration) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                if let Err(e) = self.cleanup_expired(ttl).await {
                    eprintln!("[SessionManager] Cleanup error: {:?}", e);
                }
            }
        });
    }
}
