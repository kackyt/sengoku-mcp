use crate::handler::EventHandler;
use crate::screen::{DomesticSubState, ScreenState};
use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use engine::application::usecase::{
    battle_usecase::BattleUseCase, domestic_usecase::DomesticUseCase,
    kuni_query_usecase::KuniQueryUseCase, turn_progression_usecase::TurnProgressionUseCase,
};
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
}

impl App {
    pub fn new(
        domestic_usecase: DomesticUseCase,
        battle_usecase: BattleUseCase,
        turn_progression_usecase: TurnProgressionUseCase,
        kuni_query_usecase: KuniQueryUseCase,
    ) -> Self {
        Self {
            screen: ScreenState::Title,
            running: true,
            domestic_usecase,
            battle_usecase,
            turn_progression_usecase,
            kuni_query_usecase,
            current_kuni: None,
            current_daimyo: None,
            all_daimyos: Vec::new(),
            current_turn: None,
            messages: Vec::new(),
            attacker_kuni: None,
            defender_kuni: None,
            kuni_names: std::collections::HashMap::new(),
            selected_daimyo_id: None,
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

        // 手番の国と表示されている国がズレないように強制同期
        if let Some(current) = &self.current_kuni {
            if let ScreenState::Domestic {
                selected_kuni,
                cursor,
                sub_state,
            } = &self.screen
            {
                if *selected_kuni != current.id {
                    self.screen = ScreenState::Domestic {
                        selected_kuni: current.id,
                        cursor: *cursor,
                        sub_state: sub_state.clone(),
                    };
                }
            }
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
            if self.selected_daimyo_id.is_some() && !self.is_player_turn() {
                match &self.screen {
                    ScreenState::Domestic { sub_state, .. } if matches!(sub_state, DomesticSubState::Normal) => {
                        // 1ステップ進める
                        self.turn_progression_usecase.progress().await?;
                        // CPUの行動を見せるために少し待機
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    ScreenState::War { sub_state, .. } if matches!(sub_state, crate::screen::WarSubState::Normal) => {
                        // 1ステップ進める
                        self.turn_progression_usecase.progress().await?;
                        // CPUの行動を見せるために少し待機
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    _ => {}
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
