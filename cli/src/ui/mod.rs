use crate::app::App;
use crate::screen::{DomesticCommand, DomesticSubState, ScreenState};
use engine::domain::model::value_objects::DisplayAmount;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(app: &App, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(app, f, chunks[0]);
    render_footer(app, f, chunks[2]);

    match &app.screen {
        ScreenState::Title => render_title(f, chunks[1]),
        ScreenState::SelectDaimyo { cursor } => render_select_daimyo(app, f, chunks[1], *cursor),
        ScreenState::Domestic {
            selected_kuni,
            cursor,
            sub_state,
        } => render_domestic(app, f, chunks[1], *selected_kuni, *cursor, sub_state),
        ScreenState::War {
            cursor, sub_state, ..
        } => render_war(app, f, chunks[1], *cursor, sub_state),
        ScreenState::GameOver { winner } => render_game_over(app, f, chunks[1], *winner),
    }

    render_modals(app, f);
}

fn render_header(app: &App, f: &mut Frame, area: Rect) {
    let turn = app.current_turn.unwrap_or(1);
    let year = (turn - 1) / 4 + 1560;
    let season = match (turn - 1) % 4 {
        0 => "春",
        1 => "夏",
        2 => "秋",
        3 => "冬",
        _ => "春",
    };
    let text = format!("戦国統一 - Sengoku Tounitsu ({}年 {})", year, season);
    let header = Paragraph::new(text)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_footer(app: &App, f: &mut Frame, area: Rect) {
    if !app.is_player_turn() {
        match &app.screen {
            ScreenState::Domestic { .. } | ScreenState::War { .. } => {
                let footer = Paragraph::new("他の大名が行動中です...")
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::ITALIC),
                    )
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(footer, area);
                return;
            }
            _ => {}
        }
    }

    let footer_text = match &app.screen {
        ScreenState::Title => "Enter: 開始 | Esc/q: 終了",
        ScreenState::SelectDaimyo { .. } => "↑/↓: 選択 | Enter: 決定 | Esc: 戻る",
        ScreenState::Domestic { sub_state, .. } => match sub_state {
            DomesticSubState::Normal => "↑/↓: 選択 | Enter: 決定 | Esc: 戻る",
            DomesticSubState::InputAmount { .. } => "数字: 入力 | Enter: 決定 | Esc: 戻る",
            DomesticSubState::SelectTargetKuni { .. } => "↑/↓: 選択 | Enter: 決定 | Esc: 戻る",
            DomesticSubState::ShowMessage { .. } => "Enter/Esc: 閉じる",
        },
        ScreenState::War { .. } => "Enter: 戦闘開始 | Esc: 戻る",
        ScreenState::GameOver { .. } => "Enter/Esc: タイトルへ",
    };
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title_art = r#"
    _______  _______  __    _  _______  _______  ___   _  __   __ 
    |       ||       ||  |  | ||       ||       ||   | | ||  | |  |
    |  _____||    ___||   |_| ||    ___||    _  ||   |_| ||  | |  |
    | |_____ |   |___ |       ||   | __ |   |_| ||      _||  |_|  |
    |_____  ||    ___||  _    ||   ||  ||    ___||     |_ |       |
     _____| ||   |___ | | |   ||   |_| ||   |    |    _  ||       |
    |_______||_______||_|  |__||_______||___|    |___| |_||_______|
    "#;

    let paragraph = Paragraph::new(title_art)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, area);
}

fn render_select_daimyo(app: &App, f: &mut Frame, area: Rect, cursor: usize) {
    let items: Vec<ListItem> = app
        .all_daimyos
        .iter()
        .map(|d| ListItem::new(d.name.0.clone()))
        .collect();

    let list = List::new(items)
        .block(Block::default().title("大名選択").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(cursor));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_domestic(
    app: &App,
    f: &mut Frame,
    area: Rect,
    _kuni_id: engine::domain::model::value_objects::KuniId,
    cursor: usize,
    _sub_state: &DomesticSubState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Status
    let (kin, kome, hei, jinko, koko, machi, tyu) = if let Some(kuni) = &app.current_kuni {
        (
            kuni.resource.kin.to_display(),
            kuni.resource.kome.to_display(),
            kuni.resource.hei.to_display(),
            kuni.resource.jinko.to_display(),
            kuni.stats.kokudaka.to_display(),
            kuni.stats.machi.to_display(),
            kuni.stats.tyu.value(),
        )
    } else {
        (
            DisplayAmount::new(0),
            DisplayAmount::new(0),
            DisplayAmount::new(0),
            DisplayAmount::new(0),
            DisplayAmount::new(0),
            DisplayAmount::new(0),
            0,
        )
    };

    let daimyo_name = app
        .current_daimyo
        .as_ref()
        .map(|d| d.name.0.as_str())
        .unwrap_or("不明");

    let status_text = vec![
        Line::from(vec![
            Span::raw("大名: "),
            Span::styled(daimyo_name, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![Span::raw("金: "), Span::raw(kin.to_string())]),
        Line::from(vec![Span::raw("米: "), Span::raw(kome.to_string())]),
        Line::from(vec![Span::raw("兵: "), Span::raw(hei.to_string())]),
        Line::from(vec![Span::raw("人口: "), Span::raw(jinko.to_string())]),
        Line::from(vec![Span::raw("石高: "), Span::raw(koko.to_string())]),
        Line::from(vec![Span::raw("町  : "), Span::raw(machi.to_string())]),
        Line::from(vec![Span::raw("忠誠: "), Span::raw(tyu.to_string())]),
    ];
    let status =
        Paragraph::new(status_text).block(Block::default().title("領地情報").borders(Borders::ALL));
    f.render_widget(status, chunks[0]);

    // Right: Commands (プレイヤーの手番の時だけ表示)
    if app.is_player_turn() {
        let commands = vec![
            ListItem::new(" 1. 戦争"),
            ListItem::new(" 2. 米売り"),
            ListItem::new(" 3. 米買い"),
            ListItem::new(" 4. 開墾"),
            ListItem::new(" 5. 町造り"),
            ListItem::new(" 6. 雇用"),
            ListItem::new(" 7. 解雇"),
            ListItem::new(" 8. 施し"),
            ListItem::new(" 9. 輸送"),
            ListItem::new("10. 委任"),
            ListItem::new("11. 解任"),
            ListItem::new("12. 情報"),
            ListItem::new("13. 終了"),
        ];
        let list = List::new(commands)
            .block(Block::default().title("行動メニュー").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(cursor));
        f.render_stateful_widget(list, chunks[1], &mut state);
    } else {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                " 他の大名が行動中です...",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(" しばらくお待ちください。"),
        ];
        let p = Paragraph::new(text)
            .block(Block::default().title("状況").borders(Borders::ALL))
            .alignment(Alignment::Center);
        f.render_widget(p, chunks[1]);
    }
}

fn render_war(
    app: &App,
    f: &mut Frame,
    area: Rect,
    cursor: usize,
    sub_state: &crate::screen::WarSubState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Attacker
    if let Some(attacker) = &app.attacker_kuni {
        let attacker_status = vec![
            Line::from(vec![Span::raw("自軍")]),
            Line::from(vec![
                Span::raw("兵力: "),
                Span::raw(attacker.resource.hei.to_display().to_string()),
            ]),
            Line::from(vec![
                Span::raw("食料: "),
                Span::raw(attacker.resource.kome.to_display().to_string()),
            ]),
            Line::from(vec![
                Span::raw("士気: "),
                Span::raw(attacker.stats.tyu.value().to_string()),
            ]),
        ];
        let attacker_p = Paragraph::new(attacker_status).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
        f.render_widget(attacker_p, chunks[0]);
    }

    // Right: Defender
    if let Some(defender) = &app.defender_kuni {
        let defender_status = vec![
            Line::from(vec![Span::raw("敵軍")]),
            Line::from(vec![
                Span::raw("兵力: "),
                Span::raw(defender.resource.hei.to_display().to_string()),
            ]),
            Line::from(vec![
                Span::raw("食料: "),
                Span::raw(defender.resource.kome.to_display().to_string()),
            ]),
            Line::from(vec![
                Span::raw("士気: "),
                Span::raw(defender.stats.tyu.value().to_string()),
            ]),
        ];
        let defender_p = Paragraph::new(defender_status).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        );
        f.render_widget(defender_p, chunks[1]);
    }

    // Tactic Selection Overlay
    if let crate::screen::WarSubState::SelectTactic = sub_state {
        let area = centered_rect(40, 30, area);
        f.render_widget(Clear, area);
        let tactics = vec![
            ListItem::new("1. 通常"),
            ListItem::new("2. 奇襲"),
            ListItem::new("3. 火計"),
            ListItem::new("4. 鼓舞"),
        ];
        let list = List::new(tactics)
            .block(Block::default().title("戦術選択").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(cursor));
        f.render_stateful_widget(list, area, &mut state);
    }
}

fn render_game_over(
    app: &App,
    f: &mut Frame,
    area: Rect,
    winner_id: engine::domain::model::value_objects::DaimyoId,
) {
    let winner_name = app
        .all_daimyos
        .iter()
        .find(|d| d.id == winner_id)
        .map(|d| d.name.0.as_str())
        .unwrap_or("勝者不明");

    let text = vec![
        Line::from(Span::styled(
            "全 国 統 一",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("{} 様が天下を平定されました！", winner_name)),
    ];
    let p = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn render_modals(app: &App, f: &mut Frame) {
    if let ScreenState::Domestic {
        selected_kuni,
        cursor: _,
        sub_state,
    } = &app.screen
    {
        match sub_state {
            DomesticSubState::InputAmount { command, input } => {
                render_input_modal(f, *command, input);
            }
            DomesticSubState::SelectTargetKuni { command, cursor } => {
                render_select_target_modal(app, f, *selected_kuni, *command, *cursor);
            }
            DomesticSubState::ShowMessage { message, .. } => {
                render_message_modal(f, message);
            }
            _ => {}
        }
    } else if let ScreenState::War {
        sub_state: crate::screen::WarSubState::ShowMessage { message, .. },
        ..
    } = &app.screen
    {
        render_message_modal(f, message);
    }
}

fn render_message_modal(f: &mut Frame, message: &str) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let p = Paragraph::new(message)
        .block(Block::default().title("結果").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn render_input_modal(f: &mut Frame, command: DomesticCommand, input: &str) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let title = match command {
        DomesticCommand::SellRice => "米売り (投入量入力)",
        DomesticCommand::BuyRice => "米買い (投入量入力)",
        DomesticCommand::Develop => "開墾 (資金入力)",
        DomesticCommand::BuildTown => "町造り (資金入力)",
        DomesticCommand::Hire => "雇用 (数入力)",
        DomesticCommand::Dismiss => "解雇 (数入力)",
        DomesticCommand::Give => "施し (量入力)",
        _ => "入力",
    };

    let p = Paragraph::new(format!("> {}", input)).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(p, area);
}

fn render_select_target_modal(
    app: &App,
    f: &mut Frame,
    kuni_id: engine::domain::model::value_objects::KuniId,
    command: DomesticCommand,
    cursor: usize,
) {
    let area = centered_rect(60, 40, f.area());
    f.render_widget(Clear, area);

    let neighbors = app.kuni_query_usecase.get_neighbor_ids(&kuni_id);
    let title = match command {
        DomesticCommand::War => "攻撃対象選択",
        DomesticCommand::Transport => "輸送先選択",
        _ => "対象選択",
    };

    if neighbors.is_empty() {
        let p = Paragraph::new("隣接する国がありません。")
            .block(Block::default().title(title).borders(Borders::ALL));
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = neighbors
        .iter()
        .map(|target_id| {
            let name = app
                .kuni_names
                .get(target_id)
                .cloned()
                .unwrap_or_else(|| "未知の国".to_string());
            ListItem::new(name)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(cursor));
    f.render_stateful_widget(list, area, &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
