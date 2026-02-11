use engine::battle::{
    actor_combat_stats, apply_damage_to_actor, apply_turn_start_statuses, damage_multiplier,
    enemy_combat_stats, roll_attack, BattleMode, DamageKind,
};
use engine::party::{actor_traits, row_defense_multiplier};
use engine::runtime::GameRuntime;
use rand::seq::SliceRandom;
use rand::Rng;

use super::state::{BattleMenuState, BattleTurnActor, BattleTurnState};

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
    let living_party = battle_state
        .party_order
        .iter()
        .filter(|id| {
            runtime
                .party
                .roster
                .get(*id)
                .map(|actor| actor.current_hp > 0)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    if living_party.is_empty() {
        return None;
    }
    let Some(target_id) = living_party.choose(rng).cloned() else {
        return None;
    };
    let (final_target_id, covered) = resolve_cover_target(runtime, menu_state, &target_id);
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
    let attacker_stats = enemy_combat_stats(&runtime.content, enemy);
    let defender_stats = actor_combat_stats(&target_snapshot);
    let roll = roll_attack(
        &runtime.content,
        &runtime.content.rules.battle,
        &attacker_stats,
        &defender_stats,
        DamageKind::Physical,
        0,
        rng,
    );
    if !roll.hit {
        push_battle_log(
            &mut battle_state.log,
            crate::battle::format_ui_text(
                runtime,
                "battle.log.miss",
                "{actor} misses {target}.",
                &[
                    ("actor", enemy.name.clone()),
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
                ("actor", enemy.name.clone()),
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
