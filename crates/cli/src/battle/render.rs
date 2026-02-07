use engine::battle::BattleState;
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
use crate::menu::abilities::ability_group_available;
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
    actor_id: Option<&str>,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    ability_entries_all: &[AbilityEntry],
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
            let command_count = command_entries.len();
            let page_size = battle_ui.panels.commands.page_size.max(1);
            let selected = if command_count == 0 {
                0
            } else {
                menu_state
                    .command_index
                    .min(command_count.saturating_sub(1))
            };
            let total_pages = if command_count == 0 {
                1
            } else {
                (command_count + page_size - 1) / page_size
            };
            let page = if command_count == 0 {
                0
            } else {
                selected / page_size
            };
            let start = page.saturating_mul(page_size);
            let end = (start + page_size).min(command_count);
            let title = if total_pages > 1 {
                format!(
                    "{} ({}/{})",
                    battle_ui.panels.commands.title,
                    page + 1,
                    total_pages
                )
            } else {
                battle_ui.panels.commands.title.clone()
            };
            let items = command_entries
                .get(start..end)
                .unwrap_or(&[])
                .iter()
                .map(|command| BattleCommandItem {
                    label: command.label.clone(),
                    enabled: command_enabled(
                        runtime,
                        actor_id,
                        command,
                        spell_entries,
                        ability_entries_all,
                        item_entries,
                    ),
                })
                .collect();
            BattleCommandPanelView {
                mode: BattleCommandPanelMode::Commands,
                title,
                items,
                columns: Vec::new(),
                rows: Vec::new(),
                selected: selected.saturating_sub(start),
            }
        }
    }
}

fn ability_cost_label_for_battle(runtime: &GameRuntime, entry: &AbilityEntry) -> String {
    match entry.cost_type.as_str() {
        "currency" => format!(
            " {} {}",
            runtime.content.rules.game.currency.symbol, entry.cost_value
        ),
        _ => ability_cost_label(entry),
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
    actor_id: Option<&str>,
    command: &CommandEntry,
    spell_entries: &[SpellEntry],
    ability_entries_all: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> bool {
    match command.kind {
        crate::battle::CommandKind::Magic => {
            crate::menu::system_enabled(runtime, Some("magic")) && !spell_entries.is_empty()
        }
        crate::battle::CommandKind::Abilities => !ability_entries_all.is_empty(),
        crate::battle::CommandKind::AbilitiesGroup => actor_id
            .and_then(|actor_id| {
                command
                    .ability_group
                    .as_deref()
                    .map(|group| (actor_id, group))
            })
            .map(|(actor_id, group)| ability_group_available(runtime, actor_id, group))
            .unwrap_or(false),
        crate::battle::CommandKind::Items => {
            crate::menu::system_enabled(runtime, Some("items")) && !item_entries.is_empty()
        }
        _ => true,
    }
}
