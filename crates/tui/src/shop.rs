use std::io::{self, ErrorKind};

use crossterm::event::{self, Event};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::dialog::confirm_quit;
use crate::input::{Action, InputBindings};
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

pub fn show_shop(
    session: &mut TuiSession,
    shop: &ShopView,
    bindings: &InputBindings,
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        session.terminal_mut().draw(|frame| {
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

            let list_widget =
                Paragraph::new(list_lines).block(Block::default().borders(Borders::RIGHT));
            frame.render_widget(list_widget, list_area);

            if let Some(item) = shop.items.get(selected) {
                let detail_lines: Vec<Line> =
                    item.details.lines.iter().map(render_panel_line).collect();
                let detail_widget = Paragraph::new(detail_lines)
                    .block(Block::default().title("Details").borders(Borders::NONE))
                    .wrap(Wrap { trim: false });
                frame.render_widget(detail_widget, details_area);
            }
        })?;

        if let Event::Key(key) = event::read()? {
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
