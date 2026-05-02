use crate::domain::{
    model::value_objects::TurnNumber, repository::kuni_repository::KuniRepository,
    service::turn_service::TurnService,
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
        let mut kunis = self.kuni_repo.find_all().await?;
        if kunis.is_empty() {
            return Ok(());
        }

        let turn = TurnNumber::new(current_turn);

        // ターン終了時の季節イベント（人口増加・資源生成）を処理
        TurnService::process_end_turn_events(turn, &mut kunis);
        for kuni in &kunis {
            self.kuni_repo.save(kuni).await?;
        }

        // ターン開始時の季節イベント（洪水・疫病・反乱）を処理
        TurnService::process_start_turn_events(TurnNumber::new(current_turn + 1), &mut kunis);
        for kuni in &kunis {
            self.kuni_repo.save(kuni).await?;
        }

        Ok(())
    }
}
