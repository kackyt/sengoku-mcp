use crate::domain::{
    model::action_log::{ActionLogEntry, ActionLogEvent, ActionLogVisibility, DomesticLogEvent},
    model::{
        event::GameEvent,
        game_state::{GamePhase, GameState},
        value_objects::{ActionOrderIndex, DaimyoId, EventMessage, KuniId, TurnNumber},
    },
    repository::{
        action_log_repository::ActionLogRepository, daimyo_repository::DaimyoRepository,
        event_dispatcher::EventDispatcher, game_state_repository::GameStateRepository,
        kuni_repository::KuniRepository,
    },
    service::{
        cpu_action_decision_service::{CpuActionDecision, CpuActionDecisionService},
        kuni_service::KuniService,
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
    battle_repo:
        Arc<dyn crate::domain::repository::battle_repository::BattleRepository + Send + Sync>,
    neighbor_repo:
        Arc<dyn crate::domain::repository::neighbor_repository::NeighborRepository + Send + Sync>,
}

impl TurnProgressionUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository + Send + Sync>,
        daimyo_repo: Arc<dyn DaimyoRepository + Send + Sync>,
        game_state_repo: Arc<dyn GameStateRepository + Send + Sync>,
        event_dispatcher: Arc<dyn EventDispatcher + Send + Sync>,
        action_log_repo: Arc<dyn ActionLogRepository + Send + Sync>,
        battle_repo: Arc<
            dyn crate::domain::repository::battle_repository::BattleRepository + Send + Sync,
        >,
        neighbor_repo: Arc<
            dyn crate::domain::repository::neighbor_repository::NeighborRepository + Send + Sync,
        >,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            event_dispatcher,
            action_log_repo,
            battle_repo,
            neighbor_repo,
        }
    }

    /// 現在のゲーム状態を取得します
    pub async fn get_state(&self) -> Result<Option<GameState>, anyhow::Error> {
        self.game_state_repo.get().await.map_err(|e| e.into())
    }

    /// 現在の行動を完了とし、次へ進める
    pub async fn complete_current_action(
        &self,
        player_daimyo_id: Option<DaimyoId>,
    ) -> Result<(), anyhow::Error> {
        let mut state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません。"))?;

        state.advance_action();
        self.check_victory_and_defeat(&mut state, player_daimyo_id)
            .await?;

        if state.is_turn_completed() {
            self.finish_turn(state, player_daimyo_id).await?;
        } else {
            self.game_state_repo.save(&state).await?;
        }

        Ok(())
    }

    /// 次の1ステップ（1大名の行動、またはターンの終了）を進めます。
    /// プレイヤーの手番の場合は何もしません。
    pub async fn progress(&self, player_daimyo_id: Option<DaimyoId>) -> Result<(), anyhow::Error> {
        let state = match self.game_state_repo.get().await? {
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

        if state.phase() != GamePhase::Domestic {
            return Ok(());
        }

        if state.is_turn_completed() {
            self.finish_turn(state, player_daimyo_id).await?;
            return Ok(());
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

        // プレイヤーの手番なら停止
        if let Some(player_id) = player_daimyo_id {
            if daimyo_id == player_id {
                return Ok(());
            }
        }

        // CPUのアクションを実行
        self.event_dispatcher
            .dispatch(GameEvent::DaimyoActionStarted { daimyo_id })
            .await?;

        self.execute_cpu_action(kuni_id, player_daimyo_id).await?;

        // 合戦フェーズに移行した場合は、アクション完了処理（次の国へ進める）を行わず停止する
        let state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;
        if state.phase() == GamePhase::Battle {
            return Ok(());
        }

        // 完了処理
        self.complete_current_action(player_daimyo_id).await?;

        Ok(())
    }

    /// プレイヤーの手番が来るまで自動進行します。
    pub async fn progress_until_player_turn(
        &self,
        player_daimyo_id: Option<DaimyoId>,
    ) -> Result<(), anyhow::Error> {
        loop {
            let state = self
                .game_state_repo
                .get()
                .await?
                .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;

            if state.phase() != GamePhase::Domestic {
                return Ok(());
            }

            if state.is_turn_completed() {
                self.finish_turn(state, player_daimyo_id).await?;
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
            if player_daimyo_id.is_some_and(|id| id == kuni.daimyo_id) {
                return Ok(());
            }

            self.progress(player_daimyo_id).await?;
        }
    }

    /// 指定した国のCPU行動を実行し、アクションを完了させます (原子的な実行)
    pub async fn execute_cpu_action_and_advance(
        &self,
        kuni_id: KuniId,
        player_daimyo_id: Option<DaimyoId>,
    ) -> Result<(), anyhow::Error> {
        self.execute_cpu_action(kuni_id, player_daimyo_id).await?;
        self.complete_current_action(player_daimyo_id).await?;
        Ok(())
    }

    pub async fn execute_cpu_action(
        &self,
        kuni_id: crate::domain::model::value_objects::KuniId,
        player_daimyo_id: Option<DaimyoId>,
    ) -> Result<(), anyhow::Error> {
        let mut state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;

        // すでに行動済みの場合はスキップ (冪等性)
        if state.is_action_performed() {
            return Ok(());
        }

        let mut target_kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let daimyo_id = target_kuni.daimyo_id;
        let turn = state.current_turn();

        let daimyo = self
            .daimyo_repo
            .find_by_id(&daimyo_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("大名が見つかりません: {:?}", daimyo_id))?;

        // 1. 出兵判断 (内政より先に検討)
        let neighbor_kunis = KuniService::get_neighbor_kunis(
            &kuni_id,
            self.neighbor_repo.as_ref(),
            self.kuni_repo.as_ref(),
        )
        .await?;

        if let Some(plan) = crate::domain::service::war_decision_service::WarDecisionService::new()
            .decide_invasion(
                &daimyo,
                &target_kuni,
                &neighbor_kunis,
                self.neighbor_repo.as_ref(),
                self.kuni_repo.as_ref(),
            )
            .await?
        {
            let (target_id, dispatched_hei, dispatched_kome) =
                (plan.target_kuni_id, plan.hei, plan.kome);
            let mut enemy_kuni = self
                .kuni_repo
                .find_by_id(&target_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("攻撃対象の国が見つかりません: {:?}", target_id))?;

            // 出兵リソース消費
            let attacker_army = target_kuni.dispatch_army(dispatched_hei, dispatched_kome)?;
            self.kuni_repo.save(&target_kuni).await?;

            // ログ（戦争開始 - 内政ログにも表示）
            let _ = self.action_log_repo.save(ActionLogEntry::new(
                ActionLogVisibility::Public,
                state.current_turn(),
                ActionLogEvent::Domestic(DomesticLogEvent::CpuAction {
                    daimyo_id,
                    action_msg: format!(
                        "【侵攻】{} が {} へ攻め込みました！",
                        target_kuni.name.0, enemy_kuni.name.0
                    ),
                    reasoning: None,
                }),
            ));

            let _ = self.action_log_repo.save(ActionLogEntry::new(
                ActionLogVisibility::Public,
                state.current_turn(),
                ActionLogEvent::War(crate::domain::model::action_log::WarLogEvent::WarStarted {
                    attacker_name: target_kuni.name.clone(),
                    defender_name: enemy_kuni.name.clone(),
                    attacker_id: target_kuni.daimyo_id,
                    defender_id: enemy_kuni.daimyo_id,
                }),
            ));

            let is_target_player = player_daimyo_id.is_some_and(|id| id == enemy_kuni.daimyo_id);

            if is_target_player {
                // ターゲットがプレイヤーの場合：合戦状態を保存して停止
                let defender_army = crate::domain::model::battle::ArmyStatus {
                    kuni_id: enemy_kuni.id,
                    hei: enemy_kuni.resource.hei,
                    kome: enemy_kuni.resource.kome,
                    morale: enemy_kuni.stats.tyu,
                };
                let war_status =
                    crate::domain::model::battle::WarStatus::new(attacker_army, defender_army);
                self.battle_repo.save(&war_status).await?;

                // 合戦フェーズへ移行して停止
                state.set_phase(GamePhase::Battle);
                state.mark_action_performed();
                self.game_state_repo.save(&state).await?;
                return Ok(());
            } else {
                // ターゲットがCPUの場合：自動決着
                let defender_army = crate::domain::model::battle::ArmyStatus {
                    kuni_id: enemy_kuni.id,
                    hei: enemy_kuni.resource.hei,
                    kome: enemy_kuni.resource.kome,
                    morale: enemy_kuni.stats.tyu,
                };
                let war_status =
                    crate::domain::model::battle::WarStatus::new(attacker_army, defender_army);

                let (final_status, _turns) = {
                    let mut rng = rand::thread_rng();
                    crate::domain::service::battle_service::BattleService::auto_resolve(
                        war_status, &mut rng,
                    )?
                };

                // 結果の反映
                match final_status.winner {
                    Some(crate::domain::model::battle::BattleSide::Attacker) => {
                        // 攻撃側勝利：占領
                        enemy_kuni.occupy(daimyo_id, &final_status.attacker);
                        self.kuni_repo.save(&enemy_kuni).await?;

                        let _ = self.action_log_repo.save(ActionLogEntry::new(
                            ActionLogVisibility::Public,
                            state.current_turn(),
                            ActionLogEvent::War(
                                crate::domain::model::action_log::WarLogEvent::AttackerVictory {
                                    home_name: target_kuni.name.clone(),
                                    attacker_id: target_kuni.daimyo_id,
                                    occupied_name: enemy_kuni.name.clone(),
                                    defender_id: enemy_kuni.daimyo_id,
                                },
                            ),
                        ));
                    }
                    _ => {
                        // 防衛側勝利（または引き分け）
                        enemy_kuni.survive_defense(&final_status.defender);
                        target_kuni.survive_defense(&final_status.attacker); // 帰還兵の処理
                        self.kuni_repo.save(&enemy_kuni).await?;
                        self.kuni_repo.save(&target_kuni).await?;

                        let _ = self.action_log_repo.save(ActionLogEntry::new(
                            ActionLogVisibility::Public,
                            state.current_turn(),
                            ActionLogEvent::War(
                                crate::domain::model::action_log::WarLogEvent::DefenderVictory {
                                    home_name: target_kuni.name.clone(),
                                    attacker_id: target_kuni.daimyo_id,
                                    defender_id: enemy_kuni.daimyo_id,
                                },
                            ),
                        ));
                    }
                }
            }

            // 行動済みフラグを立てて保存
            state.mark_action_performed();
            self.game_state_repo.save(&state).await?;
            return Ok(());
        }

        // 2. 内政判断 (出兵しなかった場合)
        let (decision, reasoning) = {
            let mut rng = rand::thread_rng();
            CpuActionDecisionService::decide(&daimyo.personality, &target_kuni, turn, &mut rng)
        };

        let action_msg = match decision {
            CpuActionDecision::Battle { .. } => {
                // ここには来ないはずだが念のため
                "戦況を静観しました".to_string()
            }
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

        // 行動済みフラグを立てて保存
        state.mark_action_performed();
        self.check_victory_and_defeat(&mut state, player_daimyo_id)
            .await?;
        self.game_state_repo.save(&state).await?;

        Ok(())
    }

    async fn finish_turn(
        &self,
        mut state: GameState,
        player_daimyo_id: Option<DaimyoId>,
    ) -> Result<(), anyhow::Error> {
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

        self.check_victory_and_defeat(&mut state, player_daimyo_id)
            .await?;
        self.game_state_repo.save(&state).await?;
        Ok(())
    }

    /// 勝利・敗北条件をチェックし、必要に応じてフェーズを更新します
    async fn check_victory_and_defeat(
        &self,
        state: &mut GameState,
        player_daimyo_id: Option<DaimyoId>,
    ) -> anyhow::Result<()> {
        let all_kunis = self.kuni_repo.find_all().await?;
        if all_kunis.is_empty() {
            return Ok(());
        }

        // 1. プレーヤーの敗北チェック
        if let Some(player_id) = player_daimyo_id {
            let _ = self.action_log_repo.save(ActionLogEntry::new(
                ActionLogVisibility::Internal,
                state.current_turn(),
                ActionLogEvent::Domestic(DomesticLogEvent::CpuAction {
                    daimyo_id: player_id,
                    action_msg: "敗北・勝利チェック".to_string(),
                    reasoning: Some(format!(
                        "player_id={:?}, kunis_count={}",
                        player_id,
                        all_kunis.len()
                    )),
                }),
            ));
            let player_kunis = all_kunis
                .iter()
                .filter(|k| k.daimyo_id == player_id)
                .count();

            if player_kunis == 0 {
                // 勝者を適当なCPU大名に設定（自分以外の大名のどれか）
                if let Some(other_kuni) = all_kunis.iter().find(|k| k.daimyo_id != player_id) {
                    state.set_winner(other_kuni.daimyo_id);
                    state.set_phase(GamePhase::GameOver);
                } else {
                    state.set_phase(GamePhase::GameOver);
                }
                return Ok(());
            }
        }

        // 2. 勝利チェック (一人の大名が全土統一したか)
        let first_daimyo = all_kunis[0].daimyo_id;
        if all_kunis.iter().all(|k| k.daimyo_id == first_daimyo) {
            state.set_winner(first_daimyo);
            if let Some(player_id) = player_daimyo_id {
                if first_daimyo == player_id {
                    state.set_phase(GamePhase::GameClear);
                } else {
                    state.set_phase(GamePhase::GameOver);
                }
            } else {
                state.set_phase(GamePhase::GameOver);
            }
        }

        Ok(())
    }
}
