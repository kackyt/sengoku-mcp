use crate::domain::{
    model::value_objects::KuniId,
    repository::kuni_repository::KuniRepository,
    service::battle_service::{BattleResult, BattleService, Tactic},
};
use std::sync::Arc;

#[allow(dead_code)]
pub struct BattleUseCase<R: KuniRepository> {
    kuni_repo: Arc<R>,
}

impl<R: KuniRepository> BattleUseCase<R> {
    pub fn new(kuni_repo: Arc<R>) -> Self {
        Self { kuni_repo }
    }

    pub async fn execute_battle_turn(
        &self,
        attacker_id: KuniId,
        defender_id: KuniId,
        attacker_tactic: Tactic,
        defender_tactic: Tactic,
        attacker_troops: u32,
    ) -> Result<BattleResult, anyhow::Error> {
        let attacker = self
            .kuni_repo
            .find_by_id(&attacker_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attacker not found"))?;
        let defender = self
            .kuni_repo
            .find_by_id(&defender_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Defender not found"))?;

        if attacker_troops > attacker.resource.hei.value() {
            return Err(anyhow::anyhow!("Attacker does not have enough troops"));
        }

        let result = BattleService::calculate_turn(
            attacker,
            defender,
            attacker_tactic,
            defender_tactic,
            attacker_troops,
        )?;

        // Save updated states
        self.kuni_repo.save(&result.attacker_kuni).await?;
        self.kuni_repo.save(&result.defender_kuni).await?;

        Ok(result)
    }
}
