use crate::domain::error::DomainError;
use crate::domain::model::{
    kuni::Kuni,
    value_objects::{DaimyoId, KuniId},
};

/// 国情報を管理するリポジトリのインターフェース
#[async_trait::async_trait]
pub trait KuniRepository: Send + Sync {
    /// IDで国を検索します
    async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError>;
    /// 大名IDで支配下の国を検索します
    async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, DomainError>;
    /// 国情報を保存または更新します
    async fn save(&self, kuni: &Kuni) -> Result<(), DomainError>;
    /// すべての国を取得します
    async fn find_all(&self) -> Result<Vec<Kuni>, DomainError>;
}
