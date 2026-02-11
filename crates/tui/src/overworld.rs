use std::collections::HashMap;
use std::io::{self, ErrorKind};

use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::dialog::{
    centered_dialog_width_for_area, confirm_quit, dialog_inner_width, draw_centered_dialog_overlay,
    draw_choice_box, draw_dialog_overlay, paginate_lines, wait_for_continue, ChoiceView,
};
use crate::input::{Action, InputBindings};
use crate::menu::{render_panel_line, MenuPanelView};
use crate::session::TuiSession;
use crate::ui::DialogUiFile;
use crate::utils::{centered_rect, clamp, palette_style, truncate_line, wrap_text};

#[derive(Clone)]
pub struct MapView {
    pub name: String,
    pub hide_name: bool,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<String>,
    pub legend: HashMap<char, TileRender>,
    pub transitions: Vec<TransitionView>,
    pub vehicles: Vec<VehicleView>,
    pub active_vehicle: Option<ActiveVehicleView>,
    pub npcs: Vec<NpcView>,
    pub signs: Vec<SignView>,
    pub chests: Vec<ChestView>,
    pub doors: Vec<DoorView>,
    pub puzzles: Vec<PuzzleView>,
    pub campfires: Vec<CampfireView>,
    pub save_points: Vec<(i32, i32)>,
    pub use_color: bool,
}

#[derive(Clone)]
pub struct TileRender {
    pub palette: Option<String>,
}

#[derive(Clone)]
pub struct TransitionView {
    pub pos: (i32, i32),
    pub glyph: Option<char>,
    pub palette: Option<String>,
}

#[derive(Clone)]
pub struct VehicleView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
}

#[derive(Clone)]
pub struct ActiveVehicleView {
    pub glyph: char,
    pub palette: Option<String>,
}

#[derive(Clone)]
pub struct NpcView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
}

#[derive(Clone)]
pub struct SignView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
    pub text: String,
}

#[derive(Clone)]
pub struct ChestView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph_closed: char,
    pub glyph_open: char,
    pub palette: Option<String>,
    pub opened: bool,
}

#[derive(Clone)]
pub struct DoorView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
    pub locked: bool,
}

#[derive(Clone)]
pub struct PuzzleView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
}

#[derive(Clone)]
pub struct CampfireView {
    pub id: String,
    pub pos: (i32, i32),
    pub glyph: char,
    pub palette: Option<String>,
}

const DEFAULT_PLAYER_PALETTE: &str = "bright_white";
const DEFAULT_NPC_PALETTE: &str = "bright_yellow";
const DEFAULT_TRANSITION_PALETTE: &str = "bright_magenta";
const DEFAULT_SAVE_POINT_PALETTE: &str = "bright_blue";
const DEFAULT_SIGN_PALETTE: &str = "bright_yellow";
const DEFAULT_CHEST_PALETTE: &str = "bright_yellow";
const DEFAULT_DOOR_PALETTE: &str = "bright_yellow";
const DEFAULT_PUZZLE_PALETTE: &str = "bright_magenta";
const DEFAULT_CAMPFIRE_PALETTE: &str = "bright_red";
const DEFAULT_VEHICLE_PALETTE: &str = "bright_cyan";

pub fn show_dialog_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &DialogUiFile,
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
    dialog_ui: &DialogUiFile,
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
    dialog_ui: &DialogUiFile,
    text: &str,
) -> io::Result<()> {
    let lines = tooltip_lines(session, dialog_ui, text);
    session
        .terminal_mut()
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
    dialog_ui: &DialogUiFile,
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

pub fn show_dialog_with_choices_and_details_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
    choices: &[ChoiceView],
    details: &[MenuPanelView],
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
    choose_dialog_option_with_details_on_map(
        session, map, player_pos, dialog_ui, bindings, speaker, &page, choices, details,
    )
}

pub fn draw_overworld(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_overworld_frame(frame, map, player_pos);
        })
        .map(|_| ())
}

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
                if let Some(active_vehicle) = &map.active_vehicle {
                    glyph = active_vehicle.glyph;
                    palette = active_vehicle
                        .palette
                        .as_deref()
                        .or(Some(DEFAULT_VEHICLE_PALETTE));
                } else {
                    glyph = '@';
                    palette = Some(DEFAULT_PLAYER_PALETTE);
                }
            } else if let Some(npc) = map.npcs.iter().find(|npc| npc.pos == (map_x, map_y)) {
                glyph = npc.glyph;
                palette = npc.palette.as_deref().or(Some(DEFAULT_NPC_PALETTE));
            } else if let Some(vehicle) = map
                .vehicles
                .iter()
                .find(|vehicle| vehicle.pos == (map_x, map_y))
            {
                glyph = vehicle.glyph;
                palette = vehicle.palette.as_deref().or(Some(DEFAULT_VEHICLE_PALETTE));
            } else if let Some(chest) = map.chests.iter().find(|chest| chest.pos == (map_x, map_y))
            {
                glyph = if chest.opened {
                    chest.glyph_open
                } else {
                    chest.glyph_closed
                };
                palette = chest.palette.as_deref().or(Some(DEFAULT_CHEST_PALETTE));
            } else if let Some(sign) = map.signs.iter().find(|sign| sign.pos == (map_x, map_y)) {
                glyph = sign.glyph;
                palette = sign.palette.as_deref().or(Some(DEFAULT_SIGN_PALETTE));
            } else if let Some(door) = map.doors.iter().find(|door| door.pos == (map_x, map_y)) {
                glyph = door.glyph;
                if door.locked {
                    palette = Some("bright_black");
                } else {
                    palette = door.palette.as_deref().or(Some(DEFAULT_DOOR_PALETTE));
                }
            } else if let Some(puzzle) = map
                .puzzles
                .iter()
                .find(|puzzle| puzzle.pos == (map_x, map_y))
            {
                glyph = puzzle.glyph;
                palette = puzzle.palette.as_deref().or(Some(DEFAULT_PUZZLE_PALETTE));
            } else if let Some(campfire) = map
                .campfires
                .iter()
                .find(|campfire| campfire.pos == (map_x, map_y))
            {
                glyph = campfire.glyph;
                palette = campfire
                    .palette
                    .as_deref()
                    .or(Some(DEFAULT_CAMPFIRE_PALETTE));
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
                glyph = '♦';
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

pub fn draw_overworld_with_dialog(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &DialogUiFile,
    speaker: &str,
    lines: &[String],
    choices: Option<(usize, &[ChoiceView])>,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
            if let Some((selected, choices)) = choices {
                draw_choice_box(frame, choices, selected);
            }
        })
        .map(|_| ())
}

pub fn draw_overworld_with_dialog_and_details(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &DialogUiFile,
    speaker: &str,
    lines: &[String],
    choices: &[ChoiceView],
    details: &[MenuPanelView],
    selected: usize,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
            draw_choice_details_box(frame, choices, selected, details.get(selected));
        })
        .map(|_| ())
}

pub fn draw_overworld_with_centered_dialog(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &DialogUiFile,
    lines: &[String],
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_overworld_frame(frame, map, player_pos);
            draw_centered_dialog_overlay(frame, dialog_ui, lines);
        })
        .map(|_| ())
}

pub fn choose_dialog_option_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &DialogUiFile,
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
        if let std::io::Result::Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
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

pub fn choose_dialog_option_with_details_on_map(
    session: &mut TuiSession,
    map: &MapView,
    player_pos: (i32, i32),
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    lines: &[String],
    choices: &[ChoiceView],
    details: &[MenuPanelView],
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        draw_overworld_with_dialog_and_details(
            session, map, player_pos, dialog_ui, speaker, lines, choices, details, selected,
        )?;
        if let std::io::Result::Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
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
                            draw_choice_details_box(
                                frame,
                                choices,
                                selected,
                                details.get(selected),
                            );
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

pub fn tile_at(map: &MapView, x: i32, y: i32) -> char {
    if x < 0 || y < 0 || x >= map.width as i32 || y >= map.height as i32 {
        return ' ';
    }
    map.tiles
        .get(y as usize)
        .and_then(|row| row.chars().nth(x as usize))
        .unwrap_or(' ')
}

pub fn viewport_origin(
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

fn centered_dialog_width(session: &TuiSession) -> usize {
    let area = session.terminal().size().unwrap_or_default();
    let width = centered_dialog_width_for_area(area);
    width.saturating_sub(2).max(1) as usize
}

fn tooltip_lines(session: &TuiSession, dialog_ui: &DialogUiFile, text: &str) -> Vec<String> {
    let area = session.terminal().size().unwrap_or_default();
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
    let area = crate::utils::centered_rect(frame.size(), width, height);
    let inner_width = area.width.saturating_sub(2) as usize;

    frame.render_widget(ratatui::widgets::Clear, area);

    let content = lines
        .iter()
        .map(|line| Line::from(Span::raw(crate::utils::truncate_line(line, inner_width))))
        .collect::<Vec<_>>();
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_choice_details_box(
    frame: &mut Frame,
    choices: &[ChoiceView],
    selected: usize,
    detail: Option<&MenuPanelView>,
) {
    if choices.is_empty() {
        return;
    }

    let area = frame.size();
    let max_choice = choices
        .iter()
        .map(|choice| choice.label.chars().count())
        .max()
        .unwrap_or(0);
    let detail_title_width = detail.map(|entry| entry.title.chars().count()).unwrap_or(0);
    let detail_width = detail
        .map(|entry| entry.lines.iter().map(panel_line_width).max().unwrap_or(0))
        .unwrap_or(0)
        .max(detail_title_width);

    let mut left_width = (max_choice as u16).saturating_add(4).max(12);
    let mut right_width = (detail_width as u16).saturating_add(4).max(18);
    let max_width = area.width.saturating_sub(4).max(20);
    let mut width = left_width.saturating_add(right_width);
    if width > max_width {
        let right_target = max_width.saturating_sub(left_width).max(10);
        if right_target < 12 {
            left_width = max_width.saturating_div(2).max(10);
            right_width = max_width.saturating_sub(left_width).max(10);
        } else {
            right_width = right_target;
        }
        width = left_width.saturating_add(right_width);
    }

    let list_height = choices.len() as u16;
    let detail_height = detail.map(|entry| entry.lines.len()).unwrap_or(0) as u16;
    let max_height = area.height.saturating_sub(4).max(5);
    let height = list_height
        .max(detail_height)
        .saturating_add(2)
        .min(max_height)
        .max(5);
    let panel_area = centered_rect(area, width, height);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_width),
            Constraint::Length(right_width),
        ])
        .split(panel_area);

    let list_inner_width = columns[0].width.saturating_sub(2) as usize;
    let list_inner_height = columns[0].height.saturating_sub(2) as usize;
    let mut list_lines = Vec::new();
    for (index, choice) in choices.iter().enumerate().take(list_inner_height) {
        let style = if index == selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let label = truncate_line(&choice.label, list_inner_width);
        list_lines.push(Line::from(Span::styled(label, style)));
    }
    let list_panel = Paragraph::new(list_lines)
        .block(Block::default().borders(Borders::ALL).title("Recipes"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(list_panel, columns[0]);

    let detail_title = detail
        .map(|entry| entry.title.as_str())
        .unwrap_or("Details");
    let detail_inner_height = columns[1].height.saturating_sub(2) as usize;
    let detail_lines = detail
        .map(|entry| {
            entry
                .lines
                .iter()
                .map(render_panel_line)
                .take(detail_inner_height)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let detail_panel = Paragraph::new(detail_lines)
        .block(Block::default().borders(Borders::ALL).title(detail_title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(detail_panel, columns[1]);
}

fn panel_line_width(line: &crate::menu::MenuPanelLine) -> usize {
    line.spans
        .iter()
        .map(|span| span.text.chars().count())
        .sum()
}
