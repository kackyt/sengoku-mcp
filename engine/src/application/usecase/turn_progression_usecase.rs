use crate::domain::{
    model::action_log::{ActionLogEntry, ActionLogEvent, ActionLogVisibility, DomesticLogEvent},
    model::{
        event::GameEvent,
        game_state::GameState,
        value_objects::{ActionOrderIndex, DaimyoId, EventMessage, TurnNumber},
    },
    repository::{
        action_log_repository::ActionLogRepository, daimyo_repository::DaimyoRepository,
        event_dispatcher::EventDispatcher, game_state_repository::GameStateRepository,
        kuni_repository::KuniRepository,
    },
    service::{
        cpu_action_decision_service::{CpuActionDecision, CpuActionDecisionService},
        turn_service::TurnService,
    },
};
use std::sync::Arc;

#[derive(Clone)]
pub struct TurnProgressionUseCase {
    kuni_repo: Arc<dyn KuniRepository + Send + Sync>,
    daimyo_repo: Arc<dyn DaimyoRepository + Send + Sync>,
    game_state_repo: Arc<dyn GameStateRepository + Send + Sync>,
    event_dispatcher: Arc<dyn EventDispatcher + Send + Sync>,
    action_log_repo: Arc<dyn ActionLogRepository + Send + Sync>,
}

impl TurnProgressionUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository + Send + Sync>,
        daimyo_repo: Arc<dyn DaimyoRepository + Send + Sync>,
        game_state_repo: Arc<dyn GameStateRepository + Send + Sync>,
        event_dispatcher: Arc<dyn EventDispatcher + Send + Sync>,
        action_log_repo: Arc<dyn ActionLogRepository + Send + Sync>,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            event_dispatcher,
            action_log_repo,
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
                let order = {
                    let mut rng = rand::thread_rng();
                    TurnService::determine_action_order(&kunis, &mut rng)
                };
                let initial_state =
                    GameState::new(TurnNumber::new(1), order, ActionOrderIndex::new(0))
                        .expect("valid state");
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

    pub async fn execute_cpu_action(
        &self,
        kuni_id: crate::domain::model::value_objects::KuniId,
    ) -> Result<(), anyhow::Error> {
        let mut target_kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let daimyo_id = target_kuni.daimyo_id;
        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));

        let daimyo = self
            .daimyo_repo
            .find_by_id(&daimyo_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("大名が見つかりません: {:?}", daimyo_id))?;

        let (decision, reasoning) = {
            let mut rng = rand::thread_rng();
            CpuActionDecisionService::decide(&daimyo.personality, &target_kuni, turn, &mut rng)
        };

        let action_msg = match decision {
            CpuActionDecision::Battle {
                target_kuni_id: Some(target_id),
            } => {
                self.event_dispatcher
                    .dispatch(GameEvent::BattleAction {
                        attacker_id: daimyo_id,
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
            _ => {
                match crate::domain::service::kuni_action_service::KuniActionService::apply_cpu_decision(
                    &mut target_kuni,
                    decision,
                ) {
                    Ok(msg) => {
                        self.kuni_repo.save(&target_kuni).await?;
                        msg
                    }
                    Err(e) => {
                        format!("自動アクションに失敗しました: {:?}", e)
                    }
                }
            }
        };

        self.event_dispatcher
            .dispatch(GameEvent::DomesticAction {
                daimyo_id,
                action_name: EventMessage::new("自動内政"),
                details: EventMessage::new(action_msg.clone()),
            })
            .await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Internal,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::CpuAction {
                daimyo_id,
                action_msg: action_msg.to_string(),
                reasoning: Some(reasoning),
            }),
        ));

        Ok(())
    }

    async fn finish_turn(&self, mut state: GameState) -> Result<(), anyhow::Error> {
        let current_turn = state.current_turn();

        self.event_dispatcher
            .dispatch(GameEvent::SeasonPassed { turn: current_turn })
            .await?;

        // ターン開始時の季節イベント（洪水・疫病・反乱・資源生成）を次のターン開始前に処理
        let mut kunis = self.kuni_repo.find_all().await?;
        let new_order = {
            let mut rng = rand::thread_rng();
            TurnService::determine_action_order(&kunis, &mut rng)
        };
        state.start_new_turn(new_order);
        self.game_state_repo.save(&state).await?;

        self.event_dispatcher
            .dispatch(GameEvent::TurnStarted {
                turn: state.current_turn(),
            })
            .await?;

        // ターン開始のPublicログ
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Public,
            state.current_turn(),
            ActionLogEvent::Domestic(DomesticLogEvent::TurnStart {
                turn: state.current_turn(),
                season: state.current_turn().season(),
            }),
        ));

        // 新しいターン開始時のイベントを処理
        let start_effects =
            TurnService::process_start_turn_events(state.current_turn(), &mut kunis);
        for kuni in &kunis {
            self.kuni_repo.save(kuni).await?;
        }

        // 季節イベント結果を集約してログに記録
        use crate::domain::model::event::SeasonalEventType;
        use std::collections::HashMap;
        let mut effects_by_type: HashMap<
            SeasonalEventType,
            Vec<&crate::domain::model::event::SeasonalEventEffect>,
        > = HashMap::new();
        for effect in &start_effects {
            effects_by_type
                .entry(effect.event_type.clone())
                .or_default()
                .push(effect);
        }

        // 表示順序の定義
        let display_order = vec![
            SeasonalEventType::GoldIncome,
            SeasonalEventType::RiceIncome,
            SeasonalEventType::PopulationGrowth,
            SeasonalEventType::Plague,
            SeasonalEventType::Flood,
            SeasonalEventType::Rebellion,
        ];

        for etype in display_order {
            if let Some(effects) = effects_by_type.get(&etype) {
                let affected_kuni_names: Vec<_> = effects
                    .iter()
                    .filter_map(|e| kunis.iter().find(|k| k.id == e.kuni_id))
                    .map(|k| k.name.clone())
                    .collect();
                let _ = self.action_log_repo.save(ActionLogEntry::new(
                    ActionLogVisibility::Public,
                    state.current_turn(),
                    ActionLogEvent::Domestic(DomesticLogEvent::SeasonalEvent {
                        event_type: etype.clone(),
                        kuni_names: affected_kuni_names,
                    }),
                ));
            }
        }

        // 季節イベント結果を個別に通知（イベントディスパッチのみ）
        for effect in &start_effects {
            let detail_str = format!(
                "国ID={:?} 金:{:+} 米:{:+} 兵:{:+} 人口:{:+} 忠誠:{:+} 石高:{:+} 町:{:+}",
                effect.kuni_id,
                effect.kin_diff.to_display().value(),
                effect.kome_diff.to_display().value(),
                effect.hei_diff.to_display().value(),
                effect.jinko_diff.to_display().value(),
                effect.tyu_diff,
                effect.kokudaka_diff.to_display().value(),
                effect.machi_diff.to_display().value()
            );

            self.event_dispatcher
                .dispatch(GameEvent::DomesticAction {
                    daimyo_id: kunis
                        .iter()
                        .find(|k| k.id == effect.kuni_id)
                        .map(|k| k.daimyo_id)
                        .unwrap_or_default(),
                    action_name: EventMessage::new(format!(
                        "季節イベント: {:?}",
                        effect.event_type
                    )),
                    details: EventMessage::new(detail_str),
                })
                .await?;
        }

        Ok(())
    }
}
