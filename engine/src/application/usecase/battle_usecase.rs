use crate::domain::{
    error::DomainError,
    model::battle::{BattleAdvantage, BattleSide, Tactic, WarStatus},
    model::value_objects::{Amount, DisplayAmount, KuniId},
    repository::battle_repository::BattleRepository,
    repository::kuni_repository::KuniRepository,
    repository::neighbor_repository::NeighborRepository,
    service::battle_service::BattleService,
};
use std::sync::Arc;

/// 合戦に関するユースケース
#[allow(dead_code)]
pub struct BattleUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
    battle_repo: Arc<dyn BattleRepository>,
}

impl BattleUseCase {
    /// 新しい合戦ユースケースを作成します
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
        battle_repo: Arc<dyn BattleRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            neighbor_repo,
            battle_repo,
        }
    }

    /// 合戦の1ターンを実行します
    pub async fn execute_battle_turn(
        &self,
        status: WarStatus,
        attacker_tactic: Tactic,
    ) -> Result<WarStatus, anyhow::Error> {
        let defender_tactic = BattleService::decide_tactic();

        let next_status = BattleService::calculate_turn(status, attacker_tactic, defender_tactic)?;

        // 戦争決着時の処理
        if let Some(winner) = next_status.winner {
            match winner {
                BattleSide::Attacker => {
                    // 攻撃側勝利：占領処理
                    let mut occupied = self
                        .kuni_repo
                        .find_by_id(&next_status.defender_id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;

                    // 本国の支配者を確認
                    let home = self
                        .kuni_repo
                        .find_by_id(&next_status.attacker_id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("本国が見つかりません"))?;

                    occupied.set_daimyo_id(home.daimyo_id);
                    // 占領地に軍勢を配置（統合）
                    occupied.resource.hei = next_status.attacker_hei;
                    occupied.resource.kome = next_status.attacker_kome;
                    // 忠誠度は一旦低めに設定
                    occupied.modify_tyu(-50);
                    self.kuni_repo.save(&occupied).await?;

                    // 合戦状態を削除
                    self.battle_repo.delete_by_attacker(&next_status.attacker_id).await?;
                }
                BattleSide::Defender => {
                    // 防御側勝利：領土防衛成功（防御側の損害を反映）
                    let mut defender = self
                        .kuni_repo
                        .find_by_id(&next_status.defender_id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;

                    defender.resource.hei = next_status.defender_hei;
                    defender.resource.kome = next_status.defender_kome;
                    defender.stats.tyu =
                        crate::domain::model::value_objects::Rate::new(next_status.defender_morale);
                    self.kuni_repo.save(&defender).await?;

                    // 合戦状態を削除
                    self.battle_repo.delete_by_attacker(&next_status.attacker_id).await?;
                }
            }
        } else {
            // 継続中：合戦状態のみを保存。KuniRepositoryには書き込まない。
            self.battle_repo.save(&next_status).await?;
        }

        Ok(next_status)
    }

    /// 合戦を開始します
    pub async fn start_war(
        &self,
        attacker_id: KuniId,
        defender_id: KuniId,
        hei: DisplayAmount,
        kome: DisplayAmount,
    ) -> Result<WarStatus, anyhow::Error> {
        let mut attacker = self
            .kuni_repo
            .find_by_id(&attacker_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("攻撃側の国が見つかりません: {:?}", attacker_id))?;

        if !self.neighbor_repo.are_adjacent(&attacker_id, &defender_id) {
            return Err(DomainError::NotAdjacent.into());
        }

        let hei_internal = hei.to_internal();
        let kome_internal = kome.to_internal();

        if attacker.resource.hei < hei_internal {
            return Err(anyhow::anyhow!("兵数が不足しています"));
        }
        if attacker.resource.kome < kome_internal {
            return Err(anyhow::anyhow!("兵糧が不足しています"));
        }

        attacker.consume_resource(Amount::zero(), hei_internal, kome_internal)?;
        self.kuni_repo.save(&attacker).await?;

        let defender = self
            .kuni_repo
            .find_by_id(&defender_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません: {:?}", defender_id))?;

        let status = WarStatus {
            attacker_id,
            defender_id,
            attacker_hei: hei_internal,
            attacker_kome: kome_internal,
            attacker_morale: attacker.stats.tyu.value(),
            defender_hei: defender.resource.hei,
            defender_kome: defender.resource.kome,
            defender_morale: defender.stats.tyu.value(),
            winner: None,
            advantage: BattleAdvantage::Even,
        };

        self.battle_repo.save(&status).await?;

        Ok(status)
    }

    /// 進行中の合戦情報を取得します
    pub async fn get_active_war(&self, attacker_id: KuniId) -> Result<Option<WarStatus>, anyhow::Error> {
        self.battle_repo.find_by_attacker(&attacker_id).await
    }
}
