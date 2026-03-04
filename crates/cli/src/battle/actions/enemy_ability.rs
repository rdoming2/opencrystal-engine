use engine::battle::{
    actor_combat_stats, apply_damage_to_actor, apply_damage_to_enemy, apply_status_effects,
    damage_multiplier, enemy_combat_stats, healing_inverted, roll_attack, DamageKind,
};
use engine::party::actor_traits;
use engine::runtime::GameRuntime;
use rand::seq::IndexedRandom;
use rand::Rng;

use crate::battle::logic;
use crate::battle::state::{party_target_indices, TargetMode, TargetRule, TargetSide};

use super::shared::{
    apply_attenuation, consume_enemy_ability_cost, enemy_ability_cost_available,
    enemy_target_indices_for_rule,
};

pub fn execute_enemy_ability_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    enemy_index: usize,
    ability_id: &str,
    target_side: TargetSide,
    target_mode: TargetMode,
    target_index: Option<usize>,
    rng: &mut impl Rng,
) -> Option<usize> {
    let Some(ability) = runtime
        .content
        .abilities
        .abilities
        .iter()
        .find(|ability| ability.id == ability_id)
    else {
        return None;
    };
    let (cost_type, cost_value) = ability
        .cost
        .as_ref()
        .map(|cost| (cost.r#type.as_str(), cost.value))
        .unwrap_or(("none", 0));
    let target_rule = match ability.effect.r#type.as_str() {
        "revive" => TargetRule::KnockedOut,
        _ => TargetRule::Alive,
    };
    let (enemy_name, attacker_stats) = {
        let enemy = battle_state.enemies.get(enemy_index)?;
        if !enemy_ability_cost_available(enemy, cost_type, cost_value) {
            return None;
        }
        (
            enemy.name.clone(),
            enemy_combat_stats(&runtime.content, enemy),
        )
    };
    {
        let enemy = battle_state.enemies.get_mut(enemy_index)?;
        if !consume_enemy_ability_cost(enemy, cost_type, cost_value, rng) {
            return None;
        }
    }
    let effect_ids = ability.effect.effects.clone();
    let attenuation = if target_mode == TargetMode::Multi {
        ability.multi_attenuation.unwrap_or(1.0)
    } else {
        1.0
    };
    let mut highlighted_target = None;
    match target_side {
        TargetSide::Party => {
            let mut indices = if target_mode == TargetMode::Multi {
                party_target_indices(runtime, battle_state, target_rule)
            } else {
                let valid = party_target_indices(runtime, battle_state, target_rule);
                let selected = target_index
                    .and_then(|index| {
                        if valid.contains(&index) {
                            Some(index)
                        } else {
                            None
                        }
                    })
                    .or_else(|| valid.choose(rng).copied());
                selected.map(|index| vec![index]).unwrap_or_default()
            };
            if indices.is_empty() {
                return None;
            }
            if target_mode == TargetMode::Single {
                highlighted_target = indices.first().copied();
            }
            for index in indices.drain(..) {
                let Some(target_id) = battle_state.party_order.get(index).cloned() else {
                    continue;
                };
                let Some(target_snapshot) = runtime.party.roster.get(&target_id).cloned() else {
                    continue;
                };
                let target_name = target_snapshot.name.clone();
                match ability.effect.r#type.as_str() {
                    "damage" => {
                        let defender_stats = actor_combat_stats(&target_snapshot);
                        let roll = roll_attack(
                            &runtime.content,
                            &runtime.content.rules.battle,
                            &attacker_stats,
                            &defender_stats,
                            DamageKind::Physical,
                            ability.effect.power,
                            0.0,
                            rng,
                        );
                        if !roll.hit {
                            logic::push_battle_log(
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
                            continue;
                        }
                        let mut damage = roll.base_damage;
                        let multiplier = damage_multiplier(
                            &runtime.content,
                            &target_snapshot.statuses,
                            &actor_traits(&runtime.content, &target_snapshot),
                            DamageKind::Physical,
                            None,
                        );
                        damage = ((damage as f32) * multiplier * attenuation)
                            .round()
                            .max(1.0) as i32;
                        if let Some(target) = runtime.party.roster.get_mut(&target_id) {
                            apply_damage_to_actor(target, damage);
                        }
                        logic::push_battle_log(
                            &mut battle_state.log,
                            crate::battle::format_ui_text(
                                runtime,
                                "battle.log.ability",
                                "{actor} uses {ability} on {target} for {damage} HP.",
                                &[
                                    ("actor", enemy_name.clone()),
                                    ("ability", ability.name.clone()),
                                    ("target", target_name.clone()),
                                    ("damage", damage.to_string()),
                                ],
                            ),
                        );
                        if roll.crit {
                            logic::push_critical_battle_log(runtime, &mut battle_state.log);
                        }
                    }
                    "heal" => {
                        if let Some(target) = runtime.party.roster.get_mut(&target_id) {
                            let target_name = target.name.clone();
                            let max_hp = target.derived_stats.get("hp").copied().unwrap_or(0);
                            let amount =
                                apply_attenuation(ability.effect.power.max(1), attenuation);
                            if healing_inverted(
                                &runtime.content,
                                &actor_traits(&runtime.content, target),
                            ) {
                                apply_damage_to_actor(target, amount);
                                logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.healing_damage",
                                        "{target} is harmed by healing for {damage} HP.",
                                        &[
                                            ("target", target_name.clone()),
                                            ("damage", amount.to_string()),
                                        ],
                                    ),
                                );
                            } else {
                                let before = target.current_hp;
                                target.current_hp = (target.current_hp + amount).clamp(0, max_hp);
                                let healed = target.current_hp.saturating_sub(before);
                                logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.heal",
                                        "{target} recovers {amount} HP.",
                                        &[
                                            ("target", target_name.clone()),
                                            ("amount", healed.to_string()),
                                        ],
                                    ),
                                );
                            }
                        }
                    }
                    "revive" => {
                        if let Some(target) = runtime.party.roster.get_mut(&target_id) {
                            let target_name = target.name.clone();
                            if target.current_hp <= 0 {
                                let max_hp = target.derived_stats.get("hp").copied().unwrap_or(0);
                                let amount = if ability.effect.power > 0 {
                                    ability.effect.power
                                } else {
                                    max_hp
                                };
                                target.current_hp = amount.clamp(1, max_hp);
                                logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.revive",
                                        "{target} is revived.",
                                        &[("target", target_name.clone())],
                                    ),
                                );
                            }
                        }
                    }
                    "scan" => {
                        logic::push_battle_log(
                            &mut battle_state.log,
                            crate::battle::format_ui_text(
                                runtime,
                                "battle.log.scan",
                                "{actor} scans {target}.",
                                &[
                                    ("actor", enemy_name.clone()),
                                    ("target", target_name.clone()),
                                ],
                            ),
                        );
                    }
                    "status" => {}
                    _ => {
                        logic::push_battle_log(
                            &mut battle_state.log,
                            crate::battle::ui_text(
                                runtime,
                                "battle.nothing_happens",
                                "Nothing happens.",
                            ),
                        );
                    }
                }
                if !effect_ids.is_empty() {
                    if let Some(target) = runtime.party.roster.get_mut(&target_id) {
                        let target_name = target.name.clone();
                        let applied = apply_status_effects(
                            &runtime.content,
                            &effect_ids,
                            &mut target.statuses,
                            rng,
                        );
                        for label in &applied {
                            logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.status",
                                    "{target} is affected by {status}.",
                                    &[("target", target_name.clone()), ("status", label.clone())],
                                ),
                            );
                        }
                    }
                }
            }
            if target_mode == TargetMode::Multi {
                logic::push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.ability_party",
                        "{actor} uses {ability} on the party.",
                        &[
                            ("actor", enemy_name.clone()),
                            ("ability", ability.name.clone()),
                        ],
                    ),
                );
            }
        }
        TargetSide::Enemy => {
            let mut indices = if target_mode == TargetMode::Multi {
                enemy_target_indices_for_rule(battle_state, target_rule)
            } else {
                let valid = enemy_target_indices_for_rule(battle_state, target_rule);
                let selected = target_index
                    .and_then(|index| {
                        if valid.contains(&index) {
                            Some(index)
                        } else {
                            None
                        }
                    })
                    .or_else(|| valid.choose(rng).copied())
                    .or(Some(enemy_index));
                selected.map(|index| vec![index]).unwrap_or_default()
            };
            if indices.is_empty() {
                return None;
            }
            for index in indices.drain(..) {
                if let Some(enemy_target) = battle_state.enemies.get_mut(index) {
                    match ability.effect.r#type.as_str() {
                        "damage" => {
                            let defender_stats = enemy_combat_stats(&runtime.content, enemy_target);
                            let roll = roll_attack(
                                &runtime.content,
                                &runtime.content.rules.battle,
                                &attacker_stats,
                                &defender_stats,
                                DamageKind::Physical,
                                ability.effect.power,
                                0.0,
                                rng,
                            );
                            if !roll.hit {
                                logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.miss",
                                        "{actor} misses {target}.",
                                        &[
                                            ("actor", enemy_name.clone()),
                                            ("target", enemy_target.name.clone()),
                                        ],
                                    ),
                                );
                                continue;
                            }
                            let mut damage = roll.base_damage;
                            let multiplier = damage_multiplier(
                                &runtime.content,
                                &enemy_target.statuses,
                                &enemy_target.traits,
                                DamageKind::Physical,
                                None,
                            );
                            damage = ((damage as f32) * multiplier * attenuation)
                                .round()
                                .max(1.0) as i32;
                            apply_damage_to_enemy(enemy_target, damage);
                            logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.ability",
                                    "{actor} uses {ability} on {target} for {damage} HP.",
                                    &[
                                        ("actor", enemy_name.clone()),
                                        ("ability", ability.name.clone()),
                                        ("target", enemy_target.name.clone()),
                                        ("damage", damage.to_string()),
                                    ],
                                ),
                            );
                            if roll.crit {
                                logic::push_critical_battle_log(runtime, &mut battle_state.log);
                            }
                        }
                        "heal" => {
                            let max_hp = enemy_target.max_hp();
                            let amount =
                                apply_attenuation(ability.effect.power.max(1), attenuation);
                            if healing_inverted(&runtime.content, &enemy_target.traits) {
                                apply_damage_to_enemy(enemy_target, amount);
                                logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.healing_damage",
                                        "{target} is harmed by healing for {damage} HP.",
                                        &[
                                            ("target", enemy_target.name.clone()),
                                            ("damage", amount.to_string()),
                                        ],
                                    ),
                                );
                            } else {
                                let before = enemy_target.current_hp;
                                enemy_target.current_hp =
                                    (enemy_target.current_hp + amount).clamp(0, max_hp);
                                let healed = enemy_target.current_hp.saturating_sub(before);
                                logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.heal",
                                        "{target} recovers {amount} HP.",
                                        &[
                                            ("target", enemy_target.name.clone()),
                                            ("amount", healed.to_string()),
                                        ],
                                    ),
                                );
                            }
                        }
                        "revive" => {
                            if enemy_target.current_hp <= 0 {
                                let max_hp = enemy_target.max_hp();
                                let amount = if ability.effect.power > 0 {
                                    ability.effect.power
                                } else {
                                    max_hp
                                };
                                enemy_target.current_hp = amount.clamp(1, max_hp);
                                logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.revive",
                                        "{target} is revived.",
                                        &[("target", enemy_target.name.clone())],
                                    ),
                                );
                            }
                        }
                        "scan" => {
                            enemy_target.scanned = true;
                            logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.scan",
                                    "{actor} scans {target}: {current}/{max} HP.",
                                    &[
                                        ("actor", enemy_name.clone()),
                                        ("target", enemy_target.name.clone()),
                                        ("current", enemy_target.current_hp.max(0).to_string()),
                                        ("max", enemy_target.max_hp().max(1).to_string()),
                                    ],
                                ),
                            );
                        }
                        "status" => {}
                        _ => {
                            logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::ui_text(
                                    runtime,
                                    "battle.nothing_happens",
                                    "Nothing happens.",
                                ),
                            );
                        }
                    }
                    if !effect_ids.is_empty() {
                        let applied = apply_status_effects(
                            &runtime.content,
                            &effect_ids,
                            &mut enemy_target.statuses,
                            rng,
                        );
                        for label in &applied {
                            logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.status",
                                    "{target} is affected by {status}.",
                                    &[
                                        ("target", enemy_target.name.clone()),
                                        ("status", label.clone()),
                                    ],
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
    highlighted_target
}
