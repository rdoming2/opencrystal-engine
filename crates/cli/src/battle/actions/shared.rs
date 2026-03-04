use engine::party::actor_weapon_category;
use engine::runtime::GameRuntime;
use rand::{Rng, RngExt};
use std::collections::HashMap;

use crate::battle::state::TargetRule;

pub(super) fn growth_entry<'a>(
    growth: &'a mut HashMap<String, engine::battle::BattleGrowthAccumulator>,
    actor_id: &str,
) -> &'a mut engine::battle::BattleGrowthAccumulator {
    growth.entry(actor_id.to_string()).or_default()
}

pub(super) fn activity_weapon_id(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
) -> Option<String> {
    let rules = &runtime.content.rules.activity_progression;
    actor_weapon_category(&runtime.content, actor, rules.unarmed_category.as_str())
}

pub(super) fn activity_magic_id(runtime: &GameRuntime, spell_id: &str) -> Option<String> {
    runtime
        .content
        .spells
        .spells
        .iter()
        .find(|spell| spell.id == spell_id)
        .map(|spell| spell.school.clone())
}

pub(super) fn activity_damage_multiplier(runtime: &GameRuntime, prof: f32) -> f32 {
    1.0 + prof
        * runtime
            .content
            .rules
            .activity_progression
            .effects
            .damage_scale
}

pub(super) fn activity_hit_bonus(runtime: &GameRuntime, prof: f32) -> f32 {
    prof * runtime.content.rules.activity_progression.effects.hit_bonus
}

pub(super) fn enemy_target_indices_for_rule(
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

pub(super) fn enemy_spell_cost_available(
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

pub(super) fn consume_enemy_spell_cost(
    enemy: &mut engine::battle::BattleEnemy,
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
    if enemy.current_mp >= cost {
        enemy.current_mp -= cost;
        true
    } else {
        false
    }
}

pub(super) fn enemy_ability_cost_available(
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

pub(super) fn consume_enemy_ability_cost(
    enemy: &mut engine::battle::BattleEnemy,
    cost_type: &str,
    cost_value: i32,
    rng: &mut impl Rng,
) -> bool {
    match cost_type {
        "none" => true,
        "mp" => {
            if enemy.mp_pool == "unlimited" {
                true
            } else if enemy.current_mp >= cost_value {
                enemy.current_mp -= cost_value;
                true
            } else {
                false
            }
        }
        "hp" => {
            if enemy.current_hp >= cost_value {
                enemy.current_hp -= cost_value;
                true
            } else {
                false
            }
        }
        "death" => {
            enemy.current_hp = 0;
            true
        }
        "random" => rng.random_bool(0.5),
        _ => false,
    }
}

pub(super) fn try_steal_item(
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

pub(super) fn apply_attenuation(value: i32, attenuation: f32) -> i32 {
    if value <= 0 {
        return 0;
    }
    ((value as f32) * attenuation).round().max(1.0) as i32
}

pub(super) fn push_enemy_cast_simple(
    runtime: &GameRuntime,
    log: &mut Vec<String>,
    enemy_name: &str,
    spell_name: &str,
    target_name: &str,
) {
    crate::battle::logic::push_battle_log(
        log,
        crate::battle::format_ui_text(
            runtime,
            "battle.log.cast_simple",
            "{actor} casts {spell} on {target}.",
            &[
                ("actor", enemy_name.to_string()),
                ("spell", spell_name.to_string()),
                ("target", target_name.to_string()),
            ],
        ),
    );
}

pub(super) fn party_indices_for_effect(
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
