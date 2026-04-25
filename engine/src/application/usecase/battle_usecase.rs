use crate::domain::{
    error::DomainError,
    model::value_objects::{Amount, DisplayAmount, KuniId},
    repository::kuni_repository::KuniRepository,
    repository::neighbor_repository::NeighborRepository,
    service::battle_service::{BattleService, Tactic},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::service::battle_service::BattleSide;

/// 合戦中の軍勢ステータス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarStatus {
    pub attacker_id: KuniId,
    pub defender_id: KuniId,
    pub hei: DisplayAmount,
    pub kome: DisplayAmount,
    pub morale: u32,
    pub winner: Option<BattleSide>,
}

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
        status: WarStatus,
        attacker_tactic: Tactic,
        defender_tactic: Tactic,
    ) -> Result<WarStatus, anyhow::Error> {
        let mut attacker_proxy = self
            .kuni_repo
            .find_by_id(&status.attacker_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("攻撃側の国が見つかりません: {:?}", status.attacker_id)
            })?;

        // 軍勢の状態を一時的にKuniに反映（計算用）
        attacker_proxy.resource.hei = status.hei.to_internal();
        attacker_proxy.resource.kome = status.kome.to_internal();
        attacker_proxy.stats.tyu = crate::domain::model::value_objects::Rate::new(status.morale);

        let defender = self
            .kuni_repo
            .find_by_id(&status.defender_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("防御側の国が見つかりません: {:?}", status.defender_id)
            })?;

        let result = BattleService::calculate_turn(
            attacker_proxy,
            defender,
            attacker_tactic,
            defender_tactic,
            status.hei.to_internal().value(),
        )?;

        let next_status = WarStatus {
            attacker_id: status.attacker_id,
            defender_id: status.defender_id,
            hei: result.attacker_kuni.resource.hei.to_display(),
            kome: result.attacker_kuni.resource.kome.to_display(),
            morale: result.attacker_kuni.stats.tyu.value(),
            winner: result.winner,
        };

        // 戦争決着時の処理
        if let Some(winner) = result.winner {
            match winner {
                BattleSide::Attacker => {
                    // 攻撃側勝利：占領処理
                    let mut occupied = result.defender_kuni;
                    // 本国の支配者を確認
                    let home = self
                        .kuni_repo
                        .find_by_id(&status.attacker_id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("本国が見つかりません"))?;

                    occupied.set_daimyo_id(home.daimyo_id);
                    // 占領地に軍勢を配置（統合）
                    occupied.resource.hei = result.attacker_kuni.resource.hei;
                    occupied.resource.kome = result.attacker_kuni.resource.kome;
                    // 忠誠度は一旦低めに設定
                    occupied.modify_tyu(-50);

                    self.kuni_repo.save(&occupied).await?;
                }
                BattleSide::Defender => {
                    // 防御側勝利：領土防衛成功
                    self.kuni_repo.save(&result.defender_kuni).await?;
                    // 攻撃軍は全滅または退却したため、本国への還元はなし（簡易化）
                }
            }
        } else {
            // 継続中：防御側の消耗のみ保存
            self.kuni_repo.save(&result.defender_kuni).await?;
        }

        Ok(next_status)
    }

    /// start_war の戻り値も修正
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

        if attacker.resource.hei.value() < hei_internal.value() {
            return Err(anyhow::anyhow!("兵数が不足しています"));
        }
        if attacker.resource.kome.value() < kome_internal.value() {
            return Err(anyhow::anyhow!("兵糧が不足しています"));
        }

        attacker.consume_resource(Amount::new(0), hei_internal, kome_internal)?;
        self.kuni_repo.save(&attacker).await?;

        Ok(WarStatus {
            attacker_id,
            defender_id,
            hei,
            kome,
            morale: attacker.stats.tyu.value(),
            winner: None,
        })
    }
}
