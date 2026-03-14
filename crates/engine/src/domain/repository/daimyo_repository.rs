use crate::domain::model::{daimyo::Daimyo, value_objects::DaimyoId};

#[async_trait::async_trait]
pub trait DaimyoRepository: Send + Sync {
    async fn find_by_id(&self, id: &DaimyoId) -> Result<Option<Daimyo>, anyhow::Error>;
    async fn save(&self, daimyo: &Daimyo) -> Result<(), anyhow::Error>;
    async fn find_all(&self) -> Result<Vec<Daimyo>, anyhow::Error>;
}
