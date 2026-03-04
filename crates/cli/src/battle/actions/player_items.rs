use engine::battle::{
    apply_damage_to_actor, apply_damage_to_enemy, apply_status_effects, damage_multiplier,
    enemy_combat_stats, healing_inverted, roll_attack, DamageKind,
};
use engine::runtime::GameRuntime;
use rand::Rng;

use crate::battle::logic;

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
                let attacker_stats = engine::battle::CombatantStats {
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
        logic::push_battle_log(
            &mut battle_state.log,
            crate::battle::ui_text(runtime, "battle.item_unusable", "Item unusable."),
        );
        return;
    }
    if !runtime.inventory.remove_item(&item.id, 1) {
        logic::push_battle_log(
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
                    logic::push_battle_log(&mut battle_state.log, message);
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
            logic::push_battle_log(&mut battle_state.log, message);
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
                    logic::push_battle_log(
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
    logic::push_battle_log(
        &mut battle_state.log,
        format!("{} uses {}.", actor_name, item.name),
    );
}
