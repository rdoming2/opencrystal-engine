use engine::battle::{
    apply_damage_to_actor, apply_damage_to_enemy, apply_status_effects, damage_multiplier,
    healing_inverted, physical_damage, roll_damage, DamageKind,
};
use engine::rules::MagicSystem;
use engine::runtime::GameRuntime;
use rand::Rng;

use super::state::{enemy_target_indices, TargetMode, TargetSide};
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
    let mut damage = physical_damage(atk, enemy.def(), rng);
    let multiplier = damage_multiplier(
        &runtime.content,
        &enemy.statuses,
        &enemy.traits,
        DamageKind::Physical,
        None,
    );
    damage = ((damage as f32) * multiplier).round().max(0.0) as i32;
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
    target_side: TargetSide,
    target_mode: TargetMode,
    target_index: Option<usize>,
    rng: &mut impl Rng,
) {
    fn apply_spell_to_actor_battle(
        content: &engine::content::Content,
        entry: &SpellEntry,
        actor: &mut engine::party::Actor,
        attenuation: f32,
    ) -> Option<String> {
        let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
        match entry.effect_type.as_str() {
            "heal" => {
                let inverted =
                    healing_inverted(content, &engine::party::actor_traits(content, actor));
                if inverted {
                    let amount = apply_attenuation(entry.effect_power.max(1), attenuation);
                    apply_damage_to_actor(actor, amount);
                    Some(format!("{} takes {} damage.", actor.name, amount))
                } else {
                    let before = actor.current_hp;
                    let amount = apply_attenuation(entry.effect_power.max(1), attenuation);
                    actor.current_hp = (actor.current_hp + amount).clamp(0, max_hp);
                    let healed = actor.current_hp.saturating_sub(before);
                    Some(format!("{} recovers {} HP.", actor.name, healed))
                }
            }
            "revive" => {
                if actor.current_hp <= 0 {
                    let amount = if entry.effect_power > 0 {
                        entry.effect_power
                    } else {
                        max_hp
                    };
                    actor.current_hp = amount.clamp(1, max_hp);
                    Some(format!("{} is revived.", actor.name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    let magic_system = runtime.content.rules.game.magic_system.clone();
    let spell_definition = runtime
        .content
        .spells
        .spells
        .iter()
        .find(|spell| spell.id == entry.id);
    let effect_ids = spell_definition
        .map(|spell| &spell.effect.effects)
        .cloned()
        .unwrap_or_default();
    let element = spell_definition.and_then(|spell| spell.effect.element.as_deref());
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

    let attenuation = match target_mode {
        TargetMode::Multi => entry.multi_attenuation.unwrap_or(1.0),
        TargetMode::Single => 1.0,
    };

    match target_side {
        TargetSide::Enemy => {
            let indices = if target_mode == TargetMode::Multi {
                enemy_target_indices(battle_state)
            } else {
                target_index.map(|index| vec![index]).unwrap_or_default()
            };
            for index in indices {
                if let Some(enemy) = battle_state.enemies.get_mut(index) {
                    if !enemy.is_alive() {
                        super::logic::push_battle_log(&mut battle_state.log, "No target.");
                        continue;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let base = (entry.effect_power + matk / 2).max(1);
                            let mut damage = roll_damage(base, rng);
                            let multiplier = damage_multiplier(
                                &runtime.content,
                                &enemy.statuses,
                                &enemy.traits,
                                DamageKind::Magic,
                                element,
                            );
                            damage = ((damage as f32) * multiplier * attenuation)
                                .round()
                                .max(1.0) as i32;
                            apply_damage_to_enemy(enemy, damage);
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} casts {} on {} for {} HP.",
                                    actor_name, entry.name, enemy.name, damage
                                ),
                            );
                        }
                        "heal" => {
                            let max_hp = enemy.max_hp();
                            let amount = apply_attenuation(entry.effect_power.max(1), attenuation);
                            if healing_inverted(&runtime.content, &enemy.traits) {
                                apply_damage_to_enemy(enemy, amount);
                                super::logic::push_battle_log(
                                    &mut battle_state.log,
                                    format!(
                                        "{} is harmed by healing for {} HP.",
                                        enemy.name, amount
                                    ),
                                );
                            } else {
                                let before = enemy.current_hp;
                                enemy.current_hp = (enemy.current_hp + amount).clamp(0, max_hp);
                                let healed = enemy.current_hp.saturating_sub(before);
                                super::logic::push_battle_log(
                                    &mut battle_state.log,
                                    format!("{} recovers {} HP.", enemy.name, healed),
                                );
                            }
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
                        "status" => {}
                        _ => {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                "Nothing happens.",
                            );
                        }
                    }
                    if !effect_ids.is_empty() {
                        let applied = apply_status_effects(
                            &runtime.content,
                            &effect_ids,
                            &mut enemy.statuses,
                            rng,
                        );
                        for label in applied {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!("{} is affected by {}.", enemy.name, label),
                            );
                        }
                    }
                }
            }
        }
        TargetSide::Party => {
            let indices = if target_mode == TargetMode::Multi {
                party_indices_for_effect(runtime, battle_state, entry.effect_type.as_str())
            } else {
                target_index.map(|index| vec![index]).unwrap_or_default()
            };
            for index in indices {
                let Some(target_id) = battle_state.party_order.get(index).cloned() else {
                    continue;
                };
                if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
                    if let Some(message) =
                        apply_spell_to_actor_battle(&runtime.content, entry, actor, attenuation)
                    {
                        super::logic::push_battle_log(&mut battle_state.log, message);
                    }
                    if !effect_ids.is_empty() {
                        let applied = apply_status_effects(
                            &runtime.content,
                            &effect_ids,
                            &mut actor.statuses,
                            rng,
                        );
                        for label in applied {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!("{} is affected by {}.", actor.name, label),
                            );
                        }
                    }
                }
            }
            if target_mode == TargetMode::Multi {
                super::logic::push_battle_log(
                    &mut battle_state.log,
                    format!("{} casts {} on the party.", actor_name, entry.name),
                );
            } else if let Some(target_id) = target_index
                .and_then(|index| battle_state.party_order.get(index))
                .cloned()
            {
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
}

pub fn execute_ability_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    entry: &AbilityEntry,
    target_side: TargetSide,
    target_mode: TargetMode,
    target_index: Option<usize>,
    rng: &mut impl Rng,
) {
    fn apply_ability_to_actor_battle(
        content: &engine::content::Content,
        entry: &AbilityEntry,
        actor: &mut engine::party::Actor,
        attenuation: f32,
    ) -> Option<String> {
        let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
        match entry.effect_type.as_str() {
            "heal" => {
                let inverted =
                    healing_inverted(content, &engine::party::actor_traits(content, actor));
                let amount = apply_attenuation(entry.effect_power.max(1), attenuation);
                if inverted {
                    apply_damage_to_actor(actor, amount);
                    Some(format!("{} takes {} damage.", actor.name, amount))
                } else {
                    let before = actor.current_hp;
                    actor.current_hp = (actor.current_hp + amount).clamp(0, max_hp);
                    let healed = actor.current_hp.saturating_sub(before);
                    Some(format!("{} recovers {} HP.", actor.name, healed))
                }
            }
            "revive" => {
                if actor.current_hp <= 0 {
                    let amount = if entry.effect_power > 0 {
                        entry.effect_power
                    } else {
                        max_hp
                    };
                    actor.current_hp = amount.clamp(1, max_hp);
                    Some(format!("{} is revived.", actor.name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    let ability_definition = runtime
        .content
        .abilities
        .abilities
        .iter()
        .find(|ability| ability.id == entry.id);
    let effect_ids = ability_definition
        .map(|ability| &ability.effect.effects)
        .cloned()
        .unwrap_or_default();
    let (actor_name, atk) = {
        let Some(actor) = runtime.party.roster.get(actor_id) else {
            return;
        };
        let (usable, reason) = crate::menu::abilities::ability_cost_available(
            runtime,
            actor,
            &entry.cost_type,
            entry.cost_value,
            entry.cost_item_id.as_deref(),
        );
        if !usable {
            super::logic::push_battle_log(
                &mut battle_state.log,
                reason.unwrap_or_else(|| "Cannot use ability.".to_string()),
            );
            return;
        }
        (
            actor.name.clone(),
            actor.derived_stats.get("atk").copied().unwrap_or(0),
        )
    };
    if !crate::menu::abilities::consume_ability_cost(runtime, entry, actor_id) {
        super::logic::push_battle_log(&mut battle_state.log, "Failed to pay cost.".to_string());
        return;
    }

    let attenuation = match target_mode {
        TargetMode::Multi => entry.multi_attenuation.unwrap_or(1.0),
        TargetMode::Single => 1.0,
    };

    match target_side {
        TargetSide::Enemy => {
            let indices = if target_mode == TargetMode::Multi {
                enemy_target_indices(battle_state)
            } else {
                target_index.map(|index| vec![index]).unwrap_or_default()
            };
            for index in indices {
                if let Some(enemy) = battle_state.enemies.get_mut(index) {
                    if !enemy.is_alive() {
                        super::logic::push_battle_log(&mut battle_state.log, "No target.");
                        continue;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let base = (entry.effect_power + atk / 2).max(1);
                            let mut damage = roll_damage(base, rng);
                            let multiplier = damage_multiplier(
                                &runtime.content,
                                &enemy.statuses,
                                &enemy.traits,
                                DamageKind::Physical,
                                None,
                            );
                            damage = ((damage as f32) * multiplier * attenuation)
                                .round()
                                .max(1.0) as i32;
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
                        "status" => {}
                        _ => {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                "Nothing happens.",
                            );
                        }
                    }
                    if !effect_ids.is_empty() {
                        let applied = apply_status_effects(
                            &runtime.content,
                            &effect_ids,
                            &mut enemy.statuses,
                            rng,
                        );
                        for label in applied {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!("{} is affected by {}.", enemy.name, label),
                            );
                        }
                    }
                }
            }
        }
        TargetSide::Party => {
            let indices = if target_mode == TargetMode::Multi {
                party_indices_for_effect(runtime, battle_state, entry.effect_type.as_str())
            } else {
                target_index.map(|index| vec![index]).unwrap_or_default()
            };
            for index in indices {
                let Some(target_id) = battle_state.party_order.get(index).cloned() else {
                    continue;
                };
                if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
                    if let Some(message) =
                        apply_ability_to_actor_battle(&runtime.content, entry, actor, attenuation)
                    {
                        super::logic::push_battle_log(&mut battle_state.log, message);
                    } else {
                        crate::menu::abilities::apply_ability_to_actor(runtime, entry, &target_id);
                    }
                }
                if !effect_ids.is_empty() {
                    if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
                        let applied = apply_status_effects(
                            &runtime.content,
                            &effect_ids,
                            &mut actor.statuses,
                            rng,
                        );
                        for label in applied {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                format!("{} is affected by {}.", actor.name, label),
                            );
                        }
                    }
                }
            }
            if target_mode == TargetMode::Multi {
                super::logic::push_battle_log(
                    &mut battle_state.log,
                    format!("{} uses {} on the party.", actor_name, entry.name),
                );
            } else if let Some(target_id) = target_index
                .and_then(|index| battle_state.party_order.get(index))
                .cloned()
            {
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
}

pub fn execute_item_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    item: &engine::entities::ItemDefinition,
    target_index: Option<usize>,
) {
    fn apply_item_to_actor_battle(
        content: &engine::content::Content,
        item: &engine::entities::ItemDefinition,
        actor: &mut engine::party::Actor,
    ) -> Option<String> {
        let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
        match item.effect.r#type.as_str() {
            "heal_hp" => {
                let amount = item.effect.power.unwrap_or(0).max(0);
                let inverted =
                    healing_inverted(content, &engine::party::actor_traits(content, actor));
                if inverted {
                    apply_damage_to_actor(actor, amount.max(1));
                    Some(format!("{} takes {} damage.", actor.name, amount.max(1)))
                } else {
                    let before = actor.current_hp;
                    actor.current_hp = (actor.current_hp + amount).clamp(0, max_hp);
                    let healed = actor.current_hp.saturating_sub(before);
                    Some(format!("{} recovers {} HP.", actor.name, healed))
                }
            }
            "revive" => {
                if actor.current_hp <= 0 {
                    let amount = item.effect.power.unwrap_or(0).max(1);
                    actor.current_hp = amount.clamp(1, max_hp);
                    Some(format!("{} is revived.", actor.name))
                } else {
                    None
                }
            }
            "heal_mp" => {
                let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
                let amount = item.effect.power.unwrap_or(0).max(0);
                let before = actor.current_mp;
                actor.current_mp = (actor.current_mp + amount).clamp(0, max_mp);
                let recovered = actor.current_mp.saturating_sub(before);
                Some(format!("{} recovers {} MP.", actor.name, recovered))
            }
            "cure_status" => {
                if item.effect.statuses.is_empty() {
                    return None;
                }
                let mut removed = Vec::new();
                actor.statuses.retain(|status| {
                    if item.effect.statuses.contains(&status.id) {
                        if let Some(definition) =
                            engine::battle::status_definition(content, &status.id)
                        {
                            removed.push(definition.label.clone());
                        } else {
                            removed.push(status.id.clone());
                        }
                        false
                    } else {
                        true
                    }
                });
                if removed.is_empty() {
                    None
                } else if removed.len() == 1 {
                    Some(format!("{} is cured of {}.", actor.name, removed[0]))
                } else {
                    Some(format!(
                        "{} is cured of {}.",
                        actor.name,
                        removed.join(", ")
                    ))
                }
            }
            _ => None,
        }
    }

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
        let message = if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
            apply_item_to_actor_battle(&runtime.content, item, actor)
        } else {
            None
        };
        if let Some(message) = message {
            super::logic::push_battle_log(&mut battle_state.log, message);
        } else {
            crate::menu::inventory::apply_item_to_actor(runtime, item, &target_id);
        }
        if !item.effect.effects.is_empty() {
            if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
                let applied = apply_status_effects(
                    &runtime.content,
                    &item.effect.effects,
                    &mut actor.statuses,
                    &mut rand::thread_rng(),
                );
                for label in applied {
                    super::logic::push_battle_log(
                        &mut battle_state.log,
                        format!("{} is affected by {}.", actor.name, label),
                    );
                }
            }
        }
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

fn apply_attenuation(value: i32, attenuation: f32) -> i32 {
    if value <= 0 {
        return 0;
    }
    ((value as f32) * attenuation).round().max(1.0) as i32
}

fn party_indices_for_effect(
    runtime: &GameRuntime,
    battle_state: &engine::battle::BattleState,
    effect_type: &str,
) -> Vec<usize> {
    battle_state
        .party_order
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let alive = runtime
                .party
                .roster
                .get(id)
                .map(|actor| actor.current_hp > 0)
                .unwrap_or(false);
            match effect_type {
                "revive" if !alive => Some(index),
                "revive" => None,
                _ if alive => Some(index),
                _ => None,
            }
        })
        .collect()
}
