use std::io;

use crossterm::event::{self, Event};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::dialog::confirm_quit;
use crate::input::{Action, InputBindings};
use crate::session::TuiSession;
use crate::ui::TitleUiFile;

pub enum TitleAction {
    NewGame,
    Load,
    Settings,
    Exit,
}

pub fn run_title(
    session: &mut TuiSession,
    title_ui: &TitleUiFile,
    bindings: &InputBindings,
) -> io::Result<TitleAction> {
    let mut selected = 0usize;

    loop {
        session.terminal_mut().draw(|frame| {
            draw_title_frame(frame, title_ui, selected);
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
                        if selected + 1 < title_ui.menu.len() {
                            selected += 1;
                        }
                    }
                    Action::Confirm => return Ok(map_action(&title_ui.menu[selected].id)),
                    Action::Cancel | Action::Menu => return Ok(TitleAction::Exit),
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_title_frame(frame, title_ui, selected)
                        })? {
                            return Ok(TitleAction::Exit);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn draw_title_frame(frame: &mut Frame, title_ui: &TitleUiFile, selected: usize) {
    let size = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_ui.logo.lines.len() as u16 + 2),
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(size);

    let logo_lines: Vec<Line> = title_ui
        .logo
        .lines
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect();
    let logo = Paragraph::new(logo_lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(logo, layout[0]);

    let title = Paragraph::new(title_ui.title.as_str())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(title, layout[1]);

    let menu_items: Vec<Line> = title_ui
        .menu
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let mut style = Style::default().fg(Color::White);
            if index == selected {
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }
            Line::from(Span::styled(item.label.as_str(), style))
        })
        .collect();

    let menu = Paragraph::new(menu_items)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(menu, layout[2]);

    let footer = Paragraph::new(Line::from(vec![
        Span::raw(title_ui.footer.left.as_str()),
        Span::raw("  "),
        Span::styled(
            title_ui.footer.right.as_str(),
            Style::default().fg(Color::Gray),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(footer, layout[3]);
}

fn map_action(id: &str) -> TitleAction {
    match id {
        "load_game" => TitleAction::Load,
        "settings" => TitleAction::Settings,
        "exit" => TitleAction::Exit,
        _ => TitleAction::NewGame,
    }
}
