use engine::battle::BattleState;
use engine::party::BattleRow;
use engine::runtime::GameRuntime;
use tui::battle::{
    BattleCommandItem, BattleCommandPanelMode, BattleCommandPanelView, BattleEnemyView,
    BattleFocus, BattlePartyView, BattleRenderState,
};
use tui::ui::BattleUiFile;

use super::state::{
    party_target_indices, party_target_rule, BattleMenuState, BattlePhase, TargetMode,
};
use super::CommandEntry;
use crate::menu::abilities::ability_cost_label;
use crate::menu::common::{AbilityEntry, InventoryEntry, SpellEntry};
use crate::menu::magic::spell_cost_label;

pub fn build_battle_render_state(
    runtime: &GameRuntime,
    battle_state: &BattleState,
    menu_state: &BattleMenuState,
    battle_ui: &BattleUiFile,
    command_entries: &[CommandEntry],
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    ability_entries_all: &[AbilityEntry],
    item_entries: &[InventoryEntry],
    is_player_turn: bool,
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
    let show_mp = runtime.content.rules.game.magic_system == engine::rules::MagicSystem::Mp;
    let row_rules = &runtime.content.rules.battle.rows;
    let row_shift = if row_rules.enabled {
        row_rules.battle_shift.max(0) as i32
    } else {
        0
    };
    let party = battle_state
        .party_order
        .iter()
        .enumerate()
        .filter_map(|(index, id)| runtime.party.roster.get(id).map(|actor| (index, id, actor)))
        .map(|(index, id, actor)| {
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
                show_mp,
                readiness: battle_state.readiness_party.get(id).copied().unwrap_or(0.0),
                status: actor
                    .statuses
                    .iter()
                    .filter_map(|status| {
                        engine::battle::status_short_label(&runtime.content, &status.id)
                    })
                    .collect(),
                alive: actor.current_hp > 0,
                active: index == battle_state.active_index,
                glyph,
                palette,
                art,
                art_palette,
                pos: *party_positions.get(index).unwrap_or(&(8, 4)),
                row_offset: if row_rules.enabled && actor.row == BattleRow::Back {
                    row_shift
                } else {
                    0
                },
            }
        })
        .collect();
    let command_panel = build_battle_command_panel(
        runtime,
        menu_state,
        battle_ui,
        command_entries,
        battle_state
            .party_order
            .get(battle_state.active_index)
            .map(|id| id.as_str()),
        spell_entries,
        ability_entries,
        ability_entries_all,
        item_entries,
        is_player_turn,
    );
    let mut selected_enemies = Vec::new();
    let mut selected_party_members = Vec::new();
    if menu_state.phase == BattlePhase::TargetEnemy
        && menu_state.target_mode == TargetMode::Multi
        && matches!(
            menu_state.pending_action,
            Some(super::state::PendingBattleAction::Magic(_))
                | Some(super::state::PendingBattleAction::Ability(_))
        )
    {
        selected_enemies = super::state::enemy_target_indices(battle_state);
    }
    if menu_state.phase == BattlePhase::TargetParty
        && menu_state.target_mode == TargetMode::Multi
        && matches!(
            menu_state.pending_action,
            Some(super::state::PendingBattleAction::Magic(_))
                | Some(super::state::PendingBattleAction::Ability(_))
        )
    {
        if let Some(action) = menu_state.pending_action.as_ref() {
            let rule = party_target_rule(action, runtime);
            selected_party_members = party_target_indices(runtime, battle_state, rule);
        }
    }

    BattleRenderState {
        enemies,
        party,
        command_panel,
        selected_enemy: menu_state.enemy_index,
        selected_party: menu_state.party_index,
        selected_enemies,
        selected_party_members,
        focus: battle_focus(menu_state),
        log: battle_state.log.clone(),
        paused: menu_state.paused,
        pause_title: crate::battle::ui_text(runtime, "battle.pause_title", "PAUSED"),
        pause_hint: crate::battle::ui_text(runtime, "battle.pause_hint", "Press Pause to resume"),
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
    command_entries: &[CommandEntry],
    _actor_id: Option<&str>,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    _ability_entries_all: &[AbilityEntry],
    item_entries: &[InventoryEntry],
    is_player_turn: bool,
) -> BattleCommandPanelView {
    if !is_player_turn {
        return BattleCommandPanelView {
            mode: BattleCommandPanelMode::Commands,
            title: String::new(),
            items: Vec::new(),
            columns: Vec::new(),
            rows: Vec::new(),
            selected: 0,
        };
    }
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
                .map(|entry| {
                    vec![
                        if entry.usable {
                            entry.name.clone()
                        } else {
                            format!("{} *", entry.name)
                        },
                        ability_cost_label_for_battle(runtime, entry),
                    ]
                })
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
        _ => {
            let title = battle_ui.panels.commands.title.clone();
            let selected = if command_entries.is_empty() {
                0
            } else {
                menu_state
                    .command_index
                    .min(command_entries.len().saturating_sub(1))
            };
            let items = command_entries
                .iter()
                .map(|command| BattleCommandItem {
                    label: command.label.clone(),
                    enabled: true,
                })
                .collect();
            BattleCommandPanelView {
                mode: BattleCommandPanelMode::Commands,
                title,
                items,
                columns: Vec::new(),
                rows: Vec::new(),
                selected,
            }
        }
    }
}

fn ability_cost_label_for_battle(runtime: &GameRuntime, entry: &AbilityEntry) -> String {
    ability_cost_label(runtime, entry)
}

pub fn battle_focus(menu_state: &BattleMenuState) -> BattleFocus {
    match menu_state.phase {
        BattlePhase::TargetEnemy => BattleFocus::Enemies,
        BattlePhase::TargetParty => BattleFocus::Party,
        _ => BattleFocus::Commands,
    }
}
