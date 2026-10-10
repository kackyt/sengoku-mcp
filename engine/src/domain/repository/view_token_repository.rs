use crate::domain::error::DomainError;
use crate::domain::model::value_objects::{SessionId, ViewToken};

/// 閲覧トークンからセッションIDを引くための対応表を永続化するリポジトリのインターフェース
///
/// セッション側は `SessionData::view_token` で自身のトークンを保持し、
/// 本リポジトリはその逆引き（トークン → セッションID）を提供します。
#[async_trait::async_trait]
#[allow(
    clippy::double_must_use,
    reason = "async_trait が Future と重複する must_use 属性を自動生成するため"
)]
pub trait ViewTokenRepository: Send + Sync {
    /// トークンとセッションIDの対応を保存します
    async fn register(&self, token: &ViewToken, session_id: &SessionId) -> Result<(), DomainError>;

    /// トークンに対応するセッションIDを取得します
    async fn find_session_id(&self, token: &ViewToken) -> Result<Option<SessionId>, DomainError>;

    /// トークンを削除（失効）します。存在した場合は `true` を返します。
    async fn revoke(&self, token: &ViewToken) -> Result<bool, DomainError>;
}
