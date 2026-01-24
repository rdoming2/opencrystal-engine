use std::io::{self, Stdout};

use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use crate::input::{Action, InputBindings};
use crate::ui::TitleUiFile;

pub enum TitleAction {
    NewGame,
    Load,
    Settings,
    Exit,
}

pub fn run_title(title_ui: &TitleUiFile, bindings: &InputBindings) -> io::Result<TitleAction> {
    let mut terminal = setup_terminal()?;
    let mut selected = 0usize;

    loop {
        terminal.draw(|frame| {
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
                    Action::Confirm => {
                        let action = map_action(&title_ui.menu[selected].id);
                        teardown_terminal(&mut terminal)?;
                        return Ok(action);
                    }
                    Action::Cancel | Action::Menu => {
                        teardown_terminal(&mut terminal)?;
                        return Ok(TitleAction::Exit);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn map_action(id: &str) -> TitleAction {
    match id {
        "load_game" => TitleAction::Load,
        "settings" => TitleAction::Settings,
        "exit" => TitleAction::Exit,
        _ => TitleAction::NewGame,
    }
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
