use crate::domain::error::DomainError;
use crate::domain::model::join_ticket::{JoinCode, JoinTicket};
use chrono::{DateTime, Utc};

/// 参加チケット（参加コード → 参加待ちのゲーム）を永続化するリポジトリのインターフェース
#[async_trait::async_trait]
#[allow(
    clippy::double_must_use,
    reason = "async_trait が Future と重複する must_use 属性を自動生成するため"
)]
pub trait JoinTicketRepository: Send + Sync {
    /// チケットを保存します
    async fn save_ticket(&self, ticket: &JoinTicket) -> Result<(), DomainError>;

    /// 参加コードに対応するチケットを取得します
    async fn find_ticket(&self, code: &JoinCode) -> Result<Option<JoinTicket>, DomainError>;

    /// チケットを削除します。存在した場合は `true` を返します（使用済みにする際に使用）。
    async fn delete_ticket(&self, code: &JoinCode) -> Result<bool, DomainError>;

    /// 指定時刻時点で有効期限切れのチケットを削除し、削除件数を返します
    async fn cleanup_expired_tickets(&self, now: DateTime<Utc>) -> Result<usize, DomainError>;
}
