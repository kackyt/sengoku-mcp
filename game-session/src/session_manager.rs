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
            let has_view_token = data.view_token.is_some();
            let ctx = Arc::new(
                GameContextFactory::create_from_data(data, self.master_data.clone()).await?,
            );
            ctx.touch().await;
            if !has_view_token {
                // 閲覧トークン導入前に保存されたセッションには、ここで発行して保存する
                self.attach_new_view_token(&ctx, session_id).await?;
                self.persist(&ctx, session_id).await?;
            }
            ctx
        } else {
            // 3. 新規作成（Webアプリから閲覧できるよう、閲覧トークンも同時に発行する）
            let ctx = Arc::new(GameContextFactory::create_initial(self.master_data.clone()).await?);
            self.attach_new_view_token(&ctx, session_id).await?;
            self.persist(&ctx, session_id).await?;
            ctx
        };

        write_guard.insert(session_id.clone(), ctx.clone());
        Ok(ctx)
    }

    /// コンテキストの現在状態をストレージに保存します
    async fn persist(
        &self,
        ctx: &GameContext,
        session_id: &SessionId,
    ) -> Result<(), anyhow::Error> {
        let data = ctx.export_session_data(session_id).await?;
        self.persistence.save(&data).await?;
        Ok(())
    }

    /// 新しい閲覧トークンを発行して対応表に登録し、コンテキストに設定します（保存は呼び出し側で行う）
    async fn attach_new_view_token(
        &self,
        ctx: &GameContext,
        session_id: &SessionId,
    ) -> Result<ViewToken, anyhow::Error> {
        let token = ViewToken::generate();
        // 対応表を先に保存し、セッション側から参照される時点で必ず逆引きできるようにする
        self.view_tokens.register(&token, session_id).await?;
        *ctx.view_token.lock().await = Some(token.clone());
        Ok(token)
    }

    /// セッションの現在状態をストレージに保存します
    pub async fn save_session(&self, session_id: &SessionId) -> Result<(), anyhow::Error> {
        let ctx = {
            let guard = self.sessions.read().await;
            guard.get(session_id).cloned()
        };

        if let Some(ctx) = ctx {
            self.persist(&ctx, session_id).await?;
        }
        Ok(())
    }

    /// セッションの閲覧トークンを取得します。`regenerate` 指定時は再発行します。
    ///
    /// 閲覧トークンはセッション作成時に自動発行されます。再発行した場合、以前のトークンは失効します。
    pub async fn issue_view_token(
        &self,
        session_id: &SessionId,
        regenerate: bool,
    ) -> Result<ViewToken, anyhow::Error> {
        let ctx = self.get_or_create(session_id).await?;
        let current = ctx.view_token.lock().await.clone();
        match (current, regenerate) {
            (Some(token), false) => Ok(token),
            (old_token, _) => {
                // 1. 新トークンを登録・保存してから、2. 旧トークンを失効させる
                let new_token = self.attach_new_view_token(&ctx, session_id).await?;
                self.persist(&ctx, session_id).await?;
                if let Some(old) = old_token {
                    self.view_tokens.revoke(&old).await?;
                }
                Ok(new_token)
            }
        }
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
