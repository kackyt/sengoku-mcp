use crate::domain::{
    error::DomainError,
    model::value_objects::{DisplayAmount, KuniId},
    repository::kuni_repository::KuniRepository,
    repository::neighbor_repository::NeighborRepository,
    service::battle_service::{BattleResult, BattleService, Tactic},
};
use std::sync::Arc;

/// 合戦に関するユースケース
#[allow(dead_code)]
pub struct BattleUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
}

impl BattleUseCase {
    /// 新しい合戦ユースケースを作成します
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            neighbor_repo,
        }
    }

    /// 合戦の1ターンを実行します
    pub async fn execute_battle_turn(
        &self,
        attacker_id: KuniId,
        defender_id: KuniId,
        attacker_tactic: Tactic,
        defender_tactic: Tactic,
        attacker_troops: DisplayAmount,
    ) -> Result<BattleResult, anyhow::Error> {
        let attacker = self
            .kuni_repo
            .find_by_id(&attacker_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("攻撃側の国が見つかりません: {:?}", attacker_id))?;
        let defender = self
            .kuni_repo
            .find_by_id(&defender_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません: {:?}", defender_id))?;

        if !self.neighbor_repo.are_adjacent(&attacker_id, &defender_id) {
            return Err(DomainError::NotAdjacent.into());
        }

        let internal_troops = attacker_troops.to_internal();
        if internal_troops.value() > attacker.resource.hei.value() {
            return Err(anyhow::anyhow!("攻撃側の兵数が不足しています"));
        }

        let result = BattleService::calculate_turn(
            attacker,
            defender,
            attacker_tactic,
            defender_tactic,
            internal_troops.value(),
        )?;

        // 戦闘後の状態を保存
        self.kuni_repo.save(&result.attacker_kuni).await?;
        self.kuni_repo.save(&result.defender_kuni).await?;

        Ok(result)
    }
}
