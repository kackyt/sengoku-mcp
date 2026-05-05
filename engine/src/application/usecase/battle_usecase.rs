use crate::domain::{
    error::DomainError,
    model::action_log::{
        ActionLogEntry, ActionLogEvent, ActionLogVisibility, DomesticLogEvent, WarLogEvent,
    },
    model::battle::{BattleAdvantage, BattleSide, Tactic, WarStatus},
    model::value_objects::{Amount, DisplayAmount, KuniId},
    repository::action_log_repository::ActionLogRepository,
    repository::battle_repository::BattleRepository,
    repository::game_state_repository::GameStateRepository,
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
    action_log_repo: Arc<dyn ActionLogRepository>,
    game_state_repo: Arc<dyn GameStateRepository>,
}

impl BattleUseCase {
    /// 新しい合戦ユースケースを作成します
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
        battle_repo: Arc<dyn BattleRepository>,
        action_log_repo: Arc<dyn ActionLogRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            neighbor_repo,
            battle_repo,
            action_log_repo,
            game_state_repo,
        }
    }

    /// 合戦の1ターンを実行します
    pub async fn execute_battle_turn(
        &self,
        status: WarStatus,
        attacker_tactic: Tactic,
    ) -> Result<WarStatus, anyhow::Error> {
        let defender_tactic = BattleService::decide_tactic();

        let next_status =
            BattleService::calculate_turn(status.clone(), attacker_tactic, defender_tactic)?;

        // ターン経過（ダメージ）の記録
        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Public,
            turn,
            ActionLogEvent::War(WarLogEvent::Damage {
                attacker_tactic,
                defender_tactic,
                attacker_damage: Amount::new(
                    status.defender.hei.value() - next_status.defender.hei.value(),
                ),
                defender_damage: Amount::new(
                    status.attacker.hei.value() - next_status.attacker.hei.value(),
                ),
            }),
        ))?;

        // 戦争決着時の処理
        if let Some(winner) = next_status.winner {
            match winner {
                BattleSide::Attacker => {
                    // 攻撃側勝利：占領処理
                    let mut occupied = self
                        .kuni_repo
                        .find_by_id(&next_status.defender_id())
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;

                    let home = self
                        .kuni_repo
                        .find_by_id(&next_status.attacker_id())
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("本国が見つかりません"))?;

                    // 勝利ログの記録
                    self.action_log_repo.save(ActionLogEntry::new(
                        ActionLogVisibility::Public,
                        turn,
                        ActionLogEvent::War(WarLogEvent::AttackerVictory {
                            home_name: home.name.clone(),
                            attacker_id: home.daimyo_id,
                            occupied_name: occupied.name.clone(),
                            defender_id: occupied.daimyo_id,
                        }),
                    ))?;

                    // 内政ログにも記録
                    self.action_log_repo.save(ActionLogEntry::new(
                        ActionLogVisibility::Public,
                        turn,
                        ActionLogEvent::Domestic(DomesticLogEvent::WarAttackerOccupied {
                            home_name: home.name.clone(),
                            occupied_name: occupied.name.clone(),
                        }),
                    ))?;

                    // 占領処理
                    occupied.occupy(home.daimyo_id, &next_status.attacker);
                    self.kuni_repo.save(&occupied).await?;

                    self.battle_repo
                        .delete_by_attacker(&next_status.attacker_id())
                        .await?;
                }
                BattleSide::Defender => {
                    // 防御側勝利
                    let mut defender = self
                        .kuni_repo
                        .find_by_id(&next_status.defender_id())
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;

                    let attacker_kuni = self
                        .kuni_repo
                        .find_by_id(&next_status.attacker_id())
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("攻撃側の国が見つかりません"))?;

                    // 敗北ログの記録
                    self.action_log_repo.save(ActionLogEntry::new(
                        ActionLogVisibility::Public,
                        turn,
                        ActionLogEvent::War(WarLogEvent::DefenderVictory {
                            home_name: attacker_kuni.name.clone(),
                            attacker_id: attacker_kuni.daimyo_id,
                            defender_id: defender.daimyo_id,
                        }),
                    ))?;

                    // 内政ログにも記録
                    self.action_log_repo.save(ActionLogEntry::new(
                        ActionLogVisibility::Public,
                        turn,
                        ActionLogEvent::Domestic(DomesticLogEvent::WarDefenderDefended {
                            defender_name: defender.name.clone(),
                        }),
                    ))?;

                    defender.survive_defense(&next_status.defender);
                    self.kuni_repo.save(&defender).await?;

                    self.battle_repo
                        .delete_by_attacker(&next_status.attacker_id())
                        .await?;
                }
            }
        } else {
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

        let defender = self
            .kuni_repo
            .find_by_id(&defender_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません: {:?}", defender_id))?;

        if attacker.daimyo_id == defender.daimyo_id {
            return Err(anyhow::anyhow!("自領には攻め込めません"));
        }

        let hei_internal = hei.to_internal();
        let kome_internal = kome.to_internal();

        // 開戦ログの記録
        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));

        // 過去の合戦ログをクリア
        self.action_log_repo
            .clear(crate::domain::model::action_log::ActionLogCategory::War)?;

        self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Public,
            turn,
            ActionLogEvent::War(WarLogEvent::WarStarted {
                attacker_name: attacker.name.clone(),
                defender_name: defender.name.clone(),
                attacker_id: attacker.daimyo_id,
                defender_id: defender.daimyo_id,
            }),
        ))?;

        // 内政ログにも記録
        self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Public,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::WarStarted {
                attacker_name: attacker.name.clone(),
                defender_name: defender.name.clone(),
            }),
        ))?;

        // 出陣処理
        let attacker_army = attacker.dispatch_army(hei_internal, kome_internal)?;
        self.kuni_repo.save(&attacker).await?;

        // 防御側の軍勢ステータス作成
        let defender_army = crate::domain::model::battle::ArmyStatus {
            kuni_id: defender_id,
            hei: defender.resource.hei,
            kome: defender.resource.kome,
            morale: defender.stats.tyu,
        };

        let status = WarStatus {
            attacker: attacker_army,
            defender: defender_army,
            winner: None,
            advantage: BattleAdvantage::Even,
        };

        self.battle_repo.save(&status).await?;

        Ok(status)
    }

    /// 進行中の合戦情報を取得します
    pub async fn get_active_war(
        &self,
        attacker_id: KuniId,
    ) -> Result<Option<WarStatus>, anyhow::Error> {
        Ok(self.battle_repo.find_by_attacker(&attacker_id).await?)
    }
}
