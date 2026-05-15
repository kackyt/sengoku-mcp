use crate::domain::error::DomainError;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{DaimyoId, KuniId};
use crate::domain::repository::kuni_repository::KuniRepository;
use crate::domain::repository::neighbor_repository::NeighborRepository;
use async_trait::async_trait;
use std::collections::HashMap;

/// シミュレーション用の読み取り専用国リポジトリ
pub struct SimulationKuniRepository<'a> {
    pub kunis: &'a [Kuni],
}

#[async_trait]
impl<'a> KuniRepository for SimulationKuniRepository<'a> {
    async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError> {
        Ok(self.kunis.iter().find(|k| k.id == *id).cloned())
    }
    async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, DomainError> {
        Ok(self
            .kunis
            .iter()
            .filter(|k| k.daimyo_id == *daimyo_id)
            .cloned()
            .collect())
    }
    async fn save(&self, _kuni: &Kuni) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_all(&self) -> Result<Vec<Kuni>, DomainError> {
        Ok(self.kunis.to_vec())
    }
    async fn clear(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

/// シミュレーション用の隣接情報リポジトリ
pub struct SimulationNeighborRepository<'a> {
    pub neighbors: &'a HashMap<KuniId, Vec<KuniId>>,
}

impl<'a> NeighborRepository for SimulationNeighborRepository<'a> {
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.neighbors.get(kuni_id).cloned().unwrap_or_default()
    }
    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool {
        self.neighbors.get(a).is_some_and(|l| l.contains(b))
    }
    fn reset(&self, _adjacency_map: HashMap<KuniId, Vec<KuniId>>) -> Result<(), DomainError> {
        Ok(())
    }
}
