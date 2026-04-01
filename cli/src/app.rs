use crate::handler::EventHandler;
use crate::screen::ScreenState;
use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use engine::application::usecase::{
    battle_usecase::BattleUseCase, domestic_usecase::DomesticUseCase,
    kuni_query_usecase::KuniQueryUseCase, turn_progression_usecase::TurnProgressionUseCase,
};
use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::value_objects::{DaimyoId, KuniId};
use engine::domain::repository::daimyo_repository::DaimyoRepository;
use engine::domain::repository::event_dispatcher::EventDispatcher;
use engine::domain::repository::game_state_repository::GameStateRepository;
use engine::domain::repository::kuni_repository::KuniRepository;
use engine::domain::repository::neighbor_repository::NeighborRepository;
use ratatui::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub struct App {
    pub screen: ScreenState,
    pub running: bool,
    pub kuni_repo: Arc<dyn KuniRepository>,
    pub daimyo_repo: Arc<dyn DaimyoRepository>,
    pub game_state_repo: Arc<dyn GameStateRepository>,
    pub neighbor_repo: Arc<dyn NeighborRepository>,
    pub event_dispatcher: Arc<dyn EventDispatcher>,

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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        daimyo_repo: Arc<dyn DaimyoRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
        event_dispatcher: Arc<dyn EventDispatcher>,
        domestic_usecase: DomesticUseCase,
        battle_usecase: BattleUseCase,
        turn_progression_usecase: TurnProgressionUseCase,
        kuni_query_usecase: KuniQueryUseCase,
    ) -> Self {
        Self {
            screen: ScreenState::Title,
            running: true,
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            neighbor_repo,
            event_dispatcher,
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
        self.all_daimyos = self.daimyo_repo.find_all().await?;
        let all_kunis = self.kuni_repo.find_all().await?;
        self.kuni_names = all_kunis.into_iter().map(|k| (k.id, k.name.0)).collect();

        if let Some(state) = self.game_state_repo.get().await? {
            self.current_turn = Some(state.current_turn().value());
        }

        match &self.screen {
            ScreenState::SelectDaimyo { cursor } => {
                if let Some(daimyo) = self.all_daimyos.get(*cursor) {
                    self.current_daimyo = Some(daimyo.clone());
                }
            }
            ScreenState::Domestic { selected_kuni, .. } => {
                if let Some(kuni) = self.kuni_repo.find_by_id(selected_kuni).await? {
                    self.current_kuni = Some(kuni.clone());
                    if let Some(daimyo) = self.daimyo_repo.find_by_id(&kuni.daimyo_id).await? {
                        self.current_daimyo = Some(daimyo);
                    }
                }
            }
            ScreenState::War {
                attacker_kuni,
                defender_kuni,
                ..
            } => {
                if let Some(kuni) = self.kuni_repo.find_by_id(attacker_kuni).await? {
                    self.attacker_kuni = Some(kuni.clone());
                    if let Some(daimyo) = self.daimyo_repo.find_by_id(&kuni.daimyo_id).await? {
                        self.current_daimyo = Some(daimyo);
                    }
                }
                if let Some(kuni) = self.kuni_repo.find_by_id(defender_kuni).await? {
                    self.defender_kuni = Some(kuni.clone());
                }
            }
            _ => {
                self.current_kuni = None;
                self.current_daimyo = None;
                self.attacker_kuni = None;
                self.defender_kuni = None;
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

            match get_event(Duration::from_millis(16))? {
                Some(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    EventHandler::handle_key(self, key).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
