use engine::battle::{apply_damage_to_enemy, physical_damage, roll_damage};
use engine::rules::MagicSystem;
use engine::runtime::GameRuntime;
use rand::Rng;

use crate::menu::common::{AbilityEntry, SpellEntry};

pub fn execute_attack_action(
    runtime: &GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    enemy_index: usize,
    rng: &mut impl Rng,
) {
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return;
    };
    let Some(enemy) = battle_state.enemies.get_mut(enemy_index) else {
        return;
    };
    if !enemy.is_alive() {
        super::logic::push_battle_log(&mut battle_state.log, "No target.");
        return;
    }
    let atk = actor.derived_stats.get("atk").copied().unwrap_or(0);
    let damage = physical_damage(atk, enemy.def(), rng);
    apply_damage_to_enemy(enemy, damage);
    super::logic::push_battle_log(
        &mut battle_state.log,
        format!("{} attacks {} for {} HP.", actor.name, enemy.name, damage),
    );
}

pub fn execute_magic_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    entry: &SpellEntry,
    target_index: Option<usize>,
    rng: &mut impl Rng,
) {
    let magic_system = runtime.content.rules.game.magic_system.clone();
    let (actor_name, matk) = {
        let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
            return;
        };
        if !crate::menu::magic::spell_cost_available(
            magic_system.clone(),
            actor,
            entry.cost_type.as_str(),
            entry.tier,
            entry.cost_value,
        ) {
            let reason = match magic_system {
                MagicSystem::Mp => "Not enough MP.",
                MagicSystem::TierCharges => "No tier charges.",
            };
            super::logic::push_battle_log(&mut battle_state.log, reason);
            return;
        }
        if !crate::menu::magic::consume_spell_cost(magic_system.clone(), actor, entry) {
            let reason = match magic_system {
                MagicSystem::Mp => "Not enough MP.",
                MagicSystem::TierCharges => "No tier charges.",
            };
            super::logic::push_battle_log(&mut battle_state.log, reason);
            return;
        }
        (
            actor.name.clone(),
            actor.derived_stats.get("matk").copied().unwrap_or(0),
        )
    };

    match entry.default_target.as_str() {
        "enemy" => {
            if let Some(index) = target_index {
                if let Some(enemy) = battle_state.enemies.get_mut(index) {
                    if !enemy.is_alive() {
                        super::logic::push_battle_log(&mut battle_state.log, "No target.");
                        return;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let base = (entry.effect_power + matk / 2).max(1);
                            let damage = roll_damage(base, rng);
                            apply_damage_to_enemy(enemy, damage);
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} casts {} on {} for {} HP.",
                                    actor_name, entry.name, enemy.name, damage
                                ),
                            );
                        }
                        "scan" => {
                            enemy.scanned = true;
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} scans {}: {}/{} HP.",
                                    actor_name,
                                    enemy.name,
                                    enemy.current_hp.max(0),
                                    enemy.max_hp().max(1)
                                ),
                            );
                        }
                        _ => {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                "Nothing happens.",
                            );
                        }
                    }
                }
            }
        }
        "party" => {
            for id in battle_state.party_order.clone() {
                crate::menu::magic::apply_spell_to_actor(runtime, entry, &id);
            }
            super::logic::push_battle_log(
                &mut battle_state.log,
                format!("{} casts {} on the party.", actor_name, entry.name),
            );
        }
        _ => {
            let target_id = target_index
                .and_then(|index| battle_state.party_order.get(index))
                .cloned()
                .unwrap_or_else(|| actor_id.to_string());
            crate::menu::magic::apply_spell_to_actor(runtime, entry, &target_id);
            let target_name = runtime
                .party
                .roster
                .get(&target_id)
                .map(|actor| actor.name.clone())
                .unwrap_or_else(|| target_id.clone());
            super::logic::push_battle_log(
                &mut battle_state.log,
                format!("{} casts {} on {}.", actor_name, entry.name, target_name),
            );
        }
    }
}

pub fn execute_ability_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    entry: &AbilityEntry,
    target_index: Option<usize>,
    rng: &mut impl Rng,
) {
    let (actor_name, atk) = {
        let Some(actor) = runtime.party.roster.get(actor_id) else {
            return;
        };
        (
            actor.name.clone(),
            actor.derived_stats.get("atk").copied().unwrap_or(0),
        )
    };

    match entry.default_target.as_str() {
        "enemy" => {
            if let Some(index) = target_index {
                if let Some(enemy) = battle_state.enemies.get_mut(index) {
                    if !enemy.is_alive() {
                        super::logic::push_battle_log(&mut battle_state.log, "No target.");
                        return;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let base = (entry.effect_power + atk / 2).max(1);
                            let damage = roll_damage(base, rng);
                            apply_damage_to_enemy(enemy, damage);
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} uses {} on {} for {} HP.",
                                    actor_name, entry.name, enemy.name, damage
                                ),
                            );
                        }
                        "scan" => {
                            enemy.scanned = true;
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} scans {}: {}/{} HP.",
                                    actor_name,
                                    enemy.name,
                                    enemy.current_hp.max(0),
                                    enemy.max_hp().max(1)
                                ),
                            );
                        }
                        _ => {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                "Nothing happens.",
                            );
                        }
                    }
                }
            }
        }
        "party" => {
            for id in battle_state.party_order.clone() {
                crate::menu::abilities::apply_ability_to_actor(runtime, entry, &id);
            }
            super::logic::push_battle_log(
                &mut battle_state.log,
                format!("{} uses {} on the party.", actor_name, entry.name),
            );
        }
        _ => {
            let target_id = target_index
                .and_then(|index| battle_state.party_order.get(index))
                .cloned()
                .unwrap_or_else(|| actor_id.to_string());
            crate::menu::abilities::apply_ability_to_actor(runtime, entry, &target_id);
            let target_name = runtime
                .party
                .roster
                .get(&target_id)
                .map(|actor| actor.name.clone())
                .unwrap_or_else(|| target_id.clone());
            super::logic::push_battle_log(
                &mut battle_state.log,
                format!("{} uses {} on {}.", actor_name, entry.name, target_name),
            );
        }
    }
}

pub fn execute_item_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    item: &engine::entities::ItemDefinition,
    target_index: Option<usize>,
) {
    let target_ids = match item.usage.target.as_str() {
        "party" => battle_state.party_order.clone(),
        "enemy" => Vec::new(),
        _ => target_index
            .and_then(|index| battle_state.party_order.get(index))
            .map(|id| vec![id.clone()])
            .unwrap_or_else(|| vec![actor_id.to_string()]),
    };

    if !crate::menu::inventory::item_usage_allows_battle(&item.usage.context) {
        super::logic::push_battle_log(&mut battle_state.log, "Item unusable.");
        return;
    }
    if !runtime.inventory.remove_item(&item.id, 1) {
        super::logic::push_battle_log(&mut battle_state.log, "No items left.");
        return;
    }
    for target_id in target_ids {
        crate::menu::inventory::apply_item_to_actor(runtime, item, &target_id);
    }
    let actor_name = runtime
        .party
        .roster
        .get(actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_else(|| actor_id.to_string());
    super::logic::push_battle_log(
        &mut battle_state.log,
        format!("{} uses {}.", actor_name, item.name),
    );
}
