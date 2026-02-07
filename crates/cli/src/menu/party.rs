use engine::party::actor_row_label;
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, MenuPanelView, PanelSpanStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyList {
    Active,
    Reserve,
}

impl PartyList {
    pub fn toggle(self) -> Self {
        match self {
            PartyList::Active => PartyList::Reserve,
            PartyList::Reserve => PartyList::Active,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PartyMenuEntry {
    pub id: String,
    pub name: String,
    pub row: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyActionId {
    MoveUp,
    MoveDown,
    Swap,
    ToggleRow,
}

#[derive(Clone, Debug)]
pub struct PartyAction {
    pub id: PartyActionId,
    pub label: String,
    pub enabled: bool,
}

pub fn party_menu_entries(runtime: &GameRuntime) -> (Vec<PartyMenuEntry>, Vec<PartyMenuEntry>) {
    let rows_enabled = runtime.content.rules.battle.rows.enabled;
    let active = build_entries(runtime, &runtime.party.active, rows_enabled);
    let reserve = build_entries(runtime, &runtime.party.reserve, rows_enabled);
    (active, reserve)
}

pub fn party_list_entries(runtime: &GameRuntime, list: PartyList) -> Vec<PartyMenuEntry> {
    let (active, reserve) = party_menu_entries(runtime);
    match list {
        PartyList::Active => active,
        PartyList::Reserve => reserve,
    }
}

pub fn party_member_id(runtime: &GameRuntime, list: PartyList, index: usize) -> Option<String> {
    let entries = party_list_entries(runtime, list);
    entries.get(index).map(|entry| entry.id.clone())
}

pub fn party_actions(
    runtime: &GameRuntime,
    list: PartyList,
    member_index: usize,
    swap_allowed: bool,
) -> Vec<PartyAction> {
    let rows_enabled = runtime.content.rules.battle.rows.enabled;
    let active_len = runtime.party.active.len();
    let reserve_len = runtime.party.reserve.len();
    let can_swap = swap_allowed
        && ((list == PartyList::Active && reserve_len > 0)
            || (list == PartyList::Reserve && active_len > 0));
    let mut actions = Vec::new();
    if list == PartyList::Active {
        actions.push(PartyAction {
            id: PartyActionId::MoveUp,
            label: "Move Up".to_string(),
            enabled: member_index > 0,
        });
        actions.push(PartyAction {
            id: PartyActionId::MoveDown,
            label: "Move Down".to_string(),
            enabled: member_index + 1 < active_len,
        });
    }
    actions.push(PartyAction {
        id: PartyActionId::Swap,
        label: if list == PartyList::Active {
            "Swap with Reserve".to_string()
        } else {
            "Swap with Active".to_string()
        },
        enabled: can_swap,
    });
    actions.push(PartyAction {
        id: PartyActionId::ToggleRow,
        label: "Switch Row".to_string(),
        enabled: rows_enabled,
    });
    actions
}

pub fn build_party_panel(
    runtime: &GameRuntime,
    list: PartyList,
    page: usize,
    selection: usize,
    action_selection: usize,
    selected_member: Option<usize>,
    swap_allowed: bool,
) -> MenuPanelView {
    let mut lines = Vec::new();
    match page {
        1 => {
            let entries = party_list_entries(runtime, list);
            let member_index = selected_member
                .unwrap_or(0)
                .min(entries.len().saturating_sub(1));
            let member_name = entries
                .get(member_index)
                .map(|entry| entry.name.as_str())
                .unwrap_or("(None)");
            lines.push(panel_line("Actions", PanelSpanStyle::Accent));
            lines.push(panel_line(
                format!("  {}", member_name),
                PanelSpanStyle::Normal,
            ));
            lines.push(panel_line("", PanelSpanStyle::Normal));
            let actions = party_actions(runtime, list, member_index, swap_allowed);
            if actions.is_empty() {
                lines.push(panel_line("  (None)", PanelSpanStyle::Muted));
            } else {
                for (index, action) in actions.iter().enumerate() {
                    let is_selected = index == action_selection;
                    let mut style = if action.enabled {
                        PanelSpanStyle::Normal
                    } else {
                        PanelSpanStyle::Muted
                    };
                    if is_selected {
                        style = PanelSpanStyle::Highlight;
                    }
                    let prefix = if is_selected { "> " } else { "  " };
                    lines.push(panel_line(format!("{}{}", prefix, action.label), style));
                }
            }
        }
        2 => {
            let (active, reserve) = party_menu_entries(runtime);
            let target_list = list.toggle();
            let source_index = selected_member.unwrap_or(0);
            let target_index = selection;
            lines.push(panel_line("Active", PanelSpanStyle::Accent));
            if active.is_empty() {
                lines.push(panel_line("  (None)", PanelSpanStyle::Muted));
            } else {
                let active_target = if target_list == PartyList::Active {
                    Some(target_index.min(active.len().saturating_sub(1)))
                } else {
                    None
                };
                for (index, entry) in active.iter().enumerate() {
                    let is_target = active_target == Some(index);
                    let is_source = list == PartyList::Active && index == source_index;
                    let (prefix, style) = if is_target {
                        ("> ", PanelSpanStyle::Highlight)
                    } else if is_source {
                        ("* ", PanelSpanStyle::Accent)
                    } else {
                        ("  ", PanelSpanStyle::Normal)
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
                let reserve_target = if target_list == PartyList::Reserve {
                    Some(target_index.min(reserve.len().saturating_sub(1)))
                } else {
                    None
                };
                for (index, entry) in reserve.iter().enumerate() {
                    let is_target = reserve_target == Some(index);
                    let is_source = list == PartyList::Reserve && index == source_index;
                    let (prefix, style) = if is_target {
                        ("> ", PanelSpanStyle::Highlight)
                    } else if is_source {
                        ("* ", PanelSpanStyle::Accent)
                    } else {
                        ("  ", PanelSpanStyle::Normal)
                    };
                    lines.push(panel_line(
                        format!("{}{}", prefix, format_entry(entry)),
                        style,
                    ));
                }
            }
        }
        _ => {
            let (active, reserve) = party_menu_entries(runtime);
            lines.push(panel_line("Active", PanelSpanStyle::Accent));
            if active.is_empty() {
                lines.push(panel_line("  (None)", PanelSpanStyle::Muted));
            } else {
                let active_selection = if list == PartyList::Active {
                    selection.min(active.len().saturating_sub(1))
                } else {
                    usize::MAX
                };
                for (index, entry) in active.iter().enumerate() {
                    let is_selected = index == active_selection;
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

            lines.push(panel_line("", PanelSpanStyle::Normal));
            lines.push(panel_line("Reserve", PanelSpanStyle::Accent));
            if reserve.is_empty() {
                lines.push(panel_line("  (None)", PanelSpanStyle::Muted));
            } else {
                let reserve_selection = if list == PartyList::Reserve {
                    selection.min(reserve.len().saturating_sub(1))
                } else {
                    usize::MAX
                };
                for (index, entry) in reserve.iter().enumerate() {
                    let is_selected = index == reserve_selection;
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
        }
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
