use crate::domain::error::DomainError;
use crate::domain::model::{daimyo::Daimyo, value_objects::DaimyoId};

/// 大名情報を管理するリポジトリのインターフェース
#[async_trait::async_trait]
pub trait DaimyoRepository: Send + Sync {
    /// IDで大名を検索します
    async fn find_by_id(&self, id: &DaimyoId) -> Result<Option<Daimyo>, DomainError>;
    /// 大名情報を保存または更新します
    async fn save(&self, daimyo: &Daimyo) -> Result<(), DomainError>;
    /// すべての大名を取得します
    async fn find_all(&self) -> Result<Vec<Daimyo>, DomainError>;
}
