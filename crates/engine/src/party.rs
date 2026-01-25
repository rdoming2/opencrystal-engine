use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::content::Content;
use crate::entities::{EquipmentDefinition, JobDefinition};
use crate::rules::{PartyCreateRules, PartyMode, Ruleset};

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
    pub base_stats: HashMap<String, i32>,
    pub derived_stats: HashMap<String, i32>,
    pub equipment: HashMap<String, String>,
    pub spells: Vec<String>,
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

    pub fn from_created(content: &Content, rules: &Ruleset, names: Vec<String>) -> Self {
        build_created_party(content, &rules.party_create, rules.party_size, names)
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
        let built = build_actor(actor, job, &equipment_lookup);
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
    names: Vec<String>,
) -> PartyState {
    let job_lookup = build_job_lookup(content);
    let equipment_lookup = build_equipment_lookup(content);
    let job = job_lookup.get(create_rules.default_job.as_str()).copied();

    let mut roster = HashMap::new();
    let mut active = Vec::new();

    for (index, name) in names.into_iter().enumerate() {
        if active.len() >= party_size {
            break;
        }
        let actor_id = format!("create_{}", index + 1);
        let actor = ActorDefinition {
            id: actor_id.clone(),
            name,
            job_id: create_rules.default_job.clone(),
            level: create_rules.starting_level,
            base_stats: HashMap::new(),
            starting_equipment: create_rules.starting_equipment.clone(),
            spells: Vec::new(),
        };
        let built = build_actor(&actor, job, &equipment_lookup);
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
        for (slot, item_id) in &actor.starting_equipment {
            if allowed_slots.contains(slot) {
                equipment.insert(slot.clone(), item_id.clone());
            }
        }
    }
    let mut derived_stats = apply_job_modifiers(&base_stats, job);
    apply_equipment_modifiers(&mut derived_stats, &equipment, equipment_lookup);

    Actor {
        id: actor.id.clone(),
        name: actor.name.clone(),
        job_id: actor.job_id.clone(),
        level: actor.level,
        base_stats,
        derived_stats,
        equipment,
        spells: actor.spells.clone(),
    }
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
