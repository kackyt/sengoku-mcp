use anyhow::Result;
use cli::app::App;
#[cfg(not(feature = "ai-debug"))]
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use engine::application::usecase::{
    battle_usecase::BattleUseCase, domestic_usecase::DomesticUseCase,
    kuni_query_usecase::KuniQueryUseCase, turn_progression_usecase::TurnProgressionUseCase,
};
use infrastructure::master_data::MasterDataLoader;
use infrastructure::persistence::{
    InMemoryDaimyoRepository, InMemoryEventDispatcher, InMemoryGameStateRepository,
    InMemoryKuniRepository, InMemoryNeighborRepository,
};
use ratatui::prelude::*;
use std::path::Path;
use std::sync::Arc;
#[cfg(not(feature = "ai-debug"))]
use std::{io, panic};

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(feature = "ai-debug")]
    {
        run_ai_debug().await
    }
    #[cfg(not(feature = "ai-debug"))]
    {
        // ターミナルの初期化
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // パニック時のクリーンアップ処理
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen);
            original_hook(panic_info);
        }));

        // アプリケーションの構築 (Composition Root)
        let app = build_app().await?;
        let mut app = app;

        // メインループの実行
        let get_event = |_timeout: std::time::Duration| -> Result<Option<crossterm::event::Event>> {
            if crossterm::event::poll(_timeout)? {
                Ok(Some(crossterm::event::read()?))
            } else {
                Ok(None)
            }
        };
        let on_draw = |_terminal: &mut Terminal<CrosstermBackend<io::Stdout>>| {};
        let res = app.run(&mut terminal, get_event, on_draw).await;

        // クリーンアップ
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            eprintln!("{:?}", err);
        }

        Ok(())
    }
}

/// アプリケーションと依存関係を構築する (Composition Root)
async fn build_app() -> Result<App> {
    let kuni_repo = Arc::new(InMemoryKuniRepository::new());
    let daimyo_repo = Arc::new(InMemoryDaimyoRepository::new());
    let game_state_repo = Arc::new(InMemoryGameStateRepository::new());
    let event_dispatcher = Arc::new(InMemoryEventDispatcher::new());
    let neighbor_repo = Arc::new(InMemoryNeighborRepository::new());

    // マスターデータのロードと初期化
    let base_dir = Path::new("static/master_data");
    let bundle = MasterDataLoader::load(base_dir)?;

    kuni_repo.init_with_data(bundle.kunis).await;
    daimyo_repo.init_with_data(bundle.daimyos).await;
    neighbor_repo.init_with_data(bundle.adjacency_map);

    // ユースケースの構築
    let domestic_usecase = DomesticUseCase::new(kuni_repo.clone(), neighbor_repo.clone());
    let battle_usecase = BattleUseCase::new(kuni_repo.clone(), neighbor_repo.clone());
    let turn_progression_usecase = TurnProgressionUseCase::new(
        kuni_repo.clone(),
        daimyo_repo.clone(),
        game_state_repo.clone(),
        event_dispatcher.clone(),
    );
    let kuni_query_usecase = KuniQueryUseCase::new(kuni_repo.clone(), neighbor_repo.clone());

    Ok(App::new(
        kuni_repo,
        daimyo_repo,
        game_state_repo,
        neighbor_repo,
        event_dispatcher,
        domestic_usecase,
        battle_usecase,
        turn_progression_usecase,
        kuni_query_usecase,
    ))
}

#[cfg(feature = "ai-debug")]
async fn run_ai_debug() -> Result<()> {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use ratatui::backend::TestBackend;
    use std::io::{self, BufRead};
    use std::sync::atomic::{AtomicBool, Ordering};

    let mut app = build_app().await?;
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend)?;

    println!("--- AI TUI Debugger Started ---");

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    static SHOULD_DUMP: AtomicBool = AtomicBool::new(false);

    let get_event = |_timeout: std::time::Duration| -> Result<Option<Event>> {
        if let Some(Ok(line_str)) = lines.next() {
            let cmd = line_str.trim();
            if cmd == "q" || cmd == "quit" {
                return Err(anyhow::anyhow!("Quit requested by AI"));
            }
            if cmd == "dump" {
                SHOULD_DUMP.store(true, Ordering::SeqCst);
                // ダミーキーを送ってループを1周回し、on_drawを発火させる
                return Ok(Some(Event::Key(KeyEvent {
                    code: KeyCode::Null,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                })));
            }
            let key_code = match cmd {
                "up" => Some(KeyCode::Up),
                "down" => Some(KeyCode::Down),
                "left" => Some(KeyCode::Left),
                "right" => Some(KeyCode::Right),
                "enter" => Some(KeyCode::Enter),
                "esc" => Some(KeyCode::Esc),
                "space" => Some(KeyCode::Char(' ')),
                "" => None,
                s if s.chars().count() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
                _ => None,
            };
            if let Some(code) = key_code {
                return Ok(Some(Event::Key(KeyEvent {
                    code,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                })));
            }
        }
        Ok(None)
    };

    let on_draw = |term: &mut Terminal<TestBackend>| {
        if SHOULD_DUMP.swap(false, Ordering::SeqCst) {
            let buffer = term.backend().buffer();
            println!("=== SCREEN BUFFER DUMP ===");
            for y in 0..buffer.area.height {
                let mut line = String::with_capacity(buffer.area.width as usize);
                for x in 0..buffer.area.width {
                    let cell = &buffer[(x, y)];
                    if !cell.symbol().is_empty() {
                        line.push_str(cell.symbol());
                    }
                }
                println!("{}", line.trim_end());
            }
            println!("==========================");
        }
    };

    if let Err(err) = app.run(&mut terminal, get_event, on_draw).await {
        println!("Debugger exited: {:?}", err);
    }
    Ok(())
}
