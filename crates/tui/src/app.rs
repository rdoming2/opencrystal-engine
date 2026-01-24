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

pub struct TuiSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

pub struct MapView {
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<String>,
    pub npcs: Vec<NpcView>,
}

pub struct NpcView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
}

impl TuiSession {
    pub fn start() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn finish(mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

pub fn run_title(
    session: &mut TuiSession,
    title_ui: &TitleUiFile,
    bindings: &InputBindings,
) -> io::Result<TitleAction> {
    let mut selected = 0usize;

    loop {
        session.terminal.draw(|frame| {
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
                    Action::Confirm => return Ok(map_action(&title_ui.menu[selected].id)),
                    Action::Cancel | Action::Menu => return Ok(TitleAction::Exit),
                    _ => {}
                }
            }
        }
    }
}

pub fn show_dialog(
    session: &mut TuiSession,
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
) -> io::Result<()> {
    let lines = wrap_text(text, 80);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while let Some(page) = pages.pop() {
        draw_dialog(&mut session.terminal, dialog_ui, speaker, &page, None)?;
        wait_for_continue(bindings)?;
    }

    Ok(())
}

pub fn show_dialog_with_choices(
    session: &mut TuiSession,
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
    choices: &[String],
) -> io::Result<Option<usize>> {
    let lines = wrap_text(text, 80);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while pages.len() > 1 {
        if let Some(page) = pages.pop() {
            draw_dialog(&mut session.terminal, dialog_ui, speaker, &page, None)?;
            wait_for_continue(bindings)?;
        }
    }

    let page = pages.pop().unwrap_or_default();
    choose_dialog_option(
        &mut session.terminal,
        dialog_ui,
        bindings,
        speaker,
        &page,
        choices,
    )
}

pub fn draw_overworld(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
) -> io::Result<()> {
    session
        .terminal
        .draw(|frame| {
            let area = frame.size();
            let view_width = area.width.saturating_sub(2);
            let view_height = area.height.saturating_sub(2);
            let (start_x, start_y) = viewport_origin(map, player_pos, view_width, view_height);
            let mut lines = Vec::new();

            for y in 0..view_height {
                let mut row = String::new();
                for x in 0..view_width {
                    let map_x = start_x + x as i32;
                    let map_y = start_y + y as i32;
                    if (map_x, map_y) == player_pos {
                        row.push('@');
                        continue;
                    }
                    if let Some(npc) = map.npcs.iter().find(|npc| npc.pos == (map_x, map_y)) {
                        row.push(npc.glyph);
                        continue;
                    }
                    row.push(tile_at(map, map_x, map_y));
                }
                lines.push(Line::from(row));
            }

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        })
        .map(|_| ())
}

fn map_action(id: &str) -> TitleAction {
    match id {
        "load_game" => TitleAction::Load,
        "settings" => TitleAction::Settings,
        "exit" => TitleAction::Exit,
        _ => TitleAction::NewGame,
    }
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

fn wait_for_continue(bindings: &InputBindings) -> io::Result<()> {
    loop {
        if let Event::Key(key) = event::read()? {
            if let Some(action) = bindings.action_for(key.code) {
                if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                    return Ok(());
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

fn truncate_line(line: &str, width: usize) -> String {
    if line.len() <= width {
        return line.to_string();
    }
    line.chars().take(width).collect()
}

fn tile_at(map: &MapView, x: i32, y: i32) -> char {
    if x < 0 || y < 0 || x >= map.width as i32 || y >= map.height as i32 {
        return ' ';
    }
    map.tiles
        .get(y as usize)
        .and_then(|row| row.chars().nth(x as usize))
        .unwrap_or(' ')
}

fn viewport_origin(
    map: &MapView,
    player_pos: (i32, i32),
    view_width: u16,
    view_height: u16,
) -> (i32, i32) {
    let half_width = (view_width as i32) / 2;
    let half_height = (view_height as i32) / 2;
    let max_x = map.width as i32 - view_width as i32;
    let max_y = map.height as i32 - view_height as i32;

    let start_x = clamp(player_pos.0 - half_width, 0, max_x.max(0));
    let start_y = clamp(player_pos.1 - half_height, 0, max_y.max(0));
    (start_x, start_y)
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}
