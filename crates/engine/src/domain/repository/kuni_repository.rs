use crate::domain::model::{
    kuni::Kuni,
    value_objects::{DaimyoId, KuniId},
};

#[async_trait::async_trait]
pub trait KuniRepository: Send + Sync {
    async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, anyhow::Error>;
    async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, anyhow::Error>;
    async fn save(&self, kuni: &Kuni) -> Result<(), anyhow::Error>;
    async fn find_all(&self) -> Result<Vec<Kuni>, anyhow::Error>;
}
