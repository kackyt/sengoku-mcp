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
use std::sync::Arc;
#[cfg(not(feature = "ai-debug"))]
use std::{io, panic};

#[cfg(not(feature = "ai-debug"))]
#[allow(dead_code)]
struct TerminalGuard;

#[cfg(not(feature = "ai-debug"))]
impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

#[cfg(not(feature = "ai-debug"))]
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        use std::io::Write;
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        let _ = std::io::stdout().flush();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(feature = "ai-debug")]
    {
        run_ai_debug().await
    }
    #[cfg(not(feature = "ai-debug"))]
    {
        // ターミナルの初期化（RAIIガード）
        let _guard = TerminalGuard::new()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        // パニック時のクリーンアップ処理（バックアップ的に維持）
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
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

        if let Err(ref err) = res {
            eprintln!("{:?}", err);
        }

        res
    }
}

/// アプリケーションと依存関係を構築する (Composition Root)
async fn build_app() -> Result<App> {
    let kuni_repo = Arc::new(InMemoryKuniRepository::new());
    let daimyo_repo = Arc::new(InMemoryDaimyoRepository::new());
    let game_state_repo = Arc::new(InMemoryGameStateRepository::new());
    let event_dispatcher = Arc::new(InMemoryEventDispatcher::new());
    let neighbor_repo = Arc::new(InMemoryNeighborRepository::new());
    let battle_repo = Arc::new(infrastructure::persistence::InMemoryBattleRepository::new());

    // マスターデータのロードと初期化
    let base_dir = if let Ok(env_path) = std::env::var("SENGOKU_MASTER_DATA") {
        std::path::PathBuf::from(env_path)
    } else {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dev_path = manifest_dir.join("../static/master_data");

        let exe_path = std::env::current_exe().ok();
        let exe_dir = exe_path.as_ref().and_then(|p| p.parent());
        let rel_path = exe_dir.map(|d| d.join("static/master_data"));

        if dev_path.exists() {
            dev_path
        } else if let Some(path) = rel_path.filter(|p| p.exists()) {
            path
        } else {
            let cwd_path = std::path::PathBuf::from("static/master_data");
            if cwd_path.exists() {
                cwd_path
            } else {
                anyhow::bail!(
                    "マスターデータが見つかりません。\n\
                    探索したパス:\n\
                    1. (env) SENGOKU_MASTER_DATA\n\
                    2. (dev) {:?}\n\
                    3. (rel) {:?}\n\
                    4. (cwd) static/master_data",
                    dev_path,
                    exe_dir.map(|d| d.join("static/master_data"))
                );
            }
        }
    };

    let bundle = MasterDataLoader::load(&base_dir)?;

    kuni_repo.init_with_data(bundle.kunis).await;
    daimyo_repo.init_with_data(bundle.daimyos).await;
    neighbor_repo.init_with_data(bundle.adjacency_map);

    // ユースケースの構築
    let domestic_usecase = DomesticUseCase::new(kuni_repo.clone(), neighbor_repo.clone());
    let battle_usecase =
        BattleUseCase::new(kuni_repo.clone(), neighbor_repo.clone(), battle_repo.clone());
    let turn_progression_usecase = TurnProgressionUseCase::new(
        kuni_repo.clone(),
        daimyo_repo.clone(),
        game_state_repo.clone(),
        event_dispatcher.clone(),
    );
    let kuni_query_usecase = KuniQueryUseCase::new(
        kuni_repo.clone(),
        daimyo_repo.clone(),
        game_state_repo.clone(),
        neighbor_repo.clone(),
    );

    Ok(App::new(
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
