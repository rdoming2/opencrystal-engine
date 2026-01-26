use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::dialog::DialogFile;
use crate::encounters::EncountersFile;
use crate::entities::{
    AbilitiesFile, EnemiesFile, EquipmentFile, ItemsFile, JobsFile, NpcsFile, ShopsFile,
    SpellsFile, VehiclesFile,
};
use crate::events::EventFile;
use crate::maps::MapFile;
use crate::party::PartyFile;
use crate::rules::{PartyMode, RulesFile};
use crate::stats::StatsFile;
use crate::world::WorldsFile;

pub struct Content {
    pub rules: RulesFile,
    pub worlds: WorldsFile,
    pub stats: StatsFile,
    pub encounters: EncountersFile,
    pub jobs: JobsFile,
    pub spells: SpellsFile,
    pub abilities: AbilitiesFile,
    pub items: ItemsFile,
    pub equipment: EquipmentFile,
    pub enemies: EnemiesFile,
    pub vehicles: VehiclesFile,
    pub shops: ShopsFile,
    pub npcs: NpcsFile,
    pub party: Option<PartyFile>,
    pub maps: Vec<MapFile>,
    pub events: Vec<EventFile>,
    pub dialogs: Vec<DialogFile>,
    pub map_index: HashMap<String, usize>,
    pub event_index: HashMap<String, usize>,
    pub dialog_index: HashMap<String, usize>,
}

impl Content {
    pub fn load(content_dir: impl AsRef<Path>) -> Result<Self, Vec<String>> {
        let content_dir = content_dir.as_ref();
        let mut errors = Vec::new();

        let rules = load_single(content_dir.join("rules.json"), RulesFile::load, &mut errors);
        let party = match rules.as_ref().map(|file| &file.party_mode) {
            Some(PartyMode::Predefined) => {
                load_single(content_dir.join("party.json"), PartyFile::load, &mut errors)
            }
            Some(PartyMode::Create) => {
                load_optional(content_dir.join("party.json"), PartyFile::load)
            }
            None => None,
        };
        let worlds = load_single(
            content_dir.join("worlds.json"),
            WorldsFile::load,
            &mut errors,
        );
        let stats = load_single(content_dir.join("stats.json"), StatsFile::load, &mut errors);
        let encounters = load_single(
            content_dir.join("entities").join("encounters.json"),
            EncountersFile::load,
            &mut errors,
        );
        let jobs = load_single(
            content_dir.join("entities").join("jobs.json"),
            JobsFile::load,
            &mut errors,
        );
        let spells = load_single(
            content_dir.join("entities").join("spells.json"),
            SpellsFile::load,
            &mut errors,
        );
        let abilities = load_single(
            content_dir.join("entities").join("abilities.json"),
            AbilitiesFile::load,
            &mut errors,
        );
        let items = load_single(
            content_dir.join("entities").join("items.json"),
            ItemsFile::load,
            &mut errors,
        );
        let equipment = load_single(
            content_dir.join("entities").join("equipment.json"),
            EquipmentFile::load,
            &mut errors,
        );
        let enemies = load_single(
            content_dir.join("entities").join("enemies.json"),
            EnemiesFile::load,
            &mut errors,
        );
        let vehicles = load_single(
            content_dir.join("entities").join("vehicles.json"),
            VehiclesFile::load,
            &mut errors,
        );
        let shops = load_single(
            content_dir.join("entities").join("shops.json"),
            ShopsFile::load,
            &mut errors,
        );
        let npcs = load_single(
            content_dir.join("entities").join("npcs.json"),
            NpcsFile::load,
            &mut errors,
        );

        let maps = load_dir(content_dir.join("maps"), MapFile::load, &mut errors, "maps");
        let events = load_dir(
            content_dir.join("events"),
            EventFile::load,
            &mut errors,
            "events",
        );
        let dialogs = load_dir(
            content_dir.join("dialog"),
            DialogFile::load,
            &mut errors,
            "dialog",
        );

        if !errors.is_empty() {
            return Err(errors);
        }

        let rules = rules.unwrap();
        let worlds = worlds.unwrap();
        let stats = stats.unwrap();
        let encounters = encounters.unwrap();
        let jobs = jobs.unwrap();
        let spells = spells.unwrap();
        let abilities = abilities.unwrap();
        let items = items.unwrap();
        let equipment = equipment.unwrap();
        let enemies = enemies.unwrap();
        let vehicles = vehicles.unwrap();
        let shops = shops.unwrap();
        let npcs = npcs.unwrap();

        let map_index = maps
            .iter()
            .enumerate()
            .map(|(idx, map)| (map.id.clone(), idx))
            .collect();
        let event_index = events
            .iter()
            .enumerate()
            .map(|(idx, event)| (event.id.clone(), idx))
            .collect();
        let dialog_index = dialogs
            .iter()
            .enumerate()
            .map(|(idx, dialog)| (dialog.id.clone(), idx))
            .collect();

        Ok(Self {
            rules,
            worlds,
            stats,
            encounters,
            jobs,
            spells,
            abilities,
            items,
            equipment,
            enemies,
            vehicles,
            shops,
            npcs,
            party,
            maps,
            events,
            dialogs,
            map_index,
            event_index,
            dialog_index,
        })
    }
}

fn load_single<T>(
    path: PathBuf,
    loader: fn(PathBuf) -> Result<T, String>,
    errors: &mut Vec<String>,
) -> Option<T> {
    if !path.exists() {
        errors.push(format!("{}: file not found", path.display()));
        return None;
    }
    match loader(path.clone()) {
        Ok(data) => Some(data),
        Err(err) => {
            errors.push(err);
            None
        }
    }
}

fn load_optional<T>(path: PathBuf, loader: fn(PathBuf) -> Result<T, String>) -> Option<T> {
    if !path.exists() {
        return None;
    }
    loader(path).ok()
}

fn load_dir<T>(
    dir: PathBuf,
    loader: fn(PathBuf) -> Result<T, String>,
    errors: &mut Vec<String>,
    label: &str,
) -> Vec<T> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(format!("{}: {}", dir.display(), err));
            return files;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match loader(path.clone()) {
            Ok(file) => files.push(file),
            Err(err) => errors.push(format!("{}: {}", label, err)),
        }
    }

    if files.is_empty() {
        errors.push(format!("{}: no files found", dir.display()));
    }

    files
}
