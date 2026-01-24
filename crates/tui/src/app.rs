use std::io::{self, Stdout};

use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
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

pub fn show_dialog(
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let lines = wrap_text(text, 80);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while let Some(page) = pages.pop() {
        draw_dialog(&mut terminal, dialog_ui, speaker, &page, None)?;
        wait_for_continue(bindings)?;
    }

    teardown_terminal(&mut terminal)
}

pub fn show_dialog_with_choices(
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
    choices: &[String],
) -> io::Result<Option<usize>> {
    let mut terminal = setup_terminal()?;
    let lines = wrap_text(text, 80);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while pages.len() > 1 {
        if let Some(page) = pages.pop() {
            draw_dialog(&mut terminal, dialog_ui, speaker, &page, None)?;
            wait_for_continue(bindings)?;
        }
    }

    let page = pages.pop().unwrap_or_default();
    let selection =
        choose_dialog_option(&mut terminal, dialog_ui, bindings, speaker, &page, choices)?;
    teardown_terminal(&mut terminal)?;
    Ok(selection)
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

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }
        if current.len() + word.len() + 1 > width {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn paginate_lines(
    mut lines: Vec<String>,
    dialog_ui: &crate::ui::DialogUiFile,
    speaker: &str,
) -> Vec<Vec<String>> {
    let mut pages = Vec::new();
    let available_lines = dialog_ui.height.saturating_sub(2) as usize;
    let speaker_offset = if dialog_ui.show_speaker && !speaker.is_empty() {
        1
    } else {
        0
    };
    let page_size = available_lines.saturating_sub(speaker_offset).max(1);

    while !lines.is_empty() {
        let count = page_size.min(lines.len());
        let page = lines.drain(0..count).collect::<Vec<_>>();
        pages.insert(0, page);
    }

    pages
}

fn dialog_area(area: Rect, dialog_ui: &crate::ui::DialogUiFile) -> Rect {
    let height = dialog_ui.height.min(area.height);
    match dialog_ui.position.as_str() {
        "top" => Rect::new(area.x, area.y, area.width, height),
        _ => Rect::new(
            area.x,
            area.y + area.height.saturating_sub(height),
            area.width,
            height,
        ),
    }
}

fn draw_dialog(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    dialog_ui: &crate::ui::DialogUiFile,
    speaker: &str,
    lines: &[String],
    choices: Option<(usize, &[String])>,
) -> io::Result<()> {
    terminal
        .draw(|frame| {
            let area = dialog_area(frame.size(), dialog_ui);
            let inner_width = area.width.saturating_sub(2) as usize;
            let mut content = Vec::new();

            if dialog_ui.show_speaker && !speaker.is_empty() {
                content.push(Line::from(Span::styled(
                    speaker,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
            }

            for line in lines {
                content.push(Line::from(Span::raw(truncate_line(line, inner_width))));
            }

            if let Some((selected, choices)) = choices {
                for (index, choice) in choices.iter().enumerate() {
                    let prefix = if index == selected { "> " } else { "  " };
                    let text = format!("{}{}", prefix, choice);
                    content.push(Line::from(Span::raw(truncate_line(&text, inner_width))));
                }
            }

            let paragraph = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        })
        .map(|_| ())
}

fn truncate_line(line: &str, width: usize) -> String {
    if line.len() <= width {
        return line.to_string();
    }
    let mut result = String::new();
    for ch in line.chars().take(width) {
        result.push(ch);
    }
    result
}

fn wait_for_continue(bindings: &InputBindings) -> io::Result<()> {
    loop {
        if let Event::Key(key) = event::read()? {
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::Confirm | Action::Cancel | Action::Menu => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

fn choose_dialog_option(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    lines: &[String],
    choices: &[String],
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        draw_dialog(
            terminal,
            dialog_ui,
            speaker,
            lines,
            Some((selected, choices)),
        )?;
        if let Event::Key(key) = event::read()? {
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    Action::MoveDown => {
                        if selected + 1 < choices.len() {
                            selected += 1;
                        }
                    }
                    Action::Confirm => return Ok(Some(selected)),
                    Action::Cancel | Action::Menu => return Ok(None),
                    _ => {}
                }
            }
        }
    }
}
