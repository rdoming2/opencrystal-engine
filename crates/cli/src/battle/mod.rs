pub mod actions;
pub mod logic;
pub mod render;
pub mod state;

use std::collections::{HashMap, HashSet};
use std::thread::sleep;
use std::time::{Duration, Instant};

use self::logic::update_readiness;
use engine::battle::{
    apply_turn_start_statuses, build_battle_state, collect_rewards, is_enemies_defeated,
    is_party_defeated, BattleMode, BattleResult, BattleState, LevelUpDiff,
};
use engine::party::{actor_row_label, gain_exp, toggle_actor_row};
use engine::rules::{JobProgressionMode, Ruleset};
use engine::runtime::GameRuntime;
use rand::Rng;
use tui::battle::{draw_battle, draw_battle_frame, BattleRenderState};
use tui::dialog::confirm_quit;
use tui::input::{is_actionable_key, Action, InputBindings};
use tui::session::TuiSession;
use tui::ui::BattleUiFile;

use self::actions::{
    execute_ability_action, execute_attack_action, execute_item_action, execute_magic_action,
};
use self::logic::{advance_turn, build_turn_order, enemy_take_turn, push_battle_log};
use self::render::build_battle_render_state;
use self::state::{
    enemy_target_indices, ensure_valid_index, party_target_indices, party_target_rule,
    set_initial_enemy_target, set_initial_party_target, step_target_index, BattleMenuState,
    BattlePhase, BattleTurnActor, BattleTurnState, PendingBattleAction, TargetMode, TargetRule,
    TargetSide, VictoryState,
};
use crate::menu::abilities::build_battle_ability_entries;
use crate::menu::common::{AbilityEntry, InventoryEntry, SpellEntry};
use crate::menu::inventory::build_battle_item_entries;
use crate::menu::magic::build_battle_spell_entries;
use crate::utils::read_action;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BattleOutcome {
    Victory,
    Defeat,
    Escaped,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BattleSource {
    Random,
    Event { event_id: String, event_step: usize },
}

#[derive(Clone, Debug)]
pub struct BattleSnapshot {
    pub save: engine::save::SaveFile,
    pub event_queue: Vec<String>,
    pub active_event: Option<String>,
    pub event_step: usize,
    pub state: engine::runtime::GameState,
}

impl BattleSnapshot {
    pub fn capture(runtime: &GameRuntime) -> Self {
        Self {
            save: engine::save::SaveFile::from_runtime(runtime, 0),
            event_queue: runtime.event_queue.clone(),
            active_event: runtime.active_event.clone(),
            event_step: runtime.event_step,
            state: runtime.state.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LastBattleContext {
    pub formation: Vec<engine::encounters::EncounterMember>,
    pub snapshot: BattleSnapshot,
    pub source: BattleSource,
}

impl LastBattleContext {
    pub fn new(
        formation: Vec<engine::encounters::EncounterMember>,
        snapshot: BattleSnapshot,
        source: BattleSource,
    ) -> Self {
        Self {
            formation,
            snapshot,
            source,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BattleReport {
    pub outcome: BattleOutcome,
    pub formation: Vec<engine::encounters::EncounterMember>,
    pub snapshot: BattleSnapshot,
}

pub fn ui_text(runtime: &GameRuntime, key: &str, default: &str) -> String {
    runtime.content.ui_text(key).unwrap_or(default).to_string()
}

pub fn format_ui_text(
    runtime: &GameRuntime,
    key: &str,
    default: &str,
    vars: &[(&str, String)],
) -> String {
    let mut text = ui_text(runtime, key, default);
    for (name, value) in vars {
        text = text.replace(&format!("{{{}}}", name), value);
    }
    text
}

fn format_currency_rewards(
    rules: &engine::rules::RulesFile,
    rewards: &HashMap<String, i32>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut seen = HashSet::new();
    for currency in &rules.game.currencies {
        let amount = rewards.get(currency.id.as_str()).copied().unwrap_or(0);
        if amount <= 0 {
            continue;
        }
        seen.insert(currency.id.as_str());
        if currency.symbol.trim().is_empty() {
            lines.push(format!("{} {}", amount, currency.name));
        } else {
            lines.push(format!("{}{}", currency.symbol, amount));
        }
    }

    let mut extras: Vec<_> = rewards
        .iter()
        .filter(|(id, amount)| **amount > 0 && !seen.contains(id.as_str()))
        .collect();
    extras.sort_by(|left, right| left.0.cmp(right.0));
    for (currency_id, amount) in extras {
        lines.push(format!("{} {}", amount, currency_id));
    }

    lines
}

pub fn run_battle(
    runtime: &mut GameRuntime,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    formation: &[engine::encounters::EncounterMember],
    rng: &mut impl Rng,
) -> std::io::Result<BattleOutcome> {
    let mut battle_state = build_battle_state(
        &runtime.content,
        &runtime.party,
        formation,
        runtime.effective_battle_mode(),
    );
    let Some(start_index) = engine::battle::next_living_party_index(
        &runtime.party,
        &battle_state.party_order,
        battle_state.active_index,
    ) else {
        return Ok(BattleOutcome::Defeat);
    };
    battle_state.active_index = start_index;
    push_battle_log(
        &mut battle_state.log,
        ui_text(runtime, "battle.start", "A battle begins!"),
    );

    let mut menu_state = BattleMenuState::new();
    let mut turn_state = if matches!(
        battle_state.mode,
        BattleMode::Dynamic | BattleMode::DynamicWait
    ) {
        BattleTurnState::new(Vec::new())
    } else {
        BattleTurnState::new(build_turn_order(runtime, &battle_state))
    };
    let mut last_tick = Instant::now();
    let mut last_actor_id: Option<String> = None;
    let mut battle_result: Option<BattleResult> = None;
    let mut victory_state: Option<VictoryState> = None;

    loop {
        if is_enemies_defeated(&battle_state.enemies) {
            if menu_state.phase != BattlePhase::Victory {
                let empty_commands: Vec<CommandEntry> = Vec::new();
                let empty_spells: Vec<SpellEntry> = Vec::new();
                let empty_abilities: Vec<AbilityEntry> = Vec::new();
                let empty_items: Vec<InventoryEntry> = Vec::new();
                let render_state = build_battle_render_state(
                    runtime,
                    &battle_state,
                    &menu_state,
                    battle_ui,
                    &empty_commands,
                    &empty_spells,
                    &empty_abilities,
                    &empty_abilities,
                    &empty_items,
                    false,
                );
                pause_after_action(
                    session,
                    battle_ui,
                    bindings,
                    &render_state,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )?;
                menu_state.phase = BattlePhase::Victory;
                battle_result = Some(apply_battle_rewards(runtime, &mut battle_state, rng));
                victory_state = if battle_result
                    .as_ref()
                    .map(|r| !r.level_ups.is_empty())
                    .unwrap_or(false)
                {
                    Some(VictoryState::LevelUp(0))
                } else {
                    Some(VictoryState::Summary)
                };
                push_battle_log(
                    &mut battle_state.log,
                    ui_text(runtime, "battle.victory", "Victory!"),
                );
            }
        }
        if is_party_defeated(&runtime.party, &battle_state.party_order) {
            if menu_state.phase != BattlePhase::Defeat {
                menu_state.phase = BattlePhase::Defeat;
                push_battle_log(
                    &mut battle_state.log,
                    ui_text(runtime, "battle.defeat", "Defeat..."),
                );
            }
        }

        let now = Instant::now();
        let delta = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;

        if matches!(battle_state.mode, BattleMode::Turn) {
            if turn_state.order.is_empty() || turn_state.index >= turn_state.order.len() {
                turn_state.reset(build_turn_order(runtime, &battle_state));
            }
            if turn_state.order.is_empty() {
                return Ok(BattleOutcome::Defeat);
            }
        } else {
            // Readiness logic
            let paused = menu_state.paused
                || (battle_state.mode == BattleMode::DynamicWait
                    && matches!(
                        menu_state.phase,
                        BattlePhase::Magic
                            | BattlePhase::Abilities
                            | BattlePhase::Items
                            | BattlePhase::TargetEnemy
                            | BattlePhase::TargetParty
                    ));

            if !paused {
                let new_ready = update_readiness(runtime, &mut battle_state, delta);
                turn_state.order.extend(new_ready);
            }

            // Process ready enemies immediately for Dynamic modes
            if matches!(
                battle_state.mode,
                BattleMode::Dynamic | BattleMode::DynamicWait
            ) && !paused
            {
                let mut enemy_indices = Vec::new();
                turn_state.order.retain(|actor| {
                    if let BattleTurnActor::Enemy(index) = actor {
                        enemy_indices.push(*index);
                        false
                    } else {
                        true
                    }
                });

                for enemy_index in enemy_indices {
                    // Reset readiness
                    if enemy_index < battle_state.readiness_enemy.len() {
                        battle_state.readiness_enemy[enemy_index] = 0.0;
                    }

                    if let Some(target_index) = enemy_take_turn(
                        runtime,
                        &mut battle_state,
                        enemy_index,
                        &mut menu_state,
                        rng,
                    ) {
                        let command_entries =
                            command_entries_for_active_actor(runtime, &battle_state);
                        let render_state = build_battle_render_state(
                            runtime,
                            &battle_state,
                            &menu_state,
                            battle_ui,
                            &command_entries,
                            &[],
                            &[],
                            &[],
                            &[],
                            false,
                        );
                        pause_after_action(
                            session,
                            battle_ui,
                            bindings,
                            &render_state,
                            vec![enemy_index],
                            Vec::new(),
                            Vec::new(),
                            vec![target_index],
                        )?;
                    }
                }
            }
        }

        let current_turn = if matches!(
            battle_state.mode,
            BattleMode::Dynamic | BattleMode::DynamicWait
        ) {
            turn_state.order.first().cloned()
        } else {
            turn_state.order.get(turn_state.index).cloned()
        };

        let mut actor_id = battle_state
            .party_order
            .get(battle_state.active_index)
            .cloned()
            .unwrap_or_default();

        if let Some(current_turn) = current_turn {
            if !matches!(menu_state.phase, BattlePhase::Victory | BattlePhase::Defeat) {
                match current_turn {
                    BattleTurnActor::Party(party_index) => {
                        let Some(current_id) = battle_state.party_order.get(party_index).cloned()
                        else {
                            advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                            continue;
                        };
                        if let Some(actor) = runtime.party.roster.get(&current_id) {
                            if actor.current_hp <= 0 {
                                advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                                continue;
                            }
                        }
                        battle_state.active_index = party_index;
                        menu_state.defending.remove(&current_id);
                        menu_state.parrying.remove(&current_id);
                        menu_state.countering.remove(&current_id);
                        menu_state
                            .covering
                            .retain(|_, coverer| coverer != &current_id);
                        if last_actor_id.as_deref() != Some(current_id.as_str()) {
                            menu_state.reset_for_actor();
                            last_actor_id = Some(current_id.clone());
                            if let Some(actor) = runtime.party.roster.get_mut(&current_id) {
                                let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
                                let mut turn_result = apply_turn_start_statuses(
                                    &runtime.content,
                                    &actor.name,
                                    max_hp,
                                    &mut actor.current_hp,
                                    &mut actor.statuses,
                                    rng,
                                );
                                for message in turn_result.messages.drain(..) {
                                    push_battle_log(&mut battle_state.log, message);
                                }
                                if actor.current_hp <= 0 {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        format!("{} falls!", actor.name),
                                    );
                                    advance_turn(
                                        &mut menu_state,
                                        &mut turn_state,
                                        &mut battle_state,
                                    );
                                    continue;
                                }
                                if !turn_result.can_act {
                                    advance_turn(
                                        &mut menu_state,
                                        &mut turn_state,
                                        &mut battle_state,
                                    );
                                    continue;
                                }
                            }
                        }
                        actor_id = current_id;
                    }
                    BattleTurnActor::Enemy(enemy_index) => {
                        if !menu_state.paused {
                            if let Some(target_index) = enemy_take_turn(
                                runtime,
                                &mut battle_state,
                                enemy_index,
                                &mut menu_state,
                                rng,
                            ) {
                                let command_entries =
                                    command_entries_for_active_actor(runtime, &battle_state);
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &command_entries,
                                    &[],
                                    &[],
                                    &[],
                                    &[],
                                    false,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    vec![enemy_index],
                                    Vec::new(),
                                    Vec::new(),
                                    vec![target_index],
                                )?;
                            }
                            advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                            continue;
                        }
                    }
                }
            }
        }

        let spell_entries = build_battle_spell_entries(runtime, &actor_id);
        let mut command_entries = command_entries_for_actor(runtime, &actor_id);
        let ability_groups = ability_groups_for_commands(&command_entries);
        let ability_ids = ability_ids_for_commands(&command_entries);
        let ability_entries_raw = build_battle_ability_entries(runtime, &actor_id, None);
        let mut ability_entries_all = ability_entries_raw.clone();
        if !ability_groups.is_empty() || !ability_ids.is_empty() {
            ability_entries_all.retain(|entry| {
                if ability_ids.contains(entry.id.as_str()) {
                    return false;
                }
                let Some(ability) = runtime
                    .content
                    .abilities
                    .abilities
                    .iter()
                    .find(|ability| ability.id == entry.id)
                else {
                    return true;
                };
                match ability.command_group.as_deref() {
                    Some(group) => !ability_groups.contains(group),
                    None => true,
                }
            });
        }
        let command_group = ability_group_for_command(runtime, menu_state.command_id.as_deref());
        let ability_entries =
            build_battle_ability_entries(runtime, &actor_id, command_group.as_deref());
        let item_entries = build_battle_item_entries(runtime);
        command_entries.retain(|command| {
            command_is_enabled(
                runtime,
                &actor_id,
                command,
                &spell_entries,
                &ability_entries_all,
                &ability_entries_raw,
                &item_entries,
            )
        });
        if menu_state.command_index >= command_entries.len() {
            menu_state.command_index = command_entries.len().saturating_sub(1);
        }
        let render_state = build_battle_render_state(
            runtime,
            &battle_state,
            &menu_state,
            battle_ui,
            &command_entries,
            &spell_entries,
            &ability_entries,
            &ability_entries_all,
            &item_entries,
            current_turn.is_some(),
        );

        if menu_state.phase == BattlePhase::Victory {
            match victory_state {
                Some(VictoryState::Summary) => {
                    if let Some(ref result) = battle_result {
                        let currency_lines = format_currency_rewards(
                            &runtime.content.rules,
                            &result.rewards.currency,
                        );
                        tui::battle::draw_victory_summary(
                            session,
                            result.rewards.exp,
                            result.rewards.jp,
                            runtime.content.rules.job_system.progression_mode
                                == JobProgressionMode::JobPoints,
                            &currency_lines,
                            &result.rewards.items,
                            &ui_text(runtime, "battle.victory_title", "Victory!"),
                            &ui_text(runtime, "battle.items_found", "Items found:"),
                            &ui_text(
                                runtime,
                                "battle.victory_prompt",
                                "Press Confirm to continue.",
                            ),
                        )?;
                    }
                }
                Some(VictoryState::LevelUp(index)) => {
                    if let Some(ref result) = battle_result {
                        if let Some(diff) = result.level_ups.get(index) {
                            let headline = format_ui_text(
                                runtime,
                                "battle.level_up",
                                "{actor} reached Level {level}!",
                                &[
                                    ("actor", diff.actor_name.clone()),
                                    ("level", diff.new_level.to_string()),
                                ],
                            );
                            tui::battle::draw_level_up_modal(
                                session,
                                &headline,
                                &diff.stat_changes,
                                &ui_text(
                                    runtime,
                                    "battle.level_up_prompt",
                                    "Press Confirm to continue.",
                                ),
                            )?;
                        }
                    }
                }
                None => draw_battle(session, battle_ui, &render_state)?,
            }
        } else {
            draw_battle(session, battle_ui, &render_state)?;
        }

        let action = if crossterm::event::poll(Duration::from_millis(16))? {
            read_action(bindings)
        } else {
            None
        };

        let Some(action) = action else {
            continue;
        };

        if action == Action::Quit {
            if confirm_quit(session, |frame| {
                draw_battle_frame(frame, battle_ui, &render_state);
            })? {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit"));
            }
            continue;
        }

        if action == Action::Pause {
            if !matches!(menu_state.phase, BattlePhase::Victory | BattlePhase::Defeat) {
                menu_state.toggle_pause();
            }
            continue;
        }

        if menu_state.paused {
            continue;
        }

        let is_player_turn = current_turn
            .as_ref()
            .map(|actor| matches!(actor, BattleTurnActor::Party(_)))
            .unwrap_or(false);

        if !is_player_turn && matches!(menu_state.phase, BattlePhase::Command) {
            continue;
        }

        match menu_state.phase {
            BattlePhase::Victory => {
                if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                    match victory_state {
                        Some(VictoryState::Summary) => {
                            if let Some(ref result) = battle_result {
                                if !result.level_ups.is_empty() {
                                    victory_state = Some(VictoryState::LevelUp(0));
                                } else {
                                    cleanup_party_statuses_after_battle(runtime);
                                    return Ok(BattleOutcome::Victory);
                                }
                            } else {
                                cleanup_party_statuses_after_battle(runtime);
                                return Ok(BattleOutcome::Victory);
                            }
                        }
                        Some(VictoryState::LevelUp(index)) => {
                            if let Some(ref result) = battle_result {
                                if index + 1 < result.level_ups.len() {
                                    victory_state = Some(VictoryState::LevelUp(index + 1));
                                } else {
                                    cleanup_party_statuses_after_battle(runtime);
                                    return Ok(BattleOutcome::Victory);
                                }
                            } else {
                                cleanup_party_statuses_after_battle(runtime);
                                return Ok(BattleOutcome::Victory);
                            }
                        }
                        None => {
                            cleanup_party_statuses_after_battle(runtime);
                            return Ok(BattleOutcome::Victory);
                        }
                    }
                }
            }
            BattlePhase::Defeat => {
                if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                    cleanup_party_statuses_after_battle(runtime);
                    return Ok(BattleOutcome::Defeat);
                }
            }
            BattlePhase::Command => match action {
                Action::MoveUp => {
                    if menu_state.command_index > 0 {
                        menu_state.command_index -= 1;
                    }
                }
                Action::MoveDown => {
                    let max = command_entries.len();
                    if menu_state.command_index + 1 < max {
                        menu_state.command_index += 1;
                    }
                }
                Action::MoveLeft => {
                    let page_size = battle_ui.panels.commands.page_size.max(1);
                    if menu_state.command_index >= page_size {
                        menu_state.command_index -= page_size;
                    } else {
                        menu_state.command_index = 0;
                    }
                }
                Action::MoveRight => {
                    let page_size = battle_ui.panels.commands.page_size.max(1);
                    if menu_state.command_index + page_size < command_entries.len() {
                        menu_state.command_index += page_size;
                    } else if !command_entries.is_empty() {
                        menu_state.command_index = command_entries.len() - 1;
                    }
                }
                Action::Confirm => {
                    let Some(command) = command_entries.get(menu_state.command_index) else {
                        continue;
                    };
                    if !command_is_enabled(
                        runtime,
                        &actor_id,
                        command,
                        &spell_entries,
                        &ability_entries_all,
                        &ability_entries_raw,
                        &item_entries,
                    ) {
                        push_battle_log(
                            &mut battle_state.log,
                            ui_text(
                                runtime,
                                "battle.command_unavailable",
                                "Command unavailable.",
                            ),
                        );
                        continue;
                    }
                    match command.kind {
                        CommandKind::Attack => {
                            menu_state.phase = BattlePhase::TargetEnemy;
                            menu_state.pending_action = Some(PendingBattleAction::Attack);
                            if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                push_battle_log(
                                    &mut battle_state.log,
                                    ui_text(runtime, "battle.no_targets", "No valid targets."),
                                );
                                menu_state.reset_for_actor();
                            }
                        }
                        CommandKind::Magic => {
                            if spell_entries.is_empty() {
                                push_battle_log(
                                    &mut battle_state.log,
                                    ui_text(runtime, "battle.no_spells", "No spells available."),
                                );
                            } else {
                                menu_state.phase = BattlePhase::Magic;
                                menu_state.magic_index = 0;
                            }
                        }
                        CommandKind::Abilities | CommandKind::AbilitiesGroup => {
                            if let Some(ability_id) = command.ability_id.as_deref() {
                                let entry = ability_entries_raw
                                    .iter()
                                    .find(|entry| entry.id == ability_id)
                                    .cloned();
                                if let Some(entry) = entry {
                                    if !entry.usable {
                                        push_battle_log(
                                            &mut battle_state.log,
                                            ui_text(
                                                runtime,
                                                "battle.no_abilities",
                                                "No abilities available.",
                                            ),
                                        );
                                        continue;
                                    }
                                    if !begin_ability_targeting(
                                        runtime,
                                        &mut battle_state,
                                        &mut menu_state,
                                        entry,
                                    ) {
                                        menu_state.reset_for_actor();
                                    }
                                } else {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        ui_text(
                                            runtime,
                                            "battle.no_abilities",
                                            "No abilities available.",
                                        ),
                                    );
                                }
                            } else if command.kind == CommandKind::AbilitiesGroup {
                                let usable_group = usable_group_abilities(
                                    runtime,
                                    &actor_id,
                                    command.ability_group.as_deref(),
                                );
                                if usable_group.is_empty() {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        ui_text(
                                            runtime,
                                            "battle.no_abilities",
                                            "No abilities available.",
                                        ),
                                    );
                                    continue;
                                }
                                if usable_group.len() == 1 {
                                    let entry = usable_group[0].clone();
                                    if !begin_ability_targeting(
                                        runtime,
                                        &mut battle_state,
                                        &mut menu_state,
                                        entry,
                                    ) {
                                        menu_state.reset_for_actor();
                                    }
                                } else {
                                    menu_state.command_id = Some(command.id.clone());
                                    menu_state.phase = BattlePhase::Abilities;
                                    menu_state.ability_index = 0;
                                }
                            } else if ability_entries_all.is_empty() {
                                push_battle_log(
                                    &mut battle_state.log,
                                    ui_text(
                                        runtime,
                                        "battle.no_abilities",
                                        "No abilities available.",
                                    ),
                                );
                            } else {
                                menu_state.command_id = Some(command.id.clone());
                                menu_state.phase = BattlePhase::Abilities;
                                menu_state.ability_index = 0;
                            }
                        }
                        CommandKind::Items => {
                            if item_entries.is_empty() {
                                push_battle_log(
                                    &mut battle_state.log,
                                    ui_text(runtime, "battle.no_items", "No items available."),
                                );
                            } else {
                                menu_state.phase = BattlePhase::Items;
                                menu_state.item_index = 0;
                            }
                        }
                        CommandKind::Run => {
                            if rng.r#gen::<f32>() < 0.5 {
                                push_battle_log(
                                    &mut battle_state.log,
                                    ui_text(runtime, "battle.escape_success", "Escaped!"),
                                );
                                cleanup_party_statuses_after_battle(runtime);
                                return Ok(BattleOutcome::Escaped);
                            }
                            push_battle_log(
                                &mut battle_state.log,
                                ui_text(runtime, "battle.escape_fail", "Escape failed!"),
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &command_entries,
                                &spell_entries,
                                &ability_entries,
                                &ability_entries_all,
                                &item_entries,
                                false,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                Vec::new(),
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                        }
                        CommandKind::Defend => {
                            menu_state.defending.insert(actor_id.clone());
                            let actor_name = runtime
                                .party
                                .roster
                                .get(&actor_id)
                                .map(|actor| actor.name.clone())
                                .unwrap_or_else(|| actor_id.clone());
                            push_battle_log(
                                &mut battle_state.log,
                                format_ui_text(
                                    runtime,
                                    "battle.log.defend",
                                    "{actor} defends.",
                                    &[("actor", actor_name)],
                                ),
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &command_entries,
                                &spell_entries,
                                &ability_entries,
                                &ability_entries_all,
                                &item_entries,
                                false,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                Vec::new(),
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                        }
                        CommandKind::Row => {
                            let mut row_message: Option<(String, String)> = None;
                            if let Some(actor) = runtime.party.roster.get_mut(&actor_id) {
                                toggle_actor_row(actor);
                                let row_label = actor_row_label(actor).to_string();
                                row_message = Some((actor.name.clone(), row_label));
                            }
                            if let Some((actor_name, row_label)) = row_message {
                                push_battle_log(
                                    &mut battle_state.log,
                                    format_ui_text(
                                        runtime,
                                        "battle.log.row",
                                        "{actor} moves to the {row} row.",
                                        &[("actor", actor_name), ("row", row_label)],
                                    ),
                                );
                            }
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &command_entries,
                                &spell_entries,
                                &ability_entries,
                                &ability_entries_all,
                                &item_entries,
                                false,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                Vec::new(),
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                        }
                    }
                }
                _ => {}
            },
            BattlePhase::Magic => match action {
                Action::MoveUp => {
                    if menu_state.magic_index > 0 {
                        menu_state.magic_index -= 1;
                    }
                }
                Action::MoveDown => {
                    if menu_state.magic_index + 1 < spell_entries.len() {
                        menu_state.magic_index += 1;
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(entry) = spell_entries.get(menu_state.magic_index) else {
                        continue;
                    };
                    if !entry.usable {
                        let reason = entry.reason.clone().unwrap_or_else(|| {
                            ui_text(runtime, "battle.cast_unavailable", "Cannot cast.")
                        });
                        push_battle_log(&mut battle_state.log, reason);
                        continue;
                    }
                    menu_state.pending_action = Some(PendingBattleAction::Magic(entry.clone()));
                    let options = target_options_for_magic(entry);
                    if options.is_empty() {
                        push_battle_log(
                            &mut battle_state.log,
                            ui_text(runtime, "battle.no_targets", "No valid targets."),
                        );
                        menu_state.reset_for_actor();
                        continue;
                    }
                    let (target_side, target_mode) = select_initial_target_option(
                        entry.default_target.as_str(),
                        entry.target_mode.as_str(),
                        &options,
                    );
                    menu_state.target_side = target_side;
                    menu_state.target_mode = target_mode;
                    menu_state.phase = match target_side {
                        TargetSide::Enemy => BattlePhase::TargetEnemy,
                        TargetSide::Party => BattlePhase::TargetParty,
                    };
                    if menu_state.phase == BattlePhase::TargetEnemy {
                        if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                            push_battle_log(
                                &mut battle_state.log,
                                ui_text(runtime, "battle.no_targets", "No valid targets."),
                            );
                            menu_state.reset_for_actor();
                        }
                    } else if let Some(action) = menu_state.pending_action.clone() {
                        if !set_initial_party_target(
                            &mut menu_state,
                            &battle_state,
                            runtime,
                            &action,
                        ) {
                            push_battle_log(
                                &mut battle_state.log,
                                ui_text(runtime, "battle.no_targets", "No valid targets."),
                            );
                            menu_state.reset_for_actor();
                        }
                    }
                }
                _ => {}
            },
            BattlePhase::Abilities => match action {
                Action::MoveUp => {
                    if menu_state.ability_index > 0 {
                        menu_state.ability_index -= 1;
                    }
                }
                Action::MoveDown => {
                    if menu_state.ability_index + 1 < ability_entries.len() {
                        menu_state.ability_index += 1;
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(entry) = ability_entries.get(menu_state.ability_index) else {
                        continue;
                    };
                    menu_state.pending_action = Some(PendingBattleAction::Ability(entry.clone()));
                    let options = target_options_for_ability(entry);
                    if options.is_empty() {
                        push_battle_log(
                            &mut battle_state.log,
                            ui_text(runtime, "battle.no_targets", "No valid targets."),
                        );
                        menu_state.reset_for_actor();
                        continue;
                    }
                    let (target_side, target_mode) = select_initial_target_option(
                        entry.default_target.as_str(),
                        entry.target_mode.as_str(),
                        &options,
                    );
                    menu_state.target_side = target_side;
                    menu_state.target_mode = target_mode;
                    menu_state.phase = match target_side {
                        TargetSide::Enemy => BattlePhase::TargetEnemy,
                        TargetSide::Party => BattlePhase::TargetParty,
                    };
                    if menu_state.phase == BattlePhase::TargetEnemy {
                        if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                            push_battle_log(
                                &mut battle_state.log,
                                ui_text(runtime, "battle.no_targets", "No valid targets."),
                            );
                            menu_state.reset_for_actor();
                        }
                    } else if let Some(action) = menu_state.pending_action.clone() {
                        if !set_initial_party_target(
                            &mut menu_state,
                            &battle_state,
                            runtime,
                            &action,
                        ) {
                            push_battle_log(
                                &mut battle_state.log,
                                ui_text(runtime, "battle.no_targets", "No valid targets."),
                            );
                            menu_state.reset_for_actor();
                        }
                    }
                }
                _ => {}
            },
            BattlePhase::Items => match action {
                Action::MoveUp => {
                    if menu_state.item_index > 0 {
                        menu_state.item_index -= 1;
                    }
                }
                Action::MoveDown => {
                    if menu_state.item_index + 1 < item_entries.len() {
                        menu_state.item_index += 1;
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(entry) = item_entries.get(menu_state.item_index) else {
                        continue;
                    };
                    if !entry.usable {
                        push_battle_log(
                            &mut battle_state.log,
                            ui_text(runtime, "battle.item_unusable", "Item unusable."),
                        );
                        continue;
                    }
                    let Some(item) = runtime
                        .content
                        .items
                        .items
                        .iter()
                        .find(|item| item.id == entry.id)
                        .cloned()
                    else {
                        continue;
                    };
                    match item.usage.target.as_str() {
                        "party" => {
                            execute_item_action(runtime, &mut battle_state, &actor_id, &item, None);
                            let render_state = build_battle_render_state(
                                runtime,
                                &mut battle_state,
                                &menu_state,
                                battle_ui,
                                &command_entries,
                                &spell_entries,
                                &ability_entries,
                                &ability_entries_all,
                                &item_entries,
                                false,
                            );
                            let party_indices =
                                (0..battle_state.party_order.len()).collect::<Vec<_>>();
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                party_indices,
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                        }
                        "enemy" => {
                            menu_state.phase = BattlePhase::TargetEnemy;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Item(item.id.clone()));
                            menu_state.target_side = TargetSide::Enemy;
                            menu_state.target_mode = TargetMode::Single;
                            if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                push_battle_log(
                                    &mut battle_state.log,
                                    ui_text(runtime, "battle.no_targets", "No valid targets."),
                                );
                                menu_state.reset_for_actor();
                            }
                        }
                        _ => {
                            menu_state.phase = BattlePhase::TargetParty;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Item(item.id.clone()));
                            menu_state.target_side = TargetSide::Party;
                            menu_state.target_mode = TargetMode::Single;
                            if let Some(action) = menu_state.pending_action.clone() {
                                if !set_initial_party_target(
                                    &mut menu_state,
                                    &battle_state,
                                    runtime,
                                    &action,
                                ) {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        ui_text(runtime, "battle.no_targets", "No valid targets."),
                                    );
                                    menu_state.reset_for_actor();
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            BattlePhase::TargetEnemy => match action {
                Action::MoveUp => {
                    let valid = enemy_target_indices(&battle_state);
                    menu_state.enemy_index = step_target_index(menu_state.enemy_index, &valid, -1);
                }
                Action::MoveDown => {
                    let valid = enemy_target_indices(&battle_state);
                    menu_state.enemy_index = step_target_index(menu_state.enemy_index, &valid, 1);
                }
                Action::MoveLeft | Action::MoveRight => {
                    let direction = if action == Action::MoveLeft { -1 } else { 1 };
                    if let Some(action) = menu_state.pending_action.as_ref() {
                        if let Some(option) = step_target_option(
                            action,
                            menu_state.target_side,
                            menu_state.target_mode,
                            direction,
                        ) {
                            menu_state.target_side = option.side;
                            menu_state.target_mode = option.mode;
                            menu_state.phase = match option.side {
                                TargetSide::Enemy => BattlePhase::TargetEnemy,
                                TargetSide::Party => BattlePhase::TargetParty,
                            };
                            if menu_state.phase == BattlePhase::TargetEnemy {
                                if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        ui_text(runtime, "battle.no_targets", "No valid targets."),
                                    );
                                }
                            } else if let Some(action) = menu_state.pending_action.clone() {
                                if !set_initial_party_target(
                                    &mut menu_state,
                                    &battle_state,
                                    runtime,
                                    &action,
                                ) {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        ui_text(runtime, "battle.no_targets", "No valid targets."),
                                    );
                                }
                            }
                        }
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let valid = enemy_target_indices(&battle_state);
                    let Some(target_index) = ensure_valid_index(menu_state.enemy_index, &valid)
                    else {
                        push_battle_log(
                            &mut battle_state.log,
                            ui_text(runtime, "battle.no_targets", "No valid targets."),
                        );
                        menu_state.reset_for_actor();
                        continue;
                    };
                    menu_state.enemy_index = target_index;
                    let was_alive = battle_state
                        .enemies
                        .get(menu_state.enemy_index)
                        .map(|enemy| enemy.is_alive())
                        .unwrap_or(false);
                    if let Some(action) = menu_state.pending_action.take() {
                        match action {
                            PendingBattleAction::Attack => {
                                execute_attack_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    menu_state.enemy_index,
                                    rng,
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &command_entries,
                                    &spell_entries,
                                    &ability_entries,
                                    &ability_entries_all,
                                    &item_entries,
                                    false,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                            PendingBattleAction::Magic(entry) => {
                                let target_index = if menu_state.target_mode == TargetMode::Single {
                                    Some(menu_state.enemy_index)
                                } else {
                                    None
                                };
                                execute_magic_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    &entry,
                                    menu_state.target_side,
                                    menu_state.target_mode,
                                    target_index,
                                    rng,
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &command_entries,
                                    &spell_entries,
                                    &ability_entries,
                                    &ability_entries_all,
                                    &item_entries,
                                    false,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                            PendingBattleAction::Ability(entry) => {
                                let target_index = if menu_state.target_mode == TargetMode::Single {
                                    Some(menu_state.enemy_index)
                                } else {
                                    None
                                };
                                let target_side = menu_state.target_side;
                                let target_mode = menu_state.target_mode;
                                execute_ability_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    &entry,
                                    &mut menu_state,
                                    target_side,
                                    target_mode,
                                    target_index,
                                    rng,
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &command_entries,
                                    &spell_entries,
                                    &ability_entries,
                                    &ability_entries_all,
                                    &item_entries,
                                    false,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                            PendingBattleAction::Item(item_id) => {
                                if let Some(item) = runtime
                                    .content
                                    .items
                                    .items
                                    .iter()
                                    .find(|item| item.id == item_id)
                                    .cloned()
                                {
                                    execute_item_action(
                                        runtime,
                                        &mut battle_state,
                                        &actor_id,
                                        &item,
                                        Some(menu_state.enemy_index),
                                    );
                                    let render_state = build_battle_render_state(
                                        runtime,
                                        &battle_state,
                                        &menu_state,
                                        battle_ui,
                                        &command_entries,
                                        &spell_entries,
                                        &ability_entries,
                                        &ability_entries_all,
                                        &item_entries,
                                        false,
                                    );
                                    pause_after_action(
                                        session,
                                        battle_ui,
                                        bindings,
                                        &render_state,
                                        Vec::new(),
                                        vec![battle_state.active_index],
                                        vec![menu_state.enemy_index],
                                        Vec::new(),
                                    )?;
                                }
                            }
                        }
                        pause_on_enemy_defeat(
                            session,
                            battle_ui,
                            bindings,
                            runtime,
                            &mut battle_state,
                            &menu_state,
                            &command_entries,
                            &spell_entries,
                            &ability_entries,
                            &ability_entries_all,
                            &item_entries,
                            was_alive,
                            menu_state.enemy_index,
                        )?;
                        advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                    }
                }
                _ => {}
            },
            BattlePhase::TargetParty => match action {
                Action::MoveUp => {
                    if let Some(action) = menu_state.pending_action.as_ref() {
                        let rule = party_target_rule(action, runtime);
                        let valid = party_target_indices(runtime, &battle_state, rule);
                        menu_state.party_index =
                            step_target_index(menu_state.party_index, &valid, -1);
                    }
                }
                Action::MoveDown => {
                    if let Some(action) = menu_state.pending_action.as_ref() {
                        let rule = party_target_rule(action, runtime);
                        let valid = party_target_indices(runtime, &battle_state, rule);
                        menu_state.party_index =
                            step_target_index(menu_state.party_index, &valid, 1);
                    }
                }
                Action::MoveLeft | Action::MoveRight => {
                    let direction = if action == Action::MoveLeft { -1 } else { 1 };
                    if let Some(action) = menu_state.pending_action.as_ref() {
                        if let Some(option) = step_target_option(
                            action,
                            menu_state.target_side,
                            menu_state.target_mode,
                            direction,
                        ) {
                            menu_state.target_side = option.side;
                            menu_state.target_mode = option.mode;
                            menu_state.phase = match option.side {
                                TargetSide::Enemy => BattlePhase::TargetEnemy,
                                TargetSide::Party => BattlePhase::TargetParty,
                            };
                            if menu_state.phase == BattlePhase::TargetEnemy {
                                if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        ui_text(runtime, "battle.no_targets", "No valid targets."),
                                    );
                                }
                            } else if let Some(action) = menu_state.pending_action.clone() {
                                if !set_initial_party_target(
                                    &mut menu_state,
                                    &battle_state,
                                    runtime,
                                    &action,
                                ) {
                                    push_battle_log(
                                        &mut battle_state.log,
                                        ui_text(runtime, "battle.no_targets", "No valid targets."),
                                    );
                                }
                            }
                        }
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(action) = menu_state.pending_action.take() else {
                        continue;
                    };
                    let rule = party_target_rule(&action, runtime);
                    let valid = party_target_indices(runtime, &battle_state, rule);
                    let Some(target_index) = ensure_valid_index(menu_state.party_index, &valid)
                    else {
                        let message = match rule {
                            TargetRule::KnockedOut => {
                                ui_text(runtime, "battle.no_fallen", "No fallen allies.")
                            }
                            TargetRule::Alive => {
                                ui_text(runtime, "battle.no_targets", "No valid targets.")
                            }
                        };
                        push_battle_log(&mut battle_state.log, message);
                        menu_state.reset_for_actor();
                        continue;
                    };
                    menu_state.party_index = target_index;
                    match action {
                        PendingBattleAction::Magic(entry) => {
                            let target_index = if menu_state.target_mode == TargetMode::Single {
                                Some(menu_state.party_index)
                            } else {
                                None
                            };
                            execute_magic_action(
                                runtime,
                                &mut battle_state,
                                &actor_id,
                                &entry,
                                menu_state.target_side,
                                menu_state.target_mode,
                                target_index,
                                rng,
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &command_entries,
                                &spell_entries,
                                &ability_entries,
                                &ability_entries_all,
                                &item_entries,
                                false,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                vec![menu_state.party_index],
                            )?;
                        }
                        PendingBattleAction::Ability(entry) => {
                            let target_index = if menu_state.target_mode == TargetMode::Single {
                                Some(menu_state.party_index)
                            } else {
                                None
                            };
                            let target_side = menu_state.target_side;
                            let target_mode = menu_state.target_mode;
                            execute_ability_action(
                                runtime,
                                &mut battle_state,
                                &actor_id,
                                &entry,
                                &mut menu_state,
                                target_side,
                                target_mode,
                                target_index,
                                rng,
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &command_entries,
                                &spell_entries,
                                &ability_entries,
                                &ability_entries_all,
                                &item_entries,
                                false,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                vec![menu_state.party_index],
                            )?;
                        }
                        PendingBattleAction::Item(item_id) => {
                            if let Some(item) = runtime
                                .content
                                .items
                                .items
                                .iter()
                                .find(|item| item.id == item_id)
                                .cloned()
                            {
                                execute_item_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    &item,
                                    Some(menu_state.party_index),
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &command_entries,
                                    &spell_entries,
                                    &ability_entries,
                                    &ability_entries_all,
                                    &item_entries,
                                    false,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                        }
                        PendingBattleAction::Attack => {}
                    }
                    advance_turn(&mut menu_state, &mut turn_state, &mut battle_state);
                }
                _ => {}
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TargetOption {
    side: TargetSide,
    mode: TargetMode,
}

fn target_options_for_magic(entry: &SpellEntry) -> Vec<TargetOption> {
    target_options(
        entry.default_target.as_str(),
        &entry.allowed_targets,
        entry.target_mode.as_str(),
    )
}

fn target_options_for_ability(entry: &AbilityEntry) -> Vec<TargetOption> {
    target_options(
        entry.default_target.as_str(),
        &entry.allowed_targets,
        entry.target_mode.as_str(),
    )
}

fn target_options_for_action(action: &PendingBattleAction) -> Vec<TargetOption> {
    match action {
        PendingBattleAction::Magic(entry) => target_options_for_magic(entry),
        PendingBattleAction::Ability(entry) => target_options_for_ability(entry),
        _ => Vec::new(),
    }
}

fn target_options(
    default_target: &str,
    allowed_targets: &[String],
    target_mode: &str,
) -> Vec<TargetOption> {
    let modes = target_modes_for_value(target_mode);
    let (allows_enemy, allows_party) = allowed_target_sides(default_target, allowed_targets);
    let mut party_options = Vec::new();
    let mut enemy_options = Vec::new();

    if allows_party {
        for mode in &modes {
            party_options.push(TargetOption {
                side: TargetSide::Party,
                mode: *mode,
            });
        }
    }
    if allows_enemy {
        for mode in &modes {
            enemy_options.push(TargetOption {
                side: TargetSide::Enemy,
                mode: *mode,
            });
        }
    }

    if default_target == "enemy" {
        enemy_options.extend(party_options);
        enemy_options
    } else {
        party_options.extend(enemy_options);
        party_options
    }
}

fn allowed_target_sides(default_target: &str, allowed_targets: &[String]) -> (bool, bool) {
    if allowed_targets.is_empty() {
        let allows_enemy = default_target == "enemy";
        let allows_party = matches!(default_target, "ally" | "party" | "self");
        return (allows_enemy, allows_party);
    }
    let mut allows_enemy = false;
    let mut allows_party = false;
    for target in allowed_targets {
        match target.as_str() {
            "enemy" => allows_enemy = true,
            "party" | "ally" | "self" => allows_party = true,
            _ => {}
        }
    }
    (allows_enemy, allows_party)
}

fn target_modes_for_value(target_mode: &str) -> Vec<TargetMode> {
    match target_mode {
        "multi" => vec![TargetMode::Multi],
        "both" => vec![TargetMode::Single, TargetMode::Multi],
        _ => vec![TargetMode::Single],
    }
}

fn default_mode_for_value(target_mode: &str) -> TargetMode {
    if target_mode == "multi" {
        TargetMode::Multi
    } else {
        TargetMode::Single
    }
}

fn select_initial_target_option(
    default_target: &str,
    target_mode: &str,
    options: &[TargetOption],
) -> (TargetSide, TargetMode) {
    let preferred_side = if default_target == "enemy" {
        TargetSide::Enemy
    } else {
        TargetSide::Party
    };
    let preferred_mode = default_mode_for_value(target_mode);
    options
        .iter()
        .find(|option| option.side == preferred_side && option.mode == preferred_mode)
        .or_else(|| options.first())
        .map(|option| (option.side, option.mode))
        .unwrap_or((TargetSide::Party, TargetMode::Single))
}

fn step_target_option(
    action: &PendingBattleAction,
    current_side: TargetSide,
    current_mode: TargetMode,
    direction: i32,
) -> Option<TargetOption> {
    let options = target_options_for_action(action);
    if options.is_empty() {
        return None;
    }
    let current_index = options
        .iter()
        .position(|option| option.side == current_side && option.mode == current_mode)
        .unwrap_or(0);
    let len = options.len();
    let next_index = if direction >= 0 {
        (current_index + 1) % len
    } else {
        (current_index + len - 1) % len
    };
    options.get(next_index).copied()
}

pub fn try_start_random_battle(
    runtime: &mut GameRuntime,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    map_id: &str,
    player_pos: (i32, i32),
    encounter_meter: &mut f32,
    rng: &mut impl Rng,
) -> std::io::Result<Option<BattleReport>> {
    let map_index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return Ok(None),
    };
    let map = match runtime.content.maps.get(map_index) {
        Some(map) => map,
        None => return Ok(None),
    };
    if map.encounter_rate <= 0.0 || map.encounters.is_empty() {
        *encounter_meter = 0.0;
        return Ok(None);
    }
    let Some(zone) = encounter_zone_for_pos(map, player_pos) else {
        return Ok(None);
    };
    let rate = map.encounter_rate.clamp(0.0, 1.0);
    let jitter = 0.5 + rng.r#gen::<f32>();
    *encounter_meter += rate * jitter;
    if *encounter_meter < 1.0 {
        return Ok(None);
    }
    *encounter_meter = (*encounter_meter - 1.0).clamp(0.0, 1.0);
    let entry = match select_encounter_entry(&runtime.content.encounters, &zone.table, rng) {
        Some(entry) => entry,
        None => return Ok(None),
    };
    let formation = entry.formation.clone();
    let snapshot = BattleSnapshot::capture(runtime);
    let outcome = run_battle(runtime, battle_ui, bindings, session, &formation, rng)?;
    Ok(Some(BattleReport {
        outcome,
        formation,
        snapshot,
    }))
}

pub fn run_event_battle_with_result(
    runtime: &mut GameRuntime,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    encounter_id: &str,
    formation: &[engine::events::FormationMember],
    snapshot: BattleSnapshot,
) -> std::io::Result<BattleReport> {
    let mut rng = rand::thread_rng();
    let formation = if formation.is_empty() {
        if encounter_id.is_empty() {
            Vec::new()
        } else {
            match select_encounter_entry(&runtime.content.encounters, encounter_id, &mut rng) {
                Some(entry) => entry.formation,
                None => Vec::new(),
            }
        }
    } else {
        formation
            .iter()
            .map(|member| engine::encounters::EncounterMember {
                enemy: member.enemy.clone(),
                pos: member.pos,
            })
            .collect()
    };
    if formation.is_empty() {
        return Ok(BattleReport {
            outcome: BattleOutcome::Victory,
            formation,
            snapshot,
        });
    }
    let outcome = run_battle(runtime, battle_ui, bindings, session, &formation, &mut rng)?;
    Ok(BattleReport {
        outcome,
        formation,
        snapshot,
    })
}

fn pause_after_action(
    session: &mut TuiSession,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    render_state: &BattleRenderState,
    acting_enemies: Vec<usize>,
    acting_party: Vec<usize>,
    flash_enemies: Vec<usize>,
    flash_party: Vec<usize>,
) -> std::io::Result<()> {
    if let Some(animation) = &battle_ui.animation {
        if !flash_enemies.is_empty()
            || !flash_party.is_empty()
            || !acting_enemies.is_empty()
            || !acting_party.is_empty()
        {
            let base_acting_state = BattleRenderState {
                acting_enemies: acting_enemies.clone(),
                acting_party: acting_party.clone(),
                ..render_state.clone()
            };
            let cycles = animation.flash_cycles.max(1);
            let delay = Duration::from_millis(animation.flash_ms.max(1));
            for _ in 0..cycles {
                let mut flash_state = base_acting_state.clone();
                flash_state.flash_enemies = flash_enemies.clone();
                flash_state.flash_party = flash_party.clone();
                draw_battle(session, battle_ui, &flash_state)?;
                sleep(delay);
                draw_battle(session, battle_ui, &base_acting_state)?;
                sleep(delay);
            }
        }
    }
    draw_battle(session, battle_ui, render_state)?;
    wait_for_battle_dialog(session, bindings, battle_ui, render_state)
}

fn pause_on_enemy_defeat(
    session: &mut TuiSession,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    runtime: &GameRuntime,
    battle_state: &mut BattleState,
    menu_state: &BattleMenuState,
    command_entries: &[CommandEntry],
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    ability_entries_all: &[AbilityEntry],
    item_entries: &[InventoryEntry],
    was_alive: bool,
    target_index: usize,
) -> std::io::Result<()> {
    let defeated = was_alive
        && battle_state
            .enemies
            .get(target_index)
            .map(|enemy| !enemy.is_alive())
            .unwrap_or(false);
    if !defeated {
        return Ok(());
    }
    if let Some(enemy) = battle_state.enemies.get(target_index) {
        battle_state.log.push(format!("{} defeated.", enemy.name));
        let render_state = build_battle_render_state(
            runtime,
            battle_state,
            menu_state,
            battle_ui,
            command_entries,
            spell_entries,
            ability_entries,
            ability_entries_all,
            item_entries,
            false,
        );
        pause_after_action(
            session,
            battle_ui,
            bindings,
            &render_state,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?;
    }
    Ok(())
}

fn wait_for_battle_dialog(
    session: &mut TuiSession,
    bindings: &InputBindings,
    battle_ui: &BattleUiFile,
    render_state: &BattleRenderState,
) -> std::io::Result<()> {
    let Some(log) = &battle_ui.log else {
        return Ok(());
    };
    if log.auto_advance_ms == 0 && !log.allow_skip {
        return Ok(());
    }
    let timeout = Duration::from_millis(log.auto_advance_ms);
    let start = Instant::now();
    loop {
        let elapsed = start.elapsed();
        if log.auto_advance_ms > 0 && elapsed >= timeout {
            break;
        }
        if log.allow_skip {
            let wait = if log.auto_advance_ms == 0 {
                Duration::from_millis(50)
            } else {
                timeout.saturating_sub(elapsed)
            };
            if crossterm::event::poll(wait)? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if !is_actionable_key(&key) {
                        continue;
                    }
                    if let Some(action) = bindings.action_for(key.code) {
                        if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                            break;
                        }
                        if action == Action::Quit {
                            if confirm_quit(session, |frame| {
                                draw_battle_frame(frame, battle_ui, render_state);
                            })? {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::Interrupted,
                                    "quit",
                                ));
                            }
                            draw_battle(session, battle_ui, render_state)?;
                        }
                    }
                }
            }
        } else if log.auto_advance_ms > 0 {
            sleep(timeout);
            break;
        }
    }
    Ok(())
}

fn cleanup_party_statuses_after_battle(runtime: &mut GameRuntime) {
    for actor in runtime.party.roster.values_mut() {
        engine::battle::retain_statuses_after_battle(&runtime.content, &mut actor.statuses);
    }
}

fn apply_battle_rewards(
    runtime: &mut GameRuntime,
    battle_state: &mut BattleState,
    rng: &mut impl Rng,
) -> BattleResult {
    let base_rewards = collect_rewards(&battle_state.enemies, rng);
    let mut result = BattleResult {
        rewards: base_rewards.clone(),
        level_ups: Vec::new(),
    };

    let rules = Ruleset::from_file(runtime.content.rules.clone());
    let eligible_actor_ids: Vec<String> = runtime
        .party
        .active_ids()
        .into_iter()
        .filter(|actor_id| {
            if rules.battle.exp_for_fallen {
                return true;
            }
            runtime
                .party
                .roster
                .get(actor_id)
                .map(|actor| actor.current_hp > 0)
                .unwrap_or(false)
        })
        .collect();

    if result.rewards.exp > 0 {
        for actor_id in &eligible_actor_ids {
            if let Some(actor) = runtime.party.roster.get_mut(actor_id.as_str()) {
                let old_level = actor.level;
                let old_stats = actor.derived_stats.clone();

                let levels_gained = gain_exp(&runtime.content, &rules, actor, result.rewards.exp);

                if levels_gained > 0 {
                    let new_stats = actor.derived_stats.clone();
                    let mut stat_changes = std::collections::HashMap::new();

                    for (stat, new_value) in new_stats {
                        let old_value = old_stats.get(&stat).copied().unwrap_or(0);
                        let diff = new_value - old_value;
                        if diff != 0 {
                            stat_changes.insert(stat, (new_value, diff));
                        }
                    }

                    result.level_ups.push(LevelUpDiff {
                        actor_name: actor.name.clone(),
                        old_level,
                        new_level: actor.level,
                        stat_changes,
                    });
                }
            }
        }
    }

    if result.rewards.jp > 0 {
        for actor_id in &eligible_actor_ids {
            if let Some(actor) = runtime.party.roster.get_mut(actor_id.as_str()) {
                engine::party::gain_jp(&runtime.content, &rules, actor, result.rewards.jp);
            }
        }
    }

    for (currency_id, amount) in &result.rewards.currency {
        if *amount > 0 {
            runtime
                .inventory
                .add_currency(currency_id.as_str(), *amount);
        }
    }

    if !result.rewards.items.is_empty() {
        let max_stack = runtime.content.rules.inventory.max_stack;
        for (item_id, qty) in &result.rewards.items {
            if runtime
                .content
                .items
                .items
                .iter()
                .any(|item| item.id == *item_id)
            {
                runtime.inventory.add_item(item_id, *qty, max_stack);
            } else if runtime
                .content
                .equipment
                .equipment
                .iter()
                .any(|item| item.id == *item_id)
            {
                runtime.inventory.add_equipment(item_id, *qty, max_stack);
            }
        }
    }

    let defeated = battle_state
        .enemies
        .iter()
        .filter(|enemy| enemy.current_hp <= 0)
        .count();
    if defeated > 0 {
        runtime.add_stat("enemies_defeated", defeated as i32);
    }

    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommandKind {
    Attack,
    Magic,
    Abilities,
    AbilitiesGroup,
    Items,
    Run,
    Defend,
    Row,
}

#[derive(Clone, Debug)]
pub struct CommandEntry {
    pub id: String,
    pub label: String,
    pub kind: CommandKind,
    pub sort_order: i32,
    pub ability_group: Option<String>,
    pub ability_id: Option<String>,
}

fn command_kind_from_rule(kind: &str) -> Option<CommandKind> {
    match kind {
        "attack" => Some(CommandKind::Attack),
        "magic" => Some(CommandKind::Magic),
        "abilities" => Some(CommandKind::Abilities),
        "abilities_group" => Some(CommandKind::AbilitiesGroup),
        "items" => Some(CommandKind::Items),
        "run" => Some(CommandKind::Run),
        "defend" => Some(CommandKind::Defend),
        "row" => Some(CommandKind::Row),
        _ => None,
    }
}

pub fn command_entries_for_actor(runtime: &GameRuntime, actor_id: &str) -> Vec<CommandEntry> {
    let mut command_ids: HashSet<String> = runtime
        .content
        .rules
        .battle
        .global_commands
        .iter()
        .cloned()
        .collect();
    if let Some(actor) = runtime.party.roster.get(actor_id) {
        if let Some(job) = runtime
            .content
            .jobs
            .jobs
            .iter()
            .find(|job| job.id == actor.job_id)
        {
            command_ids.extend(job.commands.iter().cloned());
        }
        if runtime.content.rules.job_system.secondary_jobs {
            if let Some(secondary_job_id) = actor.secondary_job_id.as_deref() {
                if let Some(job) = runtime
                    .content
                    .jobs
                    .jobs
                    .iter()
                    .find(|job| job.id == secondary_job_id)
                {
                    command_ids.extend(job.commands.iter().cloned());
                }
            }
        }
    }
    let mut entries = Vec::new();
    for command in &runtime.content.rules.battle.commands {
        if !command_ids.contains(&command.id) {
            continue;
        }
        let Some(kind) = command_kind_from_rule(command.kind.as_str()) else {
            continue;
        };
        let label_key = format!("command.{}", command.id);
        entries.push(CommandEntry {
            id: command.id.clone(),
            label: ui_text(runtime, &label_key, command.label.as_str()),
            kind,
            sort_order: command.sort_order,
            ability_group: command.ability_group.clone(),
            ability_id: command.ability_id.clone(),
        });
    }
    entries.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.id.cmp(&right.id))
    });
    entries
}

fn ability_groups_for_commands(command_entries: &[CommandEntry]) -> HashSet<String> {
    command_entries
        .iter()
        .filter(|command| command.kind == CommandKind::AbilitiesGroup)
        .filter_map(|command| command.ability_group.as_deref())
        .map(str::to_string)
        .collect()
}

fn ability_ids_for_commands(command_entries: &[CommandEntry]) -> HashSet<String> {
    command_entries
        .iter()
        .filter(|command| command.kind == CommandKind::Abilities)
        .filter_map(|command| command.ability_id.as_deref())
        .map(str::to_string)
        .collect()
}

pub fn command_definition_for_id<'a>(
    runtime: &'a GameRuntime,
    command_id: &str,
) -> Option<&'a engine::rules::BattleCommandDefinition> {
    runtime
        .content
        .rules
        .battle
        .commands
        .iter()
        .find(|command| command.id == command_id)
}

fn command_entries_for_active_actor(
    runtime: &GameRuntime,
    battle_state: &BattleState,
) -> Vec<CommandEntry> {
    let Some(actor_id) = battle_state.party_order.get(battle_state.active_index) else {
        return Vec::new();
    };
    command_entries_for_actor(runtime, actor_id)
}

fn ability_group_for_command(runtime: &GameRuntime, command_id: Option<&str>) -> Option<String> {
    let command_id = command_id?;
    let command = command_definition_for_id(runtime, command_id)?;
    if command.kind != "abilities_group" {
        return None;
    }
    command.ability_group.clone()
}

fn group_abilities(
    runtime: &GameRuntime,
    actor_id: &str,
    group: Option<&str>,
) -> Vec<AbilityEntry> {
    build_battle_ability_entries(runtime, actor_id, group)
}

fn usable_group_abilities(
    runtime: &GameRuntime,
    actor_id: &str,
    group: Option<&str>,
) -> Vec<AbilityEntry> {
    group_abilities(runtime, actor_id, group)
        .into_iter()
        .filter(|entry| entry.usable)
        .collect()
}

fn begin_ability_targeting(
    runtime: &GameRuntime,
    battle_state: &mut BattleState,
    menu_state: &mut BattleMenuState,
    entry: AbilityEntry,
) -> bool {
    menu_state.pending_action = Some(PendingBattleAction::Ability(entry.clone()));
    let options = target_options_for_ability(&entry);
    if options.is_empty() {
        push_battle_log(
            &mut battle_state.log,
            ui_text(runtime, "battle.no_targets", "No valid targets."),
        );
        return false;
    }
    let (target_side, target_mode) = select_initial_target_option(
        entry.default_target.as_str(),
        entry.target_mode.as_str(),
        &options,
    );
    menu_state.target_side = target_side;
    menu_state.target_mode = target_mode;
    match target_side {
        TargetSide::Enemy => {
            menu_state.phase = BattlePhase::TargetEnemy;
            if !set_initial_enemy_target(menu_state, battle_state) {
                push_battle_log(
                    &mut battle_state.log,
                    ui_text(runtime, "battle.no_targets", "No valid targets."),
                );
                return false;
            }
        }
        TargetSide::Party => {
            menu_state.phase = BattlePhase::TargetParty;
            let rule = party_target_rule(menu_state.pending_action.as_ref().unwrap(), runtime);
            let valid = party_target_indices(runtime, battle_state, rule);
            if valid.is_empty() {
                push_battle_log(
                    &mut battle_state.log,
                    ui_text(runtime, "battle.no_targets", "No valid targets."),
                );
                return false;
            }
            if let Some(index) = valid.first().copied() {
                menu_state.party_index = index;
            }
        }
    }
    true
}

fn command_is_enabled(
    runtime: &GameRuntime,
    actor_id: &str,
    command: &CommandEntry,
    spell_entries: &[SpellEntry],
    ability_entries_all: &[AbilityEntry],
    ability_entries_raw: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> bool {
    match command.kind {
        CommandKind::Magic => {
            crate::menu::system_enabled(runtime, Some("magic")) && !spell_entries.is_empty()
        }
        CommandKind::Abilities => {
            if let Some(ability_id) = command.ability_id.as_deref() {
                ability_entries_raw
                    .iter()
                    .find(|entry| entry.id == ability_id)
                    .map(|entry| entry.usable)
                    .unwrap_or(false)
            } else {
                ability_entries_all.iter().any(|entry| entry.usable)
            }
        }
        CommandKind::AbilitiesGroup => command
            .ability_group
            .as_deref()
            .map(|group| !usable_group_abilities(runtime, actor_id, Some(group)).is_empty())
            .unwrap_or(false),
        CommandKind::Items => {
            crate::menu::system_enabled(runtime, Some("items")) && !item_entries.is_empty()
        }
        CommandKind::Row => {
            let rows = &runtime.content.rules.battle.rows;
            rows.enabled && rows.allow_battle_switch
        }
        _ => true,
    }
}

fn encounter_zone_for_pos(
    map: &engine::maps::MapFile,
    pos: (i32, i32),
) -> Option<&engine::maps::EncounterZone> {
    map.encounters
        .iter()
        .find(|zone| pos_in_rect(pos, zone.rect))
}

fn pos_in_rect(pos: (i32, i32), rect: [i32; 4]) -> bool {
    let (x, y) = pos;
    x >= rect[0] && y >= rect[1] && x < rect[0] + rect[2] && y < rect[1] + rect[3]
}

fn select_encounter_entry(
    encounters: &engine::encounters::EncountersFile,
    table_id: &str,
    rng: &mut impl Rng,
) -> Option<engine::encounters::EncounterEntry> {
    let table = encounters
        .tables
        .iter()
        .find(|table| table.id == table_id)?;
    let total_weight: i32 = table.entries.iter().map(|entry| entry.weight.max(0)).sum();
    if total_weight <= 0 {
        return table.entries.first().cloned();
    }
    let roll = rng.gen_range(0..total_weight);
    let mut cursor = 0;
    for entry in &table.entries {
        cursor += entry.weight.max(0);
        if roll < cursor {
            return Some(entry.clone());
        }
    }
    table.entries.first().cloned()
}
