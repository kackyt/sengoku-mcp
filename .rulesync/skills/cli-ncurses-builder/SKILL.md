---
name: cli-ncurses-builder
description: >-
  Guide for creating interactive ncurses-style CLI applications reading from stdin and writing to stdout in Rust, similar to rhex. Use this skill when asked to create an interactive terminal GUI, roguelike interface, or any CLI that requires ncurses, crossterm, or ratatui for rendering cells, handling keyboard/mouse input, or hexagonal grids. 
  It strictly enforces an architecture-discovery phase to ensure the CLI app integrates seamlessly with the project's specific architecture (e.g., DDD, ECS, MVC).
---
# CLI Ncurses Builder (Architecture Agnostic)

This skill provides patterns and templates for building interactive CLI tools in Rust using `stdin` and `stdout` for ncurses-like terminal user interfaces. The CLI app must act cleanly as a "Presentation Layer."

## Step 1: Architecture Discovery (Mandatory)

Before writing any CLI code, you MUST understand how the underlying game/application engine is structured. Rust projects employ various architectures (e.g., Domain-Driven Design (DDD), Entity-Component-System (ECS), MVC).

1. Read `AGENTS.md`, `project.md`, or similar convention files in the workspace.
2. If those do not exist or are vague, inspect the structure of the `engine` or `core` crate (e.g., look for `usecase/` directories for DDD, or `bevy`/`shipyard`/`specs` usages for ECS).
3. Determine how the CLI will interface with the logic:
   - **DDD**: The CLI app is the Composition Root. You must instantiate infrastructure repositories and inject them into application use-cases.
   - **ECS**: The CLI app may need to initialize an ECS `World` or `App` (e.g., Bevy App) and run a headless simulation loop alongside the UI loop.
   - **Simple Model**: The CLI app instantiates a central `GameState` struct.

## Step 2: Implementation Proposal (Mandatory)

Before proceeding with any implementation, you MUST use the `openspec-propose` skill (e.g. via `@[/opsx-propose]`) to generate the implementation proposal and tasks. Do not skip this!

1. Trigger the `openspec-propose` skill to create the change.
2. Set the description of the change to explicitly detail:
   - What UI components will be built.
   - Which specific UseCases, Repositories, or ECS Systems from the core application will be utilized or injected.
   - Any dummy/mock implementations needed if the infrastructure layer is not yet complete.
3. The `openspec-propose` skill will generate the necessary artifacts (e.g., `proposal.md`, `design.md`, `tasks.md`).
4. DO NOT proceed to Step 3 until the user approves the generated proposal and design.
5. **生成される各種ドキュメントは必ず日本語で出力されるように指示してください**

## Step 3: Modern Rust TUI Approach (Recommended)

When users ask for "ncurses" in Rust or a "CLI app", the modern standard is to use `ratatui` with `crossterm`. This avoids C-binding issues and provides a robust, pure-Rust way to draw to the terminal.

- **Architecture Guide**: Review [references/ratatui_guide.md](./references/ratatui_guide.md) to understand the event loop and how to integrate the project's specific architecture into the `App` state.
- **Base Cargo.toml**: Use [assets/ratatui_cargo.toml](./assets/ratatui_cargo.toml) to get started with dependencies (`ratatui`, `crossterm`, `tokio`).
- **Base Template**: Use [assets/ratatui_template.rs](./assets/ratatui_template.rs) for a functioning raw-mode TUI loop that shows how to hold generic state and run it non-blockingly.

### Implementation Workflow
1. Add dependencies from `ratatui_cargo.toml`. Add the necessary path dependencies for your core logic (`engine`, `infrastructure`, etc.).
2. In `main.rs`, perform the integration setup discovered in Step 1 (e.g., DI for DDD, World setup for ECS) as approved in Step 2.
3. Create an `App` state struct holding the initialized core state (UseCases, ECS World, or GameState).
4. Implement the `ui` loop: Render UI based on the `App` state.
5. In the event loop, delegate commands triggered by keys (e.g., Space to progress turn) to the core logic asynchronously or synchronously depending on the framework.

## Step 4: Legacy Approach (Direct ncurses bindings)

If the user strictly requests direct `ncurses` bindings (like the original `rhex` codebase), use the `ncurses-rs` crate.

- **Guide**: Review [references/ncurses_guide.md](./references/ncurses_guide.md) for a basic `ncurses` setup and loop in Rust. Note that you would still need to apply the Architecture Discovery principles discussed above.

### Important Notes
- `ratatui` is almost always better unless they have specific legacy C-linking requirements.
- Never write core domain/game logic inside the CLI rendering loop; always delegate to the underlying architecture.
