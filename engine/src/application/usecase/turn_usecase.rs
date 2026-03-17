use crate::domain::{
    repository::kuni_repository::KuniRepository, service::turn_service::TurnService,
};
use std::sync::Arc;

#[allow(dead_code)]
pub struct TurnUseCase<R: KuniRepository> {
    kuni_repo: Arc<R>,
}

impl<R: KuniRepository> TurnUseCase<R> {
    pub fn new(kuni_repo: Arc<R>) -> Self {
        Self { kuni_repo }
    }

    #[allow(dead_code)]
    async fn progress_turn(&self, current_turn: u32) -> Result<(), anyhow::Error> {
        let kunis = self.kuni_repo.find_all().await?;
        if kunis.is_empty() {
            return Ok(());
        }

        let mut rng = rand::thread_rng();
        let updated_kunis = TurnService::process_season(current_turn, kunis, &mut rng);

        for kuni in updated_kunis {
            self.kuni_repo.save(&kuni).await?;
        }

        Ok(())
    }
}
