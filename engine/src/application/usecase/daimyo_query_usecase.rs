use crate::application::dto::daimyo_dto::DaimyoSummaryDto;
use crate::domain::model::value_objects::DaimyoId;
use crate::domain::repository::daimyo_repository::DaimyoRepository;
use std::sync::Arc;

/// 大名情報の照会に関するユースケース
pub struct DaimyoQueryUseCase {
    daimyo_repo: Arc<dyn DaimyoRepository + Send + Sync>,
}

impl DaimyoQueryUseCase {
    pub fn new(daimyo_repo: Arc<dyn DaimyoRepository + Send + Sync>) -> Self {
        Self { daimyo_repo }
    }

    /// 全ての大名を取得します
    pub async fn list(&self) -> anyhow::Result<Vec<DaimyoSummaryDto>> {
        let daimyos = self.daimyo_repo.find_all().await?;
        Ok(daimyos
            .into_iter()
            .map(|d| DaimyoSummaryDto {
                id: d.id.0,
                name: d.name.0.clone(),
            })
            .collect())
    }

    /// 指定したIDの大名を取得します
    pub async fn find(&self, id: DaimyoId) -> anyhow::Result<Option<DaimyoSummaryDto>> {
        let daimyo = self.daimyo_repo.find_by_id(&id).await?;
        Ok(daimyo.map(|d| DaimyoSummaryDto {
            id: d.id.0,
            name: d.name.0.clone(),
        }))
    }
}
