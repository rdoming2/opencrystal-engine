use std::io::{self, ErrorKind};

use crossterm::event::{self, Event};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::dialog::confirm_quit;
use crate::input::{is_actionable_key, Action, InputBindings};
use crate::menu::{render_panel_line, MenuPanelView};
use crate::session::TuiSession;
use crate::utils::centered_rect;

pub struct ShopView {
    pub name: String,
    pub currency: i32,
    pub items: Vec<ShopItem>,
}

pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub price: i32,
    pub details: MenuPanelView,
    pub owned: i32,
    pub max: i32,
}

pub fn draw_shop_frame(frame: &mut Frame, shop: &ShopView, selected: usize) {
    let size = frame.size();
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(size);

    let area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(vertical_layout[1])[1];

    frame.render_widget(Clear, area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .title(shop.name.as_str());
    frame.render_widget(main_block.clone(), area);

    let inner_area = main_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_area);

    let list_area = chunks[0];
    let details_area = chunks[1];

    let mut list_lines = Vec::new();
    list_lines.push(Line::from(Span::styled(
        format!("Currency: {} G", shop.currency),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    list_lines.push(Line::from(""));

    for (index, item) in shop.items.iter().enumerate() {
        let is_selected = index == selected;
        let prefix = if is_selected { "> " } else { "  " };
        let mut style = Style::default().fg(Color::White);

        if is_selected {
            style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
        }

        let text = format!(
            "{}{:<16} {:>4} G ({}/{})",
            prefix, item.name, item.price, item.owned, item.max
        );
        list_lines.push(Line::from(Span::styled(text, style)));
    }

    let list_widget = Paragraph::new(list_lines).block(Block::default().borders(Borders::RIGHT));
    frame.render_widget(list_widget, list_area);

    if let Some(item) = shop.items.get(selected) {
        let detail_lines: Vec<Line> = item.details.lines.iter().map(render_panel_line).collect();
        let detail_widget = Paragraph::new(detail_lines)
            .block(Block::default().title("Details").borders(Borders::NONE))
            .wrap(Wrap { trim: false });
        frame.render_widget(detail_widget, details_area);
    }
}

pub fn show_shop(
    session: &mut TuiSession,
    shop: &ShopView,
    bindings: &InputBindings,
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        session.terminal_mut().draw(|frame| {
            draw_shop_frame(frame, shop, selected);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    Action::MoveDown => {
                        if selected + 1 < shop.items.len() {
                            selected += 1;
                        }
                    }
                    Action::Confirm => return Ok(Some(selected)),
                    Action::Cancel | Action::Menu => return Ok(None),
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            let area = centered_rect(frame.size(), 50, 12);
                            frame.render_widget(Clear, area);
                            let paragraph = Paragraph::new(Line::from("Shop"))
                                .block(Block::default().borders(Borders::ALL))
                                .alignment(Alignment::Center);
                            frame.render_widget(paragraph, area);
                        })? {
                            return Err(io::Error::new(ErrorKind::Interrupted, "quit"));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn show_quantity_picker(
    session: &mut TuiSession,
    shop: &ShopView,
    selected_index: usize,
    bindings: &InputBindings,
    max_quantity: i32,
) -> io::Result<Option<i32>> {
    let mut quantity = 1;
    let item = &shop.items[selected_index];
    let max_val = max_quantity;

    loop {
        session.terminal_mut().draw(|frame| {
            draw_shop_frame(frame, shop, selected_index);

            let area = centered_rect(frame.size(), 40, 12);
            frame.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .title("Purchase")
                .style(Style::default().bg(Color::Black));

            frame.render_widget(block.clone(), area);

            let inner = block.inner(area);

            let lines = vec![
                Line::from(vec![
                    Span::raw("Item: "),
                    Span::styled(
                        &item.name,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(format!("Price: {} G", item.price)),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Quantity: < "),
                    Span::styled(
                        format!("{}", quantity),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" >"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Total: "),
                    Span::styled(
                        format!("{} G", item.price * quantity),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "(Left/Right -1/+1, Up/Down -10/+10)",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let p = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Spacer
                    Constraint::Min(0),    // Content
                    Constraint::Length(1), // Spacer
                ])
                .split(inner);

            frame.render_widget(p, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveLeft => {
                        quantity = (quantity - 1).max(1);
                    }
                    Action::MoveRight => {
                        quantity = (quantity + 1).min(max_val);
                    }
                    Action::MoveUp => {
                        quantity = (quantity - 10).max(1);
                    }
                    Action::MoveDown => {
                        quantity = (quantity + 10).min(max_val);
                    }
                    Action::Confirm => return Ok(Some(quantity)),
                    Action::Cancel => return Ok(None),
                    _ => {}
                }
            }
        }
    }
}

pub fn show_info_popup(
    session: &mut TuiSession,
    shop: &ShopView,
    selected_index: usize,
    bindings: &InputBindings,
    title: &str,
    message: &str,
) -> io::Result<()> {
    loop {
        session.terminal_mut().draw(|frame| {
            draw_shop_frame(frame, shop, selected_index);

            let area = centered_rect(frame.size(), 50, 10);
            frame.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().bg(Color::Black));
            frame.render_widget(block.clone(), area);

            let inner = block.inner(area);
            let p = Paragraph::new(message)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                ])
                .split(inner);

            frame.render_widget(p, layout[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::Confirm | Action::Cancel => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}
