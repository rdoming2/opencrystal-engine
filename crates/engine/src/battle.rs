use rand::{Rng, RngExt};
use std::collections::HashMap;

use crate::content::Content;
use crate::encounters::EncounterMember;
use crate::entities::{EnemyAiConfig, EnemyArt, EnemyDefinition, EnemyLoot, EnemySprite};
use crate::expr::eval_expression;
use crate::maps::MapCurrencyStack;
use crate::party::{Actor, PartyState, StatusInstance};
use crate::rules::BattleRules;

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
    pub turns: u32,
    pub floor_depth: u32,
    pub growth: HashMap<String, BattleGrowthAccumulator>,
}

#[derive(Clone, Debug, Default)]
pub struct BattleGrowthAccumulator {
    pub damage_taken: f32,
    pub damage_dealt_physical: f32,
    pub damage_dealt_magic: f32,
    pub mp_spent: f32,
    pub status_inflicted: f32,
    pub crits: f32,
    pub dodges: f32,
    pub turns_targeted: f32,
    pub turns_acted: f32,
    pub hp_below_25: bool,
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
    pub currency: Vec<MapCurrencyStack>,
    pub jp: i32,
    pub pos: (i32, i32),
    pub current_hp: i32,
    pub current_mp: i32,
    pub scanned: bool,
    pub statuses: Vec<StatusInstance>,
    pub spells: Vec<String>,
    pub abilities: Vec<String>,
    pub mp_pool: String,
    pub ai: EnemyAiConfig,
}

#[derive(Clone, Debug, Default)]
pub struct BattleRewards {
    pub exp: i32,
    pub currency: HashMap<String, i32>,
    pub jp: i32,
    pub items: HashMap<String, i32>,
}

#[derive(Clone, Debug, Default)]
pub struct LevelUpDiff {
    pub actor_id: String,
    pub actor_name: String,
    pub old_level: u32,
    pub new_level: u32,
    pub stat_changes: HashMap<String, (i32, i32)>,
}

#[derive(Clone, Debug, Default)]
pub struct LearnedDiff {
    pub actor_id: String,
    pub actor_name: String,
    pub spells: Vec<String>,
    pub abilities: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ActivityGrowthDiff {
    pub actor_id: String,
    pub actor_name: String,
    pub stat_changes: HashMap<String, (i32, i32)>,
}

#[derive(Clone, Debug, Default)]
pub struct BattleResult {
    pub rewards: BattleRewards,
    pub level_ups: Vec<LevelUpDiff>,
    pub learned: Vec<LearnedDiff>,
    pub activity_growth: Vec<ActivityGrowthDiff>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageKind {
    Physical,
    Magic,
}

#[derive(Clone, Debug)]
pub struct CombatantStats {
    pub atk: i32,
    pub def: i32,
    pub matk: i32,
    pub mdef: i32,
    pub agi: i32,
    pub lck: i32,
    pub eva: i32,
    pub lvl: i32,
}

#[derive(Clone, Debug)]
pub struct AttackRoll {
    pub hit: bool,
    pub crit: bool,
    pub base_damage: i32,
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
    battle_mode: crate::rules::BattleMode,
    difficulty_scale: f32,
) -> BattleState {
    let party_order = party.active_ids();
    let enemies = formation
        .iter()
        .filter_map(|member| {
            let enemy = content
                .enemies
                .enemies
                .iter()
                .find(|enemy| enemy.id == member.enemy)?;
            Some(build_enemy(content, enemy, member.pos, difficulty_scale))
        })
        .collect::<Vec<_>>();

    let mode = match battle_mode {
        crate::rules::BattleMode::Turn => BattleMode::Turn,
        crate::rules::BattleMode::Dynamic => BattleMode::Dynamic,
        crate::rules::BattleMode::DynamicWait => BattleMode::DynamicWait,
    };

    let mut rng = rand::rng();
    let readiness_party = party_order
        .iter()
        .map(|id| (id.clone(), rng.random_range(0.0..10.0)))
        .collect();
    let readiness_enemy = (0..enemies.len())
        .map(|_| rng.random_range(0.0..10.0))
        .collect();

    BattleState {
        party_order,
        enemies,
        active_index: 0,
        log: Vec::new(),
        readiness_party,
        readiness_enemy,
        mode,
        turns: 0,
        floor_depth: 1,
        growth: HashMap::new(),
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

pub fn actor_combat_stats(actor: &Actor) -> CombatantStats {
    CombatantStats {
        atk: actor.derived_stats.get("atk").copied().unwrap_or(0),
        def: actor.derived_stats.get("def").copied().unwrap_or(0),
        matk: actor.derived_stats.get("matk").copied().unwrap_or(0),
        mdef: actor.derived_stats.get("mdef").copied().unwrap_or(0),
        agi: actor.base_stats.get("agi").copied().unwrap_or(0),
        lck: actor.base_stats.get("lck").copied().unwrap_or(0),
        eva: actor.derived_stats.get("eva").copied().unwrap_or(0),
        lvl: actor.level as i32,
    }
}

pub fn enemy_combat_stats(content: &Content, enemy: &BattleEnemy) -> CombatantStats {
    let derived = derived_stats_for_enemy(content, &enemy.stats);
    CombatantStats {
        atk: derived.get("atk").copied().unwrap_or_else(|| enemy.atk()),
        def: derived.get("def").copied().unwrap_or_else(|| enemy.def()),
        matk: derived
            .get("matk")
            .copied()
            .unwrap_or_else(|| stat_value(&enemy.stats, "int") * 2),
        mdef: derived
            .get("mdef")
            .copied()
            .unwrap_or_else(|| stat_value(&enemy.stats, "int") + stat_value(&enemy.stats, "vit")),
        agi: stat_value(&enemy.stats, "agi"),
        lck: stat_value(&enemy.stats, "lck"),
        eva: derived.get("eva").copied().unwrap_or(0),
        lvl: stat_value(&enemy.stats, "lvl").max(1),
    }
}

pub fn actor_power(_content: &Content, actor: &Actor) -> f32 {
    let stats = &actor.derived_stats;
    let hp = stats.get("hp").copied().unwrap_or(0) as f32;
    let mp = stats.get("mp").copied().unwrap_or(0) as f32;
    let atk = stats.get("atk").copied().unwrap_or(0) as f32;
    let def = stats.get("def").copied().unwrap_or(0) as f32;
    let matk = stats.get("matk").copied().unwrap_or(0) as f32;
    let mdef = stats.get("mdef").copied().unwrap_or(0) as f32;
    let agi = actor.base_stats.get("agi").copied().unwrap_or(0) as f32;
    let lck = actor.base_stats.get("lck").copied().unwrap_or(0) as f32;
    hp + mp + atk + def + matk + mdef + agi + lck
}

pub fn enemy_power(content: &Content, enemy: &BattleEnemy) -> f32 {
    let derived = derived_stats_for_enemy(content, &enemy.stats);
    let hp = derived.get("hp").copied().unwrap_or(0) as f32;
    let mp = derived.get("mp").copied().unwrap_or(0) as f32;
    let atk = derived.get("atk").copied().unwrap_or_else(|| enemy.atk()) as f32;
    let def = derived.get("def").copied().unwrap_or_else(|| enemy.def()) as f32;
    let matk = derived
        .get("matk")
        .copied()
        .unwrap_or_else(|| stat_value(&enemy.stats, "int") * 2) as f32;
    let mdef = derived
        .get("mdef")
        .copied()
        .unwrap_or_else(|| stat_value(&enemy.stats, "int") + stat_value(&enemy.stats, "vit"))
        as f32;
    let agi = stat_value(&enemy.stats, "agi") as f32;
    let lck = stat_value(&enemy.stats, "lck") as f32;
    hp + mp + atk + def + matk + mdef + agi + lck
}

pub fn roll_attack(
    _content: &Content,
    rules: &BattleRules,
    attacker: &CombatantStats,
    defender: &CombatantStats,
    kind: DamageKind,
    power: i32,
    hit_bonus: f32,
    rng: &mut impl Rng,
) -> AttackRoll {
    let mut hit_chance = evaluate_chance_formula(
        rules.formulas.hit.as_deref(),
        attacker,
        defender,
        power,
        1.0,
    );
    hit_chance = (hit_chance + hit_bonus).clamp(0.0, 1.0);
    if rng.random::<f32>() > hit_chance {
        return AttackRoll {
            hit: false,
            crit: false,
            base_damage: 0,
        };
    }

    let crit_chance = evaluate_chance_formula(
        rules.formulas.crit.as_deref(),
        attacker,
        defender,
        power,
        0.05,
    );
    let crit = rng.random::<f32>() <= crit_chance;
    let base_damage = match kind {
        DamageKind::Physical => evaluate_damage_formula(
            rules.formulas.physical.as_deref(),
            attacker,
            defender,
            power,
            default_physical_base(attacker, defender, power),
        ),
        DamageKind::Magic => evaluate_damage_formula(
            rules.formulas.magic.as_deref(),
            attacker,
            defender,
            power,
            default_magic_base(attacker, defender, power),
        ),
    };
    let mut damage = roll_damage(base_damage, rng);
    if crit {
        damage = ((damage as f32) * rules.formulas.crit_multiplier.max(0.1))
            .round()
            .max(1.0) as i32;
    }
    AttackRoll {
        hit: true,
        crit,
        base_damage: damage.max(1),
    }
}

pub fn physical_damage(attacker_atk: i32, defender_def: i32, rng: &mut impl Rng) -> i32 {
    let base = attacker_atk.saturating_sub(defender_def / 2).max(1);
    roll_damage(base, rng)
}

pub fn roll_damage(base: i32, rng: &mut impl Rng) -> i32 {
    if base <= 1 {
        return base.max(1);
    }
    let variance = rng.random_range(90..=110) as f32 / 100.0;
    ((base as f32) * variance).round().max(1.0) as i32
}

pub fn collect_rewards(enemies: &[BattleEnemy], rng: &mut impl Rng) -> BattleRewards {
    let mut rewards = BattleRewards::default();
    for enemy in enemies {
        rewards.exp += enemy.exp.max(0);
        for currency in &enemy.currency {
            if currency.amount <= 0 {
                continue;
            }
            let entry = rewards.currency.entry(currency.id.clone()).or_insert(0);
            *entry = entry.saturating_add(currency.amount);
        }
        rewards.jp += enemy.jp.max(0);
        for loot in &enemy.loot {
            if loot.chance <= 0.0 {
                continue;
            }
            if rng.random::<f32>() <= loot.chance {
                let entry = rewards.items.entry(loot.item.clone()).or_insert(0);
                *entry += 1;
            }
        }
    }
    rewards
}

fn build_enemy(
    content: &Content,
    enemy: &EnemyDefinition,
    pos: [i32; 2],
    difficulty_scale: f32,
) -> BattleEnemy {
    let mut stats = enemy.stats.clone();
    apply_enemy_scaling(content, enemy, &mut stats, difficulty_scale);
    let max_hp = stat_value(&stats, "hp");
    let max_mp = stat_value(&stats, "mp");
    let current_mp = if enemy.mp_pool == "unlimited" {
        max_mp.max(0)
    } else {
        max_mp.max(0)
    };
    BattleEnemy {
        id: enemy.id.clone(),
        name: enemy.name.clone(),
        stats,
        traits: enemy.traits.clone(),
        sprite: enemy.sprite.clone(),
        art: enemy.art.clone(),
        loot: enemy.loot.clone(),
        exp: enemy.exp,
        currency: enemy.currency.clone(),
        jp: enemy.jp,
        pos: (pos[0], pos[1]),
        current_hp: max_hp.max(1),
        current_mp,
        scanned: false,
        statuses: Vec::new(),
        spells: enemy.spells.clone(),
        abilities: enemy.abilities.clone(),
        mp_pool: enemy.mp_pool.clone(),
        ai: enemy.ai.clone(),
    }
}

fn stat_value(stats: &HashMap<String, i32>, key: &str) -> i32 {
    stats.get(key).copied().unwrap_or(0)
}

fn apply_enemy_scaling(
    content: &Content,
    enemy: &EnemyDefinition,
    stats: &mut HashMap<String, i32>,
    difficulty_scale: f32,
) {
    let scaling = &content.rules.battle.boss_scaling;
    let is_boss = scaling.enabled && enemy.traits.iter().any(|trait_id| trait_id == "boss");
    let difficulty = difficulty_scale.max(0.0);
    for (stat, value) in stats.iter_mut() {
        let mut multiplier = difficulty;
        if is_boss {
            multiplier *= if stat == "hp" {
                scaling.hp_multiplier
            } else {
                scaling.stat_multiplier
            };
        }
        *value = ((*value as f32) * multiplier).round().max(1.0) as i32;
    }
}

fn derived_stats_for_enemy(
    content: &Content,
    base_stats: &HashMap<String, i32>,
) -> HashMap<String, i32> {
    let mut vars = HashMap::new();
    for (stat, value) in base_stats {
        vars.insert(stat.clone(), *value as f64);
    }
    vars.insert(
        "lvl".to_string(),
        base_stats.get("lvl").copied().unwrap_or(1) as f64,
    );
    for stat in &content.stats.stats.base {
        vars.entry(format!("gear.{}", stat.id)).or_insert(0.0);
        vars.entry(format!("buffs.{}", stat.id)).or_insert(0.0);
    }
    for stat in &content.stats.stats.derived {
        vars.entry(format!("gear.{}", stat.id)).or_insert(0.0);
        vars.entry(format!("buffs.{}", stat.id)).or_insert(0.0);
    }
    let mut derived = base_stats.clone();
    for stat in &content.stats.stats.derived {
        if let Some(formula) = content.stats.stats.formulas.get(&stat.id) {
            if let Ok(result) = eval_expression(formula, &vars) {
                derived.insert(stat.id.clone(), result.floor() as i32);
            }
        }
    }
    derived
}

fn evaluate_damage_formula(
    formula: Option<&str>,
    attacker: &CombatantStats,
    defender: &CombatantStats,
    power: i32,
    fallback: i32,
) -> i32 {
    let Some(formula) = formula else {
        return fallback.max(1);
    };
    let vars = combat_formula_vars(attacker, defender, power);
    match eval_expression(formula, &vars) {
        Ok(result) => result.round().max(1.0) as i32,
        Err(_) => fallback.max(1),
    }
}

fn evaluate_chance_formula(
    formula: Option<&str>,
    attacker: &CombatantStats,
    defender: &CombatantStats,
    power: i32,
    fallback: f32,
) -> f32 {
    let Some(formula) = formula else {
        return clamp_chance(fallback);
    };
    let vars = combat_formula_vars(attacker, defender, power);
    match eval_expression(formula, &vars) {
        Ok(result) => clamp_chance(result as f32),
        Err(_) => clamp_chance(fallback),
    }
}

fn combat_formula_vars(
    attacker: &CombatantStats,
    defender: &CombatantStats,
    power: i32,
) -> HashMap<String, f64> {
    let mut vars = HashMap::new();
    vars.insert("atk".to_string(), attacker.atk as f64);
    vars.insert("def".to_string(), defender.def as f64);
    vars.insert("matk".to_string(), attacker.matk as f64);
    vars.insert("mdef".to_string(), defender.mdef as f64);
    vars.insert("agi".to_string(), attacker.agi as f64);
    vars.insert("lck".to_string(), attacker.lck as f64);
    vars.insert("eva".to_string(), attacker.eva as f64);
    vars.insert("lvl".to_string(), attacker.lvl as f64);
    vars.insert("target_eva".to_string(), defender.eva as f64);
    vars.insert("target_lvl".to_string(), defender.lvl as f64);
    vars.insert("power".to_string(), power as f64);
    vars
}

fn clamp_chance(value: f32) -> f32 {
    value.max(0.05).min(0.99)
}

fn default_physical_base(attacker: &CombatantStats, defender: &CombatantStats, power: i32) -> i32 {
    attacker
        .atk
        .saturating_sub(defender.def / 2)
        .saturating_add(power)
        .max(1)
}

fn default_magic_base(attacker: &CombatantStats, defender: &CombatantStats, power: i32) -> i32 {
    power
        .saturating_add(attacker.matk)
        .saturating_sub(defender.mdef / 2)
        .max(1)
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
            if chance < 1.0 && rng.random::<f32>() > chance.max(0.0) {
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
                            if rng.random::<f32>() <= chance {
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
