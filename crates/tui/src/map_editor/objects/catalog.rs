use super::super::state::{CursorObject, EditorState, ObjectGlyphMode};
use super::geometry::pos_in_rect;
use super::ObjectGlyph;

pub(super) fn cursor_objects(
    state: &EditorState,
    pos: [i32; 2],
) -> (Vec<String>, Vec<CursorObject>) {
    let entries = object_entries_at_pos(state, pos);
    let choices = entries.iter().map(|entry| entry.label.clone()).collect();
    let refs = entries.iter().map(|entry| entry.cursor).collect();
    (choices, refs)
}

pub(super) fn object_glyph_at(state: &EditorState, x: i32, y: i32) -> Option<ObjectGlyph> {
    let pos = [x, y];
    let use_configured = matches!(state.object_glyphs, ObjectGlyphMode::Configured);
    let entry = object_entries_at_pos(state, pos)
        .into_iter()
        .find(|entry| !matches!(entry.cursor, CursorObject::EncounterZone(_)))?;
    Some({
        let (glyph, palette) = if use_configured {
            let glyph = entry.configured_glyph.unwrap_or(entry.marker_glyph);
            let palette = entry.configured_glyph.and_then(|_| entry.palette.clone());
            (glyph, palette)
        } else {
            (entry.marker_glyph, None)
        };
        ObjectGlyph { glyph, palette }
    })
}

pub(super) fn objects_at_cursor(state: &EditorState) -> Vec<String> {
    let pos = [state.cursor.0, state.cursor.1];
    object_entries_at_pos(state, pos)
        .into_iter()
        .map(|entry| entry.label)
        .collect()
}

pub(super) fn object_entries_at_pos(state: &EditorState, pos: [i32; 2]) -> Vec<ObjectEntry> {
    let mut entries = Vec::new();
    for (index, item) in state.map.transitions.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            entries.push(ObjectEntry {
                label: format!(
                    "transition:{} -> {}@{},{}",
                    item.id, item.target_map, item.target_pos[0], item.target_pos[1]
                ),
                cursor: CursorObject::Transition(index),
                marker_glyph: 'T',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.doors.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            let label = if let Some(target_map) = item.target_map.as_deref() {
                if let Some(target_pos) = item.target_pos {
                    format!(
                        "door:{} -> {}@{},{}",
                        item.id, target_map, target_pos[0], target_pos[1]
                    )
                } else {
                    format!("door:{} -> {}", item.id, target_map)
                }
            } else {
                format!("door:{}", item.id)
            };
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Door(index),
                marker_glyph: '+',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.puzzles.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            let label = if let Some(event) = item.event.as_deref() {
                format!("puzzle:{} event:{}", item.id, event)
            } else if let Some(set_flag) = item.set_flag.as_deref() {
                format!("puzzle:{} set:{}", item.id, set_flag)
            } else {
                format!("puzzle:{}", item.id)
            };
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Puzzle(index),
                marker_glyph: '?',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.signs.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            let preview = truncate_label(&item.text, 24);
            entries.push(ObjectEntry {
                label: format!("sign:{} \"{}\"", item.id, preview),
                cursor: CursorObject::Sign(index),
                marker_glyph: '!',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.chests.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph_closed)
                .or_else(|| glyph_from_option(&item.glyph_open));
            entries.push(ObjectEntry {
                label: format!("chest:{} flag:{}", item.id, item.opened_flag),
                cursor: CursorObject::Chest(index),
                marker_glyph: 'C',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.vehicles.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("vehicle:{}", item.vehicle_id),
                cursor: CursorObject::Vehicle(index),
                marker_glyph: 'V',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    for (index, item) in state.map.campfires.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            entries.push(ObjectEntry {
                label: format!("campfire:{} set:{}", item.id, item.campfire_id),
                cursor: CursorObject::Campfire(index),
                marker_glyph: 'F',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.events.iter().enumerate() {
        if item.pos == Some(pos) {
            let mut label = format!("event:{} {} script:{}", item.id, item.trigger, item.script);
            if let Some(zone) = item.zone.as_deref() {
                label.push_str(&format!(" zone:{}", zone));
            }
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Event(index),
                marker_glyph: 'E',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    for (index, item) in state.map.npcs.iter().enumerate() {
        if item.pos == pos {
            let label = if let Some(script) = item.script.as_deref() {
                format!("npc:{} script:{}", item.id, script)
            } else {
                format!("npc:{}", item.id)
            };
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Npc(index),
                marker_glyph: 'N',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    for (index, item) in state.map.encounters.iter().enumerate() {
        if pos_in_rect(pos, item.rect) {
            entries.push(ObjectEntry {
                label: format!(
                    "encounter_zone:{} table:{} rect:{},{},{}x{}",
                    item.zone_id,
                    item.table,
                    item.rect[0],
                    item.rect[1],
                    item.rect[2],
                    item.rect[3]
                ),
                cursor: CursorObject::EncounterZone(index),
                marker_glyph: 'Z',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    if state.map.save_points.iter().any(|entry| *entry == pos) {
        entries.push(ObjectEntry {
            label: "save_point".to_string(),
            cursor: CursorObject::SavePoint,
            marker_glyph: 'S',
            configured_glyph: None,
            palette: None,
        });
    }
    entries
}

pub(super) struct ObjectEntry {
    pub(super) label: String,
    pub(super) cursor: CursorObject,
    pub(super) marker_glyph: char,
    pub(super) configured_glyph: Option<char>,
    pub(super) palette: Option<String>,
}

fn glyph_from_option(value: &Option<String>) -> Option<char> {
    value.as_ref().and_then(|glyph| glyph.chars().next())
}

fn truncate_label(value: &str, max: usize) -> String {
    let mut chars = value.chars();
    let collected: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{}...", collected)
    } else {
        collected
    }
}
