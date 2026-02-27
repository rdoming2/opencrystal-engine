use engine::battle::{
    actor_combat_stats, apply_damage_to_actor, apply_damage_to_enemy, apply_status_effects,
    damage_multiplier, enemy_combat_stats, healing_inverted, roll_attack, CombatantStats,
    DamageKind,
};
use engine::party::{
    activity_proficiency, actor_weapon_category, apply_activity_gain, row_attack_multiplier,
    ActivityKind,
};
use engine::rules::{MagicSystem, ProgressionMode};
use engine::runtime::GameRuntime;
use rand::{Rng, RngExt};
use std::collections::HashMap;

use super::state::{
    enemy_target_indices, BattleMenuState, PendingChargeAction, TargetMode, TargetSide,
};
use crate::menu::common::{AbilityEntry, SpellEntry};

fn growth_entry<'a>(
    growth: &'a mut HashMap<String, engine::battle::BattleGrowthAccumulator>,
    actor_id: &str,
) -> &'a mut engine::battle::BattleGrowthAccumulator {
    growth.entry(actor_id.to_string()).or_default()
}

fn activity_weapon_id(runtime: &GameRuntime, actor: &engine::party::Actor) -> Option<String> {
    let rules = &runtime.content.rules.activity_progression;
    actor_weapon_category(&runtime.content, actor, rules.unarmed_category.as_str())
}

fn activity_magic_id(runtime: &GameRuntime, spell_id: &str) -> Option<String> {
    runtime
        .content
        .spells
        .spells
        .iter()
        .find(|spell| spell.id == spell_id)
        .map(|spell| spell.school.clone())
}

fn activity_damage_multiplier(runtime: &GameRuntime, prof: f32) -> f32 {
    1.0 + prof
        * runtime
            .content
            .rules
            .activity_progression
            .effects
            .damage_scale
}

fn activity_hit_bonus(runtime: &GameRuntime, prof: f32) -> f32 {
    prof * runtime.content.rules.activity_progression.effects.hit_bonus
}

pub fn execute_attack_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    enemy_index: usize,
    rng: &mut impl Rng,
) {
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return;
    };
    let actor_name = actor.name.clone();
    if runtime.content.rules.progression_mode == ProgressionMode::Activity {
        growth_entry(&mut battle_state.growth, actor_id).turns_acted += 1.0;
    }
    let weapon_id = if runtime.content.rules.progression_mode == ProgressionMode::Activity {
        activity_weapon_id(runtime, actor)
    } else {
        None
    };
    let (activity_hit_bonus, activity_damage_multiplier) = if let Some(ref weapon_id) = weapon_id {
        let prof = activity_proficiency(actor, ActivityKind::Weapon, weapon_id);
        (
            activity_hit_bonus(runtime, prof),
            activity_damage_multiplier(runtime, prof),
        )
    } else {
        (0.0, 1.0)
    };
    let (enemy_name, damage, crit) = {
        let Some(enemy) = battle_state.enemies.get_mut(enemy_index) else {
            return;
        };
        if !enemy.is_alive() {
            super::logic::push_battle_log(
                &mut battle_state.log,
                crate::battle::ui_text(runtime, "battle.no_target", "No target."),
            );
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
            activity_hit_bonus,
            rng,
        );
        if !roll.hit {
            super::logic::push_battle_log(
                &mut battle_state.log,
                crate::battle::format_ui_text(
                    runtime,
                    "battle.log.miss",
                    "{actor} misses {target}.",
                    &[
                        ("actor", actor_name.clone()),
                        ("target", enemy.name.clone()),
                    ],
                ),
            );
            return;
        }
        let mut damage = roll.base_damage;
        let row_multiplier = row_attack_multiplier(&runtime.content, actor);
        damage = ((damage as f32) * row_multiplier * activity_damage_multiplier)
            .round()
            .max(0.0) as i32;
        let multiplier = damage_multiplier(
            &runtime.content,
            &enemy.statuses,
            &enemy.traits,
            DamageKind::Physical,
            None,
        );
        damage = ((damage as f32) * multiplier).round().max(0.0) as i32;
        apply_damage_to_enemy(enemy, damage);
        (enemy.name.clone(), damage, roll.crit)
    };
    if runtime.content.rules.progression_mode == ProgressionMode::Activity {
        let growth = growth_entry(&mut battle_state.growth, actor_id);
        growth.damage_dealt_physical += damage.max(0) as f32;
        if crit {
            growth.crits += 1.0;
        }
    }
    if let Some(weapon_id) = weapon_id.as_deref() {
        if let Some(actor) = runtime.party.roster.get_mut(actor_id) {
            apply_activity_gain(
                actor,
                ActivityKind::Weapon,
                weapon_id,
                runtime
                    .content
                    .rules
                    .activity_progression
                    .weapon_gain
                    .attack,
            );
        }
    }
    runtime.track_max_stat("max_damage", damage);
    if crit {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.log.critical", "Critical hit!"),
        );
    }
    super::logic::push_battle_log(
        &mut battle_state.log,
        crate::battle::format_ui_text(
            runtime,
            "battle.log.attack",
            "{actor} attacks {target} for {damage} HP.",
            &[
                ("actor", actor_name),
                ("target", enemy_name),
                ("damage", damage.to_string()),
            ],
        ),
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
    let (effect_ids, element) = {
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
        let element = spell_definition
            .and_then(|spell| spell.effect.element.as_deref())
            .map(|element| element.to_string());
        (effect_ids, element)
    };
    let actor_name = {
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
        actor.name.clone()
    };
    if runtime.content.rules.progression_mode == ProgressionMode::Activity {
        let growth = growth_entry(&mut battle_state.growth, actor_id);
        growth.turns_acted += 1.0;
        if entry.cost_type == "mp" {
            growth.mp_spent += entry.cost_value.max(0) as f32;
        }
    }
    let magic_id = if runtime.content.rules.progression_mode == ProgressionMode::Activity {
        activity_magic_id(runtime, entry.id.as_str())
    } else {
        None
    };
    let (activity_hit_bonus, activity_damage_multiplier) = if let Some(ref magic_id) = magic_id {
        let prof = runtime
            .party
            .roster
            .get(actor_id)
            .map(|actor| activity_proficiency(actor, ActivityKind::Magic, magic_id))
            .unwrap_or(0.0);
        (
            activity_hit_bonus(runtime, prof),
            activity_damage_multiplier(runtime, prof),
        )
    } else {
        (0.0, 1.0)
    };
    let Some(actor_stats) = runtime.party.roster.get(actor_id).map(actor_combat_stats) else {
        return;
    };

    let attenuation = match target_mode {
        TargetMode::Multi => entry.multi_attenuation.unwrap_or(1.0),
        TargetMode::Single => 1.0,
    };

    let mut applied_gain = false;
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
                        super::logic::push_battle_log(
                            &mut battle_state.log,
                            crate::battle::ui_text(runtime, "battle.no_target", "No target."),
                        );
                        continue;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let (enemy_name, damage, crit) = {
                                let attacker_stats = actor_stats.clone();
                                let defender_stats = enemy_combat_stats(&runtime.content, enemy);
                                let roll = roll_attack(
                                    &runtime.content,
                                    &runtime.content.rules.battle,
                                    &attacker_stats,
                                    &defender_stats,
                                    DamageKind::Magic,
                                    entry.effect_power,
                                    activity_hit_bonus,
                                    rng,
                                );
                                if !roll.hit {
                                    super::logic::push_battle_log(
                                        &mut battle_state.log,
                                        crate::battle::format_ui_text(
                                            runtime,
                                            "battle.log.miss",
                                            "{actor} misses {target}.",
                                            &[
                                                ("actor", actor_name.clone()),
                                                ("target", enemy.name.clone()),
                                            ],
                                        ),
                                    );
                                    continue;
                                }
                                let mut damage = roll.base_damage;
                                let multiplier = damage_multiplier(
                                    &runtime.content,
                                    &enemy.statuses,
                                    &enemy.traits,
                                    DamageKind::Magic,
                                    element.as_deref(),
                                );
                                damage = ((damage as f32)
                                    * multiplier
                                    * attenuation
                                    * activity_damage_multiplier)
                                    .round()
                                    .max(1.0) as i32;
                                apply_damage_to_enemy(enemy, damage);
                                (enemy.name.clone(), damage, roll.crit)
                            };
                            runtime.track_max_stat("max_damage", damage);
                            if runtime.content.rules.progression_mode == ProgressionMode::Activity {
                                let growth = growth_entry(&mut battle_state.growth, actor_id);
                                growth.damage_dealt_magic += damage.max(0) as f32;
                                if crit {
                                    growth.crits += 1.0;
                                }
                            }
                            if !applied_gain {
                                if let Some(magic_id) = magic_id.as_deref() {
                                    if let Some(actor) = runtime.party.roster.get_mut(actor_id) {
                                        apply_activity_gain(
                                            actor,
                                            ActivityKind::Magic,
                                            magic_id,
                                            runtime
                                                .content
                                                .rules
                                                .activity_progression
                                                .magic_gain
                                                .cast,
                                        );
                                        applied_gain = true;
                                    }
                                }
                            }
                            if crit {
                                super::logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::ui_text(
                                        runtime,
                                        "battle.log.critical",
                                        "Critical hit!",
                                    ),
                                );
                            }
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.cast",
                                    "{actor} casts {spell} on {target} for {damage} HP.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("spell", entry.name.clone()),
                                        ("target", enemy_name),
                                        ("damage", damage.to_string()),
                                    ],
                                ),
                            );
                        }
                        "heal" => {
                            let max_hp = enemy.max_hp();
                            let amount = apply_attenuation(entry.effect_power.max(1), attenuation);
                            if healing_inverted(&runtime.content, &enemy.traits) {
                                apply_damage_to_enemy(enemy, amount);
                                runtime.track_max_stat("max_damage", amount);
                                super::logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.healing_damage",
                                        "{target} is harmed by healing for {damage} HP.",
                                        &[
                                            ("target", enemy.name.clone()),
                                            ("damage", amount.to_string()),
                                        ],
                                    ),
                                );
                            } else {
                                let before = enemy.current_hp;
                                enemy.current_hp = (enemy.current_hp + amount).clamp(0, max_hp);
                                let healed = enemy.current_hp.saturating_sub(before);
                                super::logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::format_ui_text(
                                        runtime,
                                        "battle.log.heal",
                                        "{target} recovers {amount} HP.",
                                        &[
                                            ("target", enemy.name.clone()),
                                            ("amount", healed.to_string()),
                                        ],
                                    ),
                                );
                            }
                            if !applied_gain {
                                if let Some(magic_id) = magic_id.as_deref() {
                                    if let Some(actor) = runtime.party.roster.get_mut(actor_id) {
                                        apply_activity_gain(
                                            actor,
                                            ActivityKind::Magic,
                                            magic_id,
                                            runtime
                                                .content
                                                .rules
                                                .activity_progression
                                                .magic_gain
                                                .cast,
                                        );
                                        applied_gain = true;
                                    }
                                }
                            }
                        }
                        "scan" => {
                            enemy.scanned = true;
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.scan",
                                    "{actor} scans {target}: {current}/{max} HP.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("target", enemy.name.clone()),
                                        ("current", enemy.current_hp.max(0).to_string()),
                                        ("max", enemy.max_hp().max(1).to_string()),
                                    ],
                                ),
                            );
                            if !applied_gain {
                                if let Some(magic_id) = magic_id.as_deref() {
                                    if let Some(actor) = runtime.party.roster.get_mut(actor_id) {
                                        apply_activity_gain(
                                            actor,
                                            ActivityKind::Magic,
                                            magic_id,
                                            runtime
                                                .content
                                                .rules
                                                .activity_progression
                                                .magic_gain
                                                .cast,
                                        );
                                        applied_gain = true;
                                    }
                                }
                            }
                        }
                        "status" => {}
                        _ => {
                            super::logic::push_battle_log(
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
                            &mut enemy.statuses,
                            rng,
                        );
                        for label in &applied {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.status",
                                    "{target} is affected by {status}.",
                                    &[("target", enemy.name.clone()), ("status", label.clone())],
                                ),
                            );
                        }
                        if runtime.content.rules.progression_mode == ProgressionMode::Activity
                            && !applied.is_empty()
                        {
                            growth_entry(&mut battle_state.growth, actor_id).status_inflicted +=
                                applied.len() as f32;
                        }
                        if !applied_gain {
                            if let Some(magic_id) = magic_id.as_deref() {
                                if let Some(actor) = runtime.party.roster.get_mut(actor_id) {
                                    apply_activity_gain(
                                        actor,
                                        ActivityKind::Magic,
                                        magic_id,
                                        runtime.content.rules.activity_progression.magic_gain.cast,
                                    );
                                    applied_gain = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        TargetSide::Party => {
            let mut gain_pending = false;
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
                        if !applied_gain {
                            applied_gain = true;
                            gain_pending = true;
                        }
                    }
                    if !effect_ids.is_empty() {
                        let (actor_name, applied) = {
                            let applied = apply_status_effects(
                                &runtime.content,
                                &effect_ids,
                                &mut actor.statuses,
                                rng,
                            );
                            (actor.name.clone(), applied)
                        };
                        for label in &applied {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.status",
                                    "{target} is affected by {status}.",
                                    &[("target", actor_name.clone()), ("status", label.clone())],
                                ),
                            );
                        }
                        if runtime.content.rules.progression_mode == ProgressionMode::Activity
                            && !applied.is_empty()
                        {
                            growth_entry(&mut battle_state.growth, actor_id).status_inflicted +=
                                applied.len() as f32;
                        }
                        if !applied_gain {
                            applied_gain = true;
                            gain_pending = true;
                        }
                    }
                }
            }
            if gain_pending {
                if let Some(magic_id) = magic_id.as_deref() {
                    if let Some(caster) = runtime.party.roster.get_mut(actor_id) {
                        apply_activity_gain(
                            caster,
                            ActivityKind::Magic,
                            magic_id,
                            runtime.content.rules.activity_progression.magic_gain.cast,
                        );
                    }
                }
            }
            if target_mode == TargetMode::Multi {
                super::logic::push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.cast_party",
                        "{actor} casts {spell} on the party.",
                        &[("actor", actor_name.clone()), ("spell", entry.name.clone())],
                    ),
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
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.cast",
                        "{actor} casts {spell} on {target} for {damage} HP.",
                        &[
                            ("actor", actor_name.clone()),
                            ("spell", entry.name.clone()),
                            ("target", target_name),
                            ("damage", "0".to_string()),
                        ],
                    ),
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
    menu_state: &mut BattleMenuState,
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
    let actor_name = {
        let Some(actor) = runtime.party.roster.get(actor_id) else {
            return;
        };
        let (usable, reason) = crate::menu::abilities::ability_cost_available(
            runtime,
            actor,
            &entry.cost_type,
            entry.cost_value,
            entry.cost_item_id.as_deref(),
            entry.cost_currency_id.as_deref(),
        );
        if !usable {
            super::logic::push_battle_log(
                &mut battle_state.log,
                reason.unwrap_or_else(|| {
                    crate::battle::ui_text(
                        runtime,
                        "battle.ability_unavailable",
                        "Cannot use ability.",
                    )
                }),
            );
            return;
        }
        actor.name.clone()
    };
    if runtime.content.rules.progression_mode == ProgressionMode::Activity {
        growth_entry(&mut battle_state.growth, actor_id).turns_acted += 1.0;
    }
    let weapon_id = if runtime.content.rules.progression_mode == ProgressionMode::Activity {
        runtime
            .party
            .roster
            .get(actor_id)
            .and_then(|actor| activity_weapon_id(runtime, actor))
    } else {
        None
    };
    let (activity_hit_bonus, activity_damage_multiplier) = if let Some(ref weapon_id) = weapon_id {
        let prof = runtime
            .party
            .roster
            .get(actor_id)
            .map(|actor| activity_proficiency(actor, ActivityKind::Weapon, weapon_id))
            .unwrap_or(0.0);
        (
            activity_hit_bonus(runtime, prof),
            activity_damage_multiplier(runtime, prof),
        )
    } else {
        (0.0, 1.0)
    };
    let Some(actor_stats) = runtime.party.roster.get(actor_id).map(actor_combat_stats) else {
        return;
    };
    if !crate::menu::abilities::consume_ability_cost(runtime, entry, actor_id) {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.cost_failed", "Failed to pay cost."),
        );
        return;
    }

    if entry.effect_type == "charge" {
        let windup_turns = entry.windup_turns.max(1);
        menu_state.pending_charge.insert(
            actor_id.to_string(),
            PendingChargeAction {
                entry: entry.clone(),
                target_index,
                target_side,
                turns_remaining: windup_turns,
            },
        );
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::format_ui_text(
                runtime,
                "battle.log.charge_start",
                "{actor} begins charging {ability}.",
                &[
                    ("actor", actor_name.clone()),
                    ("ability", entry.name.clone()),
                ],
            ),
        );
        if entry.vanish_during_windup {
            super::logic::push_battle_log(
                &mut battle_state.log,
                crate::battle::format_ui_text(
                    runtime,
                    "battle.log.charge_vanish",
                    "{actor} disappears from sight!",
                    &[("actor", actor_name)],
                ),
            );
        }
        return;
    }

    let attenuation = match target_mode {
        TargetMode::Multi => entry.multi_attenuation.unwrap_or(1.0),
        TargetMode::Single => 1.0,
    };

    let mut applied_gain = false;
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
                        super::logic::push_battle_log(
                            &mut battle_state.log,
                            crate::battle::ui_text(runtime, "battle.no_target", "No target."),
                        );
                        continue;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let (enemy_name, damage, crit) = {
                                let attacker_stats = actor_stats.clone();
                                let defender_stats = enemy_combat_stats(&runtime.content, enemy);
                                let roll = roll_attack(
                                    &runtime.content,
                                    &runtime.content.rules.battle,
                                    &attacker_stats,
                                    &defender_stats,
                                    DamageKind::Physical,
                                    entry.effect_power,
                                    activity_hit_bonus,
                                    rng,
                                );
                                if !roll.hit {
                                    super::logic::push_battle_log(
                                        &mut battle_state.log,
                                        crate::battle::format_ui_text(
                                            runtime,
                                            "battle.log.miss",
                                            "{actor} misses {target}.",
                                            &[
                                                ("actor", actor_name.clone()),
                                                ("target", enemy.name.clone()),
                                            ],
                                        ),
                                    );
                                    continue;
                                }
                                let mut damage = roll.base_damage;
                                let multiplier = damage_multiplier(
                                    &runtime.content,
                                    &enemy.statuses,
                                    &enemy.traits,
                                    DamageKind::Physical,
                                    None,
                                );
                                damage = ((damage as f32)
                                    * multiplier
                                    * attenuation
                                    * activity_damage_multiplier)
                                    .round()
                                    .max(1.0) as i32;
                                apply_damage_to_enemy(enemy, damage);
                                (enemy.name.clone(), damage, roll.crit)
                            };
                            runtime.track_max_stat("max_damage", damage);
                            if runtime.content.rules.progression_mode == ProgressionMode::Activity {
                                let growth = growth_entry(&mut battle_state.growth, actor_id);
                                growth.damage_dealt_physical += damage.max(0) as f32;
                                if crit {
                                    growth.crits += 1.0;
                                }
                            }
                            if !applied_gain {
                                if let Some(weapon_id) = weapon_id.as_deref() {
                                    if let Some(actor) = runtime.party.roster.get_mut(actor_id) {
                                        apply_activity_gain(
                                            actor,
                                            ActivityKind::Weapon,
                                            weapon_id,
                                            runtime
                                                .content
                                                .rules
                                                .activity_progression
                                                .weapon_gain
                                                .ability,
                                        );
                                        applied_gain = true;
                                    }
                                }
                            }
                            if crit {
                                super::logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::ui_text(
                                        runtime,
                                        "battle.log.critical",
                                        "Critical hit!",
                                    ),
                                );
                            }
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.ability",
                                    "{actor} uses {ability} on {target} for {damage} HP.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("ability", entry.name.clone()),
                                        ("target", enemy_name),
                                        ("damage", damage.to_string()),
                                    ],
                                ),
                            );
                        }
                        "scan" => {
                            enemy.scanned = true;
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.scan",
                                    "{actor} scans {target}: {current}/{max} HP.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("target", enemy.name.clone()),
                                        ("current", enemy.current_hp.max(0).to_string()),
                                        ("max", enemy.max_hp().max(1).to_string()),
                                    ],
                                ),
                            );
                        }
                        "steal" => {
                            let chance = if entry.effect_power > 0 {
                                (entry.effect_power as f32) / 100.0
                            } else {
                                0.5
                            };
                            let stolen = try_steal_item(runtime, enemy, chance, rng);
                            match stolen {
                                Some(item_name) => {
                                    super::logic::push_battle_log(
                                        &mut battle_state.log,
                                        crate::battle::format_ui_text(
                                            runtime,
                                            "battle.log.steal_success",
                                            "{actor} steals {item}.",
                                            &[("actor", actor_name.clone()), ("item", item_name)],
                                        ),
                                    );
                                }
                                None => {
                                    super::logic::push_battle_log(
                                        &mut battle_state.log,
                                        crate::battle::format_ui_text(
                                            runtime,
                                            "battle.log.steal_fail",
                                            "{actor} fails to steal anything.",
                                            &[("actor", actor_name.clone())],
                                        ),
                                    );
                                }
                            }
                        }
                        "throw" => {
                            let item_name = entry
                                .cost_item_id
                                .as_deref()
                                .and_then(|item_id| {
                                    runtime
                                        .content
                                        .items
                                        .items
                                        .iter()
                                        .find(|item| item.id == item_id)
                                        .map(|item| item.name.clone())
                                })
                                .unwrap_or_else(|| "Item".to_string());
                            let (enemy_name, damage, crit) = {
                                let attacker_stats = actor_stats.clone();
                                let defender_stats = enemy_combat_stats(&runtime.content, enemy);
                                let roll = roll_attack(
                                    &runtime.content,
                                    &runtime.content.rules.battle,
                                    &attacker_stats,
                                    &defender_stats,
                                    DamageKind::Physical,
                                    entry.effect_power,
                                    activity_hit_bonus,
                                    rng,
                                );
                                if !roll.hit {
                                    super::logic::push_battle_log(
                                        &mut battle_state.log,
                                        crate::battle::format_ui_text(
                                            runtime,
                                            "battle.log.miss",
                                            "{actor} misses {target}.",
                                            &[
                                                ("actor", actor_name.clone()),
                                                ("target", enemy.name.clone()),
                                            ],
                                        ),
                                    );
                                    continue;
                                }
                                let mut damage = roll.base_damage;
                                let multiplier = damage_multiplier(
                                    &runtime.content,
                                    &enemy.statuses,
                                    &enemy.traits,
                                    DamageKind::Physical,
                                    None,
                                );
                                damage = ((damage as f32) * multiplier * activity_damage_multiplier)
                                    .round()
                                    .max(1.0) as i32;
                                apply_damage_to_enemy(enemy, damage);
                                (enemy.name.clone(), damage, roll.crit)
                            };
                            runtime.track_max_stat("max_damage", damage);
                            if runtime.content.rules.progression_mode == ProgressionMode::Activity {
                                let growth = growth_entry(&mut battle_state.growth, actor_id);
                                growth.damage_dealt_physical += damage.max(0) as f32;
                                if crit {
                                    growth.crits += 1.0;
                                }
                            }
                            if !applied_gain {
                                if let Some(weapon_id) = weapon_id.as_deref() {
                                    if let Some(actor) = runtime.party.roster.get_mut(actor_id) {
                                        apply_activity_gain(
                                            actor,
                                            ActivityKind::Weapon,
                                            weapon_id,
                                            runtime
                                                .content
                                                .rules
                                                .activity_progression
                                                .weapon_gain
                                                .ability,
                                        );
                                        applied_gain = true;
                                    }
                                }
                            }
                            if crit {
                                super::logic::push_battle_log(
                                    &mut battle_state.log,
                                    crate::battle::ui_text(
                                        runtime,
                                        "battle.log.critical",
                                        "Critical hit!",
                                    ),
                                );
                            }
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.throw",
                                    "{actor} throws {item} at {target} for {damage} HP.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("item", item_name),
                                        ("target", enemy_name),
                                        ("damage", damage.to_string()),
                                    ],
                                ),
                            );
                        }
                        "status" => {}
                        _ => {
                            super::logic::push_battle_log(
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
                            &mut enemy.statuses,
                            rng,
                        );
                        for label in &applied {
                            super::logic::push_battle_log(
                                &mut battle_state.log,
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.status",
                                    "{target} is affected by {status}.",
                                    &[("target", enemy.name.clone()), ("status", label.clone())],
                                ),
                            );
                        }
                        if runtime.content.rules.progression_mode == ProgressionMode::Activity
                            && !applied.is_empty()
                        {
                            growth_entry(&mut battle_state.growth, actor_id).status_inflicted +=
                                applied.len() as f32;
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
                let target_name = runtime
                    .party
                    .roster
                    .get(&target_id)
                    .map(|entry| entry.name.clone())
                    .unwrap_or_else(|| target_id.clone());
                let mut pending_logs: Vec<(&str, &str, Vec<(&str, String)>)> = Vec::new();
                let mut status_log: Option<(String, Vec<String>)> = None;
                let mut apply_field_ability = false;
                if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
                    match entry.effect_type.as_str() {
                        "parry" => {
                            menu_state.parrying.insert(target_id.clone());
                            pending_logs.push((
                                "battle.log.parry_ready",
                                "{actor} readies a parry.",
                                vec![("actor", actor.name.clone())],
                            ));
                        }
                        "counter" => {
                            menu_state.countering.insert(target_id.clone());
                            pending_logs.push((
                                "battle.log.counter_ready",
                                "{actor} readies a counter.",
                                vec![("actor", actor.name.clone())],
                            ));
                        }
                        "pray" => {}
                        "cover" => {
                            menu_state.covering.retain(|_, coverer| coverer != actor_id);
                            menu_state
                                .covering
                                .insert(target_id.clone(), actor_id.to_string());
                            pending_logs.push((
                                "battle.log.cover_ready",
                                "{actor} will cover {target}.",
                                vec![
                                    ("actor", actor.name.clone()),
                                    ("target", target_name.clone()),
                                ],
                            ));
                        }
                        _ => {
                            if let Some(message) = apply_ability_to_actor_battle(
                                &runtime.content,
                                entry,
                                actor,
                                attenuation,
                            ) {
                                super::logic::push_battle_log(&mut battle_state.log, message);
                            } else {
                                apply_field_ability = true;
                            }
                        }
                    }
                    if !effect_ids.is_empty() {
                        let applied = apply_status_effects(
                            &runtime.content,
                            &effect_ids,
                            &mut actor.statuses,
                            rng,
                        );
                        status_log = Some((actor.name.clone(), applied));
                    }
                }
                if apply_field_ability {
                    crate::menu::abilities::apply_ability_to_actor(runtime, entry, &target_id);
                }
                for (key, default, vars) in pending_logs {
                    super::logic::push_battle_log(
                        &mut battle_state.log,
                        crate::battle::format_ui_text(runtime, key, default, &vars),
                    );
                }
                if let Some((actor_name, applied)) = status_log {
                    for label in &applied {
                        super::logic::push_battle_log(
                            &mut battle_state.log,
                            crate::battle::format_ui_text(
                                runtime,
                                "battle.log.status",
                                "{target} is affected by {status}.",
                                &[("target", actor_name.clone()), ("status", label.clone())],
                            ),
                        );
                    }
                    if runtime.content.rules.progression_mode == ProgressionMode::Activity
                        && !applied.is_empty()
                    {
                        growth_entry(&mut battle_state.growth, actor_id).status_inflicted +=
                            applied.len() as f32;
                    }
                }
            }
            if target_mode == TargetMode::Multi {
                super::logic::push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.ability_party",
                        "{actor} uses {ability} on the party.",
                        &[
                            ("actor", actor_name.clone()),
                            ("ability", entry.name.clone()),
                        ],
                    ),
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
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.ability",
                        "{actor} uses {ability} on {target} for {damage} HP.",
                        &[
                            ("actor", actor_name.clone()),
                            ("ability", entry.name.clone()),
                            ("target", target_name),
                            ("damage", "0".to_string()),
                        ],
                    ),
                );
            }
            if entry.effect_type == "pray" {
                let amount = apply_attenuation(entry.effect_power.max(1), attenuation);
                for member_id in battle_state.party_order.iter() {
                    if let Some(member) = runtime.party.roster.get_mut(member_id) {
                        if member.current_hp > 0 {
                            let max_hp = member.derived_stats.get("hp").copied().unwrap_or(0);
                            member.current_hp = (member.current_hp + amount).clamp(0, max_hp);
                        }
                    }
                }
                super::logic::push_battle_log(
                    &mut battle_state.log,
                    crate::battle::format_ui_text(
                        runtime,
                        "battle.log.pray",
                        "{actor} prays and the party recovers {amount} HP.",
                        &[
                            ("actor", actor_name.clone()),
                            ("amount", amount.to_string()),
                        ],
                    ),
                );
            }
        }
    }
}

pub fn resolve_pending_charge_action(
    runtime: &mut GameRuntime,
    battle_state: &mut engine::battle::BattleState,
    actor_id: &str,
    menu_state: &mut BattleMenuState,
    rng: &mut impl Rng,
) -> Option<usize> {
    let Some(mut pending) = menu_state.pending_charge.remove(actor_id) else {
        return None;
    };
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return None;
    };
    let actor_name = actor.name.clone();
    if pending.turns_remaining > 1 {
        pending.turns_remaining -= 1;
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::format_ui_text(
                runtime,
                "battle.log.charge_hold",
                "{actor} keeps charging {ability}.",
                &[
                    ("actor", actor_name),
                    ("ability", pending.entry.name.clone()),
                ],
            ),
        );
        menu_state
            .pending_charge
            .insert(actor_id.to_string(), pending);
        return None;
    }

    if pending.entry.vanish_during_windup {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::format_ui_text(
                runtime,
                "battle.log.charge_return",
                "{actor} reappears!",
                &[("actor", actor_name.clone())],
            ),
        );
    }

    if pending.target_side != TargetSide::Enemy {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.nothing_happens", "Nothing happens."),
        );
        return None;
    }

    let target_index = pending
        .target_index
        .filter(|index| {
            battle_state
                .enemies
                .get(*index)
                .map(|enemy| enemy.is_alive())
                .unwrap_or(false)
        })
        .or_else(|| enemy_target_indices(battle_state).first().copied());
    let Some(target_index) = target_index else {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.no_target", "No target."),
        );
        return None;
    };

    let Some(actor_stats) = runtime.party.roster.get(actor_id).map(actor_combat_stats) else {
        return None;
    };
    let effect_ids = runtime
        .content
        .abilities
        .abilities
        .iter()
        .find(|ability| ability.id == pending.entry.id)
        .map(|ability| ability.effect.effects.clone())
        .unwrap_or_default();

    let Some(enemy) = battle_state.enemies.get_mut(target_index) else {
        return None;
    };
    if !enemy.is_alive() {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.no_target", "No target."),
        );
        return None;
    }

    let defender_stats = enemy_combat_stats(&runtime.content, enemy);
    let roll = roll_attack(
        &runtime.content,
        &runtime.content.rules.battle,
        &actor_stats,
        &defender_stats,
        DamageKind::Physical,
        pending.entry.effect_power,
        0.0,
        rng,
    );
    if !roll.hit {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::format_ui_text(
                runtime,
                "battle.log.miss",
                "{actor} misses {target}.",
                &[("actor", actor_name), ("target", enemy.name.clone())],
            ),
        );
        return Some(target_index);
    }

    let mut damage = roll.base_damage;
    let multiplier = damage_multiplier(
        &runtime.content,
        &enemy.statuses,
        &enemy.traits,
        DamageKind::Physical,
        None,
    );
    damage = ((damage as f32) * multiplier).round().max(1.0) as i32;
    apply_damage_to_enemy(enemy, damage);
    runtime.track_max_stat("max_damage", damage);

    if !effect_ids.is_empty() {
        let applied = apply_status_effects(&runtime.content, &effect_ids, &mut enemy.statuses, rng);
        for label in &applied {
            super::logic::push_battle_log(
                &mut battle_state.log,
                crate::battle::format_ui_text(
                    runtime,
                    "battle.log.status",
                    "{target} is affected by {status}.",
                    &[("target", enemy.name.clone()), ("status", label.clone())],
                ),
            );
        }
    }

    if roll.crit {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.log.critical", "Critical hit!"),
        );
    }
    super::logic::push_battle_log(
        &mut battle_state.log,
        crate::battle::format_ui_text(
            runtime,
            "battle.log.charge_release",
            "{actor} unleashes {ability} on {target} for {damage} HP.",
            &[
                ("actor", actor_name),
                ("ability", pending.entry.name),
                ("target", enemy.name.clone()),
                ("damage", damage.to_string()),
            ],
        ),
    );
    Some(target_index)
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
            "damage" => {
                let amount = item.effect.power.unwrap_or(0).max(1);
                apply_damage_to_actor(actor, amount);
                Some(format!("{} takes {} damage.", actor.name, amount))
            }
            _ => None,
        }
    }

    fn apply_item_to_enemy_battle(
        content: &engine::content::Content,
        item: &engine::entities::ItemDefinition,
        enemy: &mut engine::battle::BattleEnemy,
        rng: &mut impl Rng,
    ) -> Option<String> {
        match item.effect.r#type.as_str() {
            "damage" => {
                let attacker_stats = CombatantStats {
                    atk: 0,
                    def: 0,
                    matk: 0,
                    mdef: 0,
                    agi: 0,
                    lck: 0,
                    eva: 0,
                    lvl: 1,
                };
                let defender_stats = enemy_combat_stats(content, enemy);
                let roll = roll_attack(
                    content,
                    &content.rules.battle,
                    &attacker_stats,
                    &defender_stats,
                    DamageKind::Physical,
                    item.effect.power.unwrap_or(0),
                    0.0,
                    rng,
                );
                if !roll.hit {
                    return None;
                }
                let mut damage = roll.base_damage;
                let multiplier = damage_multiplier(
                    content,
                    &enemy.statuses,
                    &enemy.traits,
                    DamageKind::Physical,
                    None,
                );
                damage = ((damage as f32) * multiplier).round().max(1.0) as i32;
                apply_damage_to_enemy(enemy, damage);
                Some(format!("{} takes {} damage.", enemy.name, damage))
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
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.item_unusable", "Item unusable."),
        );
        return;
    }
    if !runtime.inventory.remove_item(&item.id, 1) {
        super::logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.item_none_left", "No items left."),
        );
        return;
    }
    if item.usage.target == "enemy" {
        if let Some(enemy_index) = target_index {
            if let Some(enemy) = battle_state.enemies.get_mut(enemy_index) {
                if let Some(message) =
                    apply_item_to_enemy_battle(&runtime.content, item, enemy, &mut rand::rng())
                {
                    super::logic::push_battle_log(&mut battle_state.log, message);
                }
            }
        }
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
                    &mut rand::rng(),
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

fn try_steal_item(
    runtime: &mut GameRuntime,
    enemy: &engine::battle::BattleEnemy,
    chance: f32,
    rng: &mut impl Rng,
) -> Option<String> {
    if chance <= 0.0 || rng.random::<f32>() > chance.min(1.0) {
        return None;
    }
    for loot in &enemy.loot {
        if loot.chance <= 0.0 {
            continue;
        }
        if rng.random::<f32>() <= loot.chance {
            let max_stack = runtime.content.rules.inventory.max_stack;
            if let Some(item) = runtime
                .content
                .items
                .items
                .iter()
                .find(|item| item.id == loot.item)
            {
                runtime.inventory.add_item(&item.id, 1, max_stack);
                return Some(item.name.clone());
            }
            if let Some(item) = runtime
                .content
                .equipment
                .equipment
                .iter()
                .find(|item| item.id == loot.item)
            {
                runtime.inventory.add_equipment(&item.id, 1, max_stack);
                return Some(item.name.clone());
            }
        }
    }
    None
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
