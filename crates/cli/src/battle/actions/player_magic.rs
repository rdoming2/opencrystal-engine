use engine::battle::{
    actor_combat_stats, apply_damage_to_actor, apply_damage_to_enemy, apply_status_effects,
    damage_multiplier, enemy_combat_stats, healing_inverted, magic_heal_base, roll_attack,
    CombatantStats, DamageKind,
};
use engine::party::{activity_proficiency, actor_traits, apply_activity_gain, ActivityKind};
use engine::rules::{MagicSystem, ProgressionMode};
use engine::runtime::GameRuntime;
use rand::Rng;

use crate::battle::logic;
use crate::battle::state::{enemy_target_indices, TargetMode, TargetSide};
use crate::menu::common::SpellEntry;

use super::shared::{
    activity_damage_multiplier, activity_hit_bonus, activity_magic_id, apply_attenuation,
    growth_entry, party_indices_for_effect,
};

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
        rules: &engine::rules::BattleRules,
        entry: &SpellEntry,
        caster_stats: &CombatantStats,
        actor: &mut engine::party::Actor,
        attenuation: f32,
    ) -> Option<String> {
        let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
        match entry.effect_type.as_str() {
            "heal" => {
                let inverted = healing_inverted(content, &actor_traits(content, actor));
                let target_stats = actor_combat_stats(actor);
                let base_heal =
                    magic_heal_base(rules, caster_stats, &target_stats, entry.effect_power);
                let amount = apply_attenuation(base_heal.max(1), attenuation);
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
            logic::push_battle_log(&mut battle_state.log, reason);
            return;
        }
        if !crate::menu::magic::consume_spell_cost(magic_system.clone(), actor, entry) {
            let reason = match magic_system {
                MagicSystem::Mp => "Not enough MP.",
                MagicSystem::TierCharges => "No tier charges.",
            };
            logic::push_battle_log(&mut battle_state.log, reason);
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
                        logic::push_battle_log(
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
                                    logic::push_battle_log(
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
                            logic::push_battle_log(
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
                            if crit {
                                logic::push_critical_battle_log(runtime, &mut battle_state.log);
                            }
                        }
                        "heal" => {
                            let max_hp = enemy.max_hp();
                            let target_stats = enemy_combat_stats(&runtime.content, enemy);
                            let base_heal = magic_heal_base(
                                &runtime.content.rules.battle,
                                &actor_stats,
                                &target_stats,
                                entry.effect_power,
                            );
                            let amount = apply_attenuation(base_heal.max(1), attenuation);
                            if healing_inverted(&runtime.content, &enemy.traits) {
                                apply_damage_to_enemy(enemy, amount);
                                runtime.track_max_stat("max_damage", amount);
                                logic::push_battle_log(
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
                                logic::push_battle_log(
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
                            logic::push_battle_log(
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
                            &mut enemy.statuses,
                            rng,
                        );
                        for label in &applied {
                            logic::push_battle_log(
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
            let mut single_target_hp_delta: Option<i32> = None;
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
                    let before_hp = actor.current_hp;
                    if let Some(message) = apply_spell_to_actor_battle(
                        &runtime.content,
                        &runtime.content.rules.battle,
                        entry,
                        &actor_stats,
                        actor,
                        attenuation,
                    ) {
                        logic::push_battle_log(&mut battle_state.log, message);
                        if !applied_gain {
                            applied_gain = true;
                            gain_pending = true;
                        }
                    }
                    if target_mode == TargetMode::Single {
                        single_target_hp_delta = Some(actor.current_hp - before_hp);
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
                            logic::push_battle_log(
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
                logic::push_battle_log(
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
                logic::push_battle_log(
                    &mut battle_state.log,
                    match entry.effect_type.as_str() {
                        "heal" => {
                            if let Some(healed) = single_target_hp_delta.filter(|delta| *delta > 0)
                            {
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.cast",
                                    "{actor} casts {spell} on {target} for {damage} HP.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("spell", entry.name.clone()),
                                        ("target", target_name),
                                        ("damage", healed.to_string()),
                                    ],
                                )
                            } else {
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.cast_simple",
                                    "{actor} casts {spell} on {target}.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("spell", entry.name.clone()),
                                        ("target", target_name),
                                    ],
                                )
                            }
                        }
                        "damage" => {
                            if let Some(damage) = single_target_hp_delta
                                .filter(|delta| *delta < 0)
                                .map(i32::abs)
                            {
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.cast",
                                    "{actor} casts {spell} on {target} for {damage} HP.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("spell", entry.name.clone()),
                                        ("target", target_name),
                                        ("damage", damage.to_string()),
                                    ],
                                )
                            } else {
                                crate::battle::format_ui_text(
                                    runtime,
                                    "battle.log.cast_simple",
                                    "{actor} casts {spell} on {target}.",
                                    &[
                                        ("actor", actor_name.clone()),
                                        ("spell", entry.name.clone()),
                                        ("target", target_name),
                                    ],
                                )
                            }
                        }
                        _ => crate::battle::format_ui_text(
                            runtime,
                            "battle.log.cast_simple",
                            "{actor} casts {spell} on {target}.",
                            &[
                                ("actor", actor_name.clone()),
                                ("spell", entry.name.clone()),
                                ("target", target_name),
                            ],
                        ),
                    },
                );
            }
        }
    }
}
