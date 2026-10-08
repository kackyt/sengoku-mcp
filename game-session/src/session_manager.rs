use crate::game_context::{GameContext, GameContextFactory};
use chrono::{Duration, Utc};
use engine::domain::model::value_objects::{SessionId, ViewToken};
use engine::domain::repository::session_repository::SessionRepository;
use engine::domain::repository::view_token_repository::ViewTokenRepository;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 複数セッションを管理するセッションマネージャー
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<GameContext>>>>,
    persistence: Arc<dyn SessionRepository>,
    view_tokens: Arc<dyn ViewTokenRepository>,
    master_data: Arc<MasterDataLoader>,
}

impl SessionManager {
    /// 新規セッションマネージャーを初期化します
    ///
    /// `storage` にはファイル保存・GCS保存など任意の保存先のリポジトリ群を注入できます。
    pub fn new(storage: SessionStorage, master_data: Arc<MasterDataLoader>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            persistence: storage.sessions,
            view_tokens: storage.view_tokens,
            master_data,
        }
    }

    /// セッションIDに対応する GameContext を取得または作成します
    /// 1. メモリ内に存在すればそれを返す
    /// 2. ストレージに存在すれば復元してキャッシュして返す
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

        // 2. ストレージからの復元確認
        let mut write_guard = self.sessions.write().await;
        if let Some(ctx) = write_guard.get(session_id) {
            ctx.touch().await;
            return Ok(ctx.clone());
        }

        let ctx = if let Some(data) = self.persistence.load(session_id).await? {
            let ctx = Arc::new(
                GameContextFactory::create_from_data(data, self.master_data.clone()).await?,
            );
            ctx.touch().await;
            ctx
        } else {
            // 3. 新規作成
            let ctx = Arc::new(GameContextFactory::create_initial(self.master_data.clone()).await?);
            let session_data = ctx.export_session_data(session_id).await?;
            self.persistence.save(&session_data).await?;
            ctx
        };

        write_guard.insert(session_id.clone(), ctx.clone());
        Ok(ctx)
    }

    /// セッションの現在状態をストレージに保存します
    pub async fn save_session(&self, session_id: &SessionId) -> Result<(), anyhow::Error> {
        let ctx = {
            let guard = self.sessions.read().await;
            guard.get(session_id).cloned()
        };

        if let Some(ctx) = ctx {
            let data = ctx.export_session_data(session_id).await?;
            self.persistence.save(&data).await?;
        }
        Ok(())
    }

    /// セッションの閲覧トークンを取得します。未発行または `regenerate` 指定時は新規発行します。
    ///
    /// 再発行した場合、以前のトークンは失効します。
    pub async fn issue_view_token(
        &self,
        session_id: &SessionId,
        regenerate: bool,
    ) -> Result<ViewToken, anyhow::Error> {
        let ctx = self.get_or_create(session_id).await?;
        // 同一セッションで同時に発行されないよう、発行処理中はトークンのロックを保持する
        let mut current = ctx.view_token.lock().await;
        if let (Some(token), false) = (current.as_ref(), regenerate) {
            return Ok(token.clone());
        }

        // 1. 新しいトークンの対応表を先に保存し、2. セッション側に反映してから、3. 旧トークンを失効させる
        let new_token = ViewToken::generate();
        self.view_tokens.register(&new_token, session_id).await?;
        let old_token = current.replace(new_token.clone());
        drop(current);
        self.save_session(session_id).await?;
        if let Some(old) = old_token {
            self.view_tokens.revoke(&old).await?;
        }
        Ok(new_token)
    }

    /// 期限切れのセッションをメモリおよびストレージから削除します
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

        // ストレージのクリーンアップ
        let file_deleted = self.persistence.cleanup_expired(ttl).await?;
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
