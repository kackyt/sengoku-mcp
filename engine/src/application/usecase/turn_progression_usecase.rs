use crate::domain::{
    model::{
        event::GameEvent,
        game_state::GameState,
        value_objects::{ActionOrderIndex, EventMessage, TurnNumber},
    },
    repository::{
        daimyo_repository::DaimyoRepository, event_dispatcher::EventDispatcher,
        game_state_repository::GameStateRepository, kuni_repository::KuniRepository,
    },
    service::{
        cpu_action_decision_service::{CpuActionDecision, CpuActionDecisionService},
        turn_service::TurnService,
    },
};
use std::sync::Arc;

pub struct TurnProgressionUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    daimyo_repo: Arc<dyn DaimyoRepository>,
    game_state_repo: Arc<dyn GameStateRepository>,
    event_dispatcher: Arc<dyn EventDispatcher>,
}

impl TurnProgressionUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        daimyo_repo: Arc<dyn DaimyoRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
        event_dispatcher: Arc<dyn EventDispatcher>,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            event_dispatcher,
        }
    }

    /// 現在の大名の行動を完了とし、次の大名（または次のターン）へ進める
    pub async fn complete_current_action(&self) -> Result<(), anyhow::Error> {
        let mut state = self.game_state_repo.get().await?.ok_or_else(|| {
            anyhow::anyhow!("GameStateが見つかりません。progressを先に呼んでください。")
        })?;

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
        player_id: Option<crate::domain::model::value_objects::DaimyoId>,
    ) -> Result<(), anyhow::Error> {
        for _ in 0..100 {
            // 安全のため上限回数を設ける
            let state = self.game_state_repo.get().await?;

            // 状態がない場合は progress を呼んで初期化
            if state.is_none() {
                self.progress().await?;
                continue;
            }

            let state = state.unwrap();

            // 現在の大名を取得
            let current_daimyo = state.current_daimyo();

            // プレイヤーの手番であり、かつターンが完了していなければ停止
            if let (Some(pid), Some(cid)) = (player_id, current_daimyo) {
                if pid == cid && !state.is_turn_completed() {
                    break;
                }
            }

            // プレイヤー以外の番、またはターン完了時は進める
            self.progress().await?;

            // プレイヤー未指定（観戦モードなど）の場合は1回で抜ける
            if player_id.is_none() {
                break;
            }
        }
        Ok(())
    }

    /// ターンフェーズを進める（自動行動を一回行う、またはターン終了処理を行う）
    pub async fn progress(&self) -> Result<(), anyhow::Error> {
        let mut state = match self.game_state_repo.get().await? {
            Some(s) => s,
            None => {
                // 初回起動時など
                let daimyos = self.daimyo_repo.find_all().await?;
                let mut rng = rand::thread_rng();
                let order = TurnService::determine_action_order(&daimyos, &mut rng);
                let initial_state =
                    GameState::new(TurnNumber::new(1), order, ActionOrderIndex::new(0))?;
                self.game_state_repo.save(&initial_state).await?;
                self.event_dispatcher
                    .dispatch(GameEvent::TurnStarted {
                        turn: TurnNumber::new(1),
                    })
                    .await?;
                initial_state
            }
        };

        if state.is_turn_completed() {
            // ターン終了で次のターンへ移行
            self.finish_turn(state).await?;
            return Ok(());
        }

        // 行動大名を取得
        if let Some(daimyo_id) = state.current_daimyo() {
            self.event_dispatcher
                .dispatch(GameEvent::DaimyoActionStarted { daimyo_id })
                .await?;

            // CPUの自動行動を実行
            self.execute_cpu_action(daimyo_id).await?;

            // 次の大名へ
            state.advance_action();
            if state.is_turn_completed() {
                self.finish_turn(state).await?;
            } else {
                self.game_state_repo.save(&state).await?;
            }
        }

        Ok(())
    }

    async fn execute_cpu_action(
        &self,
        daimyo_id: crate::domain::model::value_objects::DaimyoId,
    ) -> Result<(), anyhow::Error> {
        let kunis = self.kuni_repo.find_by_daimyo_id(&daimyo_id).await?;
        if kunis.is_empty() {
            return Ok(()); // 滅亡している場合は何もしない
        }

        let mut rng = rand::thread_rng();
        let decision = CpuActionDecisionService::decide(daimyo_id, &kunis, &mut rng);

        let action_msg = match decision {
            CpuActionDecision::DevelopLand { target_kuni_id, .. }
            | CpuActionDecision::BuildTown { target_kuni_id, .. } => {
                let Some(mut target_kuni) = kunis.into_iter().find(|k| k.id == target_kuni_id)
                else {
                    return Err(anyhow::anyhow!(
                        "対象国が見つかりません: {:?}",
                        target_kuni_id
                    ));
                };

                match crate::domain::service::kuni_action_service::KuniActionService::apply_cpu_decision(&mut target_kuni, decision) {
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

        let daimyos = self.daimyo_repo.find_all().await?;
        let new_order = TurnService::determine_action_order(&daimyos, &mut rng);
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
