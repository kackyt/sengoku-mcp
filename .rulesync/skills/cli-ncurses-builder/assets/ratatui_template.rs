use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{error::Error, io, sync::Arc};

// --- [ARCHITECTURE INTEGRATION] ---
// The exact imports and state structs depend on your project's architecture
// (e.g., DDD `usecases`, ECS `hecs::World`, or MVC `controllers`).
// Read `AGENTS.md` and `project.md` to identify the required structure.

/// The presentation layer state
struct App {
    // 1. [DDD] turn_usecase: TurnUseCase<DummyKuniRepository>,
    // 2. [ECS] world: World,
    // 3. [MVC] game_controller: GameController,
    
    // UI-specific view state
    action_counter: u32,
    should_quit: bool,
}

impl App {
    /// Initialize with injected core dependencies (DI, ECS World, etc.)
    fn new(/* core_dependencies */) -> Self {
        Self {
            action_counter: 0,
            should_quit: false,
        }
    }

    /// Event handler: Delegate user intents to core architectural logic.
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char(' ') => {
                // Example of delegating command:
                // [DDD] self.turn_usecase.progress_turn(1).await?;
                // [ECS] self.world.run_systems();
                self.action_counter += 1;
            }
            _ => {}
        }
    }
}

// NOTE: use #[tokio::main] if calling async use-cases (Common in DDD architectures).
fn main() -> Result<(), Box<dyn Error>> {
    // 1. Composition Root / Architecture Setup
    // Initialize Repositories and UseCases (DDD), or configure World and Systems (ECS).

    // 2. Create the App with integrated dependencies
    let mut app = App::new(/* dependencies */);

    // 3. Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 4. Main Event/Draw loop
    let res = run_app(&mut terminal, &mut app);

    // 5. Terminal Teardown
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
            if app.should_quit {
                return Ok(());
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let block = Block::default()
        .title("Generic CLI TUI (Press 'q' to quit, 'Space' to trigger action)")
        .borders(Borders::ALL);
    
    let text = format!("Simulated Actions Executed: {}\n\nPress Space to simulate usecase/system execution.", app.action_counter);
    let paragraph = Paragraph::new(text).block(block);
    
    f.render_widget(paragraph, chunks[0]);
}
