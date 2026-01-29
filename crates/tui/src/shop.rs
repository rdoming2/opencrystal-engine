use std::io::{self, ErrorKind};

use crossterm::event::{self, Event};
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::dialog::confirm_quit;
use crate::input::{Action, InputBindings};
use crate::session::TuiSession;
use crate::utils::centered_rect;

pub struct ShopView {
    pub name: String,
    pub items: Vec<ShopItem>,
}

pub struct ShopItem {
    pub name: String,
    pub price: i32,
}

pub fn show_shop(
    session: &mut TuiSession,
    shop: &ShopView,
    bindings: &InputBindings,
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        session.terminal_mut().draw(|frame| {
            let area = centered_rect(frame.size(), 50, 12);
            frame.render_widget(Clear, area);
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                shop.name.as_str(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            for (index, item) in shop.items.iter().enumerate() {
                let prefix = if index == selected { "> " } else { "  " };
                let text = format!("{}{} - {} G", prefix, item.name, item.price);
                lines.push(Line::from(Span::raw(text)));
            }

            lines.push(Line::from(Span::raw(" ")));
            lines.push(Line::from(Span::raw("Confirm to select, Cancel to exit.")));

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
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
