use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::encounters::EncountersFile;
use crate::entities::{
    EnemiesFile, EquipmentFile, ItemsFile, JobsFile, ShopsFile, SpellsFile, VehiclesFile,
};
use crate::events::EventFile;
use crate::maps::MapFile;
use crate::rules::RulesFile;
use crate::stats::StatsFile;
use crate::world::WorldsFile;

const BATTLE_POS_MAX_X: i32 = 9;
const BATTLE_POS_MAX_Y: i32 = 5;

pub fn validate_content(content_dir: impl AsRef<Path>) -> Vec<String> {
    let content_dir = content_dir.as_ref();
    let mut errors = Vec::new();

    let rules_path = content_dir.join("rules.json");
    let worlds_path = content_dir.join("worlds.json");
    let stats_path = content_dir.join("stats.json");
    let encounters_path = content_dir.join("entities").join("encounters.json");
    let jobs_path = content_dir.join("entities").join("jobs.json");
    let spells_path = content_dir.join("entities").join("spells.json");
    let items_path = content_dir.join("entities").join("items.json");
    let equipment_path = content_dir.join("entities").join("equipment.json");
    let enemies_path = content_dir.join("entities").join("enemies.json");
    let vehicles_path = content_dir.join("entities").join("vehicles.json");
    let shops_path = content_dir.join("entities").join("shops.json");

    let rules = load_single(&rules_path, |path| RulesFile::load(path), &mut errors);
    let worlds = load_single(&worlds_path, |path| WorldsFile::load(path), &mut errors);
    let stats = load_single(&stats_path, |path| StatsFile::load(path), &mut errors);
    let encounters = load_single(
        &encounters_path,
        |path| EncountersFile::load(path),
        &mut errors,
    );
    let jobs = load_single(&jobs_path, |path| JobsFile::load(path), &mut errors);
    let spells = load_single(&spells_path, |path| SpellsFile::load(path), &mut errors);
    let items = load_single(&items_path, |path| ItemsFile::load(path), &mut errors);
    let equipment = load_single(
        &equipment_path,
        |path| EquipmentFile::load(path),
        &mut errors,
    );
    let enemies = load_single(&enemies_path, |path| EnemiesFile::load(path), &mut errors);
    let vehicles = load_single(&vehicles_path, |path| VehiclesFile::load(path), &mut errors);
    let shops = load_single(&shops_path, |path| ShopsFile::load(path), &mut errors);

    if let Some(rules) = &rules {
        if rules.game.party_size > 4 {
            errors.push("rules.json: party_size must be <= 4".to_string());
        }
    }

    if let Some(stats) = &stats {
        let base_ids: HashSet<&str> = stats
            .stats
            .base
            .iter()
            .map(|stat| stat.id.as_str())
            .collect();
        if !base_ids.contains("hp") {
            errors.push("stats.json: base stats must include 'hp'".to_string());
        }
        if !base_ids.contains("mp") {
            errors.push("stats.json: base stats must include 'mp'".to_string());
        }
    }

    let maps = load_map_files(content_dir.join("maps"), &mut errors);
    let events = load_event_files(content_dir.join("events"), &mut errors);

    let map_ids: HashSet<String> = maps.iter().map(|map| map.id.clone()).collect();
    if let Some(worlds) = &worlds {
        for world in &worlds.worlds {
            if !map_ids.contains(&world.starting_map) {
                errors.push(format!(
                    "worlds.json: world '{}' starting_map '{}' not found",
                    world.id, world.starting_map
                ));
            }
        }
    }

    if let Some(encounters) = &encounters {
        let tables: HashSet<&str> = encounters
            .tables
            .iter()
            .map(|table| table.id.as_str())
            .collect();
        for map in &maps {
            for zone in &map.encounters {
                if !tables.contains(zone.table.as_str()) {
                    errors.push(format!(
                        "maps/{}: encounter table '{}' not found",
                        map.id, zone.table
                    ));
                }
            }
        }
    }

    if let Some(shops) = &shops {
        let shop_ids: HashSet<&str> = shops.shops.iter().map(|shop| shop.id.as_str()).collect();
        for map in &maps {
            for shop in &map.shops {
                if !shop_ids.contains(shop.id.as_str()) {
                    errors.push(format!("maps/{}: shop '{}' not found", map.id, shop.id));
                }
            }
        }
    }

    let event_ids: HashSet<String> = events.iter().map(|event| event.id.clone()).collect();
    for map in &maps {
        for event in &map.events {
            if !event_ids.contains(&event.script) {
                errors.push(format!(
                    "maps/{}: event script '{}' not found",
                    map.id, event.script
                ));
            }
        }
    }

    if let (Some(spells), Some(jobs)) = (&spells, &jobs) {
        let spell_ids: HashSet<&str> = spells
            .spells
            .iter()
            .map(|spell| spell.id.as_str())
            .collect();
        for job in &jobs.jobs {
            for spell in &job.spells {
                if !spell_ids.contains(spell.as_str()) {
                    errors.push(format!(
                        "jobs.json: job '{}' references unknown spell '{}'",
                        job.id, spell
                    ));
                }
            }
        }
    }

    if let (Some(items), Some(equipment), Some(shops)) = (&items, &equipment, &shops) {
        let item_ids: HashSet<&str> = items.items.iter().map(|item| item.id.as_str()).collect();
        let equipment_ids: HashSet<&str> = equipment
            .equipment
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        for shop in &shops.shops {
            for entry in &shop.inventory {
                if !item_ids.contains(entry.item.as_str())
                    && !equipment_ids.contains(entry.item.as_str())
                {
                    errors.push(format!(
                        "shops.json: shop '{}' references unknown item '{}'",
                        shop.id, entry.item
                    ));
                }
            }
        }
    }

    if let (Some(enemies), Some(encounters)) = (&enemies, &encounters) {
        let enemy_ids: HashSet<&str> = enemies
            .enemies
            .iter()
            .map(|enemy| enemy.id.as_str())
            .collect();
        for table in &encounters.tables {
            for entry in &table.entries {
                for member in &entry.formation {
                    if !enemy_ids.contains(member.enemy.as_str()) {
                        errors.push(format!(
                            "encounters.json: table '{}' references unknown enemy '{}'",
                            table.id, member.enemy
                        ));
                    }
                    if member.pos[0] < 0 || member.pos[1] < 0 {
                        errors.push(format!(
                            "encounters.json: table '{}' enemy '{}' has negative position",
                            table.id, member.enemy
                        ));
                    }
                    if member.pos[0] > BATTLE_POS_MAX_X || member.pos[1] > BATTLE_POS_MAX_Y {
                        errors.push(format!(
                            "encounters.json: table '{}' enemy '{}' position {:?} exceeds battle grid",
                            table.id, member.enemy, member.pos
                        ));
                    }
                }
            }
        }
    }

    if let (Some(vehicles), Some(worlds)) = (&vehicles, &worlds) {
        let vehicle_ids: HashSet<&str> = vehicles
            .vehicles
            .iter()
            .map(|vehicle| vehicle.id.as_str())
            .collect();
        for world in &worlds.worlds {
            for vehicle in &world.vehicles {
                if !vehicle_ids.contains(vehicle.as_str()) {
                    errors.push(format!(
                        "worlds.json: world '{}' references unknown vehicle '{}'",
                        world.id, vehicle
                    ));
                }
            }
        }
    }

    errors
}

fn load_single<T, F>(path: &PathBuf, loader: F, errors: &mut Vec<String>) -> Option<T>
where
    F: FnOnce(&Path) -> Result<T, String>,
{
    if !path.exists() {
        errors.push(format!("{}: file not found", path.display()));
        return None;
    }
    match loader(path) {
        Ok(data) => Some(data),
        Err(err) => {
            errors.push(err);
            None
        }
    }
}

fn load_map_files(dir: PathBuf, errors: &mut Vec<String>) -> Vec<MapFile> {
    let mut maps = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(format!("{}: {}", dir.display(), err));
            return maps;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match MapFile::load(&path) {
            Ok(map) => {
                if let Err(err) = validate_map(&map) {
                    errors.push(format!("{}: {}", path.display(), err));
                }
                maps.push(map);
            }
            Err(err) => errors.push(err),
        }
    }

    if maps.is_empty() {
        errors.push(format!("{}: no map files found", dir.display()));
    }

    maps
}

fn load_event_files(dir: PathBuf, errors: &mut Vec<String>) -> Vec<EventFile> {
    let mut events = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(format!("{}: {}", dir.display(), err));
            return events;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match EventFile::load(&path) {
            Ok(event) => events.push(event),
            Err(err) => errors.push(err),
        }
    }

    events
}

fn validate_map(map: &MapFile) -> Result<(), String> {
    if map.tiles.len() != map.height as usize {
        return Err(format!(
            "map '{}' tiles height {} does not match height {}",
            map.id,
            map.tiles.len(),
            map.height
        ));
    }
    for (row_index, row) in map.tiles.iter().enumerate() {
        if row.chars().count() != map.width as usize {
            return Err(format!(
                "map '{}' row {} length {} does not match width {}",
                map.id,
                row_index,
                row.chars().count(),
                map.width
            ));
        }
    }
    Ok(())
}
