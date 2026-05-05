use crate::domain::error::DomainError;
use crate::domain::model::action_log::{ActionLogCategory, ActionLogEntry};

/// アクションログのリポジトリ
pub trait ActionLogRepository: Send + Sync {
    /// ログエントリを保存する
    fn save(&self, entry: ActionLogEntry) -> Result<(), DomainError>;

    /// 表示用：Public と Player のログを最新 `limit` 件取得する
    fn find_visible(
        &self,
        category: ActionLogCategory,
        limit: usize,
    ) -> Result<Vec<ActionLogEntry>, DomainError>;

    /// デバッグ・記録用：全件（Internalを含む）を取得する
    fn find_all(&self, category: ActionLogCategory) -> Result<Vec<ActionLogEntry>, DomainError>;

    /// 指定されたカテゴリのログをすべてクリアする
    fn clear(&self, category: ActionLogCategory) -> Result<(), DomainError>;
}
