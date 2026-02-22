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
    pub currency_id: String,
    pub currency_name: String,
    pub currency_symbol: String,
    pub currency_amount: i32,
    pub merchant_currency_amount: Option<i32>,
    pub buy_categories: Vec<String>,
    pub sell_categories: Vec<String>,
    pub buy_items: Vec<ShopItem>,
    pub sell_items: Vec<ShopItem>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShopMode {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShopItemKind {
    Item,
    Equipment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShopSelection {
    pub mode: ShopMode,
    pub index: usize,
}

pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub price: i32,
    pub details: MenuPanelView,
    pub owned: i32,
    pub max: i32,
    pub category: String,
    pub stock: Option<i32>,
    pub enabled: bool,
    pub kind: ShopItemKind,
}

fn format_currency_amount(shop: &ShopView, amount: i32) -> String {
    if shop.currency_symbol.trim().is_empty() {
        format!("{} {}", amount, shop.currency_name)
    } else {
        format!("{}{}", shop.currency_symbol, amount)
    }
}

pub fn draw_shop_frame(
    frame: &mut Frame,
    shop: &ShopView,
    mode: ShopMode,
    categories: &[String],
    category_index: usize,
    filtered_indices: &[usize],
    selected_pos: Option<usize>,
) {
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

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner_area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[2]);

    let list_area = chunks[0];
    let details_area = chunks[1];
    let items = match mode {
        ShopMode::Buy => &shop.buy_items,
        ShopMode::Sell => &shop.sell_items,
    };

    let mut list_lines = Vec::new();
    list_lines.push(Line::from(Span::styled(
        format!(
            "Currency: {}",
            format_currency_amount(shop, shop.currency_amount)
        ),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    if let Some(amount) = shop.merchant_currency_amount {
        list_lines.push(Line::from(Span::styled(
            format!("Merchant: {}", format_currency_amount(shop, amount)),
            Style::default().fg(Color::Cyan),
        )));
    }
    list_lines.push(Line::from(""));

    let header_lines = list_lines.len();
    let available_lines = list_area.height.saturating_sub(header_lines as u16) as usize;
    let page_size = available_lines.max(1);
    let total = filtered_indices.len();
    let selected_index = selected_pos.unwrap_or(0);
    let page = if total == 0 {
        0
    } else {
        selected_index / page_size
    };
    let total_pages = if total == 0 {
        0
    } else {
        (total + page_size - 1) / page_size
    };
    let start = page.saturating_mul(page_size);
    let end = (start + page_size).min(total);

    let mut mode_spans = Vec::new();
    mode_spans.push(Span::raw("Mode: "));
    let buy_style = if mode == ShopMode::Buy {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let sell_style = if mode == ShopMode::Sell {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    mode_spans.push(Span::styled("Buy", buy_style));
    mode_spans.push(Span::raw(" | "));
    mode_spans.push(Span::styled("Sell", sell_style));
    mode_spans.push(Span::raw("   Toggle: "));
    mode_spans.push(Span::styled(
        "Pause",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    if total_pages > 1 {
        mode_spans.push(Span::raw("   "));
        mode_spans.push(Span::styled(
            format!("< {}/{} >", page + 1, total_pages),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let header = Paragraph::new(Line::from(mode_spans))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(header, rows[0]);

    let category_label = categories
        .get(category_index)
        .map(|value| value.as_str())
        .unwrap_or("All");
    let category_line = Line::from(vec![
        Span::raw("Category: "),
        Span::styled(
            category_label,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   (Left/Right)").style(Style::default().fg(Color::DarkGray)),
    ]);
    let category_header = Paragraph::new(category_line)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(category_header, rows[1]);

    if filtered_indices.is_empty() {
        list_lines.push(Line::from(Span::styled(
            "No items.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (index, item_index) in filtered_indices
            .iter()
            .enumerate()
            .skip(start)
            .take(end.saturating_sub(start))
        {
            let item = &items[*item_index];
            let is_selected = Some(index) == selected_pos;
            let prefix = if is_selected { "> " } else { "  " };
            let mut style = if item.enabled {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if is_selected {
                style = if item.enabled {
                    style.fg(Color::Yellow)
                } else {
                    style.fg(Color::DarkGray)
                }
                .add_modifier(Modifier::BOLD);
            }

            let text = match mode {
                ShopMode::Buy => {
                    let mut line = format!(
                        "{}{: <16} {:>5} ({}/{})",
                        prefix,
                        item.name,
                        format_currency_amount(shop, item.price),
                        item.owned,
                        item.max
                    );
                    if let Some(stock) = item.stock {
                        line.push_str(" Stock: ");
                        line.push_str(&stock.to_string());
                    }
                    line
                }
                ShopMode::Sell => format!(
                    "{}{: <16} {:>5} x{}",
                    prefix,
                    item.name,
                    format_currency_amount(shop, item.price),
                    item.owned
                ),
            };
            list_lines.push(Line::from(Span::styled(text, style)));
        }
    }

    let list_widget = Paragraph::new(list_lines).block(Block::default().borders(Borders::RIGHT));
    frame.render_widget(list_widget, list_area);

    let selected_item = selected_pos
        .and_then(|pos| filtered_indices.get(pos))
        .and_then(|index| items.get(*index));
    if let Some(item) = selected_item {
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
) -> io::Result<Option<ShopSelection>> {
    let mut mode = ShopMode::Buy;
    let mut category_index = 0usize;
    let mut selected_buy = 0usize;
    let mut selected_sell = 0usize;
    loop {
        let items = match mode {
            ShopMode::Buy => &shop.buy_items,
            ShopMode::Sell => &shop.sell_items,
        };
        let categories = match mode {
            ShopMode::Buy => &shop.buy_categories,
            ShopMode::Sell => &shop.sell_categories,
        };
        if !categories.is_empty() {
            category_index = category_index.min(categories.len().saturating_sub(1));
        } else {
            category_index = 0;
        }
        let category_label = categories
            .get(category_index)
            .map(|value| value.as_str())
            .unwrap_or("All");
        let filtered_indices = filter_shop_indices(items, category_label);

        let selected_index = match mode {
            ShopMode::Buy => &mut selected_buy,
            ShopMode::Sell => &mut selected_sell,
        };
        let selected_pos = if filtered_indices.is_empty() {
            *selected_index = 0;
            None
        } else {
            if !filtered_indices.contains(selected_index) {
                *selected_index = filtered_indices[0];
            }
            filtered_indices
                .iter()
                .position(|index| index == selected_index)
        };

        session.terminal_mut().draw(|frame| {
            draw_shop_frame(
                frame,
                shop,
                mode,
                categories,
                category_index,
                &filtered_indices,
                selected_pos,
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        if let Some(pos) = selected_pos {
                            if pos > 0 {
                                *selected_index = filtered_indices[pos - 1];
                            }
                        }
                    }
                    Action::MoveDown => {
                        if let Some(pos) = selected_pos {
                            if pos + 1 < filtered_indices.len() {
                                *selected_index = filtered_indices[pos + 1];
                            }
                        }
                    }
                    Action::MoveLeft => {
                        if categories.len() > 1 {
                            if let Some(index) = show_category_picker(
                                session,
                                shop,
                                mode,
                                categories,
                                category_index,
                                bindings,
                            )? {
                                category_index = index;
                            }
                        }
                    }
                    Action::MoveRight => {
                        if categories.len() > 1 {
                            if let Some(index) = show_category_picker(
                                session,
                                shop,
                                mode,
                                categories,
                                category_index,
                                bindings,
                            )? {
                                category_index = index;
                            }
                        }
                    }
                    Action::Pause => {
                        mode = match mode {
                            ShopMode::Buy => ShopMode::Sell,
                            ShopMode::Sell => ShopMode::Buy,
                        };
                        category_index = 0;
                    }
                    Action::Confirm => {
                        if let Some(pos) = selected_pos {
                            if let Some(item_index) = filtered_indices.get(pos) {
                                if items
                                    .get(*item_index)
                                    .map(|item| item.enabled)
                                    .unwrap_or(false)
                                {
                                    return Ok(Some(ShopSelection {
                                        mode,
                                        index: *item_index,
                                    }));
                                }
                            }
                        }
                    }
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

fn filter_shop_indices(items: &[ShopItem], category: &str) -> Vec<usize> {
    if category == "All" {
        return (0..items.len()).collect();
    }
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.category == category)
        .map(|(index, _)| index)
        .collect()
}

pub fn show_category_picker(
    session: &mut TuiSession,
    shop: &ShopView,
    mode: ShopMode,
    categories: &[String],
    current_index: usize,
    bindings: &InputBindings,
) -> io::Result<Option<usize>> {
    if categories.is_empty() {
        return Ok(None);
    }
    let mut selected = current_index.min(categories.len().saturating_sub(1));
    loop {
        let items = match mode {
            ShopMode::Buy => &shop.buy_items,
            ShopMode::Sell => &shop.sell_items,
        };
        let current_label = categories
            .get(current_index)
            .map(|value| value.as_str())
            .unwrap_or("All");
        let filtered = filter_shop_indices(items, current_label);
        let selected_pos = None;

        session.terminal_mut().draw(|frame| {
            draw_shop_frame(
                frame,
                shop,
                mode,
                categories,
                current_index,
                &filtered,
                selected_pos,
            );

            let popup_height =
                (categories.len() as u16 + 4).min(frame.size().height.saturating_sub(2));
            let area = centered_rect(frame.size(), 50, popup_height);
            frame.render_widget(Clear, area);

            let block = Block::default().borders(Borders::ALL).title("Categories");
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let mut lines = Vec::new();
            for (index, label) in categories.iter().enumerate() {
                let is_selected = index == selected;
                let prefix = if is_selected { "> " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, label),
                    style,
                )));
            }
            let list = Paragraph::new(lines)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(list, inner);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp | Action::MoveLeft => {
                        if selected > 0 {
                            selected -= 1;
                        } else {
                            selected = categories.len().saturating_sub(1);
                        }
                    }
                    Action::MoveDown | Action::MoveRight => {
                        if selected + 1 < categories.len() {
                            selected += 1;
                        } else {
                            selected = 0;
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
    mode: ShopMode,
    selected_index: usize,
    bindings: &InputBindings,
    title: &str,
    price_label: &str,
    max_quantity: i32,
) -> io::Result<Option<i32>> {
    let mut quantity = 1;
    let items = match mode {
        ShopMode::Buy => &shop.buy_items,
        ShopMode::Sell => &shop.sell_items,
    };
    let categories = match mode {
        ShopMode::Buy => &shop.buy_categories,
        ShopMode::Sell => &shop.sell_categories,
    };
    let item = &items[selected_index];
    let max_val = max_quantity;

    loop {
        session.terminal_mut().draw(|frame| {
            let filtered = filter_shop_indices(items, "All");
            let selected_pos = filtered.iter().position(|index| *index == selected_index);
            draw_shop_frame(frame, shop, mode, categories, 0, &filtered, selected_pos);

            let area = centered_rect(frame.size(), 40, 12);
            frame.render_widget(Clear, area);

            let block = Block::default().borders(Borders::ALL).title(title);

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
                Line::from(format!(
                    "{}: {}",
                    price_label,
                    format_currency_amount(shop, item.price)
                )),
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
                        format_currency_amount(shop, item.price * quantity),
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
    mode: ShopMode,
    selected_index: usize,
    bindings: &InputBindings,
    title: &str,
    message: &str,
) -> io::Result<()> {
    loop {
        session.terminal_mut().draw(|frame| {
            let items = match mode {
                ShopMode::Buy => &shop.buy_items,
                ShopMode::Sell => &shop.sell_items,
            };
            let categories = match mode {
                ShopMode::Buy => &shop.buy_categories,
                ShopMode::Sell => &shop.sell_categories,
            };
            let filtered = filter_shop_indices(items, "All");
            let selected_pos = filtered.iter().position(|index| *index == selected_index);
            draw_shop_frame(frame, shop, mode, categories, 0, &filtered, selected_pos);

            let area = centered_rect(frame.size(), 50, 10);
            frame.render_widget(Clear, area);

            let block = Block::default().borders(Borders::ALL).title(title);
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
