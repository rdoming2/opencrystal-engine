use std::io;

use crossterm::event::{self, Event, KeyCode};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::is_actionable_key;
use crate::session::TuiSession;

use super::render::draw_editor_frame;
use super::state::{push_undo, tile_at, EditorState};

pub(super) fn resize_map(
    session: &mut TuiSession,
    bindings: &crate::input::InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    let width_default = state.map.width.to_string();
    let height_default = state.map.height.to_string();
    let width_input = prompt_text(session, "Resize", "New width:", &width_default, 6)?;
    let Some(width_input) = width_input else {
        return Ok(());
    };
    let height_input = prompt_text(session, "Resize", "New height:", &height_default, 6)?;
    let Some(height_input) = height_input else {
        return Ok(());
    };
    let new_width: i32 = width_input.trim().parse().unwrap_or(state.map.width as i32);
    let new_height: i32 = height_input
        .trim()
        .parse()
        .unwrap_or(state.map.height as i32);
    if new_width <= 0 || new_height <= 0 {
        state.status = "Invalid size".to_string();
        return Ok(());
    }
    let anchors = vec![
        "center".to_string(),
        "top_left".to_string(),
        "top".to_string(),
        "top_right".to_string(),
        "left".to_string(),
        "right".to_string(),
        "bottom_left".to_string(),
        "bottom".to_string(),
        "bottom_right".to_string(),
    ];
    let anchor_choice = prompt_choice(session, bindings, "Resize", "Anchor:", &anchors, 0)?;
    let Some(anchor_choice) = anchor_choice else {
        return Ok(());
    };
    let anchor = anchors[anchor_choice].as_str();
    let old_width = state.map.width as i32;
    let old_height = state.map.height as i32;
    let (offset_x, offset_y) =
        resize_anchor_offsets(anchor, old_width, old_height, new_width, new_height);
    let warnings = resize_warnings(&state.map, offset_x, offset_y, new_width, new_height);
    if !warnings.is_empty() {
        let proceed = confirm_resize_warnings(session, &warnings, |frame| {
            draw_editor_frame(frame, state, &[], &[], &[], &[]);
        })?;
        if !proceed {
            state.status = "Resize cancelled".to_string();
            return Ok(());
        }
    }
    push_undo(state);
    apply_resize(state, offset_x, offset_y, new_width, new_height);
    state.status = format!("Resized to {}x{}", new_width, new_height);
    state.dirty = true;
    Ok(())
}

fn resize_anchor_offsets(
    anchor: &str,
    old_width: i32,
    old_height: i32,
    new_width: i32,
    new_height: i32,
) -> (i32, i32) {
    let dx = new_width - old_width;
    let dy = new_height - old_height;
    match anchor {
        "top_left" => (0, 0),
        "top" => (dx / 2, 0),
        "top_right" => (dx, 0),
        "left" => (0, dy / 2),
        "right" => (dx, dy / 2),
        "bottom_left" => (0, dy),
        "bottom" => (dx / 2, dy),
        "bottom_right" => (dx, dy),
        _ => (dx / 2, dy / 2),
    }
}

fn resize_warnings(
    map: &super::MapData,
    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for transition in &map.transitions {
        if !pos_in_bounds(transition.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("transition:{}", transition.id));
        }
    }
    for door in &map.doors {
        if !pos_in_bounds(door.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("door:{}", door.id));
        }
    }
    for puzzle in &map.puzzles {
        if !pos_in_bounds(puzzle.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("puzzle:{}", puzzle.id));
        }
    }
    for sign in &map.signs {
        if !pos_in_bounds(sign.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("sign:{}", sign.id));
        }
    }
    for chest in &map.chests {
        if !pos_in_bounds(chest.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("chest:{}", chest.id));
        }
    }
    for vehicle in &map.vehicles {
        if !pos_in_bounds(vehicle.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("vehicle:{}", vehicle.vehicle_id));
        }
    }
    for campfire in &map.campfires {
        if !pos_in_bounds(campfire.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("campfire:{}", campfire.id));
        }
    }
    for save in &map.save_points {
        if !pos_in_bounds(*save, offset_x, offset_y, width, height) {
            warnings.push(format!("save_point:[{},{}]", save[0], save[1]));
        }
    }
    for event in &map.events {
        if let Some(pos) = event.pos {
            if !pos_in_bounds(pos, offset_x, offset_y, width, height) {
                warnings.push(format!("event:{}", event.id));
            }
        }
    }
    for npc in &map.npcs {
        if !pos_in_bounds(npc.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("npc:{}", npc.id));
        }
    }
    for zone in &map.encounters {
        let outcome = truncate_rect(zone.rect, offset_x, offset_y, width, height);
        if outcome.truncated || outcome.removed {
            if outcome.removed {
                warnings.push(format!("encounter:{} (removed)", zone.zone_id));
            } else {
                warnings.push(format!("encounter:{} (truncated)", zone.zone_id));
            }
        }
    }
    warnings
}

fn pos_in_bounds(pos: [i32; 2], offset_x: i32, offset_y: i32, width: i32, height: i32) -> bool {
    let x = pos[0] + offset_x;
    let y = pos[1] + offset_y;
    x >= 0 && y >= 0 && x < width && y < height
}

struct RectOutcome {
    rect: Option<[i32; 4]>,
    truncated: bool,
    removed: bool,
}

fn truncate_rect(
    rect: [i32; 4],
    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,
) -> RectOutcome {
    let x = rect[0] + offset_x;
    let y = rect[1] + offset_y;
    let w = rect[2];
    let h = rect[3];
    if w <= 0 || h <= 0 {
        return RectOutcome {
            rect: None,
            truncated: true,
            removed: true,
        };
    }
    let x2 = x + w;
    let y2 = y + h;
    let new_x = x.max(0);
    let new_y = y.max(0);
    let new_x2 = x2.min(width);
    let new_y2 = y2.min(height);
    if new_x2 <= new_x || new_y2 <= new_y {
        return RectOutcome {
            rect: None,
            truncated: true,
            removed: true,
        };
    }
    let new_w = new_x2 - new_x;
    let new_h = new_y2 - new_y;
    let truncated = new_x != x || new_y != y || new_w != w || new_h != h;
    RectOutcome {
        rect: Some([new_x, new_y, new_w, new_h]),
        truncated,
        removed: false,
    }
}

fn apply_resize(state: &mut EditorState, offset_x: i32, offset_y: i32, width: i32, height: i32) {
    let fill = state.active_glyph();
    let mut tiles = vec![vec![fill; width as usize]; height as usize];
    for y in 0..state.map.height as i32 {
        for x in 0..state.map.width as i32 {
            let new_x = x + offset_x;
            let new_y = y + offset_y;
            if new_x < 0 || new_y < 0 || new_x >= width || new_y >= height {
                continue;
            }
            tiles[new_y as usize][new_x as usize] = tile_at(&state.map, x, y);
        }
    }
    state.map.tiles = tiles;
    state.map.width = width as u32;
    state.map.height = height as u32;

    shift_objects(state, offset_x, offset_y, width, height);
}

fn shift_objects(state: &mut EditorState, offset_x: i32, offset_y: i32, width: i32, height: i32) {
    let within = |pos: [i32; 2]| pos_in_bounds(pos, offset_x, offset_y, width, height);
    for transition in &mut state.map.transitions {
        transition.pos[0] += offset_x;
        transition.pos[1] += offset_y;
    }
    state.map.transitions.retain(|item| within(item.pos));
    for door in &mut state.map.doors {
        door.pos[0] += offset_x;
        door.pos[1] += offset_y;
    }
    state.map.doors.retain(|item| within(item.pos));
    for puzzle in &mut state.map.puzzles {
        puzzle.pos[0] += offset_x;
        puzzle.pos[1] += offset_y;
    }
    state.map.puzzles.retain(|item| within(item.pos));
    for sign in &mut state.map.signs {
        sign.pos[0] += offset_x;
        sign.pos[1] += offset_y;
    }
    state.map.signs.retain(|item| within(item.pos));
    for chest in &mut state.map.chests {
        chest.pos[0] += offset_x;
        chest.pos[1] += offset_y;
    }
    state.map.chests.retain(|item| within(item.pos));
    for vehicle in &mut state.map.vehicles {
        vehicle.pos[0] += offset_x;
        vehicle.pos[1] += offset_y;
    }
    state.map.vehicles.retain(|item| within(item.pos));
    for campfire in &mut state.map.campfires {
        campfire.pos[0] += offset_x;
        campfire.pos[1] += offset_y;
    }
    state.map.campfires.retain(|item| within(item.pos));
    for save in &mut state.map.save_points {
        save[0] += offset_x;
        save[1] += offset_y;
    }
    state.map.save_points.retain(|item| within(*item));
    let mut new_events = Vec::new();
    for mut event in state.map.events.drain(..) {
        if let Some(pos) = event.pos {
            let updated = [pos[0] + offset_x, pos[1] + offset_y];
            if within(updated) {
                event.pos = Some(updated);
                new_events.push(event);
            }
        } else {
            new_events.push(event);
        }
    }
    state.map.events = new_events;
    for npc in &mut state.map.npcs {
        npc.pos[0] += offset_x;
        npc.pos[1] += offset_y;
    }
    state.map.npcs.retain(|item| within(item.pos));

    let mut new_zones = Vec::new();
    for zone in &state.map.encounters {
        let outcome = truncate_rect(zone.rect, offset_x, offset_y, width, height);
        if let Some(rect) = outcome.rect {
            let mut updated = zone.clone();
            updated.rect = rect;
            new_zones.push(updated);
        }
    }
    state.map.encounters = new_zones;
}

fn confirm_resize_warnings<F>(
    session: &mut TuiSession,
    warnings: &[String],
    draw_background: F,
) -> io::Result<bool>
where
    F: Fn(&mut Frame),
{
    let mut offset = 0usize;
    loop {
        session.terminal_mut().draw(|frame| {
            draw_background(frame);
            let area = centered_rect(frame.size(), 60, 18);
            frame.render_widget(ratatui::widgets::Clear, area);
            let title = "Resize Warnings";
            let mut lines = Vec::new();
            lines.push(Line::from(Span::raw("Objects truncated or removed:")));
            let view_height = area.height.saturating_sub(5) as usize;
            for warning in warnings.iter().skip(offset).take(view_height) {
                lines.push(Line::from(Span::raw(warning)));
            }
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::raw(
                "Enter=Proceed  Esc=Cancel  Up/Down=Scroll",
            )));
            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(title))
                .alignment(ratatui::layout::Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            match key.code {
                KeyCode::Enter => return Ok(true),
                KeyCode::Esc => return Ok(false),
                KeyCode::Up => {
                    if offset > 0 {
                        offset -= 1;
                    }
                }
                KeyCode::Down => {
                    if offset + 1 < warnings.len() {
                        offset += 1;
                    }
                }
                _ => {}
            }
        }
    }
}

fn centered_rect(area: ratatui::layout::Rect, width: u16, height: u16) -> ratatui::layout::Rect {
    crate::utils::centered_rect(area, width, height)
}
