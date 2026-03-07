use std::io;

use crate::dialog::prompt_text;
use crate::session::TuiSession;

use super::super::state::{selection_rect, EditorState};
use super::super::MapData;

pub(super) fn rect_from_selection(state: &EditorState) -> Option<[i32; 4]> {
    let (min_x, min_y, max_x, max_y) = selection_rect(state)?;
    Some([min_x, min_y, max_x - min_x + 1, max_y - min_y + 1])
}

pub(super) fn prompt_rect(
    session: &mut TuiSession,
    title: &str,
    default_rect: [i32; 4],
) -> io::Result<Option<[i32; 4]>> {
    let default = format!(
        "{},{},{},{}",
        default_rect[0], default_rect[1], default_rect[2], default_rect[3]
    );
    let value = prompt_text(session, title, "Rect (x,y,w,h):", &default, 24)?;
    let Some(value) = value else {
        return Ok(None);
    };
    parse_rect(&value)
}

pub(super) fn normalize_zone_rect(map: &MapData, rect: [i32; 4]) -> [i32; 4] {
    let max_w = map.width as i32;
    let max_h = map.height as i32;
    if max_w <= 0 || max_h <= 0 {
        return rect;
    }
    let x = rect[0].max(0).min(max_w - 1);
    let y = rect[1].max(0).min(max_h - 1);
    let mut w = rect[2].max(1);
    let mut h = rect[3].max(1);
    w = w.min(max_w - x).max(1);
    h = h.min(max_h - y).max(1);
    [x, y, w, h]
}

pub(super) fn pos_in_rect(pos: [i32; 2], rect: [i32; 4]) -> bool {
    let x = pos[0];
    let y = pos[1];
    x >= rect[0] && y >= rect[1] && x < rect[0] + rect[2] && y < rect[1] + rect[3]
}

fn parse_rect(value: &str) -> io::Result<Option<[i32; 4]>> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Ok(None);
    }
    let x: i32 = parts[0].trim().parse().unwrap_or(0);
    let y: i32 = parts[1].trim().parse().unwrap_or(0);
    let w: i32 = parts[2].trim().parse().unwrap_or(1).max(1);
    let h: i32 = parts[3].trim().parse().unwrap_or(1).max(1);
    Ok(Some([x, y, w, h]))
}
