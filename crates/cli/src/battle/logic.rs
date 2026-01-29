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

pub fn advance_turn(menu_state: &mut BattleMenuState, turn_state: &mut BattleTurnState) {
    turn_state.index = turn_state.index.saturating_add(1);
    menu_state.reset_for_actor();
}

pub fn enemy_take_turn(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    enemy_index: usize,
    rng: &mut impl Rng,
) -> Option<usize> {
    let Some(enemy) = battle_state.enemies.get_mut(enemy_index) else {
        return None;
    };
    if !enemy.is_alive() {
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
    let Some(target) = runtime.party.roster.get_mut(&target_id) else {
        return None;
    };
    let def = target.derived_stats.get("def").copied().unwrap_or(0);
    let damage = engine::battle::physical_damage(enemy.atk(), def, rng);
    engine::battle::apply_damage_to_actor(target, damage);
    push_battle_log(
        &mut battle_state.log,
        format!("{} attacks {} for {} HP.", enemy.name, target.name, damage),
    );
    if target.current_hp <= 0 {
        push_battle_log(&mut battle_state.log, format!("{} falls!", target.name));
    }
    battle_state
        .party_order
        .iter()
        .position(|id| id == &target_id)
}

pub fn push_battle_log(log: &mut Vec<String>, message: impl Into<String>) {
    log.push(message.into());
    let max_entries = 6;
    if log.len() > max_entries {
        let drain = log.len() - max_entries;
        log.drain(0..drain);
    }
}
