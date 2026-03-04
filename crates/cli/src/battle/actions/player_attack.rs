use engine::battle::{
    actor_combat_stats, apply_damage_to_enemy, damage_multiplier, enemy_combat_stats, roll_attack,
    DamageKind,
};
use engine::party::{
    activity_proficiency, apply_activity_gain, row_attack_multiplier, ActivityKind,
};
use engine::rules::ProgressionMode;
use engine::runtime::GameRuntime;
use rand::Rng;

use crate::battle::logic;

use super::shared::{
    activity_damage_multiplier, activity_hit_bonus, activity_weapon_id, growth_entry,
};

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
            logic::push_battle_log(
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
    logic::push_battle_log(
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
    if crit {
        logic::push_critical_battle_log(runtime, &mut battle_state.log);
    }
}
