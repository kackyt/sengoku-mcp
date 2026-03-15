use crate::domain::{
    model::{
        event::GameEvent,
        game_state::GameState,
    },
    repository::{
        daimyo_repository::DaimyoRepository, event_dispatcher::EventDispatcher,
        game_state_repository::GameStateRepository, kuni_repository::KuniRepository,
    },
    service::turn_service::TurnService,
};
use std::sync::Arc;
use rand::Rng;

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
                let order = TurnService::determine_action_order(&daimyos);
                let initial_state = GameState::new(1, order, 0);
                self.game_state_repo.save(&initial_state).await?;
                self.event_dispatcher
                    .dispatch(GameEvent::TurnStarted { turn: 1 })
                    .await;
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
                .await;

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
        // 簡単な自動行動ロジック（ランダムに行動を選ぶなど）
        let kunis = self.kuni_repo.find_by_daimyo_id(&daimyo_id).await?;
        if kunis.is_empty() {
            return Ok(()); // 滅亡している場合は何もしない
        }

        // 最初の領地（本拠地）で行動すると仮定
        let mut target_kuni = kunis[0].clone();

        let mut rng = rand::thread_rng();
        let action = rng.gen_range(0..4);
        let action_msg = match action {
            0 => {
                let amount = crate::domain::model::value_objects::Amount::new(100);
                if target_kuni.develop_land(amount).is_ok() {
                    self.kuni_repo.save(&target_kuni).await?;
                    "開墾を行いました".to_string()
                } else {
                    "資金不足で開墾に失敗しました".to_string()
                }
            }
            1 => {
                let amount = crate::domain::model::value_objects::Amount::new(100);
                if target_kuni.build_town(amount).is_ok() {
                    self.kuni_repo.save(&target_kuni).await?;
                    "町造りを行いました".to_string()
                } else {
                    "資金不足で町造りに失敗しました".to_string()
                }
            }
            2 => {
                // 戦争のダミー（実際には対象の敵国を探す処理などが必要だが簡易化）
                self.event_dispatcher
                    .dispatch(GameEvent::BattleAction {
                        attacker_id: daimyo_id,
                        target_kuni_id: target_kuni.id, // ダミーとして自国IDを入れる
                        result_message: "戦争を行いました（自動）".to_string(),
                    })
                    .await;
                return Ok(());
            }
            _ => "休息しました".to_string(),
        };

        self.event_dispatcher
            .dispatch(GameEvent::DomesticAction {
                daimyo_id,
                action_name: "自動内政".to_string(),
                details: action_msg,
            })
            .await;

        Ok(())
    }

    async fn finish_turn(&self, mut state: GameState) -> Result<(), anyhow::Error> {
        let kunis = self.kuni_repo.find_all().await?;
        let updated_kunis = TurnService::process_season(state.current_turn, kunis);
        for kuni in updated_kunis {
            self.kuni_repo.save(&kuni).await?;
        }

        self.event_dispatcher
            .dispatch(GameEvent::SeasonPassed {
                turn: state.current_turn,
            })
            .await;

        let daimyos = self.daimyo_repo.find_all().await?;
        let new_order = TurnService::determine_action_order(&daimyos);
        let next_turn = state.current_turn + 1;
        state.start_new_turn(new_order);
        self.game_state_repo.save(&state).await?;

        self.event_dispatcher
            .dispatch(GameEvent::TurnStarted { turn: next_turn })
            .await;

        Ok(())
    }
}
