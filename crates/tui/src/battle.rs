use std::collections::HashMap;
use std::io;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::session::TuiSession;
use crate::ui::{BattleUiFile, Breakpoint};
use crate::utils::{centered_rect, palette_style};

#[derive(Clone, Debug)]
pub struct BattleEnemyView {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub glyph: char,
    pub palette: Option<String>,
    pub art: Option<Vec<String>>,
    pub art_palette: Option<String>,
    pub pos: (i32, i32),
    pub alive: bool,
    pub show_hp: bool,
}

#[derive(Clone, Debug)]
pub struct BattlePartyView {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub show_mp: bool,
    pub atb: f32,
    pub status: Vec<String>,
    pub alive: bool,
    pub active: bool,
    pub glyph: char,
    pub palette: Option<String>,
    pub art: Option<Vec<String>>,
    pub art_palette: Option<String>,
    pub pos: (i32, i32),
}

#[derive(Clone, Debug)]
pub struct BattleCommandItem {
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub enum BattleCommandPanelMode {
    Commands,
    Magic,
    Abilities,
    Items,
}

#[derive(Clone, Debug)]
pub struct BattleCommandPanelView {
    pub mode: BattleCommandPanelMode,
    pub title: String,
    pub items: Vec<BattleCommandItem>,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub selected: usize,
}

#[derive(Clone, Debug)]
pub enum BattleFocus {
    Commands,
    Enemies,
    Party,
}

#[derive(Clone, Debug)]
pub struct BattleRenderState {
    pub enemies: Vec<BattleEnemyView>,
    pub party: Vec<BattlePartyView>,
    pub command_panel: BattleCommandPanelView,
    pub selected_enemy: usize,
    pub selected_party: usize,
    pub focus: BattleFocus,
    pub log: Vec<String>,
    pub use_color: bool,
    pub flash_enemies: Vec<usize>,
    pub flash_party: Vec<usize>,
    pub acting_enemies: Vec<usize>,
    pub acting_party: Vec<usize>,
}

pub fn draw_battle(
    session: &mut TuiSession,
    battle_ui: &BattleUiFile,
    state: &BattleRenderState,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_battle_frame(frame, battle_ui, state);
        })
        .map(|_| ())
}

pub fn draw_battle_frame(frame: &mut Frame, battle_ui: &BattleUiFile, state: &BattleRenderState) {
    let size = frame.size();
    let breakpoint = active_breakpoint(battle_ui, size);
    let hide_titles = breakpoint.behavior.hide_panel_titles;

    let (log_area, main_area) = split_battle_log_area(size, battle_ui.log.as_ref());
    if let Some(area) = log_area {
        draw_battle_log(frame, area, &state.log);
    }

    let (battlefield_area, command_area) = split_battle_layout(main_area, battle_ui);

    draw_battlefield(
        frame,
        battlefield_area,
        battle_ui,
        &breakpoint.behavior.enemy_art,
        state,
        hide_titles,
    );

    let (command_log_area, columns_area) =
        split_command_log_area(command_area, battle_ui.log.as_ref());
    if let Some(area) = command_log_area {
        draw_battle_log(frame, area, &state.log);
    }

    draw_command_row(frame, columns_area, battle_ui, state, hide_titles);
}

pub fn draw_victory_summary(
    session: &mut TuiSession,
    exp: i32,
    currency: i32,
    currency_label: &str,
    items: &HashMap<String, i32>,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            let area = centered_rect(frame.size(), 40, 15);
            frame.render_widget(Clear, area);

            let mut lines = Vec::new();
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                "Victory!",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                if exp > 0 {
                    format!("Exp: {}", exp)
                } else {
                    String::new()
                },
                Style::default(),
            )));
            lines.push(Line::from(Span::styled(
                if currency > 0 {
                    format!("{}: {}", currency_label, currency)
                } else {
                    String::new()
                },
                Style::default(),
            )));

            if !items.is_empty() {
                lines.push(Line::from(Span::raw("Items found:")));
                for (item, qty) in items {
                    lines.push(Line::from(Span::styled(
                        format!("  {} x{}", item, qty),
                        Style::default(),
                    )));
                }
            }

            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                "Press Confirm to continue.",
                Style::default().fg(Color::Gray),
            )));

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        })
        .map(|_| ())
}

pub fn draw_level_up_modal(
    session: &mut TuiSession,
    actor_name: &str,
    _old_level: u32,
    new_level: u32,
    stat_changes: &HashMap<String, (i32, i32)>,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            let area = centered_rect(frame.size(), 30, 16);
            frame.render_widget(Clear, area);

            let mut lines = Vec::new();
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                format!("{} reached Level {}!", actor_name, new_level),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));

            let mut stats: Vec<_> = stat_changes.iter().collect();
            stats.sort_by(|a, b| a.0.cmp(b.0));

            for (stat, (new_val, diff)) in stats {
                lines.push(Line::from(Span::styled(
                    format!("  {}: {} (+{})", stat, new_val, diff),
                    Style::default(),
                )));
            }

            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                "Press Confirm to continue.",
                Style::default().fg(Color::Gray),
            )));

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        })
        .map(|_| ())
}

fn active_breakpoint<'a>(battle_ui: &'a BattleUiFile, area: Rect) -> &'a Breakpoint {
    battle_ui
        .breakpoints
        .iter()
        .filter(|breakpoint| {
            area.width >= breakpoint.min_width && area.height >= breakpoint.min_height
        })
        .max_by_key(|breakpoint| (breakpoint.min_width, breakpoint.min_height))
        .unwrap_or_else(|| battle_ui.breakpoints.first().expect("battle breakpoint"))
}

fn split_battle_log_area(area: Rect, log: Option<&crate::ui::BattleLog>) -> (Option<Rect>, Rect) {
    let Some(log) = log else {
        return (None, area);
    };
    if log.position != "top" || log.height == 0 {
        return (None, area);
    }
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(log.height), Constraint::Min(1)])
        .split(area);
    (Some(split[0]), split[1])
}

fn split_command_log_area(area: Rect, log: Option<&crate::ui::BattleLog>) -> (Option<Rect>, Rect) {
    let Some(log) = log else {
        return (None, area);
    };
    if log.position != "pane_top" || log.height == 0 {
        return (None, area);
    }
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(log.height), Constraint::Min(1)])
        .split(area);
    (Some(split[0]), split[1])
}

fn split_battle_layout(area: Rect, battle_ui: &BattleUiFile) -> (Rect, Rect) {
    let battlefield_ratio = battle_ui.layout.battlefield.height_ratio.clamp(0.1, 0.9);
    let battlefield_height = ((area.height as f32) * battlefield_ratio)
        .round()
        .clamp(3.0, area.height.saturating_sub(3) as f32) as u16;
    let command_height = area.height.saturating_sub(battlefield_height).max(3);

    let constraints = match battle_ui.layout.battlefield.anchor.as_str() {
        "bottom" => [
            Constraint::Length(command_height),
            Constraint::Length(battlefield_height),
        ],
        _ => [
            Constraint::Length(battlefield_height),
            Constraint::Length(command_height),
        ],
    };
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    if battle_ui.layout.battlefield.anchor.as_str() == "bottom" {
        (split[1], split[0])
    } else {
        (split[0], split[1])
    }
}

fn draw_battle_log(frame: &mut Frame, area: Rect, lines: &[String]) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let mut log_lines = Vec::new();
    let max = area.height.saturating_sub(2) as usize;
    let start = lines.len().saturating_sub(max);
    for line in lines.iter().skip(start) {
        log_lines.push(Line::from(Span::raw(line.clone())));
    }
    let panel = Paragraph::new(log_lines)
        .block(Block::default().borders(Borders::ALL).title("Log"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn draw_command_row(
    frame: &mut Frame,
    area: Rect,
    battle_ui: &BattleUiFile,
    state: &BattleRenderState,
    hide_titles: bool,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let constraints = battle_ui
        .layout
        .command_row
        .columns
        .iter()
        .map(|column| {
            let width = (column.width_ratio * 100.0).round().clamp(10.0, 90.0) as u16;
            Constraint::Percentage(width)
        })
        .collect::<Vec<_>>();
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (index, column) in battle_ui.layout.command_row.columns.iter().enumerate() {
        let column_area = columns.get(index).copied().unwrap_or(area);
        match column.id.as_str() {
            "enemies" => draw_enemy_panel(frame, column_area, battle_ui, state, hide_titles),
            "party" => draw_party_panel(frame, column_area, battle_ui, state, hide_titles),
            _ => draw_command_panel(frame, column_area, battle_ui, state, hide_titles),
        }
    }
}

fn draw_enemy_panel(
    frame: &mut Frame,
    area: Rect,
    battle_ui: &BattleUiFile,
    state: &BattleRenderState,
    hide_titles: bool,
) {
    let mut lines = Vec::new();
    for (index, enemy) in state.enemies.iter().enumerate() {
        let focused = matches!(state.focus, BattleFocus::Enemies);
        let is_selected = focused && index == state.selected_enemy;
        let mut style = if enemy.alive {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if is_selected {
            style = list_highlight_style(style, &battle_ui.selection.list_highlight, focused);
        } else if state.flash_enemies.contains(&index) {
            style = style.fg(Color::Yellow).add_modifier(Modifier::REVERSED);
        }
        let label = if enemy.show_hp {
            format!("{} {}/{}", enemy.name, enemy.hp.max(0), enemy.max_hp.max(1))
        } else {
            enemy.name.clone()
        };
        lines.push(Line::from(Span::styled(label, style)));
    }
    let title = if hide_titles {
        ""
    } else {
        battle_ui.panels.enemies.title.as_str()
    };
    let panel = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn draw_command_panel(
    frame: &mut Frame,
    area: Rect,
    battle_ui: &BattleUiFile,
    state: &BattleRenderState,
    hide_titles: bool,
) {
    let mut lines = Vec::new();
    match state.command_panel.mode {
        BattleCommandPanelMode::Commands => {
            for (index, item) in state.command_panel.items.iter().enumerate() {
                let is_selected = index == state.command_panel.selected;
                let mut style = if item.enabled {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                if is_selected {
                    style = list_highlight_style(
                        style,
                        &battle_ui.selection.list_highlight,
                        matches!(state.focus, BattleFocus::Commands),
                    );
                }
                let prefix = if is_selected { "> " } else { "  " };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, item.label),
                    style,
                )));
            }
        }
        BattleCommandPanelMode::Magic
        | BattleCommandPanelMode::Abilities
        | BattleCommandPanelMode::Items => {
            let widths = column_widths(&state.command_panel.columns, &state.command_panel.rows);
            if !state.command_panel.columns.is_empty() {
                lines.push(Line::from(Span::styled(
                    format_row(&state.command_panel.columns, &widths),
                    Style::default().fg(Color::Cyan),
                )));
            }
            for (index, row) in state.command_panel.rows.iter().enumerate() {
                let is_selected = index == state.command_panel.selected;
                let mut style = Style::default().fg(Color::White);
                if is_selected {
                    style = list_highlight_style(
                        style,
                        &battle_ui.selection.list_highlight,
                        matches!(state.focus, BattleFocus::Commands),
                    );
                }
                lines.push(Line::from(Span::styled(format_row(row, &widths), style)));
            }
        }
    }
    let title = if hide_titles {
        ""
    } else {
        state.command_panel.title.as_str()
    };
    let panel = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn draw_party_panel(
    frame: &mut Frame,
    area: Rect,
    battle_ui: &BattleUiFile,
    state: &BattleRenderState,
    hide_titles: bool,
) {
    let mut lines = Vec::new();
    for (index, member) in state.party.iter().enumerate() {
        let focused = matches!(state.focus, BattleFocus::Party);
        let is_selected = focused && index == state.selected_party;
        let mut style = if member.alive {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if state.flash_party.contains(&index) {
            style = style.fg(Color::Yellow).add_modifier(Modifier::REVERSED);
        } else if is_selected {
            style = list_highlight_style(style, &battle_ui.selection.list_highlight, focused);
        } else if member.active {
            style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
        }
        let line = if member.show_mp {
            format!(
                "{} HP {}/{}  MP {}/{}  ATB: {:.0}%",
                member.name,
                member.hp.max(0),
                member.max_hp.max(1),
                member.mp.max(0),
                member.max_mp.max(1),
                member.atb
            )
        } else {
            format!(
                "{} HP {}/{}  ATB: {:.0}%",
                member.name,
                member.hp.max(0),
                member.max_hp.max(1),
                member.atb
            )
        };
        lines.push(Line::from(Span::styled(line, style)));
        if !member.status.is_empty() {
            let status = member.status.join(", ");
            lines.push(Line::from(Span::styled(
                format!("  {}", status),
                Style::default().fg(Color::Gray),
            )));
        }
    }
    let title = if hide_titles {
        ""
    } else {
        battle_ui.panels.party.title.as_str()
    };
    let panel = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn draw_battlefield(
    frame: &mut Frame,
    area: Rect,
    battle_ui: &BattleUiFile,
    enemy_art_mode: &str,
    state: &BattleRenderState,
    hide_titles: bool,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title(if hide_titles { "" } else { "Battle" });
    let inner = block.inner(area);
    let inner_width = inner.width as usize;
    let inner_height = inner.height as usize;
    let mut cells = vec![vec![BattleCell::new(' ', Style::default()); inner_width]; inner_height];
    let cell_width = inner.width as f32 / 10.0;
    let cell_height = inner.height as f32 / 6.0;
    let highlight_modifier =
        battlefield_highlight_modifier(&battle_ui.selection.battlefield_highlight);

    for (index, enemy) in state.enemies.iter().enumerate() {
        if enemy.max_hp <= 0 {
            continue;
        }
        let use_art = enemy_art_mode != "glyph" && enemy.art.is_some();
        let (center_x, center_y) = (
            (enemy.pos.0 as f32 + 0.5) * cell_width,
            (enemy.pos.1 as f32 + 0.5) * cell_height,
        );
        let acting_offset = if state.acting_enemies.contains(&index) {
            1
        } else {
            0
        };
        let selected = matches!(state.focus, BattleFocus::Enemies) && index == state.selected_enemy;
        let flashing = state.flash_enemies.contains(&index);
        let mut style = palette_style(state.use_color, enemy.palette.as_deref());
        if !enemy.alive {
            style = Style::default().fg(Color::DarkGray);
        }
        if flashing {
            style = style.fg(Color::Yellow).add_modifier(Modifier::REVERSED);
        } else if selected {
            style = style.add_modifier(highlight_modifier);
        }

        if use_art {
            if let Some(lines) = enemy.art.as_ref() {
                let art_palette = enemy.art_palette.as_deref().or(enemy.palette.as_deref());
                let art_style = if enemy.alive {
                    palette_style(state.use_color, art_palette)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let art_style = if flashing {
                    art_style.fg(Color::Yellow).add_modifier(Modifier::REVERSED)
                } else if selected {
                    art_style.add_modifier(highlight_modifier)
                } else {
                    art_style
                };
                let art_height = lines.len() as i32;
                for (line_index, line) in lines.iter().enumerate() {
                    let line_width = line.chars().count() as i32;
                    let start_x = center_x as i32 - line_width / 2 + acting_offset;
                    let start_y = center_y as i32 - art_height / 2 + line_index as i32;
                    place_text(&mut cells, line, start_x, start_y, art_style);
                }
            }
        } else {
            let x = center_x as i32 + acting_offset;
            let y = center_y as i32;
            place_glyph(&mut cells, enemy.glyph, x, y, style);
        }
    }

    for (index, member) in state.party.iter().enumerate() {
        let use_art = enemy_art_mode != "glyph" && member.art.is_some();
        let (center_x, center_y) = (
            (member.pos.0 as f32 + 0.5) * cell_width,
            (member.pos.1 as f32 + 0.5) * cell_height,
        );
        let acting_offset = if state.acting_party.contains(&index) {
            -1
        } else {
            0
        };
        let selected = matches!(state.focus, BattleFocus::Party) && index == state.selected_party;
        let flashing = state.flash_party.contains(&index);
        let mut style = palette_style(state.use_color, member.palette.as_deref());
        if !member.alive {
            style = Style::default().fg(Color::DarkGray);
        }
        if flashing {
            style = style.fg(Color::Yellow).add_modifier(Modifier::REVERSED);
        } else if selected {
            style = style.add_modifier(highlight_modifier);
        }
        if use_art {
            if let Some(lines) = member.art.as_ref() {
                let art_palette = member.art_palette.as_deref().or(member.palette.as_deref());
                let art_style = if member.alive {
                    palette_style(state.use_color, art_palette)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let art_style = if flashing {
                    art_style.fg(Color::Yellow).add_modifier(Modifier::REVERSED)
                } else if selected {
                    art_style.add_modifier(highlight_modifier)
                } else {
                    art_style
                };
                let art_height = lines.len() as i32;
                for (line_index, line) in lines.iter().enumerate() {
                    let line_width = line.chars().count() as i32;
                    let start_x = center_x as i32 - line_width / 2 + acting_offset;
                    let start_y = center_y as i32 - art_height / 2 + line_index as i32;
                    place_text(&mut cells, line, start_x, start_y, art_style);
                }
            }
        } else {
            let x = center_x as i32 + acting_offset;
            let y = center_y as i32;
            place_glyph(&mut cells, member.glyph, x, y, style);
        }
    }

    let lines = cells
        .iter()
        .map(|row| {
            Line::from(
                row.iter()
                    .map(|cell| Span::styled(cell.ch.to_string(), cell.style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let panel = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn list_highlight_style(style: Style, highlight: &str, focused: bool) -> Style {
    let mut style = style;
    style = if focused {
        style.fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        style.fg(Color::Cyan)
    };
    match highlight {
        "underline" => style.add_modifier(Modifier::UNDERLINED),
        "invert" => style.add_modifier(Modifier::REVERSED),
        _ => style,
    }
}

fn battlefield_highlight_modifier(highlight: &str) -> Modifier {
    match highlight {
        "underline" => Modifier::UNDERLINED,
        "invert" => Modifier::REVERSED,
        _ => Modifier::REVERSED,
    }
}

fn column_widths(headers: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<_>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if widths.len() <= index {
                widths.push(value.len());
            } else {
                widths[index] = widths[index].max(value.len());
            }
        }
    }
    widths
}

fn format_row(values: &[String], widths: &[usize]) -> String {
    let mut line = String::new();
    for (index, value) in values.iter().enumerate() {
        let width = widths.get(index).copied().unwrap_or(value.len());
        let padded = format!("{value:<width$}", value = value, width = width);
        line.push_str(&padded);
        if index + 1 < values.len() {
            line.push_str("  ");
        }
    }
    line
}

#[derive(Clone, Copy)]
struct BattleCell {
    ch: char,
    style: Style,
}

impl BattleCell {
    fn new(ch: char, style: Style) -> Self {
        Self { ch, style }
    }
}

fn place_text(cells: &mut [Vec<BattleCell>], text: &str, x: i32, y: i32, style: Style) {
    for (idx, ch) in text.chars().enumerate() {
        place_glyph(cells, ch, x + idx as i32, y, style);
    }
}

fn place_glyph(cells: &mut [Vec<BattleCell>], ch: char, x: i32, y: i32, style: Style) {
    if y < 0 || x < 0 {
        return;
    }
    let row = match cells.get_mut(y as usize) {
        Some(row) => row,
        None => return,
    };
    if let Some(cell) = row.get_mut(x as usize) {
        *cell = BattleCell::new(ch, style);
    }
}
