use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::content::Content;
use crate::entities::{EquipmentDefinition, JobDefinition};
use crate::expr::eval_expression;
use crate::rules::{ExpCurveRules, MagicSystem, PartyCreateRules, PartyMode, RulesFile, Ruleset};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyFile {
    pub version: u32,
    pub roster: Vec<ActorDefinition>,
    #[serde(default)]
    pub starting_party: Vec<String>,
    #[serde(default)]
    pub reserve: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActorDefinition {
    pub id: String,
    pub name: String,
    pub job_id: String,
    pub level: u32,
    #[serde(default)]
    pub base_stats: HashMap<String, i32>,
    #[serde(default)]
    pub starting_equipment: HashMap<String, String>,
    #[serde(default)]
    pub spells: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub job_id: String,
    pub level: u32,
    pub exp: i32,
    pub current_hp: i32,
    pub current_mp: i32,
    pub base_stats: HashMap<String, i32>,
    pub derived_stats: HashMap<String, i32>,
    pub equipment: HashMap<String, String>,
    pub spells: Vec<String>,
    pub magic_tier_charges: HashMap<u32, i32>,
}

#[derive(Clone, Debug)]
pub struct PartyState {
    pub roster: HashMap<String, Actor>,
    pub active: Vec<String>,
    pub reserve: Vec<String>,
}

impl PartyState {
    pub fn empty() -> Self {
        Self {
            roster: HashMap::new(),
            active: Vec::new(),
            reserve: Vec::new(),
        }
    }

    pub fn from_content(content: &Content, rules: &Ruleset) -> Self {
        match rules.party_mode {
            PartyMode::Create => Self::empty(),
            PartyMode::Predefined => build_predefined_party(content, rules.party_size),
        }
    }

    pub fn from_created(
        content: &Content,
        rules: &Ruleset,
        members: Vec<(String, String)>,
    ) -> Self {
        build_created_party(content, &rules.party_create, rules.party_size, members)
    }
}

pub fn reset_magic_tier_charges(party: &mut PartyState, rules: &Ruleset) {
    if rules.magic_system != MagicSystem::TierCharges {
        return;
    }
    for actor in party.roster.values_mut() {
        let mut charges = HashMap::new();
        for tier in &rules.magic_tiers {
            charges.insert(tier.tier, tier.max_charges.max(0));
        }
        actor.magic_tier_charges = charges;
    }
}

impl PartyFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

fn build_predefined_party(content: &Content, party_size: usize) -> PartyState {
    let Some(party_file) = &content.party else {
        return PartyState::empty();
    };
    let job_lookup = build_job_lookup(content);
    let equipment_lookup = build_equipment_lookup(content);

    let mut roster = HashMap::new();
    for actor in &party_file.roster {
        let job = job_lookup.get(actor.job_id.as_str()).copied();
        let built = build_actor(content, actor, job, &equipment_lookup);
        roster.insert(built.id.clone(), built);
    }

    let mut active = if party_file.starting_party.is_empty() {
        party_file
            .roster
            .iter()
            .take(party_size)
            .map(|actor| actor.id.clone())
            .collect::<Vec<_>>()
    } else {
        party_file.starting_party.clone()
    };
    active.retain(|id| roster.contains_key(id));
    active.truncate(party_size);

    let mut reserve = party_file.reserve.clone();
    reserve.retain(|id| roster.contains_key(id) && !active.contains(id));

    PartyState {
        roster,
        active,
        reserve,
    }
}

fn build_created_party(
    content: &Content,
    create_rules: &PartyCreateRules,
    party_size: usize,
    members: Vec<(String, String)>,
) -> PartyState {
    let job_lookup = build_job_lookup(content);
    let equipment_lookup = build_equipment_lookup(content);

    let mut roster = HashMap::new();
    let mut active = Vec::new();

    for (index, (name, job_id)) in members.into_iter().enumerate() {
        if active.len() >= party_size {
            break;
        }
        let actor_id = format!("create_{}", index + 1);
        let job = job_lookup.get(job_id.as_str()).copied();
        let starting_equipment = job
            .map(|job| job.starting_equipment.clone())
            .filter(|equipment| !equipment.is_empty())
            .unwrap_or_else(|| create_rules.starting_equipment.clone());
        let actor = ActorDefinition {
            id: actor_id.clone(),
            name,
            job_id: job_id.clone(),
            level: create_rules.starting_level,
            base_stats: HashMap::new(),
            starting_equipment,
            spells: Vec::new(),
        };
        let built = build_actor(content, &actor, job, &equipment_lookup);
        roster.insert(actor_id.clone(), built);
        active.push(actor_id);
    }

    PartyState {
        roster,
        active,
        reserve: Vec::new(),
    }
}

fn build_job_lookup(content: &Content) -> HashMap<&str, &JobDefinition> {
    content
        .jobs
        .jobs
        .iter()
        .map(|job| (job.id.as_str(), job))
        .collect()
}

fn build_equipment_lookup(content: &Content) -> HashMap<&str, &EquipmentDefinition> {
    content
        .equipment
        .equipment
        .iter()
        .map(|equipment| (equipment.id.as_str(), equipment))
        .collect()
}

fn build_actor(
    content: &Content,
    actor: &ActorDefinition,
    job: Option<&JobDefinition>,
    equipment_lookup: &HashMap<&str, &EquipmentDefinition>,
) -> Actor {
    let base_stats = if !actor.base_stats.is_empty() {
        actor.base_stats.clone()
    } else if let Some(job) = job {
        job.stats.clone()
    } else {
        HashMap::new()
    };
    let mut equipment = HashMap::new();
    if let Some(job) = job {
        let allowed_slots = job_slots(job);
        let mut starting_equipment = job.starting_equipment.clone();
        for (slot, item_id) in &actor.starting_equipment {
            starting_equipment.insert(slot.clone(), item_id.clone());
        }
        for (slot, item_id) in starting_equipment {
            if allowed_slots.contains(&slot) {
                equipment.insert(slot, item_id);
            }
        }
    }
    let derived_stats = build_derived_stats(
        content,
        &base_stats,
        actor.level,
        &equipment,
        job,
        equipment_lookup,
    );

    let max_hp = derived_stats.get("hp").copied().unwrap_or(0);
    let max_mp = derived_stats.get("mp").copied().unwrap_or(0);

    let mut built = Actor {
        id: actor.id.clone(),
        name: actor.name.clone(),
        job_id: actor.job_id.clone(),
        level: actor.level,
        exp: 0,
        current_hp: max_hp,
        current_mp: max_mp,
        base_stats,
        derived_stats,
        equipment,
        spells: actor.spells.clone(),
        magic_tier_charges: HashMap::new(),
    };
    learn_job_spells(content, &mut built);
    built
}

fn learn_job_spells(content: &Content, actor: &mut Actor) {
    let Some(job) = content.jobs.jobs.iter().find(|job| job.id == actor.job_id) else {
        return;
    };
    let mut learned: HashSet<String> = actor.spells.iter().cloned().collect();
    for spell in &job.spells {
        let unlocks = match spell.method.as_str() {
            "level" => spell.level.unwrap_or(0) <= actor.level,
            "tier" => spell.tier.unwrap_or(0) <= actor.level,
            _ => false,
        };
        if unlocks {
            learned.insert(spell.id.clone());
        }
    }
    let mut spells = learned.into_iter().collect::<Vec<_>>();
    spells.sort_unstable();
    actor.spells = spells;
}

fn job_slots(job: &JobDefinition) -> Vec<String> {
    let mut slots = if job.equipment_slots.is_empty() {
        let mut fallback = Vec::new();
        if !job.equipment.weapons.is_empty() {
            fallback.push("weapon".to_string());
        }
        if !job.equipment.armor.is_empty() {
            fallback.push("armor".to_string());
        }
        fallback
    } else {
        job.equipment_slots.clone()
    };
    for index in 1..=job.accessory_slots {
        slots.push(format!("accessory_{}", index));
    }
    slots
}

pub fn actor_slots(content: &Content, actor: &Actor) -> Vec<String> {
    content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id)
        .map(job_slots)
        .unwrap_or_default()
}

fn apply_job_modifiers(
    base_stats: &HashMap<String, i32>,
    job: Option<&JobDefinition>,
) -> HashMap<String, i32> {
    let mut stats = base_stats.clone();
    let Some(job) = job else {
        return stats;
    };
    for (stat, modifier) in &job.stat_modifiers {
        let base = stats.get(stat).copied().unwrap_or(0);
        let add = modifier.add.unwrap_or(0);
        let mult = modifier.mult.unwrap_or(1.0);
        let value = ((base + add) as f32 * mult).round() as i32;
        stats.insert(stat.clone(), value);
    }
    stats
}

fn apply_equipment_modifiers(
    stats: &mut HashMap<String, i32>,
    equipment: &HashMap<String, String>,
    equipment_lookup: &HashMap<&str, &EquipmentDefinition>,
) {
    for item_id in equipment.values() {
        if let Some(item) = equipment_lookup.get(item_id.as_str()) {
            for (stat, value) in &item.stats {
                let entry = stats.entry(stat.clone()).or_insert(0);
                *entry += value;
            }
        }
    }
}

pub fn recompute_derived_stats(content: &Content, actor: &mut Actor) {
    let job = content.jobs.jobs.iter().find(|job| job.id == actor.job_id);
    let equipment_lookup = build_equipment_lookup(content);
    actor.derived_stats = build_derived_stats(
        content,
        &actor.base_stats,
        actor.level,
        &actor.equipment,
        job,
        &equipment_lookup,
    );
    clamp_current_stats(actor);
}

pub fn exp_for_level(curve: &ExpCurveRules, level: u32) -> Option<i32> {
    match curve.mode.as_str() {
        "table" => {
            if level == 0 {
                return Some(0);
            }
            let index = level.saturating_sub(1) as usize;
            curve.table.get(index).copied()
        }
        "formula" => curve.formula.as_ref().and_then(|formula| {
            let mut vars = HashMap::new();
            vars.insert("lvl".to_string(), level as f64);
            eval_expression(formula, &vars)
                .ok()
                .map(|value| value.floor() as i32)
        }),
        _ => None,
    }
}

pub fn gain_exp(content: &Content, rules: &Ruleset, actor: &mut Actor, amount: i32) -> u32 {
    actor.exp = actor.exp.saturating_add(amount);
    let max_level = rules.exp_curve.max_level.max(1);
    let mut levels_gained = 0;
    while actor.level < max_level {
        let next_level = actor.level + 1;
        let required = match exp_for_level(&rules.exp_curve, next_level) {
            Some(required) => required,
            None => break,
        };
        if actor.exp < required {
            break;
        }
        actor.level = next_level;
        apply_growth(content, actor);
        levels_gained += 1;
    }
    learn_job_spells(content, actor);
    recompute_derived_stats(content, actor);
    levels_gained
}

fn apply_growth(content: &Content, actor: &mut Actor) {
    let job = match content.jobs.jobs.iter().find(|job| job.id == actor.job_id) {
        Some(job) => job,
        None => return,
    };
    let base_stats = content
        .stats
        .stats
        .base
        .iter()
        .map(|stat| stat.id.clone())
        .collect::<Vec<_>>();
    match job.growth.mode.as_str() {
        "formula" => {
            let mut vars = HashMap::new();
            for (stat, value) in &actor.base_stats {
                vars.insert(stat.clone(), *value as f64);
            }
            for stat in base_stats {
                if let Some(formula) = job.growth.per_level.get(&stat) {
                    if let Ok(result) = eval_expression(formula, &vars) {
                        let delta = result.floor() as i32;
                        let entry = actor.base_stats.entry(stat.clone()).or_insert(0);
                        *entry += delta;
                    }
                }
            }
        }
        "table" => {
            let level_index = actor.level.saturating_sub(1) as usize;
            for stat in base_stats {
                if let Some(table) = job.growth.tables.get(&stat) {
                    if let Some(value) = table.get(level_index) {
                        actor.base_stats.insert(stat.clone(), *value);
                    }
                }
            }
        }
        _ => {}
    }
}

fn build_derived_stats(
    content: &Content,
    base_stats: &HashMap<String, i32>,
    level: u32,
    equipment: &HashMap<String, String>,
    job: Option<&JobDefinition>,
    equipment_lookup: &HashMap<&str, &EquipmentDefinition>,
) -> HashMap<String, i32> {
    let mut stats = apply_job_modifiers(base_stats, job);
    apply_equipment_modifiers(&mut stats, equipment, equipment_lookup);
    let gear_stats = compute_equipment_stats(equipment, equipment_lookup);
    let mut vars = HashMap::new();
    for (stat, value) in &stats {
        vars.insert(stat.clone(), *value as f64);
    }
    for (stat, value) in &gear_stats {
        vars.insert(format!("gear.{stat}"), *value as f64);
    }
    vars.insert("lvl".to_string(), level as f64);

    let mut derived = stats.clone();
    for stat in &content.stats.stats.derived {
        if let Some(formula) = content.stats.stats.formulas.get(&stat.id) {
            if let Ok(result) = eval_expression(formula, &vars) {
                derived.insert(stat.id.clone(), result.floor() as i32);
            }
        }
    }
    derived
}

fn compute_equipment_stats(
    equipment: &HashMap<String, String>,
    equipment_lookup: &HashMap<&str, &EquipmentDefinition>,
) -> HashMap<String, i32> {
    let mut stats = HashMap::new();
    for item_id in equipment.values() {
        if let Some(item) = equipment_lookup.get(item_id.as_str()) {
            for (stat, value) in &item.stats {
                let entry = stats.entry(stat.clone()).or_insert(0);
                *entry += value;
            }
        }
    }
    stats
}

pub fn rest_party(party: &mut PartyState, rules: &RulesFile) {
    for actor in party.roster.values_mut() {
        let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
        let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
        actor.current_hp = max_hp;
        actor.current_mp = max_mp;
    }
    let ruleset = Ruleset::from_file(rules.clone());
    reset_magic_tier_charges(party, &ruleset);
}

fn clamp_current_stats(actor: &mut Actor) {
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
    actor.current_hp = actor.current_hp.clamp(0, max_hp);
    actor.current_mp = actor.current_mp.clamp(0, max_mp);
}
