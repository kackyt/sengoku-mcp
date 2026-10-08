use crate::game_context::GameContextFactory;
use chrono::{DateTime, Duration, Utc};
use engine::domain::error::DomainError;
use engine::domain::model::join_ticket::{JoinCode, JoinTicket};
use engine::domain::model::value_objects::{SessionId, ViewToken};
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::SessionStorage;
use std::sync::Arc;
use thiserror::Error;

/// 参加コードの有効期間（分）
pub const JOIN_CODE_TTL_MINUTES: i64 = 30;
/// Webで作成した参加待ちゲームのセッションIDの接頭辞
pub const PENDING_SESSION_PREFIX: &str = "web_";
/// 参加コードが既存のものと衝突した場合の再生成回数の上限
const MAX_CODE_ATTEMPTS: usize = 10;

/// ゲームへの参加処理で発生するエラー
#[derive(Debug, Error)]
pub enum JoinGameError {
    /// 参加コードの形式が不正
    #[error("参加コードの形式が正しくありません（例: KX7P2Q のような6文字）")]
    InvalidCode,
    /// 参加コードが存在しない、または使用済み
    #[error("参加コードが見つかりません。使用済みか、入力が間違っている可能性があります")]
    CodeNotFound,
    /// 参加コードの有効期限切れ
    #[error("参加コードの有効期限が切れています。ブラウザで新しいゲームを作成してください")]
    CodeExpired,
    /// 参加待ちのゲーム本体が見つからない（期限切れで削除済み等）
    #[error("参加先のゲームが見つかりません。ブラウザで新しいゲームを作成してください")]
    GameNotFound,
    /// ストレージ障害など
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<DomainError> for JoinGameError {
    fn from(e: DomainError) -> Self {
        Self::Internal(e.into())
    }
}

/// Webで作成したゲームの情報
#[derive(Debug, Clone)]
pub struct CreatedGame {
    /// ブラウザが状況を取得するための閲覧トークン
    pub view_token: ViewToken,
    /// チャット（LLM）側から参加するための参加コード
    pub join_code: JoinCode,
    /// 参加コードの有効期限
    pub join_code_expires_at: DateTime<Utc>,
}

/// Webでのゲーム作成と、参加コードの発行・消費を担当するサービス
///
/// ゲームは「参加待ち」のセッションとして保存され、チャット側が参加コードで参加すると
/// `SessionManager::join_game` によってチャットのセッションIDへ移されます。
/// 閲覧トークンは移動後も同じゲームを指し続けるため、ブラウザは同じURLを使い続けられます。
#[derive(Clone)]
pub struct GameLobby {
    storage: SessionStorage,
    master_data: Arc<MasterDataLoader>,
}

impl GameLobby {
    pub fn new(storage: SessionStorage, master_data: Arc<MasterDataLoader>) -> Self {
        Self {
            storage,
            master_data,
        }
    }

    /// 新しいゲームを参加待ち状態で作成し、閲覧トークンと参加コードを発行します
    pub async fn create_game(&self) -> Result<CreatedGame, anyhow::Error> {
        // 1. 参加待ちセッションとして初期状態のゲームを作成する
        let session_id = SessionId::new(format!(
            "{}{}",
            PENDING_SESSION_PREFIX,
            uuid::Uuid::new_v4().simple()
        ));
        let ctx = GameContextFactory::create_initial(self.master_data.clone()).await?;
        let view_token = ctx
            .assign_new_view_token(self.storage.view_tokens.as_ref(), &session_id)
            .await?;
        let data = ctx.export_session_data(&session_id).await?;
        self.storage.sessions.save(&data).await?;

        // 2. 参加コードを発行する（有効なコードと衝突した場合は再生成）
        let ticket = self.issue_ticket(session_id).await?;

        Ok(CreatedGame {
            view_token,
            join_code: ticket.code,
            join_code_expires_at: ticket.expires_at,
        })
    }

    /// 未使用の参加コードでチケットを発行して保存します
    async fn issue_ticket(&self, session_id: SessionId) -> Result<JoinTicket, anyhow::Error> {
        let now = Utc::now();
        for _ in 0..MAX_CODE_ATTEMPTS {
            let code = JoinCode::generate();
            let in_use = self
                .storage
                .join_tickets
                .find_ticket(&code)
                .await?
                .is_some_and(|t| !t.is_expired(now));
            if !in_use {
                let ticket =
                    JoinTicket::new(code, session_id, Duration::minutes(JOIN_CODE_TTL_MINUTES));
                self.storage.join_tickets.save_ticket(&ticket).await?;
                return Ok(ticket);
            }
        }
        Err(anyhow::anyhow!("参加コードの生成に失敗しました"))
    }

    /// 参加コードを検証して使用済みにし、参加待ちゲームのセッションIDを返します
    pub async fn take_ticket(&self, input: &str) -> Result<SessionId, JoinGameError> {
        let code = JoinCode::parse(input).ok_or(JoinGameError::InvalidCode)?;
        let ticket = self
            .storage
            .join_tickets
            .find_ticket(&code)
            .await?
            .ok_or(JoinGameError::CodeNotFound)?;

        // 期限切れのチケットはここで削除しておく
        if ticket.is_expired(Utc::now()) {
            self.storage.join_tickets.delete_ticket(&code).await?;
            return Err(JoinGameError::CodeExpired);
        }

        // 1回限り有効にするため、削除できた場合のみ参加を許可する
        if !self.storage.join_tickets.delete_ticket(&code).await? {
            return Err(JoinGameError::CodeNotFound);
        }
        Ok(ticket.session_id)
    }

    /// 期限切れの参加チケットを削除します
    pub async fn cleanup_expired_tickets(&self) -> Result<usize, anyhow::Error> {
        Ok(self
            .storage
            .join_tickets
            .cleanup_expired_tickets(Utc::now())
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use infrastructure::persistence::SessionPersistenceManager;
    use tempfile::tempdir;

    fn lobby(dir: &std::path::Path) -> (Arc<SessionPersistenceManager>, GameLobby) {
        let backend = Arc::new(SessionPersistenceManager::new(dir));
        let lobby = GameLobby::new(
            SessionStorage::from_backend(backend.clone()),
            Arc::new(MasterDataLoader),
        );
        (backend, lobby)
    }

    #[tokio::test]
    async fn test_create_game_and_take_ticket_once() {
        let dir = tempdir().unwrap();
        let (backend, lobby) = lobby(dir.path());

        let created = lobby.create_game().await.unwrap();

        // 閲覧トークンは参加待ちセッションを指し、セッション本体も保存されている
        let pending_id = backend
            .find_view_token(&created.view_token)
            .unwrap()
            .unwrap();
        assert!(pending_id.value().starts_with(PENDING_SESSION_PREFIX));
        let data = backend.load(&pending_id).unwrap().unwrap();
        assert_eq!(data.view_token, Some(created.view_token.clone()));
        assert!(data.selected_daimyo_id.is_none());

        // 小文字・ハイフン入りでも参加でき、2回目は使用済みになる
        let input = created.join_code.value().to_ascii_lowercase();
        let input = format!("{}-{}", &input[..3], &input[3..]);
        assert_eq!(lobby.take_ticket(&input).await.unwrap(), pending_id);
        assert!(matches!(
            lobby.take_ticket(&input).await,
            Err(JoinGameError::CodeNotFound)
        ));
    }

    #[tokio::test]
    async fn test_take_ticket_errors() {
        let dir = tempdir().unwrap();
        let (backend, lobby) = lobby(dir.path());

        assert!(matches!(
            lobby.take_ticket("??").await,
            Err(JoinGameError::InvalidCode)
        ));
        assert!(matches!(
            lobby.take_ticket("ABCDEF").await,
            Err(JoinGameError::CodeNotFound)
        ));

        // 期限切れのチケットは拒否され、削除される
        let mut expired = JoinTicket::new(
            JoinCode::parse("ABCDEF").unwrap(),
            SessionId::new("web_x"),
            Duration::minutes(1),
        );
        expired.expires_at = Utc::now() - Duration::minutes(1);
        backend.save_join_ticket(&expired).unwrap();
        assert!(matches!(
            lobby.take_ticket("ABCDEF").await,
            Err(JoinGameError::CodeExpired)
        ));
        assert!(backend.find_join_ticket(&expired.code).unwrap().is_none());
    }
}
