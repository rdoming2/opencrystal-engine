use std::io;

use crossterm::event::{self, Event};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::dialog::confirm_quit;
use crate::input::{is_actionable_key, Action, InputBindings};
use crate::session::TuiSession;
use crate::ui::{TitleLogo, TitleUiFile};
use crate::utils::{centered_rect, palette_style};

pub enum TitleAction {
    NewGame,
    Load,
    Settings,
    Exit,
}

pub struct LoadSlotEntry {
    pub slot: u8,
    pub label: String,
    pub enabled: bool,
}

pub fn run_title(
    session: &mut TuiSession,
    title_ui: &TitleUiFile,
    bindings: &InputBindings,
    load_enabled: bool,
    default_selected: usize,
) -> io::Result<TitleAction> {
    let mut selected = default_selected.min(title_ui.menu.len().saturating_sub(1));
    if let Some(index) = first_enabled_menu_item(title_ui, load_enabled) {
        if title_ui
            .menu
            .get(selected)
            .map(|item| !menu_item_enabled(item, load_enabled))
            .unwrap_or(true)
        {
            selected = index;
        }
    } else {
        selected = 0;
    }

    loop {
        session.terminal_mut().draw(|frame| {
            draw_title_frame(frame, title_ui, selected, load_enabled);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        selected = move_menu_selection(selected, title_ui, load_enabled, -1);
                    }
                    Action::MoveDown => {
                        selected = move_menu_selection(selected, title_ui, load_enabled, 1);
                    }
                    Action::Confirm => {
                        if let Some(item) = title_ui.menu.get(selected) {
                            if menu_item_enabled(item, load_enabled) {
                                return Ok(map_action(&item.id));
                            }
                        }
                    }
                    Action::Cancel | Action::Menu => return Ok(TitleAction::Exit),
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_title_frame(frame, title_ui, selected, load_enabled)
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

pub fn run_load_menu(
    session: &mut TuiSession,
    title_ui: &TitleUiFile,
    bindings: &InputBindings,
    slots: &[LoadSlotEntry],
) -> io::Result<Option<usize>> {
    if slots.is_empty() {
        return Ok(None);
    }
    let mut selected = first_enabled_slot(slots).unwrap_or(0);
    loop {
        session.terminal_mut().draw(|frame| {
            draw_load_frame(frame, title_ui, slots, selected);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        selected = move_slot_selection(selected, slots, -1);
                    }
                    Action::MoveDown => {
                        selected = move_slot_selection(selected, slots, 1);
                    }
                    Action::Confirm => {
                        if slots
                            .get(selected)
                            .map(|slot| slot.enabled)
                            .unwrap_or(false)
                        {
                            return Ok(Some(selected));
                        }
                    }
                    Action::Cancel | Action::Menu => return Ok(None),
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_load_frame(frame, title_ui, slots, selected)
                        })? {
                            return Ok(None);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn draw_title_frame(
    frame: &mut Frame,
    title_ui: &TitleUiFile,
    selected: usize,
    load_enabled: bool,
) {
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
        .enumerate()
        .map(|(index, line)| {
            let style = logo_line_style(&title_ui.logo, index);
            Line::from(Span::styled(line.as_str(), style))
        })
        .collect();
    let logo_width = logo_block_width(&title_ui.logo);
    let logo_height = title_ui.logo.lines.len().max(1) as u16;
    let logo_area = centered_rect(layout[0], logo_width, logo_height);
    let logo = Paragraph::new(logo_lines)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(logo, logo_area);

    let title_width = line_width(title_ui.title.as_str());
    let title_area = centered_rect(layout[1], title_width, 1);
    let title = Paragraph::new(title_ui.title.as_str())
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(title, title_area);

    let menu_items: Vec<Line> = title_ui
        .menu
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let mut style = Style::default().fg(Color::White);
            if !menu_item_enabled(item, load_enabled) {
                style = style.fg(Color::Gray);
            } else if index == selected {
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

fn draw_load_frame(
    frame: &mut Frame,
    title_ui: &TitleUiFile,
    slots: &[LoadSlotEntry],
    selected: usize,
) {
    let size = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_ui.logo.lines.len() as u16 + 2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(size);

    let logo_lines: Vec<Line> = title_ui
        .logo
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let style = logo_line_style(&title_ui.logo, index);
            Line::from(Span::styled(line.as_str(), style))
        })
        .collect();
    let logo_width = logo_block_width(&title_ui.logo);
    let logo_height = title_ui.logo.lines.len().max(1) as u16;
    let logo_area = centered_rect(layout[0], logo_width, logo_height);
    let logo = Paragraph::new(logo_lines)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(logo, logo_area);

    let title_width = line_width(title_ui.title.as_str());
    let title_area = centered_rect(layout[1], title_width, 1);
    let title = Paragraph::new(title_ui.title.as_str())
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(title, title_area);

    let header_text = "Load Game";
    let header_width = line_width(header_text);
    let header_area = centered_rect(layout[2], header_width, 1);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(header, header_area);

    let menu_items: Vec<Line> = slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let mut style = if slot.enabled {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            if index == selected {
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }
            Line::from(Span::styled(slot.label.as_str(), style))
        })
        .collect();

    let menu = Paragraph::new(menu_items)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(menu, layout[3]);

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
    frame.render_widget(footer, layout[4]);
}

fn map_action(id: &str) -> TitleAction {
    match id {
        "load_game" => TitleAction::Load,
        "settings" => TitleAction::Settings,
        "exit" => TitleAction::Exit,
        _ => TitleAction::NewGame,
    }
}

fn menu_item_enabled(item: &crate::ui::MenuItem, load_enabled: bool) -> bool {
    if item.id == "load_game" {
        load_enabled
    } else {
        true
    }
}

fn first_enabled_slot(slots: &[LoadSlotEntry]) -> Option<usize> {
    slots.iter().position(|slot| slot.enabled)
}

fn first_enabled_menu_item(title_ui: &TitleUiFile, load_enabled: bool) -> Option<usize> {
    title_ui
        .menu
        .iter()
        .position(|item| menu_item_enabled(item, load_enabled))
}

fn move_menu_selection(
    current: usize,
    title_ui: &TitleUiFile,
    load_enabled: bool,
    direction: i32,
) -> usize {
    if title_ui.menu.is_empty() {
        return 0;
    }
    let mut index = current.min(title_ui.menu.len().saturating_sub(1));
    loop {
        if direction < 0 {
            if index == 0 {
                return current;
            }
            index = index.saturating_sub(1);
        } else {
            if index + 1 >= title_ui.menu.len() {
                return current;
            }
            index = (index + 1).min(title_ui.menu.len().saturating_sub(1));
        }
        if title_ui
            .menu
            .get(index)
            .map(|item| menu_item_enabled(item, load_enabled))
            .unwrap_or(false)
        {
            return index;
        }
    }
}

fn move_slot_selection(current: usize, slots: &[LoadSlotEntry], direction: i32) -> usize {
    if slots.is_empty() {
        return 0;
    }
    let mut index = current.min(slots.len().saturating_sub(1));
    let mut remaining = slots.len();
    while remaining > 0 {
        if direction < 0 {
            index = index.saturating_sub(1);
        } else {
            index = (index + 1).min(slots.len().saturating_sub(1));
        }
        if slots[index].enabled {
            return index;
        }
        remaining -= 1;
    }
    current
}

fn logo_line_style(logo: &TitleLogo, index: usize) -> Style {
    let line_palette = logo
        .line_palettes
        .as_ref()
        .and_then(|palettes| palettes.get(index))
        .map(|palette| palette.as_str());
    let palette = line_palette.or(logo.palette.as_deref());
    palette_style(true, palette)
}

fn logo_block_width(logo: &TitleLogo) -> u16 {
    logo.lines
        .iter()
        .map(|line| line_width(line))
        .max()
        .unwrap_or(1)
}

fn line_width(line: &str) -> u16 {
    let width = line.chars().count();
    u16::try_from(width).unwrap_or(u16::MAX).max(1)
}
