use crate::domain::{
    error::DomainError,
    model::action_log::{
        ActionLogEntry, ActionLogEvent, ActionLogVisibility, DomesticLogEvent, WarLogEvent,
    },
    model::battle::{BattleAdvantage, BattleSide, Tactic, WarStatus},
    model::value_objects::{Amount, DaimyoId, DisplayAmount, KuniId},
    repository::action_log_repository::ActionLogRepository,
    repository::battle_repository::BattleRepository,
    repository::daimyo_repository::DaimyoRepository,
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
    daimyo_repo: Arc<dyn DaimyoRepository>,
    turn_progression_usecase:
        Arc<crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase>,
}

impl BattleUseCase {
    /// 新しい合戦ユースケースを作成します
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
        battle_repo: Arc<dyn BattleRepository>,
        action_log_repo: Arc<dyn ActionLogRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
        daimyo_repo: Arc<dyn DaimyoRepository>,
        turn_progression_usecase: Arc<
            crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase,
        >,
    ) -> Self {
        Self {
            kuni_repo,
            neighbor_repo,
            battle_repo,
            action_log_repo,
            game_state_repo,
            daimyo_repo,
            turn_progression_usecase,
        }
    }

    async fn validate_battle_turn(&self, kuni_id: KuniId) -> Result<(), anyhow::Error> {
        let state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;

        if state.phase() != crate::domain::model::game_state::GamePhase::Battle {
            return Err(anyhow::anyhow!("現在は合戦フェーズではありません"));
        }

        // 現在の手番（攻撃側）
        let current_kuni_id = state
            .current_kuni_id()
            .ok_or_else(|| anyhow::anyhow!("現在行動可能な国がありません"))?;

        // 攻撃側本人の場合
        if current_kuni_id == kuni_id {
            return Ok(());
        }

        // 防衛側の場合：現在の手番（攻撃側）が自分を攻撃しているか確認
        let status = self
            .battle_repo
            .find_by_attacker(&current_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("現在進行中の合戦が見つかりません"))?;

        if status.defender.kuni_id == kuni_id {
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "あなたの手番（または防衛対象）ではありません"
        ))
    }

    async fn validate_domestic_turn(&self, kuni_id: KuniId) -> Result<(), anyhow::Error> {
        let state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;

        if state.phase() != crate::domain::model::game_state::GamePhase::Domestic {
            return Err(anyhow::anyhow!("現在は内政フェーズではありません"));
        }

        state.check_turn(kuni_id)?;
        Ok(())
    }

    async fn advance_turn(&self, player_daimyo_id: Option<DaimyoId>) -> Result<(), anyhow::Error> {
        self.turn_progression_usecase
            .complete_current_action(player_daimyo_id)
            .await?;
        Ok(())
    }

    /// 合戦の1ターンを実行します
    pub async fn execute_battle_turn(
        &self,
        player_daimyo_id: Option<DaimyoId>,
        attacker_id: KuniId,
        attacker_tactic: Tactic,
    ) -> Result<WarStatus, anyhow::Error> {
        self.validate_battle_turn(attacker_id).await?;

        let status = self
            .battle_repo
            .find_by_attacker(&attacker_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("進行中の合戦が見つかりません"))?;

        let defender_kuni = self
            .kuni_repo
            .find_by_id(&status.defender.kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;

        let defender_daimyo = self
            .daimyo_repo
            .find_by_id(&defender_kuni.daimyo_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("防御側の大名が見つかりません"))?;

        let defender_tactic = {
            let mut rng = rand::thread_rng();
            BattleService::decide_tactic_for_defender(
                &status.defender,
                &status.attacker,
                defender_daimyo.personality.military_bias(),
                &mut rng,
            )
        };

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
                    status.attacker.hei.value() - next_status.attacker.hei.value(),
                ),
                defender_damage: Amount::new(
                    status.defender.hei.value() - next_status.defender.hei.value(),
                ),
            }),
        ))?;

        if let Some(winner) = next_status.winner {
            self.process_battle_result(player_daimyo_id, next_status.clone(), winner, turn)
                .await?;
        } else {
            self.battle_repo.save(&next_status).await?;
        }

        Ok(next_status)
    }

    /// プレイヤーが防御側の合戦ターンを実行します
    pub async fn execute_defense_turn(
        &self,
        player_daimyo_id: Option<DaimyoId>,
        defender_id: KuniId,
        defender_tactic: Tactic,
    ) -> Result<WarStatus, anyhow::Error> {
        self.validate_battle_turn(defender_id).await?;

        let status = self
            .battle_repo
            .find_by_defender(&defender_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("進行中の防御戦が見つかりません"))?;

        let attacker_kuni = self
            .kuni_repo
            .find_by_id(&status.attacker.kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("攻撃側の国が見つかりません"))?;

        let attacker_daimyo = self
            .daimyo_repo
            .find_by_id(&attacker_kuni.daimyo_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("攻撃側の大名が見つかりません"))?;

        let attacker_tactic = {
            let mut rng = rand::thread_rng();
            BattleService::decide_tactic_for_attacker(
                &status.attacker,
                &status.defender,
                attacker_daimyo.personality.military_bias(),
                &mut rng,
            )
        };

        let next_status =
            BattleService::calculate_turn(status.clone(), attacker_tactic, defender_tactic)?;

        // ログ記録と事後処理 (execute_battle_turn と同様のロジック)
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
                    status
                        .attacker
                        .hei
                        .value()
                        .saturating_sub(next_status.attacker.hei.value()),
                ),
                defender_damage: Amount::new(
                    status
                        .defender
                        .hei
                        .value()
                        .saturating_sub(next_status.defender.hei.value()),
                ),
            }),
        ))?;

        if let Some(winner) = next_status.winner {
            self.process_battle_result(player_daimyo_id, next_status.clone(), winner, turn)
                .await?;
        } else {
            self.battle_repo.save(&next_status).await?;
        }

        Ok(next_status)
    }

    /// 戦合戦結果の共通処理
    async fn process_battle_result(
        &self,
        player_daimyo_id: Option<DaimyoId>,
        status: WarStatus,
        winner: BattleSide,
        turn: crate::domain::model::value_objects::TurnNumber,
    ) -> Result<(), anyhow::Error> {
        match winner {
            BattleSide::Attacker => {
                let mut occupied = self
                    .kuni_repo
                    .find_by_id(&status.defender_id())
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;
                let home = self
                    .kuni_repo
                    .find_by_id(&status.attacker_id())
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("本国が見つかりません"))?;

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

                occupied.occupy(home.daimyo_id, &status.attacker);
                self.kuni_repo.save(&occupied).await?;
                self.battle_repo
                    .delete_by_attacker(&status.attacker_id())
                    .await?;
            }
            BattleSide::Defender => {
                let mut defender = self
                    .kuni_repo
                    .find_by_id(&status.defender_id())
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("防御側の国が見つかりません"))?;
                let attacker_kuni = self
                    .kuni_repo
                    .find_by_id(&status.attacker_id())
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("攻撃側の国が見つかりません"))?;

                self.action_log_repo.save(ActionLogEntry::new(
                    ActionLogVisibility::Public,
                    turn,
                    ActionLogEvent::War(WarLogEvent::DefenderVictory {
                        home_name: attacker_kuni.name.clone(),
                        attacker_id: attacker_kuni.daimyo_id,
                        defender_id: defender.daimyo_id,
                    }),
                ))?;

                defender.survive_defense(&status.defender);
                self.kuni_repo.save(&defender).await?;
                self.battle_repo
                    .delete_by_attacker(&status.attacker_id())
                    .await?;
            }
        }

        // 合戦終了：complete_current_action がフェーズ遷移を管理する
        self.advance_turn(player_daimyo_id).await?;

        Ok(())
    }

    /// 合戦を開始します
    pub async fn start_war(
        &self,
        _player_daimyo_id: Option<DaimyoId>,
        attacker_id: KuniId,
        defender_id: KuniId,
        hei: DisplayAmount,
        kome: DisplayAmount,
    ) -> Result<WarStatus, anyhow::Error> {
        self.validate_domestic_turn(attacker_id).await?;

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

        // ゲーム状態を合戦フェーズに移行
        let mut state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;
        state.start_war(attacker_id, defender_id)?;
        state.mark_action_performed();
        self.game_state_repo.save(&state).await?;

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
