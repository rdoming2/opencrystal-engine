use rand::Rng;
use std::collections::HashMap;

use crate::content::Content;
use crate::encounters::EncounterMember;
use crate::entities::{EnemyArt, EnemyDefinition, EnemyLoot, EnemySprite};
use crate::party::{Actor, PartyState, StatusInstance};

#[derive(Clone, Debug, PartialEq)]
pub enum BattleMode {
    Turn,
    Dynamic,
    DynamicWait,
}

#[derive(Clone, Debug)]
pub struct BattleState {
    pub party_order: Vec<String>,
    pub enemies: Vec<BattleEnemy>,
    pub active_index: usize,
    pub log: Vec<String>,
    pub readiness_party: HashMap<String, f32>,
    pub readiness_enemy: Vec<f32>,
    pub mode: BattleMode,
}

#[derive(Clone, Debug)]
pub struct BattleEnemy {
    pub id: String,
    pub name: String,
    pub stats: HashMap<String, i32>,
    pub traits: Vec<String>,
    pub sprite: EnemySprite,
    pub art: Option<EnemyArt>,
    pub loot: Vec<EnemyLoot>,
    pub exp: i32,
    pub currency: i32,
    pub jp: i32,
    pub pos: (i32, i32),
    pub current_hp: i32,
    pub current_mp: i32,
    pub scanned: bool,
    pub statuses: Vec<StatusInstance>,
}

#[derive(Clone, Debug, Default)]
pub struct BattleRewards {
    pub exp: i32,
    pub currency: i32,
    pub jp: i32,
    pub items: HashMap<String, i32>,
}

#[derive(Clone, Debug, Default)]
pub struct LevelUpDiff {
    pub actor_name: String,
    pub old_level: u32,
    pub new_level: u32,
    pub stat_changes: HashMap<String, (i32, i32)>,
}

#[derive(Clone, Debug, Default)]
pub struct BattleResult {
    pub rewards: BattleRewards,
    pub level_ups: Vec<LevelUpDiff>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageKind {
    Physical,
    Magic,
}

#[derive(Clone, Debug, Default)]
pub struct StatusTurnResult {
    pub can_act: bool,
    pub messages: Vec<String>,
}

impl BattleEnemy {
    pub fn max_hp(&self) -> i32 {
        stat_value(&self.stats, "hp")
    }

    pub fn max_mp(&self) -> i32 {
        stat_value(&self.stats, "mp")
    }

    pub fn atk(&self) -> i32 {
        if let Some(atk) = self.stats.get("atk") {
            return *atk;
        }
        stat_value(&self.stats, "str") * 2
    }

    pub fn def(&self) -> i32 {
        if let Some(def) = self.stats.get("def") {
            return *def;
        }
        stat_value(&self.stats, "vit") * 2
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }
}

pub fn build_battle_state(
    content: &Content,
    party: &PartyState,
    formation: &[EncounterMember],
) -> BattleState {
    let party_order = party.active.clone();
    let enemies = formation
        .iter()
        .filter_map(|member| {
            let enemy = content
                .enemies
                .enemies
                .iter()
                .find(|enemy| enemy.id == member.enemy)?;
            Some(build_enemy(enemy, member.pos))
        })
        .collect::<Vec<_>>();

    let mode = match content.rules.game.battle_mode {
        crate::rules::BattleMode::Turn => BattleMode::Turn,
        crate::rules::BattleMode::Dynamic => BattleMode::Dynamic,
        crate::rules::BattleMode::DynamicWait => BattleMode::DynamicWait,
    };

    let mut rng = rand::thread_rng();
    let readiness_party = party_order
        .iter()
        .map(|id| (id.clone(), rng.gen_range(0.0..10.0)))
        .collect();
    let readiness_enemy = (0..enemies.len())
        .map(|_| rng.gen_range(0.0..10.0))
        .collect();

    BattleState {
        party_order,
        enemies,
        active_index: 0,
        log: Vec::new(),
        readiness_party,
        readiness_enemy,
        mode,
    }
}

pub fn is_party_defeated(party: &PartyState, order: &[String]) -> bool {
    order.iter().all(|id| {
        party
            .roster
            .get(id)
            .map(|actor| actor.current_hp <= 0)
            .unwrap_or(true)
    })
}

pub fn is_enemies_defeated(enemies: &[BattleEnemy]) -> bool {
    enemies.iter().all(|enemy| enemy.current_hp <= 0)
}

pub fn next_living_party_index(
    party: &PartyState,
    order: &[String],
    start: usize,
) -> Option<usize> {
    if order.is_empty() {
        return None;
    }
    for offset in 0..order.len() {
        let index = (start + offset) % order.len();
        if let Some(actor) = party.roster.get(&order[index]) {
            if actor.current_hp > 0 {
                return Some(index);
            }
        }
    }
    None
}

pub fn apply_damage_to_actor(actor: &mut Actor, amount: i32) {
    actor.current_hp = (actor.current_hp - amount).max(0);
}

pub fn apply_damage_to_enemy(enemy: &mut BattleEnemy, amount: i32) {
    enemy.current_hp = (enemy.current_hp - amount).max(0);
}

pub fn physical_damage(attacker_atk: i32, defender_def: i32, rng: &mut impl Rng) -> i32 {
    let base = attacker_atk.saturating_sub(defender_def / 2).max(1);
    roll_damage(base, rng)
}

pub fn roll_damage(base: i32, rng: &mut impl Rng) -> i32 {
    if base <= 1 {
        return base.max(1);
    }
    let variance = rng.gen_range(90..=110) as f32 / 100.0;
    ((base as f32) * variance).round().max(1.0) as i32
}

pub fn collect_rewards(enemies: &[BattleEnemy], rng: &mut impl Rng) -> BattleRewards {
    let mut rewards = BattleRewards::default();
    for enemy in enemies {
        rewards.exp += enemy.exp.max(0);
        rewards.currency += enemy.currency.max(0);
        rewards.jp += enemy.jp.max(0);
        for loot in &enemy.loot {
            if loot.chance <= 0.0 {
                continue;
            }
            if rng.r#gen::<f32>() <= loot.chance {
                let entry = rewards.items.entry(loot.item.clone()).or_insert(0);
                *entry += 1;
            }
        }
    }
    rewards
}

fn build_enemy(enemy: &EnemyDefinition, pos: [i32; 2]) -> BattleEnemy {
    let max_hp = stat_value(&enemy.stats, "hp");
    let max_mp = stat_value(&enemy.stats, "mp");
    BattleEnemy {
        id: enemy.id.clone(),
        name: enemy.name.clone(),
        stats: enemy.stats.clone(),
        traits: enemy.traits.clone(),
        sprite: enemy.sprite.clone(),
        art: enemy.art.clone(),
        loot: enemy.loot.clone(),
        exp: enemy.exp,
        currency: enemy.currency,
        jp: enemy.jp,
        pos: (pos[0], pos[1]),
        current_hp: max_hp.max(1),
        current_mp: max_mp.max(0),
        scanned: false,
        statuses: Vec::new(),
    }
}

fn stat_value(stats: &HashMap<String, i32>, key: &str) -> i32 {
    stats.get(key).copied().unwrap_or(0)
}

pub fn status_definition<'a>(
    content: &'a Content,
    status_id: &str,
) -> Option<&'a crate::entities::StatusDefinition> {
    content
        .effects
        .statuses
        .iter()
        .find(|status| status.id == status_id)
}

pub fn effect_definition<'a>(
    content: &'a Content,
    effect_id: &str,
) -> Option<&'a crate::entities::EffectDefinition> {
    content
        .effects
        .effects
        .iter()
        .find(|effect| effect.id == effect_id)
}

pub fn trait_definition<'a>(
    content: &'a Content,
    trait_id: &str,
) -> Option<&'a crate::entities::TraitDefinition> {
    content
        .effects
        .traits
        .iter()
        .find(|entry| entry.id == trait_id)
}

pub fn apply_status(
    content: &Content,
    statuses: &mut Vec<StatusInstance>,
    status_id: &str,
) -> bool {
    let Some(definition) = status_definition(content, status_id) else {
        return false;
    };
    let duration = definition.default_duration;
    let reapply = if definition.reapply.trim().is_empty() {
        "refresh"
    } else {
        definition.reapply.as_str()
    };

    if let Some(existing) = statuses.iter_mut().find(|status| status.id == status_id) {
        match reapply {
            "ignore" => return false,
            "stack" => {
                if duration > 0 {
                    existing.remaining_turns = existing.remaining_turns.saturating_add(duration);
                }
            }
            _ => {
                if duration > 0 {
                    existing.remaining_turns = duration;
                }
            }
        }
        return true;
    }

    let remaining_turns = if duration > 0 { duration } else { 0 };
    statuses.push(StatusInstance {
        id: status_id.to_string(),
        remaining_turns,
    });
    true
}

pub fn apply_status_effects(
    content: &Content,
    effect_ids: &[String],
    statuses: &mut Vec<StatusInstance>,
    rng: &mut impl Rng,
) -> Vec<String> {
    let mut applied = Vec::new();
    for effect_id in effect_ids {
        let Some(effect) = effect_definition(content, effect_id) else {
            continue;
        };
        if effect.kind.as_str() != "apply_status" {
            continue;
        }
        let status_id = effect.status.as_deref().unwrap_or("");
        if status_id.is_empty() {
            continue;
        }
        if let Some(chance) = effect.chance {
            if chance < 1.0 && rng.r#gen::<f32>() > chance.max(0.0) {
                continue;
            }
        }
        if apply_status(content, statuses, status_id) {
            if let Some(definition) = status_definition(content, status_id) {
                applied.push(definition.label.clone());
            }
        }
    }
    applied
}

pub fn apply_turn_start_statuses(
    content: &Content,
    name: &str,
    max_hp: i32,
    current_hp: &mut i32,
    statuses: &mut Vec<StatusInstance>,
    rng: &mut impl Rng,
) -> StatusTurnResult {
    let mut result = StatusTurnResult {
        can_act: true,
        messages: Vec::new(),
    };
    let mut remaining = Vec::new();
    for mut status in statuses.drain(..) {
        let Some(definition) = status_definition(content, &status.id) else {
            continue;
        };
        let tick = if definition.tick.trim().is_empty() {
            "turn_start"
        } else {
            definition.tick.as_str()
        };
        if tick == "turn_start" {
            for effect_id in &definition.effects {
                if let Some(effect) = effect_definition(content, effect_id) {
                    match effect.kind.as_str() {
                        "poison_tick" => {
                            let percent = effect.percent.unwrap_or(0.0).max(0.0);
                            let flat = effect.power.unwrap_or(0).max(0);
                            let mut damage = ((max_hp.max(1) as f32) * percent).round() as i32;
                            damage = damage.max(flat).max(1);
                            *current_hp = (*current_hp - damage).max(0);
                            result
                                .messages
                                .push(format!("{} takes {} damage from poison.", name, damage));
                        }
                        "skip_turn" => {
                            let chance = effect.chance.unwrap_or(1.0).max(0.0);
                            if rng.r#gen::<f32>() <= chance {
                                result.can_act = false;
                                result.messages.push(format!("{} is unable to move.", name));
                            }
                        }
                        "immobile" => {
                            result.can_act = false;
                            result.messages.push(format!("{} is petrified.", name));
                        }
                        _ => {}
                    }
                }
            }
        }
        if definition.default_duration > 0 && tick == "turn_start" {
            status.remaining_turns = status.remaining_turns.saturating_sub(1);
        }
        if definition.default_duration <= 0 || status.remaining_turns > 0 {
            remaining.push(status);
        }
    }
    *statuses = remaining;
    result
}

pub fn damage_multiplier(
    content: &Content,
    statuses: &[StatusInstance],
    traits: &[String],
    kind: DamageKind,
    element: Option<&str>,
) -> f32 {
    let mut multiplier = 1.0;
    for status in statuses {
        if let Some(definition) = status_definition(content, &status.id) {
            for effect_id in &definition.effects {
                if let Some(effect) = effect_definition(content, effect_id) {
                    if effect.kind.as_str() == "damage_multiplier" {
                        let damage_kind = effect.damage_kind.as_deref().unwrap_or("all");
                        if damage_kind == "all"
                            || (damage_kind == "physical" && kind == DamageKind::Physical)
                            || (damage_kind == "magic" && kind == DamageKind::Magic)
                        {
                            multiplier *= effect.multiplier.unwrap_or(1.0);
                        }
                    }
                }
            }
        }
    }
    for trait_id in traits {
        if let Some(trait_def) = trait_definition(content, trait_id) {
            for effect_id in &trait_def.effects {
                if let Some(effect) = effect_definition(content, effect_id) {
                    if effect.kind.as_str() == "element_multiplier" {
                        let Some(effect_element) = effect.element.as_deref() else {
                            continue;
                        };
                        if Some(effect_element) == element {
                            multiplier *= effect.multiplier.unwrap_or(1.0);
                        }
                    }
                }
            }
        }
    }
    multiplier
}

pub fn healing_inverted(content: &Content, traits: &[String]) -> bool {
    for trait_id in traits {
        if let Some(trait_def) = trait_definition(content, trait_id) {
            for effect_id in &trait_def.effects {
                if let Some(effect) = effect_definition(content, effect_id) {
                    if effect.kind.as_str() == "healing_inversion" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn status_short_label(content: &Content, status_id: &str) -> Option<String> {
    status_definition(content, status_id).map(|status| status.short.clone())
}

pub fn trait_label(content: &Content, trait_id: &str) -> Option<String> {
    trait_definition(content, trait_id).map(|entry| entry.label.clone())
}

pub fn retain_statuses_after_battle(content: &Content, statuses: &mut Vec<StatusInstance>) {
    statuses.retain(|status| {
        status_definition(content, &status.id)
            .map(|definition| !definition.clear_on_battle_end)
            .unwrap_or(true)
    });
}

pub fn apply_overworld_poison_tick(content: &Content, actor: &mut Actor) -> Option<i32> {
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    let mut damage = 0;
    let poison = actor.statuses.iter().any(|status| status.id == "poison");
    if !poison {
        return None;
    }
    let Some(definition) = status_definition(content, "poison") else {
        return None;
    };
    for effect_id in &definition.effects {
        if let Some(effect) = effect_definition(content, effect_id) {
            if effect.kind.as_str() != "poison_tick" {
                continue;
            }
            let percent = effect.percent.unwrap_or(0.0).max(0.0);
            let flat = effect.power.unwrap_or(0).max(0);
            let mut tick = ((max_hp.max(1) as f32) * percent).round() as i32;
            tick = tick.max(flat).max(1);
            damage = damage.max(tick);
        }
    }
    if damage <= 0 {
        return None;
    }
    actor.current_hp = (actor.current_hp - damage).max(0);
    Some(damage)
}
