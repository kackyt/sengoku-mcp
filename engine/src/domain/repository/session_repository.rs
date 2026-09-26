use crate::domain::error::DomainError;
use crate::domain::model::session::SessionData;
use crate::domain::model::value_objects::SessionId;
use chrono::Duration;

/// セッションの永続化およびライフサイクル管理を行うリポジトリのインターフェース
#[async_trait::async_trait]
pub trait SessionRepository: Send + Sync {
    /// セッションデータを保存します
    async fn save(&self, data: &SessionData) -> Result<(), DomainError>;

    /// セッションデータをロードします
    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionData>, DomainError>;

    /// セッションデータを削除します
    async fn delete(&self, session_id: &SessionId) -> Result<bool, DomainError>;

    /// 指定期間アクセスのない期限切れセッションを削除します
    async fn cleanup_expired(&self, ttl: Duration) -> Result<usize, DomainError>;
}
