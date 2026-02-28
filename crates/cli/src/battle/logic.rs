use engine::battle::{
    actor_combat_stats, apply_damage_to_actor, apply_turn_start_statuses, damage_multiplier,
    enemy_combat_stats, roll_attack, BattleMode, DamageKind,
};
use engine::party::{actor_traits, row_defense_multiplier};
use engine::runtime::GameRuntime;
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

use super::actions::{execute_enemy_ability_action, execute_enemy_spell_action};
use super::state::{
    actor_is_hidden_during_windup, party_target_indices, BattleMenuState, BattleTurnActor,
    BattleTurnState, TargetRule,
};
use crate::menu::magic::spell_effect_allows_battle;

pub fn build_turn_order(
    runtime: &GameRuntime,
    battle_state: &engine::battle::BattleState,
) -> Vec<BattleTurnActor> {
    let mut entries: Vec<(i32, u8, usize, BattleTurnActor)> = Vec::new();
    for (index, id) in battle_state.party_order.iter().enumerate() {
        if let Some(actor) = runtime.party.roster.get(id) {
            if actor.current_hp <= 0 {
                continue;
            }
            let speed = actor.base_stats.get("agi").copied().unwrap_or(1).max(1);
            entries.push((speed, 0, index, BattleTurnActor::Party(index)));
        }
    }
    for (index, enemy) in battle_state.enemies.iter().enumerate() {
        if !enemy.is_alive() {
            continue;
        }
        let speed = enemy.stats.get("agi").copied().unwrap_or(1).max(1);
        entries.push((speed, 1, index, BattleTurnActor::Enemy(index)));
    }
    entries.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then(left.1.cmp(&right.1))
            .then(left.2.cmp(&right.2))
    });
    entries.into_iter().map(|entry| entry.3).collect()
}

pub fn advance_turn(
    menu_state: &mut BattleMenuState,
    turn_state: &mut BattleTurnState,
    battle_state: &mut engine::battle::BattleState,
) {
    battle_state.turns = battle_state.turns.saturating_add(1);
    match battle_state.mode {
        BattleMode::Turn => {
            turn_state.index = turn_state.index.saturating_add(1);
        }
        BattleMode::Dynamic | BattleMode::DynamicWait => {
            if !turn_state.order.is_empty() {
                let actor = turn_state.order.remove(0);
                // Reset readiness
                match actor {
                    BattleTurnActor::Party(idx) => {
                        if let Some(id) = battle_state.party_order.get(idx) {
                            battle_state.readiness_party.insert(id.clone(), 0.0);
                        }
                    }
                    BattleTurnActor::Enemy(idx) => {
                        if idx < battle_state.readiness_enemy.len() {
                            battle_state.readiness_enemy[idx] = 0.0;
                        }
                    }
                }
            }
            turn_state.index = 0;
        }
    }
    menu_state.reset_for_actor();
}

pub fn enemy_take_turn(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    enemy_index: usize,
    menu_state: &mut BattleMenuState,
    rng: &mut impl Rng,
) -> Option<usize> {
    {
        let Some(enemy) = battle_state.enemies.get_mut(enemy_index) else {
            return None;
        };
        if !enemy.is_alive() {
            return None;
        }
        let mut turn_result = apply_turn_start_statuses(
            &runtime.content,
            &enemy.name,
            enemy.max_hp(),
            &mut enemy.current_hp,
            &mut enemy.statuses,
            rng,
        );
        for message in turn_result.messages.drain(..) {
            push_battle_log(&mut battle_state.log, message);
        }
        if !enemy.is_alive() {
            push_battle_log(&mut battle_state.log, format!("{} falls!", enemy.name));
            return None;
        }
        if !turn_result.can_act {
            return None;
        }
    }
    let action = select_enemy_action(runtime, battle_state, enemy_index, menu_state, rng);
    match action {
        EnemyAction::Spell {
            spell_id,
            target_side,
            target_mode,
            target_index,
        } => execute_enemy_spell_action(
            runtime,
            battle_state,
            enemy_index,
            spell_id.as_str(),
            target_side,
            target_mode,
            target_index,
            rng,
        ),
        EnemyAction::Ability {
            ability_id,
            target_side,
            target_mode,
            target_index,
        } => execute_enemy_ability_action(
            runtime,
            battle_state,
            enemy_index,
            ability_id.as_str(),
            target_side,
            target_mode,
            target_index,
            rng,
        ),
        EnemyAction::Attack { target_id } => {
            if target_id.is_empty() {
                return None;
            }
            let (enemy_name, attacker_stats) = {
                let enemy = battle_state.enemies.get(enemy_index)?;
                (
                    enemy.name.clone(),
                    enemy_combat_stats(&runtime.content, enemy),
                )
            };
            let (final_target_id, covered) = resolve_cover_target(runtime, menu_state, &target_id);
            if runtime.content.rules.progression_mode == engine::rules::ProgressionMode::Activity {
                battle_state
                    .growth
                    .entry(final_target_id.clone())
                    .or_default()
                    .turns_targeted += 1.0;
            }
            if let Some((coverer, original)) = covered {
                push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.cover",
                        "{coverer} covers {target}!",
                        &[("coverer", coverer), ("target", original)],
                    ),
                );
            }

            let Some(target_snapshot) = runtime.party.roster.get(&final_target_id).cloned() else {
                return None;
            };
            let target_name = target_snapshot.name.clone();
            let defender_stats = actor_combat_stats(&target_snapshot);
            let roll = roll_attack(
                &runtime.content,
                &runtime.content.rules.battle,
                &attacker_stats,
                &defender_stats,
                DamageKind::Physical,
                0,
                0.0,
                rng,
            );
            if !roll.hit {
                if runtime.content.rules.progression_mode
                    == engine::rules::ProgressionMode::Activity
                {
                    battle_state
                        .growth
                        .entry(final_target_id.clone())
                        .or_default()
                        .dodges += 1.0;
                }
                push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.miss",
                        "{actor} misses {target}.",
                        &[
                            ("actor", enemy_name.clone()),
                            ("target", target_name.clone()),
                        ],
                    ),
                );
                return battle_state
                    .party_order
                    .iter()
                    .position(|id| id == &final_target_id);
            }

            let mut damage = roll.base_damage;
            if menu_state.parrying.remove(&final_target_id) {
                damage = ((damage as f32) * 0.5).round().max(1.0) as i32;
                push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.parry",
                        "{target} parries the attack!",
                        &[("target", target_name.clone())],
                    ),
                );
            }
            if menu_state.defending.remove(&final_target_id) {
                damage = ((damage as f32) * 0.5).round().max(1.0) as i32;
                push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.brace",
                        "{target} braces for impact!",
                        &[("target", target_name.clone())],
                    ),
                );
            }
            let multiplier = damage_multiplier(
                &runtime.content,
                &target_snapshot.statuses,
                &actor_traits(&runtime.content, &target_snapshot),
                DamageKind::Physical,
                None,
            );
            let row_multiplier = row_defense_multiplier(&runtime.content, &target_snapshot);
            damage = ((damage as f32) * multiplier * row_multiplier)
                .round()
                .max(0.0) as i32;
            if let Some(target) = runtime.party.roster.get_mut(&final_target_id) {
                apply_damage_to_actor(target, damage);
                if runtime.content.rules.progression_mode
                    == engine::rules::ProgressionMode::Activity
                {
                    let max_hp = target.derived_stats.get("hp").copied().unwrap_or(0).max(1) as f32;
                    let growth = battle_state
                        .growth
                        .entry(final_target_id.clone())
                        .or_default();
                    growth.damage_taken += damage.max(0) as f32;
                    if (target.current_hp as f32) / max_hp <= 0.25 {
                        growth.hp_below_25 = true;
                    }
                }
            }
            if roll.crit {
                push_battle_log(
                    &mut battle_state.log,
                    crate::battle::ui_text(runtime, "battle.log.critical", "Critical hit!"),
                );
            }
            push_battle_log(
                &mut battle_state.log,
                crate::battle::format_ui_text(
                    runtime,
                    "battle.log.attack",
                    "{actor} attacks {target} for {damage} HP.",
                    &[
                        ("actor", enemy_name.clone()),
                        ("target", target_name.clone()),
                        ("damage", damage.to_string()),
                    ],
                ),
            );
            if runtime
                .party
                .roster
                .get(&final_target_id)
                .map(|actor| actor.current_hp <= 0)
                .unwrap_or(false)
            {
                push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.fall",
                        "{actor} falls!",
                        &[("actor", target_name.clone())],
                    ),
                );
            }
            handle_counter_attack(
                runtime,
                battle_state,
                menu_state,
                enemy_index,
                &final_target_id,
                rng,
            );
            battle_state
                .party_order
                .iter()
                .position(|id| id == &final_target_id)
        }
    }
}

#[derive(Clone, Debug)]
enum EnemyAction {
    Attack {
        target_id: String,
    },
    Spell {
        spell_id: String,
        target_side: super::state::TargetSide,
        target_mode: super::state::TargetMode,
        target_index: Option<usize>,
    },
    Ability {
        ability_id: String,
        target_side: super::state::TargetSide,
        target_mode: super::state::TargetMode,
        target_index: Option<usize>,
    },
}

#[derive(Clone, Debug)]
enum EnemyActionKind {
    Spell,
    Ability,
}

#[derive(Clone, Debug)]
struct EnemyActionCandidate {
    kind: EnemyActionKind,
    id: String,
    effect_type: String,
    weight: i32,
    target_side: super::state::TargetSide,
    target_mode: super::state::TargetMode,
    target_index: Option<usize>,
}

fn select_enemy_action(
    runtime: &GameRuntime,
    battle_state: &engine::battle::BattleState,
    enemy_index: usize,
    menu_state: &BattleMenuState,
    rng: &mut impl Rng,
) -> EnemyAction {
    let enemy = match battle_state.enemies.get(enemy_index) {
        Some(enemy) => enemy,
        None => {
            return EnemyAction::Attack {
                target_id: String::new(),
            }
        }
    };
    let living_party = battle_state
        .party_order
        .iter()
        .filter(|id| {
            if actor_is_hidden_during_windup(menu_state, id) {
                return false;
            }
            runtime
                .party
                .roster
                .get(*id)
                .map(|actor| actor.current_hp > 0)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let Some(target_id) = living_party.choose(rng).cloned() else {
        return EnemyAction::Attack {
            target_id: String::new(),
        };
    };
    let living_enemies = enemy_target_indices_for_rule(battle_state, TargetRule::Alive);
    let fallen_enemies = enemy_target_indices_for_rule(battle_state, TargetRule::KnockedOut);
    let mut lowest_hp_ratio: f32 = 1.0;
    for index in &living_enemies {
        if let Some(entry) = battle_state.enemies.get(*index) {
            let ratio = if entry.max_hp() > 0 {
                (entry.current_hp.max(0) as f32) / (entry.max_hp() as f32)
            } else {
                1.0
            };
            lowest_hp_ratio = lowest_hp_ratio.min(ratio);
        }
    }
    let mut candidates = Vec::new();
    for spell_id in &enemy.spells {
        let Some(spell) = runtime
            .content
            .spells
            .spells
            .iter()
            .find(|spell| spell.id == *spell_id)
        else {
            continue;
        };
        if !spell_effect_allows_battle(spell.effect.r#type.as_str()) {
            continue;
        }
        if !enemy_spell_cost_available(enemy, spell) {
            continue;
        }
        let target_mode = default_mode_for_value(spell.target_mode.as_str());
        let preferred_side =
            enemy_preferred_side(spell.default_target.as_str(), &spell.allowed_targets);
        let target_side = if enemy_action_allows_side(
            spell.default_target.as_str(),
            &spell.allowed_targets,
            preferred_side,
        ) {
            preferred_side
        } else if enemy_action_allows_side(
            spell.default_target.as_str(),
            &spell.allowed_targets,
            opposite_side(preferred_side),
        ) {
            opposite_side(preferred_side)
        } else {
            continue;
        };
        let target_rule = match spell.effect.r#type.as_str() {
            "revive" => TargetRule::KnockedOut,
            _ => TargetRule::Alive,
        };
        let target_index = select_target_index(
            battle_state,
            runtime,
            menu_state,
            enemy_index,
            target_side,
            target_rule,
            target_mode,
            spell.default_target.as_str(),
            rng,
        );
        if target_mode == super::state::TargetMode::Single && target_index.is_none() {
            continue;
        }
        candidates.push(EnemyActionCandidate {
            kind: EnemyActionKind::Spell,
            id: spell.id.clone(),
            effect_type: spell.effect.r#type.clone(),
            weight: enemy.ai.weights.spells.max(0),
            target_side,
            target_mode,
            target_index,
        });
    }
    for ability_id in &enemy.abilities {
        let Some(ability) = runtime
            .content
            .abilities
            .abilities
            .iter()
            .find(|ability| ability.id == *ability_id)
        else {
            continue;
        };
        let (cost_type, cost_value) = ability
            .cost
            .as_ref()
            .map(|cost| (cost.r#type.as_str(), cost.value))
            .unwrap_or(("none", 0));
        if !enemy_ability_cost_available(enemy, cost_type, cost_value) {
            continue;
        }
        let target_mode = default_mode_for_value(ability.target_mode.as_str());
        let preferred_side =
            enemy_preferred_side(ability.default_target.as_str(), &ability.allowed_targets);
        let target_side = if enemy_action_allows_side(
            ability.default_target.as_str(),
            &ability.allowed_targets,
            preferred_side,
        ) {
            preferred_side
        } else if enemy_action_allows_side(
            ability.default_target.as_str(),
            &ability.allowed_targets,
            opposite_side(preferred_side),
        ) {
            opposite_side(preferred_side)
        } else {
            continue;
        };
        let target_rule = match ability.effect.r#type.as_str() {
            "revive" => TargetRule::KnockedOut,
            _ => TargetRule::Alive,
        };
        let target_index = select_target_index(
            battle_state,
            runtime,
            menu_state,
            enemy_index,
            target_side,
            target_rule,
            target_mode,
            ability.default_target.as_str(),
            rng,
        );
        if target_mode == super::state::TargetMode::Single && target_index.is_none() {
            continue;
        }
        candidates.push(EnemyActionCandidate {
            kind: EnemyActionKind::Ability,
            id: ability.id.clone(),
            effect_type: ability.effect.r#type.clone(),
            weight: enemy.ai.weights.abilities.max(0),
            target_side,
            target_mode,
            target_index,
        });
    }

    if enemy.ai.prefer_revive && !fallen_enemies.is_empty() {
        if let Some(action) = choose_weighted_action(
            candidates
                .iter()
                .filter(|candidate| {
                    candidate.effect_type == "revive"
                        && candidate.target_side == super::state::TargetSide::Enemy
                })
                .cloned()
                .collect::<Vec<_>>(),
            rng,
        ) {
            return to_enemy_action(action);
        }
    }

    if lowest_hp_ratio <= enemy.ai.heal_below_hp {
        if let Some(action) = choose_weighted_action(
            candidates
                .iter()
                .filter(|candidate| {
                    candidate.effect_type == "heal"
                        && candidate.target_side == super::state::TargetSide::Enemy
                })
                .cloned()
                .collect::<Vec<_>>(),
            rng,
        ) {
            return to_enemy_action(action);
        }
    }

    if !candidates.is_empty() {
        let candidate_weight: i32 = candidates.iter().map(|candidate| candidate.weight).sum();
        let attack_weight = enemy.ai.weights.attack.max(0);
        if candidate_weight > 0 && attack_weight > 0 {
            let roll = rng.random_range(1..=candidate_weight + attack_weight);
            if roll <= attack_weight {
                return EnemyAction::Attack { target_id };
            }
        }
        if let Some(action) = choose_weighted_action(candidates, rng) {
            return to_enemy_action(action);
        }
    }

    EnemyAction::Attack { target_id }
}

fn to_enemy_action(candidate: EnemyActionCandidate) -> EnemyAction {
    match candidate.kind {
        EnemyActionKind::Spell => EnemyAction::Spell {
            spell_id: candidate.id,
            target_side: candidate.target_side,
            target_mode: candidate.target_mode,
            target_index: candidate.target_index,
        },
        EnemyActionKind::Ability => EnemyAction::Ability {
            ability_id: candidate.id,
            target_side: candidate.target_side,
            target_mode: candidate.target_mode,
            target_index: candidate.target_index,
        },
    }
}

fn choose_weighted_action(
    candidates: Vec<EnemyActionCandidate>,
    rng: &mut impl Rng,
) -> Option<EnemyActionCandidate> {
    let total_weight: i32 = candidates.iter().map(|candidate| candidate.weight).sum();
    if candidates.is_empty() || total_weight <= 0 {
        return None;
    }
    let mut roll = rng.random_range(1..=total_weight);
    for candidate in candidates {
        roll -= candidate.weight;
        if roll <= 0 {
            return Some(candidate);
        }
    }
    None
}

fn enemy_preferred_side(
    default_target: &str,
    allowed_targets: &[String],
) -> super::state::TargetSide {
    if allowed_targets.is_empty() {
        if default_target == "enemy" {
            super::state::TargetSide::Party
        } else {
            super::state::TargetSide::Enemy
        }
    } else if allowed_targets.iter().any(|target| target == "enemy") {
        super::state::TargetSide::Party
    } else {
        super::state::TargetSide::Enemy
    }
}

fn enemy_action_allows_side(
    default_target: &str,
    allowed_targets: &[String],
    side: super::state::TargetSide,
) -> bool {
    if allowed_targets.is_empty() {
        return match default_target {
            "enemy" => side == super::state::TargetSide::Party,
            "party" | "ally" | "self" => side == super::state::TargetSide::Enemy,
            _ => false,
        };
    }
    for target in allowed_targets {
        match target.as_str() {
            "enemy" if side == super::state::TargetSide::Party => return true,
            "party" | "ally" | "self" if side == super::state::TargetSide::Enemy => return true,
            _ => {}
        }
    }
    false
}

fn opposite_side(side: super::state::TargetSide) -> super::state::TargetSide {
    match side {
        super::state::TargetSide::Enemy => super::state::TargetSide::Party,
        super::state::TargetSide::Party => super::state::TargetSide::Enemy,
    }
}

fn default_mode_for_value(target_mode: &str) -> super::state::TargetMode {
    if target_mode == "multi" {
        super::state::TargetMode::Multi
    } else {
        super::state::TargetMode::Single
    }
}

fn select_target_index(
    battle_state: &engine::battle::BattleState,
    runtime: &GameRuntime,
    menu_state: &BattleMenuState,
    enemy_index: usize,
    target_side: super::state::TargetSide,
    rule: TargetRule,
    target_mode: super::state::TargetMode,
    default_target: &str,
    rng: &mut impl Rng,
) -> Option<usize> {
    if target_mode == super::state::TargetMode::Multi {
        return None;
    }
    match target_side {
        super::state::TargetSide::Party => {
            let valid = party_target_indices(runtime, battle_state, rule)
                .into_iter()
                .filter(|index| {
                    battle_state
                        .party_order
                        .get(*index)
                        .map(|id| !actor_is_hidden_during_windup(menu_state, id))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            valid.choose(rng).copied()
        }
        super::state::TargetSide::Enemy => {
            if default_target == "self" {
                return Some(enemy_index);
            }
            let valid = enemy_target_indices_for_rule(battle_state, rule);
            valid.choose(rng).copied().or(Some(enemy_index))
        }
    }
}

fn enemy_target_indices_for_rule(
    battle_state: &engine::battle::BattleState,
    rule: TargetRule,
) -> Vec<usize> {
    battle_state
        .enemies
        .iter()
        .enumerate()
        .filter_map(|(index, enemy)| match rule {
            TargetRule::Alive if enemy.is_alive() => Some(index),
            TargetRule::KnockedOut if !enemy.is_alive() => Some(index),
            _ => None,
        })
        .collect()
}

fn enemy_spell_cost_available(
    enemy: &engine::battle::BattleEnemy,
    spell: &engine::entities::SpellDefinition,
) -> bool {
    if enemy.mp_pool == "unlimited" {
        return true;
    }
    let cost = match spell.cost.r#type.as_str() {
        "tier_charges" => 1,
        "mp" => spell.cost.value,
        _ => return false,
    };
    enemy.current_mp >= cost
}

fn enemy_ability_cost_available(
    enemy: &engine::battle::BattleEnemy,
    cost_type: &str,
    cost_value: i32,
) -> bool {
    match cost_type {
        "none" => true,
        "mp" => enemy.mp_pool == "unlimited" || enemy.current_mp >= cost_value,
        "hp" => enemy.current_hp >= cost_value,
        "death" => true,
        "random" => true,
        _ => false,
    }
}

fn resolve_cover_target(
    runtime: &GameRuntime,
    menu_state: &mut BattleMenuState,
    target_id: &str,
) -> (String, Option<(String, String)>) {
    if let Some(coverer_id) = menu_state.covering.remove(target_id) {
        let coverer_alive = runtime
            .party
            .roster
            .get(&coverer_id)
            .map(|actor| actor.current_hp > 0)
            .unwrap_or(false);
        if coverer_alive {
            let coverer_name = runtime
                .party
                .roster
                .get(&coverer_id)
                .map(|actor| actor.name.clone())
                .unwrap_or_else(|| coverer_id.clone());
            let target_name = runtime
                .party
                .roster
                .get(target_id)
                .map(|actor| actor.name.clone())
                .unwrap_or_else(|| target_id.to_string());
            return (coverer_id, Some((coverer_name, target_name)));
        }
    }
    (target_id.to_string(), None)
}

fn handle_counter_attack(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    menu_state: &mut BattleMenuState,
    enemy_index: usize,
    target_id: &str,
    rng: &mut impl Rng,
) {
    if !menu_state.countering.remove(target_id) {
        return;
    }
    let Some(actor) = runtime.party.roster.get(target_id) else {
        return;
    };
    if actor.current_hp <= 0 {
        return;
    }
    let Some(enemy) = battle_state.enemies.get_mut(enemy_index) else {
        return;
    };
    if !enemy.is_alive() {
        return;
    }
    let attacker_stats = actor_combat_stats(actor);
    let defender_stats = enemy_combat_stats(&runtime.content, enemy);
    let roll = roll_attack(
        &runtime.content,
        &runtime.content.rules.battle,
        &attacker_stats,
        &defender_stats,
        DamageKind::Physical,
        0,
        0.0,
        rng,
    );
    if !roll.hit {
        return;
    }
    let mut damage = roll.base_damage;
    let multiplier = damage_multiplier(
        &runtime.content,
        &enemy.statuses,
        &enemy.traits,
        DamageKind::Physical,
        None,
    );
    damage = ((damage as f32) * multiplier).round().max(0.0) as i32;
    engine::battle::apply_damage_to_enemy(enemy, damage);
    if roll.crit {
        push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.log.critical", "Critical hit!"),
        );
    }
    push_battle_log(
        &mut battle_state.log,
        crate::battle::format_ui_text(
            runtime,
            "battle.log.counter",
            "{actor} counters {target} for {damage} HP.",
            &[
                ("actor", actor.name.clone()),
                ("target", enemy.name.clone()),
                ("damage", damage.to_string()),
            ],
        ),
    );
}

pub fn push_battle_log(log: &mut Vec<String>, message: impl Into<String>) {
    log.push(message.into());
    let max_entries = 6;
    if log.len() > max_entries {
        let drain = log.len() - max_entries;
        log.drain(0..drain);
    }
}

pub fn update_readiness(
    runtime: &engine::runtime::GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    delta_seconds: f32,
) -> Vec<BattleTurnActor> {
    let mut ready_candidates = Vec::new();
    let multiplier = runtime.effective_readiness_speed();

    // Update Party
    for (index, id) in battle_state.party_order.iter().enumerate() {
        if let Some(actor) = runtime.party.roster.get(id) {
            if actor.current_hp > 0 {
                let current = battle_state
                    .readiness_party
                    .entry(id.clone())
                    .or_insert(0.0);
                if *current < 100.0 {
                    let speed = actor.base_stats.get("agi").copied().unwrap_or(1).max(1) as f32;
                    *current += speed * delta_seconds * multiplier;
                    if *current >= 100.0 {
                        let overflow = *current - 100.0;
                        *current = 100.0;
                        ready_candidates.push((overflow, BattleTurnActor::Party(index)));
                    }
                }
            } else {
                // Reset readiness if dead?
                battle_state.readiness_party.insert(id.clone(), 0.0);
            }
        }
    }

    // Update Enemies
    if battle_state.readiness_enemy.len() < battle_state.enemies.len() {
        battle_state
            .readiness_enemy
            .resize(battle_state.enemies.len(), 0.0);
    }
    for (index, enemy) in battle_state.enemies.iter().enumerate() {
        if enemy.is_alive() {
            let current = &mut battle_state.readiness_enemy[index];
            if *current < 100.0 {
                let speed = enemy.stats.get("agi").copied().unwrap_or(1).max(1) as f32;
                *current += speed * delta_seconds * multiplier;
                if *current >= 100.0 {
                    let overflow = *current - 100.0;
                    *current = 100.0;
                    ready_candidates.push((overflow, BattleTurnActor::Enemy(index)));
                }
            }
        } else {
            battle_state.readiness_enemy[index] = 0.0;
        }
    }

    ready_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    ready_candidates
        .into_iter()
        .map(|(_, actor)| actor)
        .collect()
}
