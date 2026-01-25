use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::dialog::DialogFile;
use crate::encounters::EncountersFile;
use crate::entities::{
    EnemiesFile, EquipmentFile, ItemsFile, JobsFile, NpcsFile, ShopsFile, SpellsFile, VehiclesFile,
};
use crate::events::EventFile;
use crate::maps::MapFile;
use crate::party::PartyFile;
use crate::rules::{PartyMode, RulesFile};
use crate::stats::StatsFile;
use crate::world::WorldsFile;

const BATTLE_POS_MAX_X: i32 = 9;
const BATTLE_POS_MAX_Y: i32 = 5;
const EVENT_TYPES: [&str; 14] = [
    "dialog",
    "narration",
    "set_flag",
    "require_flags",
    "give_item",
    "give_equipment",
    "warp",
    "start_battle",
    "start_dialog",
    "open_shop",
    "npc_show",
    "npc_hide",
    "npc_move",
    "npc_set_sprite",
];

pub fn validate_content(content_dir: impl AsRef<Path>) -> Vec<String> {
    let content_dir = content_dir.as_ref();
    let mut errors = Vec::new();

    let rules_path = content_dir.join("rules.json");
    let worlds_path = content_dir.join("worlds.json");
    let stats_path = content_dir.join("stats.json");
    let encounters_path = content_dir.join("entities").join("encounters.json");
    let npcs_path = content_dir.join("entities").join("npcs.json");
    let jobs_path = content_dir.join("entities").join("jobs.json");
    let spells_path = content_dir.join("entities").join("spells.json");
    let items_path = content_dir.join("entities").join("items.json");
    let equipment_path = content_dir.join("entities").join("equipment.json");
    let enemies_path = content_dir.join("entities").join("enemies.json");
    let vehicles_path = content_dir.join("entities").join("vehicles.json");
    let shops_path = content_dir.join("entities").join("shops.json");

    let rules = load_single(&rules_path, |path| RulesFile::load(path), &mut errors);
    let party = match rules.as_ref().map(|file| &file.party_mode) {
        Some(PartyMode::Predefined) => load_single(
            &content_dir.join("party.json"),
            |path| PartyFile::load(path),
            &mut errors,
        ),
        Some(PartyMode::Create) => load_optional(&content_dir.join("party.json"), |path| {
            PartyFile::load(path)
        }),
        None => None,
    };

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
    let npcs = load_single(&npcs_path, |path| NpcsFile::load(path), &mut errors);

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
    let dialogs = load_dialog_files(content_dir.join("dialog"), &mut errors);

    let map_ids: HashSet<String> = maps.iter().map(|map| map.id.clone()).collect();
    let map_dims: HashMap<&str, (u32, u32)> = maps
        .iter()
        .map(|map| (map.id.as_str(), (map.width, map.height)))
        .collect();
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
        for npc in &map.npcs {
            if let Some(script) = &npc.script {
                if !event_ids.contains(script) {
                    errors.push(format!(
                        "maps/{}: npc '{}' script '{}' not found",
                        map.id, npc.id, script
                    ));
                }
            }
            if npc.pos[0] < 0 || npc.pos[1] < 0 {
                errors.push(format!(
                    "maps/{}: npc '{}' has negative position",
                    map.id, npc.id
                ));
            }
        }
        for sign in &map.signs {
            if sign.pos[0] < 0 || sign.pos[1] < 0 {
                errors.push(format!(
                    "maps/{}: sign '{}' has negative position",
                    map.id, sign.id
                ));
                continue;
            }
            if sign.pos[0] >= map.width as i32 || sign.pos[1] >= map.height as i32 {
                errors.push(format!(
                    "maps/{}: sign '{}' position {:?} out of bounds",
                    map.id, sign.id, sign.pos
                ));
            }
        }
        for transition in &map.transitions {
            if !map_ids.contains(&transition.target_map) {
                errors.push(format!(
                    "maps/{}: transition '{}' target '{}' not found",
                    map.id, transition.id, transition.target_map
                ));
            }
            if transition.pos[0] < 0 || transition.pos[1] < 0 {
                errors.push(format!(
                    "maps/{}: transition '{}' has negative position",
                    map.id, transition.id
                ));
            }
            if let Some((width, height)) = map_dims.get(transition.target_map.as_str()) {
                if transition.target_pos[0] < 0
                    || transition.target_pos[1] < 0
                    || transition.target_pos[0] >= *width as i32
                    || transition.target_pos[1] >= *height as i32
                {
                    errors.push(format!(
                        "maps/{}: transition '{}' target_pos {:?} out of bounds",
                        map.id, transition.id, transition.target_pos
                    ));
                }
            }
        }
    }

    let event_types: HashSet<&str> = EVENT_TYPES.iter().copied().collect();
    for event in &events {
        for step in &event.steps {
            if !event_types.contains(step.r#type.as_str()) {
                errors.push(format!(
                    "events/{}: unknown step type '{}'",
                    event.id, step.r#type
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
        let base_stats: HashSet<&str> = stats
            .as_ref()
            .map(|stats| {
                stats
                    .stats
                    .base
                    .iter()
                    .map(|stat| stat.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let default_jobs = jobs.jobs.iter().filter(|job| job.is_default).count();
        if default_jobs == 0 {
            errors.push("jobs.json: at least one job must be marked is_default".to_string());
        } else if default_jobs > 1 {
            errors.push("jobs.json: only one job can be marked is_default".to_string());
        }
        for job in &jobs.jobs {
            match job.growth.mode.as_str() {
                "table" => {
                    if job.growth.tables.is_empty() {
                        errors.push(format!(
                            "jobs.json: job '{}' table growth requires tables",
                            job.id
                        ));
                    }
                    for stat in &base_stats {
                        if !job.growth.tables.contains_key(*stat) {
                            errors.push(format!(
                                "jobs.json: job '{}' table growth missing stat '{}'",
                                job.id, stat
                            ));
                        }
                    }
                }
                "formula" => {
                    if job.growth.per_level.is_empty() {
                        errors.push(format!(
                            "jobs.json: job '{}' formula growth requires per_level",
                            job.id
                        ));
                    }
                }
                other => {
                    errors.push(format!(
                        "jobs.json: job '{}' has unknown growth mode '{}'",
                        job.id, other
                    ));
                }
            }
            for spell in &job.spells {
                if !spell_ids.contains(spell.id.as_str()) {
                    errors.push(format!(
                        "jobs.json: job '{}' references unknown spell '{}'",
                        job.id, spell.id
                    ));
                }
            }
        }
    }

    if let Some(party) = &party {
        let job_ids: HashSet<&str> = jobs
            .as_ref()
            .map(|jobs| jobs.jobs.iter().map(|job| job.id.as_str()).collect())
            .unwrap_or_default();
        let equipment_ids: HashSet<&str> = equipment
            .as_ref()
            .map(|equipment| {
                equipment
                    .equipment
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let spell_ids: HashSet<&str> = spells
            .as_ref()
            .map(|spells| {
                spells
                    .spells
                    .iter()
                    .map(|spell| spell.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let stat_ids: HashSet<&str> = stats
            .as_ref()
            .map(|stats| {
                stats
                    .stats
                    .base
                    .iter()
                    .map(|stat| stat.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let roster_ids: HashSet<&str> =
            party.roster.iter().map(|actor| actor.id.as_str()).collect();

        for actor in &party.roster {
            if !job_ids.is_empty() && !job_ids.contains(actor.job_id.as_str()) {
                errors.push(format!(
                    "party.json: actor '{}' references unknown job '{}'",
                    actor.id, actor.job_id
                ));
            }
            for stat in actor.base_stats.keys() {
                if !stat_ids.is_empty() && !stat_ids.contains(stat.as_str()) {
                    errors.push(format!(
                        "party.json: actor '{}' references unknown stat '{}'",
                        actor.id, stat
                    ));
                }
            }
            for spell in &actor.spells {
                if !spell_ids.is_empty() && !spell_ids.contains(spell.as_str()) {
                    errors.push(format!(
                        "party.json: actor '{}' references unknown spell '{}'",
                        actor.id, spell
                    ));
                }
            }
            for (slot, item_id) in &actor.starting_equipment {
                if !equipment_ids.is_empty() && !equipment_ids.contains(item_id.as_str()) {
                    errors.push(format!(
                        "party.json: actor '{}' slot '{}' references unknown equipment '{}'",
                        actor.id, slot, item_id
                    ));
                }
            }
        }

        for actor_id in party.starting_party.iter().chain(party.reserve.iter()) {
            if !roster_ids.contains(actor_id.as_str()) {
                errors.push(format!(
                    "party.json: party member '{}' not found in roster",
                    actor_id
                ));
            }
        }
    }

    if let (Some(rules), Some(jobs)) = (&rules, &jobs) {
        if rules.party_mode == PartyMode::Create {
            let default_job = rules.party_create.default_job.as_str();
            if !jobs.jobs.iter().any(|job| job.id == default_job) {
                errors.push(format!(
                    "rules.json: party_create.default_job '{}' not found in jobs.json",
                    default_job
                ));
            }
            let default_count = jobs.jobs.iter().filter(|job| job.is_default).count();
            if default_count == 0 {
                errors.push("jobs.json: create mode requires a default job".to_string());
            } else if default_count > 1 {
                errors.push("jobs.json: create mode requires a single default job".to_string());
            }
            let default_entry = jobs.jobs.iter().find(|job| job.id == default_job);
            if let Some(job) = default_entry {
                if !job.is_default {
                    errors.push(format!(
                        "rules.json: party_create.default_job '{}' must be marked is_default",
                        default_job
                    ));
                }
                if job
                    .unlock_flag
                    .as_ref()
                    .map(|flag| !flag.trim().is_empty())
                    .unwrap_or(false)
                {
                    errors.push(format!(
                        "jobs.json: default job '{}' cannot be gated by unlock_flag",
                        default_job
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

    if let (Some(npcs), false) = (&npcs, dialogs.is_empty()) {
        let dialog_ids: HashSet<&str> = dialogs.iter().map(|dialog| dialog.id.as_str()).collect();
        for npc in &npcs.npcs {
            if !dialog_ids.contains(npc.dialog.as_str()) {
                errors.push(format!(
                    "npcs.json: npc '{}' references unknown dialog '{}'",
                    npc.id, npc.dialog
                ));
            }
        }
    }

    if let (Some(npcs), true) = (&npcs, dialogs.is_empty()) {
        errors.push("dialog/: no dialog files found".to_string());
        for npc in &npcs.npcs {
            errors.push(format!(
                "npcs.json: npc '{}' references dialog '{}'",
                npc.id, npc.dialog
            ));
        }
    }

    if let Some(npcs) = &npcs {
        let npc_ids: HashSet<&str> = npcs.npcs.iter().map(|npc| npc.id.as_str()).collect();
        for map in &maps {
            for npc in &map.npcs {
                if !npc_ids.contains(npc.id.as_str()) {
                    errors.push(format!("maps/{}: npc '{}' not found", map.id, npc.id));
                }
            }
        }
    }

    if !dialogs.is_empty() {
        let event_ids: HashSet<&str> = events.iter().map(|event| event.id.as_str()).collect();
        let shop_ids: HashSet<&str> = shops
            .as_ref()
            .map(|file| file.shops.iter().map(|shop| shop.id.as_str()).collect())
            .unwrap_or_else(HashSet::new);

        for dialog in &dialogs {
            for node in &dialog.nodes {
                if let Some(actions) = &node.actions {
                    for action in actions {
                        match action.r#type.as_str() {
                            "start_event" => {
                                if let Some(event_id) = &action.event {
                                    if !event_ids.contains(event_id.as_str()) {
                                        errors.push(format!(
                                            "dialog/{}: action references unknown event '{}'",
                                            dialog.id, event_id
                                        ));
                                    }
                                } else {
                                    errors.push(format!(
                                        "dialog/{}: start_event missing event id",
                                        dialog.id
                                    ));
                                }
                            }
                            "open_shop" => {
                                if let Some(shop_id) = &action.shop {
                                    if !shop_ids.contains(shop_id.as_str()) {
                                        errors.push(format!(
                                            "dialog/{}: action references unknown shop '{}'",
                                            dialog.id, shop_id
                                        ));
                                    }
                                } else {
                                    errors.push(format!(
                                        "dialog/{}: open_shop missing shop id",
                                        dialog.id
                                    ));
                                }
                            }
                            "set_flag" => {
                                if action.flag.is_none() {
                                    errors.push(format!(
                                        "dialog/{}: set_flag missing flag",
                                        dialog.id
                                    ));
                                }
                            }
                            "give_item" => {
                                if action.item.is_none() {
                                    errors.push(format!(
                                        "dialog/{}: give_item missing item id",
                                        dialog.id
                                    ));
                                }
                            }
                            _ => {
                                errors.push(format!(
                                    "dialog/{}: unknown action type '{}'",
                                    dialog.id, action.r#type
                                ));
                            }
                        }
                    }
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

fn load_optional<T, F>(path: &PathBuf, loader: F) -> Option<T>
where
    F: FnOnce(&Path) -> Result<T, String>,
{
    if !path.exists() {
        return None;
    }
    loader(path).ok()
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

fn load_dialog_files(dir: PathBuf, errors: &mut Vec<String>) -> Vec<DialogFile> {
    let mut dialogs = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("{}: {}", dir.display(), err));
            }
            return dialogs;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match DialogFile::load(&path) {
            Ok(dialog) => dialogs.push(dialog),
            Err(err) => errors.push(err),
        }
    }

    dialogs
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
