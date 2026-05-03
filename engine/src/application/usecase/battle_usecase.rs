use crate::domain::{
    error::DomainError,
    model::battle::{BattleAdvantage, BattleSide, Tactic, WarStatus},
    model::value_objects::{DisplayAmount, KuniId},
    repository::battle_repository::BattleRepository,
    repository::kuni_repository::KuniRepository,
    repository::neighbor_repository::NeighborRepository,
    repository::action_log_repository::ActionLogRepository,
    repository::game_state_repository::GameStateRepository,
    service::battle_service::BattleService,
    model::action_log::{ActionLogCategory, ActionLogEntry, ActionLogVisibility},
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

        let turn = self.game_state_repo.get().await?.map(|s| s.current_turn()).unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::War,
            ActionLogVisibility::Internal,
            turn,
            "".to_string(),
            format!("CPU Defender Tactic: {:?}", defender_tactic),
        ));

        let pre_attacker_hei = status.attacker.hei;
        let pre_defender_hei = status.defender.hei;
        
        let next_status = BattleService::calculate_turn(status, attacker_tactic, defender_tactic)?;

        let attacker_damage = pre_attacker_hei.value().saturating_sub(next_status.attacker.hei.value());
        let defender_damage = pre_defender_hei.value().saturating_sub(next_status.defender.hei.value());

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::War,
            ActionLogVisibility::Player,
            turn,
            format!("自軍の被害: {}、敵軍の被害: {}", attacker_damage, defender_damage),
            format!("attacker_dmg={}, defender_dmg={}, attacker_tactic={:?}, defender_tactic={:?}", attacker_damage, defender_damage, attacker_tactic, defender_tactic),
        ));

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

                    // 本国の支配者を確認
                    let home = self
                        .kuni_repo
                        .find_by_id(&next_status.attacker_id())
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("本国が見つかりません"))?;

                    // 占領処理（合算とステータス更新をドメインモデルに委譲）
                    occupied.occupy(home.daimyo_id, &next_status.attacker);
                    self.kuni_repo.save(&occupied).await?;

                    // 合戦状態を削除
                    self.battle_repo
                        .delete_by_attacker(&next_status.attacker_id())
                        .await?;

                    let _ = self.action_log_repo.save(ActionLogEntry::new(
                        ActionLogCategory::War,
                        ActionLogVisibility::Public,
                        turn,
                        format!("合戦終了：攻撃軍（{}から出陣）の勝利！領地を占領しました", home.name.0),
                        format!("Attacker {} conquered {}.", next_status.attacker_id().value(), next_status.defender_id().value()),
                    ));
                }
                BattleSide::Defender => {
                    // 防御側勝利：領土防衛成功
                    let mut defender = self
                        .kuni_repo
                        .find_by_id(&next_status.defender_id())
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;

                    defender.survive_defense(&next_status.defender);
                    self.kuni_repo.save(&defender).await?;

                    // 合戦状態を削除
                    self.battle_repo
                        .delete_by_attacker(&next_status.attacker_id())
                        .await?;

                    let _ = self.action_log_repo.save(ActionLogEntry::new(
                        ActionLogCategory::War,
                        ActionLogVisibility::Public,
                        turn,
                        format!("合戦終了：防衛軍（{}）の勝利", defender.name.0),
                        format!("Defender {} successfully defended against attacker.", next_status.defender_id().value()),
                    ));
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
        let _ = self.action_log_repo.clear(ActionLogCategory::War);
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

        // 出陣処理（兵力・兵糧の検証と消費）
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

        let turn = self.game_state_repo.get().await?.map(|s| s.current_turn()).unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::War,
            ActionLogVisibility::Public,
            turn,
            format!("{} が {} へ侵攻を開始しました", attacker.name.0, defender.name.0),
            format!("Attacker: {:?}, Defender: {:?}", attacker_id, defender_id),
        ));

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
