use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::dialog::DialogFile;
use crate::encounters::EncountersFile;
use crate::entities::{
    AbilitiesFile, EffectsFile, EnemiesFile, EquipmentFile, ItemsFile, JobsFile, NpcsFile,
    ShopsFile, SpellsFile, StringsFile, VehiclesFile,
};
use crate::events::EventFile;
use crate::maps::{MapChestLoot, MapFile};
use crate::party::PartyFile;
use crate::quests::QuestsFile;
use crate::rules::{PartyMode, RulesFile};
use crate::stats::StatsFile;
use crate::world::WorldsFile;

pub struct Content {
    pub rules: RulesFile,
    pub effects: EffectsFile,
    pub strings: Option<StringsFile>,
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
    pub quests: Vec<QuestsFile>,
    pub cooking: Option<CookingFile>,
    pub map_index: HashMap<String, usize>,
    pub event_index: HashMap<String, usize>,
    pub dialog_index: HashMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CookingFile {
    pub version: u32,
    pub recipes: Vec<CookingRecipe>,
    #[serde(default)]
    pub campfires: Vec<CookingCampfire>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CookingRecipe {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub unlock_flag: Option<String>,
    #[serde(default)]
    pub ingredients: Vec<crate::inventory::InventoryStack>,
    #[serde(default)]
    pub results: MapChestLoot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CookingCampfire {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub recipes: Vec<String>,
}

impl CookingFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl Content {
    pub fn get_map_on_enter_events(&self, map_id: &str) -> Vec<String> {
        let map_index = match self.map_index.get(map_id) {
            Some(index) => *index,
            None => return Vec::new(),
        };
        let map = &self.maps[map_index];
        let mut events = Vec::new();
        for map_event in &map.events {
            if map_event.trigger == "on_enter" {
                events.push(map_event.script.clone());
            }
        }
        events
    }

    pub fn get_map_on_step_events(&self, map_id: &str, pos: (i32, i32)) -> Vec<String> {
        let map_index = match self.map_index.get(map_id) {
            Some(index) => *index,
            None => return Vec::new(),
        };
        let map = &self.maps[map_index];
        let mut events = Vec::new();
        for map_event in &map.events {
            if map_event.trigger != "on_step" {
                continue;
            }
            if let Some(event_pos) = map_event.pos {
                if event_pos[0] == pos.0 && event_pos[1] == pos.1 {
                    events.push(map_event.script.clone());
                }
            }
        }
        events
    }

    pub fn get_zone_on_step_events(
        &self,
        map_id: &str,
        pos: (i32, i32),
        previous_pos: (i32, i32),
    ) -> Vec<String> {
        let map_index = match self.map_index.get(map_id) {
            Some(index) => *index,
            None => return Vec::new(),
        };
        let map = &self.maps[map_index];
        let mut events = Vec::new();
        for map_event in &map.events {
            if map_event.trigger != "on_step" {
                continue;
            }
            let Some(zone_id) = &map_event.zone else {
                continue;
            };
            let zone = map.encounters.iter().find(|z| &z.zone_id == zone_id);
            let Some(zone_rect) = zone.map(|z| z.rect) else {
                continue;
            };
            let in_zone_current = pos.0 >= zone_rect[0]
                && pos.0 < zone_rect[2]
                && pos.1 >= zone_rect[1]
                && pos.1 < zone_rect[3];
            let in_zone_previous = previous_pos.0 >= zone_rect[0]
                && previous_pos.0 < zone_rect[2]
                && previous_pos.1 >= zone_rect[1]
                && previous_pos.1 < zone_rect[3];
            if in_zone_current && !in_zone_previous {
                events.push(map_event.script.clone());
            }
        }
        events
    }

    pub fn ui_text(&self, key: &str) -> Option<&str> {
        self.strings
            .as_ref()
            .and_then(|file| file.strings.get(key).map(|value| value.as_str()))
    }

    pub fn load(content_dir: impl AsRef<Path>) -> Result<Self, Vec<String>> {
        let content_dir = content_dir.as_ref();
        let mut errors = Vec::new();

        let rules = load_single(content_dir.join("rules.json"), RulesFile::load, &mut errors);
        let effects = load_single(
            content_dir.join("effects.json"),
            EffectsFile::load,
            &mut errors,
        );
        let strings = load_optional(
            content_dir.join("ui").join("strings.json"),
            StringsFile::load,
        );
        let party = match rules.as_ref().map(|file| &file.party_mode) {
            Some(PartyMode::Preset) | Some(PartyMode::PresetRename) => {
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
        let quests = load_dir(
            content_dir.join("quests"),
            QuestsFile::load,
            &mut errors,
            "quests",
        );
        let cooking = load_optional(content_dir.join("cooking.json"), CookingFile::load);

        if !errors.is_empty() {
            return Err(errors);
        }

        let rules = rules.unwrap();
        let effects = effects.unwrap();
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
            effects,
            strings,
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
            quests,
            cooking,
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
