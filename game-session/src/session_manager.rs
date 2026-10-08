use crate::game_context::{GameContext, GameContextFactory};
use crate::game_lobby::{GameLobby, JoinGameError};
use chrono::{Duration, Utc};
use engine::domain::model::value_objects::{SessionId, ViewToken};
use engine::domain::repository::master_data_repository::MasterDataRepository;
use engine::domain::repository::session_repository::SessionRepository;
use engine::domain::repository::view_token_repository::ViewTokenRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 複数セッションを管理するセッションマネージャー
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<GameContext>>>>,
    persistence: Arc<dyn SessionRepository>,
    view_tokens: Arc<dyn ViewTokenRepository>,
    lobby: GameLobby,
    master_data: Arc<dyn MasterDataRepository>,
}

impl SessionManager {
    /// 新規セッションマネージャーを初期化します
    ///
    /// 依存はドメイン層のリポジトリ trait（ファイル保存・GCS保存など任意の実装）と、
    /// 参加コードを扱う GameLobby で受け取り、Composition Root（main.rs）で組み立てて注入する。
    pub fn new(
        persistence: Arc<dyn SessionRepository>,
        view_tokens: Arc<dyn ViewTokenRepository>,
        lobby: GameLobby,
        master_data: Arc<dyn MasterDataRepository>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            persistence,
            view_tokens,
            lobby,
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
        ctx.assign_new_view_token(self.view_tokens.as_ref(), session_id)
            .await
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

    /// Webで作成されたゲームに、参加コードを使ってチャットのセッションとして参加します
    ///
    /// 参加待ちのゲームを `session_id`（チャットのセッションID）へ移し、閲覧トークンの
    /// 向き先も付け替えます。ブラウザは作成時のURLのまま、このチャットのゲームを閲覧できます。
    /// チャット側で進行中のゲームがあった場合は新しいゲームで置き換え、旧閲覧URLは失効させます。
    pub async fn join_game(&self, code: &str, session_id: &SessionId) -> Result<(), JoinGameError> {
        // 1. 参加コードを消費し、参加待ちゲームを読み込む
        let pending_id = self.lobby.take_ticket(code).await?;
        let mut data = self
            .persistence
            .load(&pending_id)
            .await?
            .ok_or(JoinGameError::GameNotFound)?;
        data.session_id = session_id.clone();
        data.touch();
        let new_token = data.view_token.clone();

        // 2. キャッシュの差し替えと保存を、同一セッションへの他操作と競合しないよう書き込みロック中に行う
        let mut guard = self.sessions.write().await;
        let old_token = match guard.get(session_id) {
            Some(ctx) => ctx.view_token.lock().await.clone(),
            None => self
                .persistence
                .load(session_id)
                .await?
                .and_then(|d| d.view_token),
        };
        let ctx =
            Arc::new(GameContextFactory::create_from_data(data, self.master_data.clone()).await?);
        match &new_token {
            Some(token) => self.view_tokens.register(token, session_id).await?,
            None => {
                self.attach_new_view_token(&ctx, session_id).await?;
            }
        }
        self.persist(&ctx, session_id).await?;
        guard.insert(session_id.clone(), ctx);
        drop(guard);

        // 3. 移動元の参加待ちセッションと、置き換えられたゲームの閲覧トークンを削除する
        self.persistence.delete(&pending_id).await?;
        if let Some(old) = old_token.filter(|old| Some(old) != new_token.as_ref()) {
            self.view_tokens.revoke(&old).await?;
        }
        Ok(())
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

        // ストレージのクリーンアップ（期限切れの参加コードも併せて削除する）
        let file_deleted = self.persistence.cleanup_expired(ttl).await?;
        self.lobby.cleanup_expired_tickets().await?;
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
