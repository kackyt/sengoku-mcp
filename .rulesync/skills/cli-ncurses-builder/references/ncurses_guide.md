# Rust ncurses Guide

While `ratatui` is recommended for modern Rust applications, traditional approaches use the `ncurses` crate directly, similar to `rhex`.

## Setup
Add to `Cargo.toml`:
```toml
ncurses = "5.101.0"
```

## Core Architecture
```rust
use ncurses::*;

fn main() {
    // Setup
    initscr();
    raw();
    keypad(stdscr(), true);
    noecho();

    // Loop
    loop {
        clear();
        mvprintw(0, 0, "Hello, ncurses! (Press 'q' to quit)");
        refresh();

        let ch = getch();
        if ch == 'q' as i32 {
            break;
        }
    }

    // Teardown
    endwin();
}
```

*Note on cross-platform:* Pure `ncurses` requires C linking. On Windows, this may cause deployment pain depending on the environment. Consider `pancurses` instead if cross-platform C-bindings are strictly required, but pure Rust `crossterm`/`ratatui` is generally preferred over direct ncurses styling.
