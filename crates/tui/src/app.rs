use std::io::{self, ErrorKind, Stdout};

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    Clear as TermClear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::HashMap;

use crate::input::{Action, InputBindings};
use crate::ui::{MenuLayout, MenuUiFile, TitleUiFile};

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
    pub name: String,
    pub hide_name: bool,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<String>,
    pub legend: HashMap<char, TileRender>,
    pub transitions: Vec<TransitionView>,
    pub npcs: Vec<NpcView>,
    pub signs: Vec<SignView>,
    pub save_points: Vec<(i32, i32)>,
    pub use_color: bool,
}

pub struct TileRender {
    pub palette: Option<String>,
}

pub struct TransitionView {
    pub pos: (i32, i32),
    pub glyph: Option<char>,
    pub palette: Option<String>,
}

pub struct ShopView {
    pub name: String,
    pub items: Vec<ShopItem>,
}

pub struct ShopItem {
    pub name: String,
    pub price: i32,
}

pub struct ChoiceView {
    pub label: String,
    pub show_next: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum MenuPane {
    List,
    Detail,
}

#[derive(Clone, Debug)]
pub struct MenuEntryView {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct MenuPanelView {
    pub title: String,
    pub lines: Vec<String>,
}

pub struct NpcView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
}

pub struct SignView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
    pub text: String,
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
        self.terminal
            .backend_mut()
            .execute(TermClear(ClearType::All))?;
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

pub fn show_dialog(
    session: &mut TuiSession,
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
) -> io::Result<()> {
    let width = dialog_inner_width(session, dialog_ui);
    let lines = wrap_text(text, width);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while let Some(page) = pages.pop() {
        draw_dialog(&mut session.terminal, dialog_ui, speaker, &page, None)?;
        wait_for_continue(session, bindings, |frame| {
            draw_dialog_overlay(frame, dialog_ui, speaker, &page);
        })?;
    }

    Ok(())
}

pub fn show_dialog_with_choices(
    session: &mut TuiSession,
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
    choices: &[ChoiceView],
) -> io::Result<Option<usize>> {
    let width = dialog_inner_width(session, dialog_ui);
    let lines = wrap_text(text, width);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while pages.len() > 1 {
        if let Some(page) = pages.pop() {
            draw_dialog(&mut session.terminal, dialog_ui, speaker, &page, None)?;
            wait_for_continue(session, bindings, |frame| {
                draw_dialog_overlay(frame, dialog_ui, speaker, &page);
            })?;
        }
    }

    let page = pages.pop().unwrap_or_default();
    choose_dialog_option(session, dialog_ui, bindings, speaker, &page, choices)
}

pub fn show_dialog_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
) -> io::Result<()> {
    let width = dialog_inner_width(session, dialog_ui);
    let lines = wrap_text(text, width);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while let Some(page) = pages.pop() {
        draw_overworld_with_dialog(session, map, player_pos, dialog_ui, speaker, &page, None)?;
        wait_for_continue(session, bindings, |frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_dialog_overlay(frame, dialog_ui, speaker, &page);
        })?;
    }

    Ok(())
}

pub fn show_centered_dialog_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    text: &str,
) -> io::Result<()> {
    let width = centered_dialog_width(session);
    let lines = wrap_text(text, width);
    let mut pages = paginate_lines(lines, dialog_ui, "");

    while let Some(page) = pages.pop() {
        draw_overworld_with_centered_dialog(session, map, player_pos, dialog_ui, &page)?;
        wait_for_continue(session, bindings, |frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_centered_dialog_overlay(frame, dialog_ui, &page);
        })?;
    }

    Ok(())
}

pub fn draw_overworld_with_tooltip(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &crate::ui::DialogUiFile,
    text: &str,
) -> io::Result<()> {
    let lines = tooltip_lines(session, dialog_ui, text);
    session
        .terminal
        .draw(|frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_tooltip_overlay(frame, &lines);
        })
        .map(|_| ())
}

pub fn show_dialog_with_choices_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
    choices: &[ChoiceView],
) -> io::Result<Option<usize>> {
    let width = dialog_inner_width(session, dialog_ui);
    let lines = wrap_text(text, width);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while pages.len() > 1 {
        if let Some(page) = pages.pop() {
            draw_overworld_with_dialog(session, map, player_pos, dialog_ui, speaker, &page, None)?;
            wait_for_continue(session, bindings, |frame| {
                draw_overworld_frame(frame, map, player_pos);
                draw_dialog_overlay(frame, dialog_ui, speaker, &page);
            })?;
        }
    }

    let page = pages.pop().unwrap_or_default();
    choose_dialog_option_on_map(
        session, map, player_pos, dialog_ui, bindings, speaker, &page, choices,
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
            draw_overworld_frame(frame, map, player_pos);
        })
        .map(|_| ())
}

pub fn show_shop(
    session: &mut TuiSession,
    shop: &ShopView,
    bindings: &InputBindings,
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        session.terminal.draw(|frame| {
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

fn draw_title_frame(frame: &mut Frame, title_ui: &TitleUiFile, selected: usize) {
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

pub fn draw_menu(
    session: &mut TuiSession,
    menu_ui: &MenuUiFile,
    entries: &[MenuEntryView],
    selected: usize,
    focus: MenuPane,
    right_panel: &MenuPanelView,
) -> io::Result<()> {
    session
        .terminal
        .draw(|frame| {
            draw_menu_frame(frame, menu_ui, entries, selected, focus, right_panel);
        })
        .map(|_| ())
}

pub fn draw_menu_frame(
    frame: &mut Frame,
    menu_ui: &MenuUiFile,
    entries: &[MenuEntryView],
    selected: usize,
    focus: MenuPane,
    right_panel: &MenuPanelView,
) {
    let size = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);
    let (left_percent, right_percent) = menu_layout_percentages(&menu_ui.layout);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_percent),
            Constraint::Percentage(right_percent),
        ])
        .split(layout[0]);

    let menu_lines = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let is_selected = index == selected;
            let mut style = if entry.enabled {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            if is_selected {
                style = match focus {
                    MenuPane::List => style.fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    MenuPane::Detail => style.fg(Color::Cyan),
                };
            }
            let prefix = if is_selected && matches!(focus, MenuPane::List) {
                "> "
            } else {
                "  "
            };
            Line::from(Span::styled(format!("{}{}", prefix, entry.label), style))
        })
        .collect::<Vec<_>>();

    let menu_panel = Paragraph::new(menu_lines)
        .block(Block::default().borders(Borders::ALL).title("Menu"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(menu_panel, columns[0]);

    let detail_lines = right_panel
        .lines
        .iter()
        .map(|line| Line::from(Span::raw(line.as_str())))
        .collect::<Vec<_>>();
    let detail_panel = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(right_panel.title.as_str()),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(detail_panel, columns[1]);

    let footer_text = match focus {
        MenuPane::List => "Confirm: open  Cancel: close",
        MenuPane::Detail => "Cancel: back",
    };
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(footer, layout[1]);
}

fn menu_layout_percentages(layout: &MenuLayout) -> (u16, u16) {
    let total = layout.left_width_ratio + layout.right_width_ratio;
    let left_ratio = if total > 0.0 {
        layout.left_width_ratio / total
    } else {
        0.4
    };
    let left_percent = (left_ratio * 100.0).round().clamp(10.0, 90.0) as u16;
    let right_percent = 100u16.saturating_sub(left_percent).max(1);
    (left_percent, right_percent)
}

const DEFAULT_PLAYER_PALETTE: &str = "bright_white";
const DEFAULT_NPC_PALETTE: &str = "bright_yellow";
const DEFAULT_TRANSITION_PALETTE: &str = "bright_magenta";
const DEFAULT_SAVE_POINT_PALETTE: &str = "bright_cyan";
const DEFAULT_SIGN_PALETTE: &str = "bright_yellow";

pub fn draw_overworld_frame(frame: &mut Frame, map: &MapView, player_pos: (i32, i32)) {
    let area = frame.size();
    let view_width = area.width.saturating_sub(2);
    let view_height = area.height.saturating_sub(2);
    let (start_x, start_y) = viewport_origin(map, player_pos, view_width, view_height);
    let mut lines = Vec::new();

    for y in 0..view_height {
        let mut row = Vec::new();
        for x in 0..view_width {
            let map_x = start_x + x as i32;
            let map_y = start_y + y as i32;
            let mut glyph = tile_at(map, map_x, map_y);
            let mut palette = map
                .legend
                .get(&glyph)
                .and_then(|entry| entry.palette.as_deref());

            if (map_x, map_y) == player_pos {
                glyph = '@';
                palette = Some(DEFAULT_PLAYER_PALETTE);
            } else if let Some(npc) = map.npcs.iter().find(|npc| npc.pos == (map_x, map_y)) {
                glyph = npc.glyph;
                palette = npc.palette.as_deref().or(Some(DEFAULT_NPC_PALETTE));
            } else if let Some(sign) = map.signs.iter().find(|sign| sign.pos == (map_x, map_y)) {
                glyph = sign.glyph;
                palette = sign.palette.as_deref().or(Some(DEFAULT_SIGN_PALETTE));
            } else if let Some(transition) = map
                .transitions
                .iter()
                .find(|transition| transition.pos == (map_x, map_y))
            {
                if let Some(transition_glyph) = transition.glyph {
                    glyph = transition_glyph;
                }
                palette = transition
                    .palette
                    .as_deref()
                    .or(Some(DEFAULT_TRANSITION_PALETTE));
            } else if map.save_points.iter().any(|pos| *pos == (map_x, map_y)) {
                glyph = 'S';
                palette = Some(DEFAULT_SAVE_POINT_PALETTE);
            }

            row.push(Span::styled(
                glyph.to_string(),
                palette_style(map.use_color, palette),
            ));
        }
        lines.push(Line::from(row));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_overworld_with_dialog(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &crate::ui::DialogUiFile,
    speaker: &str,
    lines: &[String],
    choices: Option<(usize, &[ChoiceView])>,
) -> io::Result<()> {
    session
        .terminal
        .draw(|frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
            if let Some((selected, choices)) = choices {
                draw_choice_box(frame, choices, selected);
            }
        })
        .map(|_| ())
}

fn draw_overworld_with_centered_dialog(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &crate::ui::DialogUiFile,
    lines: &[String],
) -> io::Result<()> {
    session
        .terminal
        .draw(|frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_centered_dialog_overlay(frame, dialog_ui, lines);
        })
        .map(|_| ())
}

fn draw_dialog(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    dialog_ui: &crate::ui::DialogUiFile,
    speaker: &str,
    lines: &[String],
    choices: Option<(usize, &[ChoiceView])>,
) -> io::Result<()> {
    terminal
        .draw(|frame| {
            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
            if let Some((selected, choices)) = choices {
                draw_choice_box(frame, choices, selected);
            }
        })
        .map(|_| ())
}

fn draw_dialog_overlay(
    frame: &mut Frame,
    dialog_ui: &crate::ui::DialogUiFile,
    speaker: &str,
    lines: &[String],
) {
    let area = dialog_area(frame.size(), dialog_ui);
    let inner_width = area.width.saturating_sub(2) as usize;
    let mut content = Vec::new();

    frame.render_widget(Clear, area);

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

    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);

    if !dialog_ui.continue_marker.is_empty() {
        let marker_area = Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(2),
            1,
        );
        let marker = Paragraph::new(dialog_ui.continue_marker.as_str())
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: false });
        frame.render_widget(marker, marker_area);
    }
}

fn draw_tooltip_overlay(frame: &mut Frame, lines: &[String]) {
    if lines.is_empty() {
        return;
    }

    let max_len = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let width = (max_len as u16).saturating_add(2).max(12);
    let height = (lines.len() as u16).saturating_add(2).max(3);
    let area = centered_rect(frame.size(), width, height);
    let inner_width = area.width.saturating_sub(2) as usize;

    frame.render_widget(Clear, area);

    let content = lines
        .iter()
        .map(|line| Line::from(Span::raw(truncate_line(line, inner_width))))
        .collect::<Vec<_>>();
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_centered_dialog_overlay(
    frame: &mut Frame,
    dialog_ui: &crate::ui::DialogUiFile,
    lines: &[String],
) {
    let area = centered_dialog_area(frame.size(), dialog_ui);
    let inner_width = area.width.saturating_sub(2) as usize;
    let mut content = Vec::new();

    frame.render_widget(Clear, area);

    for line in lines {
        content.push(Line::from(Span::raw(truncate_line(line, inner_width))));
    }

    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);

    if !dialog_ui.continue_marker.is_empty() {
        let marker_area = Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(2),
            1,
        );
        let marker = Paragraph::new(dialog_ui.continue_marker.as_str())
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: false });
        frame.render_widget(marker, marker_area);
    }
}

fn wait_for_continue<F>(
    session: &mut TuiSession,
    bindings: &InputBindings,
    draw_background: F,
) -> io::Result<()>
where
    F: Fn(&mut Frame),
{
    loop {
        if let Event::Key(key) = event::read()? {
            if let Some(action) = bindings.action_for(key.code) {
                if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                    return Ok(());
                }
                if action == Action::Quit {
                    if confirm_quit(session, &draw_background)? {
                        return Err(io::Error::new(ErrorKind::Interrupted, "quit"));
                    }
                }
            }
        }
    }
}

fn choose_dialog_option(
    session: &mut TuiSession,
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    lines: &[String],
    choices: &[ChoiceView],
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        draw_dialog(
            &mut session.terminal,
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
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
                            draw_choice_box(frame, choices, selected);
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

fn choose_dialog_option_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &crate::ui::DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    lines: &[String],
    choices: &[ChoiceView],
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        draw_overworld_with_dialog(
            session,
            map,
            player_pos,
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
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_overworld_frame(frame, map, player_pos);
                            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
                            draw_choice_box(frame, choices, selected);
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

pub fn confirm_quit<F>(session: &mut TuiSession, draw_background: F) -> io::Result<bool>
where
    F: Fn(&mut Frame),
{
    loop {
        session.terminal.draw(|frame| {
            draw_background(frame);
            let area = centered_rect(frame.size(), 40, 3);
            frame.render_widget(Clear, area);
            let content = vec![Line::from(Span::raw("Quit the game? (Y/N)"))];
            let paragraph = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                KeyCode::Char('n') | KeyCode::Char('N') => return Ok(false),
                _ => return Ok(false),
            }
        }
    }
}

pub fn prompt_text(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
    max_len: usize,
) -> io::Result<Option<String>> {
    let mut value = default.to_string();
    loop {
        session.terminal.draw(|frame| {
            draw_text_prompt_frame(frame, title, prompt, &value, max_len);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        return Ok(Some(default.to_string()));
                    }
                    return Ok(Some(trimmed.to_string()));
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Backspace => {
                    value.pop();
                }
                KeyCode::Char(ch) => {
                    if value.chars().count() < max_len {
                        value.push(ch);
                    }
                }
                _ => {}
            }
        }
    }
}

fn draw_text_prompt_frame(
    frame: &mut Frame,
    title: &str,
    prompt: &str,
    value: &str,
    max_len: usize,
) {
    let content = vec![
        Line::from(Span::raw(prompt)),
        Line::from(Span::styled(value, Style::default().fg(Color::Yellow))),
        Line::from(Span::raw(format!("{}/{}", value.chars().count(), max_len))),
    ];
    let width = content
        .iter()
        .map(|line| line.width() as u16)
        .max()
        .unwrap_or(20)
        .saturating_add(4)
        .max(30);
    let height = content.len() as u16 + 2;
    let area = centered_rect(frame.size(), width, height);
    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_choice_box(frame: &mut Frame, choices: &[ChoiceView], selected: usize) {
    if choices.is_empty() {
        return;
    }

    let max_len = choices
        .iter()
        .map(|choice| choice.label.chars().count())
        .max()
        .unwrap_or(0);
    let width = (max_len as u16).saturating_add(2).max(12);
    let height = (choices.len() as u16).saturating_add(2);
    let area = centered_rect(frame.size(), width, height);

    frame.render_widget(Clear, area);

    let lines = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            let text = choice.label.as_str();
            let style = if index == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(text, style))
        })
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
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

fn centered_dialog_area(area: Rect, dialog_ui: &crate::ui::DialogUiFile) -> Rect {
    let height = dialog_ui.height.min(area.height).max(3);
    let width = centered_dialog_width_for_area(area);
    centered_rect(area, width, height)
}

fn centered_dialog_width(session: &TuiSession) -> usize {
    let area = session.terminal.size().unwrap_or_default();
    let width = centered_dialog_width_for_area(area);
    width.saturating_sub(2).max(1) as usize
}

fn tooltip_lines(
    session: &TuiSession,
    dialog_ui: &crate::ui::DialogUiFile,
    text: &str,
) -> Vec<String> {
    let area = session.terminal.size().unwrap_or_default();
    let max_width = centered_dialog_width_for_area(area)
        .saturating_sub(2)
        .max(10) as usize;
    let lines = wrap_text(text, max_width);
    let available_lines = dialog_ui.height.saturating_sub(1) as usize;
    if lines.len() > available_lines {
        lines.into_iter().take(available_lines.max(1)).collect()
    } else {
        lines
    }
}

fn centered_dialog_width_for_area(area: Rect) -> u16 {
    let width = area.width.saturating_sub(10).max(20);
    width.min(60).min(area.width.saturating_sub(2))
}

fn dialog_inner_width(session: &TuiSession, dialog_ui: &crate::ui::DialogUiFile) -> usize {
    let area = session.terminal.size().unwrap_or_default();
    let dialog = dialog_area(area, dialog_ui);
    dialog.width.saturating_sub(2).max(1) as usize
}

fn truncate_line(line: &str, width: usize) -> String {
    if line.len() <= width {
        return line.to_string();
    }
    line.chars().take(width).collect()
}

fn palette_style(use_color: bool, palette: Option<&str>) -> Style {
    if !use_color {
        return Style::default();
    }
    match palette.and_then(palette_color) {
        Some(color) => Style::default().fg(color),
        None => Style::default(),
    }
}

fn palette_color(name: &str) -> Option<Color> {
    let key = name.trim().to_ascii_lowercase();
    match key.as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "bright_black" | "light_black" => Some(Color::DarkGray),
        "bright_red" | "light_red" => Some(Color::LightRed),
        "bright_green" | "light_green" => Some(Color::LightGreen),
        "bright_yellow" | "light_yellow" => Some(Color::LightYellow),
        "bright_blue" | "light_blue" => Some(Color::LightBlue),
        "bright_magenta" | "light_magenta" => Some(Color::LightMagenta),
        "bright_cyan" | "light_cyan" => Some(Color::LightCyan),
        "bright_white" | "light_white" => Some(Color::White),
        "bright_gray" | "bright_grey" => Some(Color::Gray),
        _ => None,
    }
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
    let view_width = view_width as i32;
    let view_height = view_height as i32;
    let map_width = map.width as i32;
    let map_height = map.height as i32;

    let start_x = if map_width <= view_width {
        -((view_width - map_width) / 2)
    } else {
        let half_width = view_width / 2;
        let max_x = map_width - view_width;
        clamp(player_pos.0 - half_width, 0, max_x.max(0))
    };

    let start_y = if map_height <= view_height {
        -((view_height - map_height) / 2)
    } else {
        let half_height = view_height / 2;
        let max_y = map_height - view_height;
        clamp(player_pos.1 - half_height, 0, max_y.max(0))
    };

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

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
