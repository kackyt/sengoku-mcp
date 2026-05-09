use crate::domain::error::DomainError;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::KuniId;
use crate::domain::repository::{kuni_repository::KuniRepository, neighbor_repository::NeighborRepository};

/// 国に関連するドメインロジックを提供するサービス
pub struct KuniService;

impl KuniService {
    /// 指定した国の隣接する国（Kuniオブジェクト）の一覧を取得します
    pub async fn get_neighbor_kunis(
        kuni_id: &KuniId,
        neighbor_repo: &dyn NeighborRepository,
        kuni_repo: &dyn KuniRepository,
    ) -> Result<Vec<Kuni>, DomainError> {
        let neighbor_ids = neighbor_repo.get_neighbors(kuni_id);
        let mut neighbors = Vec::with_capacity(neighbor_ids.len());
        for nid in neighbor_ids {
            if let Some(kuni) = kuni_repo.find_by_id(&nid).await? {
                neighbors.push(kuni);
            }
        }
        Ok(neighbors)
    }
}
