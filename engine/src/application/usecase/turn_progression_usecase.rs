use crate::domain::{
    model::{
        event::GameEvent,
        game_state::GameState,
        value_objects::{ActionOrderIndex, DaimyoId, EventMessage, TurnNumber},
    },
    repository::{
        event_dispatcher::EventDispatcher, game_state_repository::GameStateRepository,
        kuni_repository::KuniRepository,
    },
    service::{
        cpu_action_decision_service::{CpuActionDecision, CpuActionDecisionService},
        turn_service::TurnService,
    },
};
use std::sync::Arc;

pub struct TurnProgressionUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    game_state_repo: Arc<dyn GameStateRepository>,
    event_dispatcher: Arc<dyn EventDispatcher>,
}

impl TurnProgressionUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
        event_dispatcher: Arc<dyn EventDispatcher>,
    ) -> Self {
        Self {
            kuni_repo,
            game_state_repo,
            event_dispatcher,
        }
    }

    /// 現在のゲーム状態を取得します
    pub async fn get_state(&self) -> Result<Option<GameState>, anyhow::Error> {
        self.game_state_repo.get().await.map_err(|e| e.into())
    }

    /// 現在の行動を完了とし、次へ進める
    pub async fn complete_current_action(&self) -> Result<(), anyhow::Error> {
        let mut state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません。"))?;

        state.advance_action();

        if state.is_turn_completed() {
            self.finish_turn(state).await?;
        } else {
            self.game_state_repo.save(&state).await?;
        }

        Ok(())
    }

    /// 指定した大名（プレイヤー）の手番になるまで、CPUの行動を自動的に進める
    pub async fn progress_until_player_turn(
        &self,
        player_daimyo_id: Option<DaimyoId>,
    ) -> Result<(), anyhow::Error> {
        let mut state = match self.game_state_repo.get().await? {
            Some(s) => s,
            None => {
                let kunis = self.kuni_repo.find_all().await?;
                let mut rng = rand::thread_rng();
                let order = TurnService::determine_action_order(&kunis, &mut rng);
                let initial_state =
                    GameState::new(TurnNumber::new(1), order, ActionOrderIndex::new(0))?;
                self.game_state_repo.save(&initial_state).await?;
                self.event_dispatcher
                    .dispatch(GameEvent::TurnStarted {
                        turn: TurnNumber::new(1),
                    })
                    .await?;
                return Ok(());
            }
        };

        loop {
            if state.is_turn_completed() {
                self.finish_turn(state).await?;
                state = self
                    .game_state_repo
                    .get()
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;
                continue;
            }

            let kuni_id = state
                .current_kuni_id()
                .ok_or_else(|| anyhow::anyhow!("行動中の国が見つかりません"))?;
            let kuni = self
                .kuni_repo
                .find_by_id(&kuni_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

            let daimyo_id = kuni.daimyo_id;

            if let Some(player_id) = player_daimyo_id {
                if daimyo_id == player_id {
                    // プレイヤーの番なので停止
                    return Ok(());
                }
            }

            // CPUの番なので行動を実行
            self.event_dispatcher
                .dispatch(GameEvent::DaimyoActionStarted { daimyo_id })
                .await?;

            self.execute_cpu_action(kuni_id).await?;

            state.advance_action();
            if state.is_turn_completed() {
                self.finish_turn(state).await?;
                state = self
                    .game_state_repo
                    .get()
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;
            } else {
                self.game_state_repo.save(&state).await?;
            }

            // プレイヤー指定がない場合は1回で抜ける
            if player_daimyo_id.is_none() {
                break;
            }
        }

        Ok(())
    }

    pub async fn progress(&self) -> Result<(), anyhow::Error> {
        self.progress_until_player_turn(None).await
    }

    async fn execute_cpu_action(
        &self,
        kuni_id: crate::domain::model::value_objects::KuniId,
    ) -> Result<(), anyhow::Error> {
        let mut target_kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let daimyo_id = target_kuni.daimyo_id;

        let mut rng = rand::thread_rng();
        let decision = CpuActionDecisionService::decide(daimyo_id, &target_kuni, &mut rng);

        let action_msg = match decision {
            CpuActionDecision::DevelopLand { .. } | CpuActionDecision::BuildTown { .. } => {
                match crate::domain::service::kuni_action_service::KuniActionService::apply_cpu_decision(
                    &mut target_kuni,
                    decision,
                ) {
                    Ok(msg) => {
                        self.kuni_repo.save(&target_kuni).await?;
                        msg
                    }
                    Err(e) => {
                        format!("自動内政に失敗しました: {:?}", e)
                    }
                }
            }
            CpuActionDecision::Battle {
                attacker_id,
                target_kuni_id: Some(target_id),
            } => {
                self.event_dispatcher
                    .dispatch(GameEvent::BattleAction {
                        attacker_id,
                        target_kuni_id: target_id,
                        result_message: EventMessage::new("戦争を行いました（自動）"),
                    })
                    .await?;
                return Ok(());
            }
            CpuActionDecision::Battle {
                target_kuni_id: None,
                ..
            } => "攻撃対象が不明なため待機しました".to_string(),
            CpuActionDecision::Rest => "休息しました".to_string(),
        };

        self.event_dispatcher
            .dispatch(GameEvent::DomesticAction {
                daimyo_id,
                action_name: EventMessage::new("自動内政"),
                details: EventMessage::new(action_msg),
            })
            .await?;

        Ok(())
    }

    async fn finish_turn(&self, mut state: GameState) -> Result<(), anyhow::Error> {
        let kunis = self.kuni_repo.find_all().await?;
        let mut rng = rand::thread_rng();
        let updated_kunis =
            TurnService::process_season(state.current_turn().value(), kunis, &mut rng);
        for kuni in updated_kunis {
            self.kuni_repo.save(&kuni).await?;
        }

        self.event_dispatcher
            .dispatch(GameEvent::SeasonPassed {
                turn: state.current_turn(),
            })
            .await?;

        let kunis = self.kuni_repo.find_all().await?;
        let new_order = TurnService::determine_action_order(&kunis, &mut rng);
        state.start_new_turn(new_order);
        self.game_state_repo.save(&state).await?;

        self.event_dispatcher
            .dispatch(GameEvent::TurnStarted {
                turn: state.current_turn(),
            })
            .await?;

        Ok(())
    }
}
