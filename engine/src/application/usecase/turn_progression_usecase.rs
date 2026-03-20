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

pub struct TurnProgressionUseCase<
    KR: KuniRepository,
    DR: DaimyoRepository,
    GSR: GameStateRepository,
    ED: EventDispatcher,
> {
    kuni_repo: Arc<KR>,
    daimyo_repo: Arc<DR>,
    game_state_repo: Arc<GSR>,
    event_dispatcher: Arc<ED>,
}

impl<KR, DR, GSR, ED> TurnProgressionUseCase<KR, DR, GSR, ED>
where
    KR: KuniRepository,
    DR: DaimyoRepository,
    GSR: GameStateRepository,
    ED: EventDispatcher,
{
    pub fn new(
        kuni_repo: Arc<KR>,
        daimyo_repo: Arc<DR>,
        game_state_repo: Arc<GSR>,
        event_dispatcher: Arc<ED>,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            event_dispatcher,
        }
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
            self.game_state_repo.save(&state).await?;
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
            CpuActionDecision::DevelopLand {
                target_kuni_id,
                amount,
            } => {
                let Some(mut target_kuni) = kunis.into_iter().find(|k| k.id == target_kuni_id)
                else {
                    return Err(anyhow::anyhow!(
                        "対象国が見つかりません: {:?}",
                        target_kuni_id
                    ));
                };
                if target_kuni.develop_land(amount).is_ok() {
                    self.kuni_repo.save(&target_kuni).await?;
                    "開墾を行いました"
                } else {
                    "資金不足で開墾に失敗しました"
                }
            }
            CpuActionDecision::BuildTown {
                target_kuni_id,
                amount,
            } => {
                let Some(mut target_kuni) = kunis.into_iter().find(|k| k.id == target_kuni_id)
                else {
                    return Err(anyhow::anyhow!(
                        "対象国が見つかりません: {:?}",
                        target_kuni_id
                    ));
                };
                if target_kuni.build_town(amount).is_ok() {
                    self.kuni_repo.save(&target_kuni).await?;
                    "町造りを行いました"
                } else {
                    "資金不足で町造りに失敗しました"
                }
            }
            CpuActionDecision::Battle {
                attacker_id,
                target_kuni_id,
            } => {
                self.event_dispatcher
                    .dispatch(GameEvent::BattleAction {
                        attacker_id,
                        target_kuni_id,
                        result_message: EventMessage::new("戦争を行いました（自動）"),
                    })
                    .await?;
                return Ok(());
            }
            CpuActionDecision::Rest => "休息しました",
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
