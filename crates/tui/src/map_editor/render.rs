use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::utils::{palette_style, truncate_line};

use super::objects::{moving_label, object_glyph_at, objects_at_cursor};
use super::state::{selection_rect, tile_at, EditorState};
use super::MapData;

pub(super) fn draw_editor_frame(
    frame: &mut Frame,
    state: &EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);
    let map_area = chunks[0];
    let info_area = chunks[1];

    draw_map_panel(frame, map_area, state);
    draw_info_panel(
        frame,
        info_area,
        state,
        map_ids,
        event_ids,
        vehicle_ids,
        npc_ids,
    );
}

fn draw_map_panel(frame: &mut Frame, area: Rect, state: &EditorState) {
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);
    let view_width = inner_width as i32;
    let view_height = inner_height as i32;
    let (start_x, start_y) = viewport_origin(state, view_width, view_height);
    let legend_map = legend_map(&state.map);

    let mut lines = Vec::new();
    for y in 0..view_height {
        let mut row = Vec::new();
        for x in 0..view_width {
            let map_x = start_x + x;
            let map_y = start_y + y;
            let mut glyph = if map_x >= 0
                && map_y >= 0
                && map_x < state.map.width as i32
                && map_y < state.map.height as i32
            {
                tile_at(&state.map, map_x, map_y)
            } else {
                ' '
            };
            let mut style = legend_map
                .get(&glyph)
                .and_then(|entry| entry.palette.as_deref())
                .map(|palette| palette_style(true, Some(palette)))
                .unwrap_or_default();
            if let Some(object_glyph) = object_glyph_at(state, map_x, map_y) {
                glyph = object_glyph;
                style = Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD);
            }
            if let Some((min_x, min_y, max_x, max_y)) = selection_rect(state) {
                if map_x >= min_x && map_x <= max_x && map_y >= min_y && map_y <= max_y {
                    style = style.bg(Color::Yellow).fg(Color::Black);
                }
            }
            if (map_x, map_y) == state.cursor {
                style = style
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }
            row.push(Span::styled(glyph.to_string(), style));
        }
        lines.push(Line::from(row));
    }

    let title = format!("Map: {}", state.map.id);
    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_info_panel(
    frame: &mut Frame,
    area: Rect,
    state: &EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
) {
    let mut lines = Vec::new();
    lines.push(format!("Cursor: {},{}", state.cursor.0, state.cursor.1));
    let mode = if state.selection.is_some() {
        "VISUAL"
    } else {
        "NORMAL"
    };
    lines.push(format!("Mode: {}", mode));
    lines.push(format!("Size: {}x{}", state.map.width, state.map.height));
    lines.push(format!("Active tile: {}", state.active_glyph()));
    if let Some(buffer) = &state.yank {
        lines.push(format!("Yank: {}x{}", buffer.width, buffer.height));
    }
    lines.push(String::new());
    lines.push("Keys:".to_string());
    lines.push("Arrows/HJKL move".to_string());
    lines.push("V visual, y yank, p paste".to_string());
    lines.push("R paint, t tile, L legend".to_string());
    lines.push("o add, e edit, m move".to_string());
    lines.push("x delete, u undo, U redo".to_string());
    lines.push("= resize".to_string());
    lines.push("s save, q quit".to_string());
    lines.push(String::new());
    lines.push("Objects at cursor:".to_string());
    let objects = objects_at_cursor(state);
    if objects.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(objects);
    }
    lines.push(String::new());
    lines.push("Lookups:".to_string());
    lines.push(format!(
        "maps: {}  events: {}",
        map_ids.len(),
        event_ids.len()
    ));
    lines.push(format!(
        "vehicles: {}  npcs: {}",
        vehicle_ids.len(),
        npc_ids.len()
    ));
    lines.push(String::new());
    if let Some(moving) = &state.moving {
        lines.push(format!("Moving: {}", moving_label(&moving.target)));
    }
    lines.push(format!("Status: {}", state.status));

    let width = area.width.saturating_sub(2) as usize;
    let content = lines
        .into_iter()
        .map(|line| truncate_line(&line, width))
        .map(|line| Line::from(Span::raw(line)))
        .collect::<Vec<_>>();
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn viewport_origin(state: &EditorState, view_width: i32, view_height: i32) -> (i32, i32) {
    let map_width = state.map.width as i32;
    let map_height = state.map.height as i32;
    let start_x = if map_width <= view_width {
        -((view_width - map_width) / 2)
    } else {
        let half_width = view_width / 2;
        let max_x = map_width - view_width;
        (state.cursor.0 - half_width).clamp(0, max_x.max(0))
    };
    let start_y = if map_height <= view_height {
        -((view_height - map_height) / 2)
    } else {
        let half_height = view_height / 2;
        let max_y = map_height - view_height;
        (state.cursor.1 - half_height).clamp(0, max_y.max(0))
    };
    (start_x, start_y)
}

fn legend_map(map: &MapData) -> HashMap<char, super::LegendEntry> {
    map.legend
        .iter()
        .cloned()
        .map(|entry| (entry.glyph, entry))
        .collect::<HashMap<_, _>>()
}
