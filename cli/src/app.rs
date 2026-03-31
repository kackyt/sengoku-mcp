use crate::handler::EventHandler;
use crate::screen::ScreenState;
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use engine::application::usecase::{
    battle_usecase::BattleUseCase, domestic_usecase::DomesticUseCase,
    turn_progression_usecase::TurnProgressionUseCase,
};
use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::value_objects::KuniId;
use engine::domain::repository::daimyo_repository::DaimyoRepository;
use engine::domain::repository::game_state_repository::GameStateRepository;
use engine::domain::repository::kuni_repository::KuniRepository;
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{
    InMemoryDaimyoRepository, InMemoryEventDispatcher, InMemoryGameStateRepository,
    InMemoryKuniRepository, InMemoryNeighborRepository,
};
use ratatui::prelude::*;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub struct App {
    pub screen: ScreenState,
    pub running: bool,
    pub kuni_repo: Arc<InMemoryKuniRepository>,
    pub daimyo_repo: Arc<InMemoryDaimyoRepository>,
    pub game_state_repo: Arc<InMemoryGameStateRepository>,
    pub neighbor_repo: Arc<InMemoryNeighborRepository>,
    pub event_dispatcher: Arc<InMemoryEventDispatcher>,

    pub domestic_usecase: DomesticUseCase<InMemoryKuniRepository, InMemoryNeighborRepository>,
    pub battle_usecase: BattleUseCase<InMemoryKuniRepository, InMemoryNeighborRepository>,
    pub turn_progression_usecase: TurnProgressionUseCase<
        InMemoryKuniRepository,
        InMemoryDaimyoRepository,
        InMemoryGameStateRepository,
        InMemoryEventDispatcher,
    >,

    // UI Cache
    pub current_kuni: Option<Kuni>,
    pub current_daimyo: Option<Daimyo>,
    pub all_daimyos: Vec<Daimyo>,
    pub current_turn: Option<u32>,
    pub messages: Vec<String>,
    pub attacker_kuni: Option<Kuni>,
    pub defender_kuni: Option<Kuni>,
    pub kuni_names: std::collections::HashMap<KuniId, String>,
}

impl App {
    pub fn new() -> Result<Self> {
        let kuni_repo = Arc::new(InMemoryKuniRepository::new());
        let daimyo_repo = Arc::new(InMemoryDaimyoRepository::new());
        let game_state_repo = Arc::new(InMemoryGameStateRepository::new());
        let event_dispatcher = Arc::new(InMemoryEventDispatcher::new());

        let neighbor_repo = Arc::new(InMemoryNeighborRepository::new());

        // ユースケースの初期化
        let domestic_usecase = DomesticUseCase::new(kuni_repo.clone(), neighbor_repo.clone());
        let battle_usecase = BattleUseCase::new(kuni_repo.clone(), neighbor_repo.clone());
        let turn_progression_usecase = TurnProgressionUseCase::new(
            kuni_repo.clone(),
            daimyo_repo.clone(),
            game_state_repo.clone(),
            event_dispatcher.clone(),
        );

        Ok(Self {
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
            current_kuni: None,
            current_daimyo: None,
            all_daimyos: Vec::new(),
            current_turn: None,
            messages: Vec::new(),
            attacker_kuni: None,
            defender_kuni: None,
            kuni_names: std::collections::HashMap::new(),
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        let base_dir = Path::new("static/master_data");
        let bundle = MasterDataLoader::load(base_dir)?;

        self.kuni_repo.init_with_data(bundle.kunis).await;
        self.daimyo_repo.init_with_data(bundle.daimyos).await;
        self.neighbor_repo.init_with_data(bundle.adjacency_map);

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

            if let Some(Event::Key(key)) = get_event(Duration::from_millis(16))? {
                if key.kind == KeyEventKind::Press {
                    EventHandler::handle_key(self, key).await?;
                }
            }
        }
        Ok(())
    }
}
