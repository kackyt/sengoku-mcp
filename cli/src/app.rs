use crate::handler::EventHandler;
use crate::screen::{DomesticSubState, ScreenState};
use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use engine::application::usecase::{
    battle_usecase::BattleUseCase, domestic_usecase::DomesticUseCase, info_usecase::InfoUseCase,
    kuni_query_usecase::KuniQueryUseCase, turn_progression_usecase::TurnProgressionUseCase,
};
use engine::domain::model::action_log::ActionLogEntry;
use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::value_objects::{DaimyoId, KuniId};
use ratatui::prelude::*;
use std::time::Duration;

pub struct App {
    pub screen: ScreenState,
    pub running: bool,

    pub domestic_usecase: DomesticUseCase,
    pub battle_usecase: BattleUseCase,
    pub turn_progression_usecase: TurnProgressionUseCase,
    pub kuni_query_usecase: KuniQueryUseCase,
    pub info_usecase: InfoUseCase,

    // UI Cache
    pub current_kuni: Option<Kuni>,
    pub current_daimyo: Option<Daimyo>,
    pub all_daimyos: Vec<Daimyo>,
    pub current_turn: Option<u32>,
    pub messages: Vec<String>,
    pub attacker_kuni: Option<Kuni>,
    pub defender_kuni: Option<Kuni>,
    pub kuni_names: std::collections::HashMap<KuniId, String>,
    pub selected_daimyo_id: Option<DaimyoId>,
    pub domestic_logs: Vec<ActionLogEntry>,
    pub war_logs: Vec<ActionLogEntry>,
    pub active_battles: Vec<engine::domain::model::battle::WarStatus>,
    pub all_kunis: Vec<Kuni>,
}

impl App {
    pub fn new(
        domestic_usecase: DomesticUseCase,
        battle_usecase: BattleUseCase,
        turn_progression_usecase: TurnProgressionUseCase,
        kuni_query_usecase: KuniQueryUseCase,
        info_usecase: InfoUseCase,
    ) -> Self {
        Self {
            screen: ScreenState::Title,
            running: true,
            domestic_usecase,
            battle_usecase,
            turn_progression_usecase,
            kuni_query_usecase,
            info_usecase,
            current_kuni: None,
            current_daimyo: None,
            all_daimyos: Vec::new(),
            current_turn: None,
            messages: Vec::new(),
            attacker_kuni: None,
            defender_kuni: None,
            kuni_names: std::collections::HashMap::new(),
            selected_daimyo_id: None,
            domestic_logs: Vec::new(),
            war_logs: Vec::new(),
            active_battles: Vec::new(),
            all_kunis: Vec::new(),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        self.update_cache().await?;
        Ok(())
    }

    pub async fn update_cache(&mut self) -> Result<()> {
        let (selected_id, attacker_id, defender_id) = match &self.screen {
            ScreenState::Domestic { selected_kuni, .. } => (Some(*selected_kuni), None, None),
            ScreenState::War { status, .. } => {
                (None, Some(status.attacker_id()), Some(status.defender_id()))
            }
            _ => (None, None, None),
        };

        let snapshot = self
            .kuni_query_usecase
            .get_ui_snapshot(selected_id, attacker_id, defender_id)
            .await?;

        self.all_daimyos = snapshot.all_daimyos;
        self.current_turn = snapshot.current_turn;
        self.current_kuni = snapshot.current_kuni;
        self.current_daimyo = snapshot.current_daimyo;
        self.attacker_kuni = snapshot.attacker_kuni;
        self.defender_kuni = snapshot.defender_kuni;
        self.kuni_names = snapshot.kuni_names;
        self.domestic_logs = snapshot.domestic_logs;
        self.war_logs = snapshot.war_logs;
        self.active_battles = snapshot.active_battles.clone();
        // プレイヤーが防御側となる合戦があれば、合戦画面へ遷移（モーダル表示の代わり）
        let defense_battle = self.selected_daimyo_id.and_then(|player_id| {
            snapshot.active_battles.iter().find(|b| {
                // 防御側の国が自分のものかチェック
                snapshot
                    .all_kunis
                    .iter()
                    .any(|k| k.id == b.defender.kuni_id && k.daimyo_id == player_id)
            })
        });

        let defense_battle_cloned = defense_battle.cloned();
        self.all_kunis = snapshot.all_kunis;

        if let Some(battle) = defense_battle_cloned {
            // まだ合戦画面でない、または別の合戦が表示されている場合は切り替え
            let should_switch = match &self.screen {
                ScreenState::War { status, .. } => {
                    status.attacker.kuni_id != battle.attacker.kuni_id
                        || status.defender.kuni_id != battle.defender.kuni_id
                }
                _ => true,
            };

            if should_switch {
                self.screen = ScreenState::War {
                    status: battle,
                    cursor: 0,
                    sub_state: crate::screen::WarSubState::Normal,
                };
            }
        }

        // ゲームオーバー判定
        if matches!(
            snapshot.phase,
            engine::domain::model::game_state::GamePhase::GameOver
                | engine::domain::model::game_state::GamePhase::GameClear
        ) {
            if let Some(winner) = snapshot.winner {
                self.screen = ScreenState::GameOver { winner };
            }
        }

        // 手番の国と表示されている国がズレないように強制同期
        match (&self.current_kuni, &self.screen) {
            (
                Some(current),
                ScreenState::Domestic {
                    selected_kuni,
                    cursor,
                    sub_state,
                },
            ) if *selected_kuni != current.id => {
                self.screen = ScreenState::Domestic {
                    selected_kuni: current.id,
                    cursor: *cursor,
                    sub_state: sub_state.clone(),
                };
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn run<B: Backend, E, D>(
        &mut self,
        terminal: &mut Terminal<B>,
        mut get_event: E,
        mut on_draw: D,
    ) -> Result<()>
    where
        E: FnMut(Duration) -> Result<Option<Event>>,
        D: FnMut(&mut Terminal<B>),
    {
        self.init().await?;

        while self.running {
            // 描画前にキャッシュを更新
            self.update_cache().await?;

            terminal.draw(|f| crate::ui::draw(self, f))?;
            on_draw(terminal);

            // プレイヤーの手番でない場合は自動進行
            let is_player_turn = self.is_player_turn();
            let is_player_in_war = if let ScreenState::War { status: _, .. } = &self.screen {
                let player_id = self.selected_daimyo_id;
                // アタッカーかディフェンダーのいずれかがプレイヤーであれば、合戦は自動進行させない
                // (update_cache で適切に kuni 情報が取得されている前提)
                let is_attacker = self.attacker_kuni.as_ref().map(|k| k.daimyo_id) == player_id;
                let is_defender = self.defender_kuni.as_ref().map(|k| k.daimyo_id) == player_id;
                is_attacker || is_defender
            } else {
                false
            };

            if self.selected_daimyo_id.is_some() && !is_player_turn && !is_player_in_war {
                // 進行可能なサブ状態かチェック
                let can_progress = matches!(
                    self.screen,
                    ScreenState::Domestic {
                        sub_state: DomesticSubState::Normal,
                        ..
                    } | ScreenState::War {
                        sub_state: crate::screen::WarSubState::Normal,
                        ..
                    }
                );

                if can_progress {
                    // 1ステップ進める
                    self.turn_progression_usecase.progress(self.selected_daimyo_id).await?;
                    // CPUの行動を見せるために少し待機
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            }

            match get_event(Duration::from_millis(16))? {
                Some(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    EventHandler::handle_key(self, key).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn is_player_turn(&self) -> bool {
        if let (Some(pid), Some(current)) = (self.selected_daimyo_id, &self.current_daimyo) {
            pid == current.id
        } else {
            false
        }
    }
}
