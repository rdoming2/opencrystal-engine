use std::path::{Path, PathBuf};

use engine::runtime::GameRuntime;
use engine::save::SaveFile;
use tui::menu::{MenuPanelLine, MenuPanelView, PanelSpanStyle};

use super::inventory::{panel_line, panel_line_spans, panel_span};

#[derive(Clone, Debug)]
pub(super) struct SaveSlotEntry {
    pub(super) slot: u8,
    label: String,
    save: Option<SaveFile>,
    pub(super) selectable: bool,
}

pub(super) struct SaveMessage {
    pub(super) text: String,
    pub(super) style: PanelSpanStyle,
}

pub(super) fn build_save_panel(
    runtime: &GameRuntime,
    save_dir: &Path,
    save_message: Option<&SaveMessage>,
) -> MenuPanelView {
    let slots = build_save_slots(runtime, save_dir);
    if slots.is_empty() {
        return MenuPanelView {
            title: "Save".to_string(),
            lines: vec![panel_line("No save slots configured.")],
        };
    }

    let selection = runtime
        .menu_state
        .detail_selection
        .min(slots.len().saturating_sub(1));
    let mut lines = Vec::new();
    for (index, slot) in slots.iter().enumerate() {
        lines.push(render_save_slot_line(runtime, slot, index == selection));
    }
    if let Some(message) = save_message {
        lines.push(panel_line_spans(vec![panel_span(
            message.text.clone(),
            message.style.clone(),
        )]));
    }

    MenuPanelView {
        title: "Save".to_string(),
        lines,
    }
}

fn render_save_slot_line(
    runtime: &GameRuntime,
    slot: &SaveSlotEntry,
    selected: bool,
) -> MenuPanelLine {
    let prefix = if selected { "> " } else { "  " };
    let style = if selected {
        tui::menu::PanelSpanStyle::Highlight
    } else if slot.selectable {
        tui::menu::PanelSpanStyle::Normal
    } else {
        tui::menu::PanelSpanStyle::Muted
    };

    let mut text = format!("{}{}", prefix, slot.label);
    if let Some(save) = &slot.save {
        let map_name =
            map_name_for_save(runtime, save).unwrap_or_else(|| save.world.map_id.clone());
        let playtime = format_playtime(save.metadata.play_time_seconds);
        text.push_str(" - ");
        text.push_str(map_name.as_str());
        text.push_str("  ");
        text.push_str(playtime.as_str());
    } else {
        text.push_str(" - Empty");
    }

    panel_line_spans(vec![panel_span(text, style)])
}

pub(super) fn build_save_slots(runtime: &GameRuntime, save_dir: &Path) -> Vec<SaveSlotEntry> {
    let mut slots = Vec::new();
    if runtime.effective_autosave_enabled() {
        slots.push(build_save_slot_entry(save_dir, 0, "Autosave", false));
    }
    let max_slots = runtime.content.rules.save.slots_max.max(1) as u8;
    for slot in 1..=max_slots {
        slots.push(build_save_slot_entry(
            save_dir,
            slot,
            &format!("Slot {}", slot),
            true,
        ));
    }
    slots
}

fn build_save_slot_entry(
    save_dir: &Path,
    slot: u8,
    label: &str,
    selectable: bool,
) -> SaveSlotEntry {
    let save = load_save_slot(save_dir, slot);
    SaveSlotEntry {
        slot,
        label: label.to_string(),
        save,
        selectable,
    }
}

fn load_save_slot(save_dir: &Path, slot: u8) -> Option<SaveFile> {
    let path = save_slot_path(save_dir, slot);
    let save = SaveFile::load(path).ok()?;
    if save.version == 0 {
        return None;
    }
    Some(save)
}

fn save_slot_path(save_dir: &Path, slot: u8) -> PathBuf {
    save_dir.join(format!("slot_{}.json", slot))
}

pub(super) fn write_save_slot(
    runtime: &GameRuntime,
    save_dir: &Path,
    slot: u8,
) -> Result<(), String> {
    std::fs::create_dir_all(save_dir).map_err(|err| format!("{}: {}", save_dir.display(), err))?;
    let save = SaveFile::from_runtime(runtime, slot);
    let path = save_slot_path(save_dir, slot);
    save.write(path)
}

fn first_selectable_save_slot(slots: &[SaveSlotEntry]) -> Option<usize> {
    slots.iter().position(|slot| slot.selectable)
}

pub(super) fn default_save_selection(
    runtime: &GameRuntime,
    slots: &[SaveSlotEntry],
) -> Option<usize> {
    if let Some(slot_id) = runtime.last_manual_save_slot {
        if let Some(index) = slots
            .iter()
            .position(|slot| slot.selectable && slot.slot == slot_id)
        {
            return Some(index);
        }
    }
    first_selectable_save_slot(slots)
}

pub(super) fn move_save_selection(
    current: usize,
    slots: &[SaveSlotEntry],
    direction: i32,
) -> usize {
    if slots.is_empty() {
        return 0;
    }
    let mut index = current.min(slots.len().saturating_sub(1));
    let mut remaining = slots.len();
    while remaining > 0 {
        if direction < 0 {
            index = if index == 0 {
                slots.len().saturating_sub(1)
            } else {
                index - 1
            };
        } else {
            index = if index + 1 >= slots.len() {
                0
            } else {
                index + 1
            };
        }
        if slots[index].selectable {
            return index;
        }
        remaining -= 1;
    }
    current
}

fn format_playtime(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn map_name_for_save(runtime: &GameRuntime, save: &SaveFile) -> Option<String> {
    let index = runtime.content.map_index.get(save.world.map_id.as_str())?;
    let map = runtime.content.maps.get(*index)?;
    if map.name.trim().is_empty() {
        None
    } else {
        Some(map.name.clone())
    }
}
