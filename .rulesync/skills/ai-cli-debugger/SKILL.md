---
name: ai-cli-debugger
description: ratatuiやcrosstermを用いたTUI (CLI) アプリケーションをデバッグおよびテストするAI E2Eデバッガーをセットアップ・使用するためのガイドラインです。TUIアプリを自律的にデバッグする必要がある場合にこのスキルを適用します。
---

# AI TUI デバッガースキル

このスキルは、AI（Claude）がTUIアプリケーションを安全かつ確実にE2Eデバッグするための `ai-debug` Feature flag パターンの実装・使用方法を提示するものです。このパターンを導入することで、画面バッファを標準出力にダンプし、キー入力を標準入力から受け取る透過的な構成が可能になり、AIがターミナルアプリを自律的に操作・テストできるようになります。

## なぜこの構成が必要なのか？
`crossterm` や `ratatui` などを使ったTUIアプリケーションは、OSのコンソール入力バッファ（Windowsの `CONIN$` や Unixの `/dev/tty`）から直接イベントを読み取り、コンソールへ直接ANSIエスケープシーケンスを描画します。このため、バックグラウンド実行中にAIがプロセスの標準入力・標準出力を使ってTUIを直接操作・観測することは不可能です。これを解決するために、入力と描画を抽象化します。

## Step 1: Feature flag の追加（未実装の場合）

対象プロジェクトの `Cargo.toml` に `ai-debug` のフィーチャを追加します。
```toml
[features]
ai-debug = []
```

## Step 2: メインのイベントループのリファクタリング（未実装の場合）

イベントループ（`run_app` など）を抽象化し、キーボード入力（`event::read`）の取得と画面描画フック（`terminal.draw` の後処理）を `crossterm` 固有の実装から分離します。

1. `run_app` 関数が `get_event`（入力）と `on_draw`（描画フック）をクロージャとして受け取るように変更します：
```rust
fn run_app<B: ratatui::backend::Backend, E, D>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut App, // プロジェクトの独自状態に変更すること
    mut get_event: E,
    mut on_draw: D,
) -> std::io::Result<()>
where
    E: FnMut(std::time::Duration) -> std::io::Result<Option<crossterm::event::Event>>,
    D: FnMut(&ratatui::Terminal<B>),
{
    loop {
        terminal.draw(|f| ui::ui(f, app))?;
        on_draw(terminal); // AIデバッグ用の画面描画フック

        if let Some(event) = get_event(std::time::Duration::from_millis(50))? {
            // イベント処理（キーボード入力など）
        }
        // ... アプリのロジックループ
    }
}
```

2. 通常の `main()` 内では、今まで通りの `crossterm` 実装を渡します：
```rust
let get_event = |_timeout: std::time::Duration| -> std::io::Result<Option<crossterm::event::Event>> {
    if crossterm::event::poll(std::time::Duration::from_millis(50))? {
        Ok(Some(crossterm::event::read()?))
    } else {
        Ok(None)
    }
};
let on_draw = |_terminal: &ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>| {};
run_app(&mut terminal, &mut app, get_event, on_draw)?;
```

## Step 3: `ai-debug` モードの実装

`main.rs` 内に、AIデバッグ専用の関数を条件付きコンパイル（`#[cfg(feature = "ai-debug")]`）で定義します。
```rust
#[cfg(feature = "ai-debug")]
fn run_ai_debug() -> Result<(), Box<dyn std::error::Error>> {
    use ratatui::backend::TestBackend;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyEventState};
    use std::io::{self, BufRead};
    use std::sync::atomic::{AtomicBool, Ordering};

    let mut app = App::new(); // プロジェクトの独自状態に合わせる
    let backend = TestBackend::new(120, 30);
    let mut terminal = ratatui::Terminal::new(backend)?;

    println!("--- AI TUI Debugger Started ---");

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    static SHOULD_DUMP: AtomicBool = AtomicBool::new(false);

    let get_event = |_timeout: std::time::Duration| -> io::Result<Option<Event>> {
        if let Some(Ok(line_str)) = lines.next() {
            let cmd = line_str.trim();
            if cmd == "q" || cmd == "quit" {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "Quit requested by AI"));
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
        } else {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
        }
        Ok(None)
    };

    let on_draw = |term: &ratatui::Terminal<TestBackend>| {
        if SHOULD_DUMP.swap(false, Ordering::SeqCst) {
            let buffer = term.backend().buffer();
            println!("=== SCREEN BUFFER DUMP ===");
            for y in 0..buffer.area.height {
                let mut line = String::with_capacity(buffer.area.width as usize);
                for x in 0..buffer.area.width {
                    line.push_str(buffer.get(x, y).symbol());
                }
                println!("{}", line.trim_end()); // 右側の不要な空白を削除して出力サイズを削減
            }
            println!("==========================");
        }
    };

    if let Err(err) = run_app(&mut terminal, &mut app, get_event, on_draw) {
        println!("Debugger exited: {:?}", err);
    }
    Ok(())
}
```

この関数を `main()` 関数の先頭で実行するように構成します：
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "ai-debug")]
    {
        return run_ai_debug();
    }
    // ... 以降は通常実行ロジック
}
```

## Step 4: デバッガーの使用方法

上記の設定が完了したら、Claudeは同梱されているPythonスクリプト `run_ai_scenario.py` を使用して、デバッグシナリオを一括で実行します。

[run_ai_scenario.py](./scripts/run_ai_scenario.py)
```bash
python .rulesync/skills/ai-cli-debugger/scripts/run_ai_scenario.py "cargo run -p クレート名 --features ai-debug" --keys "5*right 3*down enter dump q"
```

**コマンドのポイント:**
- `--keys` にスペース区切りでキー入力のシーケンスを渡します。
- `N*key` というマクロ構文が使用でき、`5*right` なら `right` を5回分自動で展開して入力します。
- `dump` を指定したタイミングでのみ、`=== SCREEN BUFFER DUMP ===` として画面の最終状態が出力されます。シナリオの最後など、確認したいタイミングで `dump` を挟んでください。

これにより、cargoのコンパイル出力と画面情報が混ざることを防ぎ、AIのコンテキストを無駄に消費せず、長距離のカーソル移動や深いメニュー遷移をワンライナーで非常に効率よくテストできます。
