use engine::battle::BattleState;
use engine::runtime::GameRuntime;
use tui::app::{
    BattleCommandItem, BattleCommandPanelMode, BattleCommandPanelView, BattleEnemyView,
    BattleFocus, BattlePartyView, BattleRenderState,
};
use tui::ui::BattleUiFile;

use super::state::{BattleMenuState, BattlePhase};
use crate::menu::common::{AbilityEntry, InventoryEntry, SpellEntry};
use crate::menu::magic::spell_cost_label;

pub fn build_battle_render_state(
    runtime: &GameRuntime,
    battle_state: &BattleState,
    menu_state: &BattleMenuState,
    battle_ui: &BattleUiFile,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> BattleRenderState {
    let enemies = battle_state
        .enemies
        .iter()
        .map(|enemy| BattleEnemyView {
            name: enemy.name.clone(),
            hp: enemy.current_hp,
            max_hp: enemy.max_hp(),
            glyph: enemy.sprite.glyph.chars().next().unwrap_or('!'),
            palette: Some(enemy.sprite.palette.clone()),
            art: enemy.art.as_ref().map(|art| art.lines.clone()),
            art_palette: enemy.art.as_ref().map(|art| art.palette.clone()),
            pos: enemy.pos,
            alive: enemy.is_alive(),
            show_hp: enemy.scanned,
        })
        .collect();
    let party_positions = party_sprite_positions(
        battle_state.party_order.len(),
        battle_ui.layout.party_grid.columns,
    );
    let party = battle_state
        .party_order
        .iter()
        .enumerate()
        .filter_map(|(index, id)| runtime.party.roster.get(id).map(|actor| (index, actor)))
        .map(|(index, actor)| {
            let job = runtime
                .content
                .jobs
                .jobs
                .iter()
                .find(|job| job.id == actor.job_id);
            let (glyph, palette, art, art_palette) = job
                .map(|job| {
                    (
                        job.sprite.glyph.chars().next().unwrap_or('@'),
                        Some(job.sprite.palette.clone()),
                        job.art.as_ref().map(|art| art.lines.clone()),
                        job.art.as_ref().map(|art| art.palette.clone()),
                    )
                })
                .unwrap_or((
                    actor.name.chars().next().unwrap_or('@'),
                    Some("player".to_string()),
                    None,
                    None,
                ));
            BattlePartyView {
                name: actor.name.clone(),
                hp: actor.current_hp,
                max_hp: actor.derived_stats.get("hp").copied().unwrap_or(0),
                mp: actor.current_mp,
                max_mp: actor.derived_stats.get("mp").copied().unwrap_or(0),
                status: Vec::new(),
                alive: actor.current_hp > 0,
                active: index == battle_state.active_index,
                glyph,
                palette,
                art,
                art_palette,
                pos: *party_positions.get(index).unwrap_or(&(8, 4)),
            }
        })
        .collect();
    let command_panel = build_battle_command_panel(
        runtime,
        menu_state,
        battle_ui,
        spell_entries,
        ability_entries,
        item_entries,
    );
    BattleRenderState {
        enemies,
        party,
        command_panel,
        selected_enemy: menu_state.enemy_index,
        selected_party: menu_state.party_index,
        focus: battle_focus(menu_state),
        log: battle_state.log.clone(),
        use_color: runtime
            .content
            .rules
            .render
            .palette
            .eq_ignore_ascii_case("terminal"),
        flash_enemies: Vec::new(),
        flash_party: Vec::new(),
        acting_enemies: Vec::new(),
        acting_party: Vec::new(),
    }
}

pub fn party_sprite_positions(count: usize, columns: u16) -> Vec<(i32, i32)> {
    if count == 0 {
        return Vec::new();
    }
    let columns = columns.max(1).min(10) as usize;
    let rows = (count + columns - 1) / columns;
    let rows = rows.min(6).max(1);
    let start_col = (10 - columns) as i32;
    let start_row = ((6 - rows) / 2) as i32;
    (0..count)
        .map(|index| {
            let col = (index % columns) as i32;
            let row = (index / columns) as i32;
            (start_col + col, start_row + row)
        })
        .collect()
}

pub fn build_battle_command_panel(
    runtime: &GameRuntime,
    menu_state: &BattleMenuState,
    battle_ui: &BattleUiFile,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> BattleCommandPanelView {
    match menu_state.phase {
        BattlePhase::Magic => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Magic,
            title: battle_ui.panels.commands.title.clone(),
            items: Vec::new(),
            columns: battle_ui
                .menus
                .magic
                .columns
                .iter()
                .map(|column| column.label.clone())
                .collect(),
            rows: spell_entries
                .iter()
                .map(|entry| {
                    vec![
                        if entry.usable {
                            entry.name.clone()
                        } else {
                            format!("{} *", entry.name)
                        },
                        spell_cost_label(entry),
                    ]
                })
                .collect(),
            selected: menu_state
                .magic_index
                .min(spell_entries.len().saturating_sub(1)),
        },
        BattlePhase::Abilities => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Abilities,
            title: battle_ui.panels.commands.title.clone(),
            items: Vec::new(),
            columns: battle_ui
                .menus
                .abilities
                .columns
                .iter()
                .map(|column| column.label.clone())
                .collect(),
            rows: ability_entries
                .iter()
                .map(|entry| vec![entry.name.clone()])
                .collect(),
            selected: menu_state
                .ability_index
                .min(ability_entries.len().saturating_sub(1)),
        },
        BattlePhase::Items => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Items,
            title: battle_ui.panels.commands.title.clone(),
            items: Vec::new(),
            columns: battle_ui
                .menus
                .items
                .columns
                .iter()
                .map(|column| column.label.clone())
                .collect(),
            rows: item_entries
                .iter()
                .map(|entry| vec![entry.label.clone(), entry.available_qty.to_string()])
                .collect(),
            selected: menu_state
                .item_index
                .min(item_entries.len().saturating_sub(1)),
        },
        _ => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Commands,
            title: battle_ui.panels.commands.title.clone(),
            items: battle_ui
                .panels
                .commands
                .items
                .iter()
                .map(|label| BattleCommandItem {
                    label: label.clone(),
                    enabled: command_enabled(
                        runtime,
                        label,
                        spell_entries,
                        ability_entries,
                        item_entries,
                    ),
                })
                .collect(),
            columns: Vec::new(),
            rows: Vec::new(),
            selected: menu_state
                .command_index
                .min(battle_ui.panels.commands.items.len().saturating_sub(1)),
        },
    }
}

pub fn battle_focus(menu_state: &BattleMenuState) -> BattleFocus {
    match menu_state.phase {
        BattlePhase::TargetEnemy => BattleFocus::Enemies,
        BattlePhase::TargetParty => BattleFocus::Party,
        _ => BattleFocus::Commands,
    }
}

fn command_enabled(
    runtime: &GameRuntime,
    label: &str,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> bool {
    match crate::battle::command_kind(label) {
        Some(crate::battle::CommandKind::Magic) => {
            crate::menu::system_enabled(runtime, Some("magic")) && !spell_entries.is_empty()
        }
        Some(crate::battle::CommandKind::Abilities) => !ability_entries.is_empty(),
        Some(crate::battle::CommandKind::Items) => {
            crate::menu::system_enabled(runtime, Some("items")) && !item_entries.is_empty()
        }
        _ => true,
    }
}
