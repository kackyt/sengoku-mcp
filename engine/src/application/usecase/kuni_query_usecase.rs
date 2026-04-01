use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{DaimyoId, KuniId};
use crate::domain::repository::kuni_repository::KuniRepository;
use crate::domain::repository::neighbor_repository::NeighborRepository;
use std::sync::Arc;

/// 国の情報照会に関するユースケース
pub struct KuniQueryUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
}

impl KuniQueryUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            neighbor_repo,
        }
    }

    /// 指定した大名が支配する国の一覧を取得します
    pub async fn get_kunis_by_daimyo(&self, daimyo_id: &DaimyoId) -> anyhow::Result<Vec<Kuni>> {
        self.kuni_repo
            .find_by_daimyo_id(daimyo_id)
            .await
            .map_err(|e| e.into())
    }

    /// 指定した国の隣接国を取得します
    pub async fn get_neighbors(&self, kuni_id: &KuniId) -> anyhow::Result<Vec<Kuni>> {
        let neighbor_ids = self.neighbor_repo.get_neighbors(kuni_id);
        let mut neighbors = Vec::new();
        for id in neighbor_ids {
            if let Some(kuni) = self.kuni_repo.find_by_id(&id).await? {
                neighbors.push(kuni);
            }
        }
        Ok(neighbors)
    }

    /// 指定した国の隣接国のID一覧を取得します
    pub fn get_neighbor_ids(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.neighbor_repo.get_neighbors(kuni_id)
    }
}
