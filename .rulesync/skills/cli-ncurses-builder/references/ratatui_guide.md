# Ratatui / Crossterm TUI Guide

When creating ncurses-style interfaces in Rust that use `stdin` and `stdout`, `ratatui` (formerly `tui-rs`) paired with `crossterm` is the modern standard. 
本ガイドでは、CLI アプリをシステム全体（DDD、ECS、単純なステートマシン等）に統合する構成手順を説明します。

## Setup
Add these to `Cargo.toml`. `tokio` is often needed if the underlying architecture uses async (e.g., async UseCases in DDD).
```toml
crossterm = "0.27"
ratatui = "0.26"
tokio = { version = "1.0", features = ["full"] } # If async is required
# Your specific domain dependencies:
# engine = { path = "../engine" } 
```

## Core Architecture Integration

The most important rule in CLI apps is separating UI from Business Logic. The CLI tool is purely a Presentation Layer.

### 状態保持構造体（`App`）の作成
`ratatui` ではメインループの状態を `App` 構造体などに集約するのが一般的です。ここにアーキテクチャごとのコアステートを持たせます。

**Pattern A: Domain-Driven Design (DDD) & Dependency Injection**
```rust
struct App {
    // UIはユースケースに依存し、リポジトリの具象型を知らないかジェネリクスで隠蔽する
    turn_usecase: TurnUseCase<DummyKuniRepository>,
    ui_state: UiState,
}
```

**Pattern B: Entity-Component System (ECS)**
```rust
struct App {
    // ゲームのすべてを含むECS Worldを保持し、UIループからティックを進める
    world: hecs::World, // or bevy_ecs::World etc.
    ui_state: UiState,
}
```

**Pattern C: Simple Model / Controller**
```rust
struct App {
    // シンプルなゲームステート
    game_state: GameState,
    ui_state: UiState,
}
```

### Composition Root (メイン関数)での初期化
`main` 関数で、アーキテクチャに応じた初期化（DIの組み立て、ECS Worldの構築など）を行い、`App` 構造体に格納してメインループに渡します。

```rust
// #[tokio::main] // 非同期が必要な場合（DDD等）
fn main() -> Result<(), Box<dyn Error>> {
    // アーキテクチャ固有の初期化
    // 例(DDD): let repo = Arc::new(RepoImpl::new()); let uc = UseCase::new(repo);
    // 例(ECS): let mut world = World::new(); world.spawn(...);

    // 3. アプリケーションStateへ格納
    let mut app = App {
        // uc / world / game_state
        ui_state: UiState::MainMenu,
    };

    // ... terminalの開始 ...
    // ... run_app(&mut terminal, &mut app)?;
}
```

### 操作イベントハンドリング
イベントループの中でユーザー入力を受け取り、コアロジックを呼び出します。
- **DDD**: ユースケースのメソッドを非同期/同期で呼び出す。
- **ECS**: コマンドやイベントをキューに積み、メインシミュレーションを1 tick進める (`world.run_systems()`)。

## Summary
- プロジェクトのアーキテクチャ（`AGENTS.md`や`project.md`に記載）を必ず読み解く。
- ドメインのロジックはCLI側に書かず、必ず既存のエンジン層へ処理を移譲する（`assets/ratatui_template.rs` 参照）。
