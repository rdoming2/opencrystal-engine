use engine::party::actor_row_label;
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, MenuPanelView, PanelSpanStyle};

#[derive(Clone, Debug)]
pub struct PartyMenuEntry {
    pub id: String,
    pub name: String,
    pub row: Option<String>,
}

pub fn party_menu_entries(runtime: &GameRuntime) -> (Vec<PartyMenuEntry>, Vec<PartyMenuEntry>) {
    let rows_enabled = runtime.content.rules.battle.rows.enabled;
    let active = build_entries(runtime, &runtime.party.active, rows_enabled);
    let reserve = build_entries(runtime, &runtime.party.reserve, rows_enabled);
    (active, reserve)
}

pub fn build_party_panel(
    runtime: &GameRuntime,
    column: usize,
    selection: usize,
    pending_active: Option<usize>,
    swap_allowed: bool,
) -> MenuPanelView {
    let (active, reserve) = party_menu_entries(runtime);
    let mut lines = Vec::new();

    lines.push(panel_line("Active", PanelSpanStyle::Accent));
    if active.is_empty() {
        lines.push(panel_line("  (None)", PanelSpanStyle::Muted));
    } else {
        for (index, entry) in active.iter().enumerate() {
            let is_selected = column == 0 && index == selection;
            let is_pending = pending_active == Some(index);
            let style = if is_selected {
                PanelSpanStyle::Highlight
            } else if is_pending {
                PanelSpanStyle::Accent
            } else {
                PanelSpanStyle::Normal
            };
            let prefix = if is_selected {
                "> "
            } else if is_pending {
                "* "
            } else {
                "  "
            };
            lines.push(panel_line(
                format!("{}{}", prefix, format_entry(entry)),
                style,
            ));
        }
    }

    lines.push(panel_line("", PanelSpanStyle::Normal));
    lines.push(panel_line("Reserve", PanelSpanStyle::Accent));
    if reserve.is_empty() {
        lines.push(panel_line("  (None)", PanelSpanStyle::Muted));
    } else {
        for (index, entry) in reserve.iter().enumerate() {
            let is_selected = column == 1 && index == selection;
            let style = if is_selected {
                PanelSpanStyle::Highlight
            } else {
                PanelSpanStyle::Normal
            };
            let prefix = if is_selected { "> " } else { "  " };
            lines.push(panel_line(
                format!("{}{}", prefix, format_entry(entry)),
                style,
            ));
        }
    }

    if !swap_allowed {
        lines.push(panel_line("", PanelSpanStyle::Normal));
        lines.push(panel_line("Swap unavailable here.", PanelSpanStyle::Muted));
    }

    MenuPanelView {
        title: "Party".to_string(),
        lines,
    }
}

fn build_entries(runtime: &GameRuntime, ids: &[String], rows_enabled: bool) -> Vec<PartyMenuEntry> {
    ids.iter()
        .filter_map(|id| {
            runtime.party.roster.get(id).map(|actor| PartyMenuEntry {
                id: id.clone(),
                name: actor.name.clone(),
                row: if rows_enabled {
                    Some(actor_row_label(actor).to_string())
                } else {
                    None
                },
            })
        })
        .collect()
}

fn format_entry(entry: &PartyMenuEntry) -> String {
    match &entry.row {
        Some(row) => format!("{} ({})", entry.name, row),
        None => entry.name.clone(),
    }
}

fn panel_line(text: impl Into<String>, style: PanelSpanStyle) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: text.into(),
            style,
        }],
    }
}
