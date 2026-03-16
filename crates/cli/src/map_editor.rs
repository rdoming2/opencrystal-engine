use std::fs;
use std::path::Path;

use engine::content::CookingFile;
use engine::encounters::EncountersFile;
use engine::entities::{EquipmentFile, ItemsFile, NpcsFile, VehiclesFile};
use engine::events::EventFile;
use engine::inventory::InventoryStack;
use engine::io::{load_json, write_json_pretty};
use engine::maps::{
    EncounterZone, MapCampfire, MapChest, MapChestLoot, MapCurrencyStack, MapDoor, MapEvent,
    MapFile, MapLoop, MapNpc, MapPuzzle, MapSign, MapTransition, MapVehicle, TileLegend,
};
use engine::rules::RulesFile;
use tui::dialog::prompt_text;
use tui::input::InputBindings;
use tui::map_editor::{
    EncounterZone as UiEncounterZone, FollowSaveAction, InventoryStack as UiInventoryStack,
    LegendEntry as UiLegendEntry, MapCampfire as UiMapCampfire, MapChest as UiMapChest,
    MapChestLoot as UiMapChestLoot, MapCurrencyStack as UiCurrencyStack, MapData as UiMapData,
    MapDoor as UiMapDoor, MapEditorConfig, MapEditorOutcome, MapEvent as UiMapEvent,
    MapNpc as UiMapNpc, MapPuzzle as UiMapPuzzle, MapSign as UiMapSign,
    MapTransition as UiMapTransition, MapVehicle as UiMapVehicle,
};
use tui::session::TuiSession;

pub fn run_map_editor(content_dir: &Path, id: &str) -> Result<(), String> {
    let mut session = TuiSession::start().map_err(|err| format!("Failed to start TUI: {}", err))?;

    let event_ids = load_event_ids(content_dir);
    let vehicle_ids = load_vehicle_ids(content_dir);
    let npc_ids = load_npc_ids(content_dir);
    let item_ids = load_item_ids(content_dir);
    let equipment_ids = load_equipment_ids(content_dir);
    let currency_ids = load_currency_ids(content_dir);
    let campfire_ids = load_campfire_ids(content_dir);
    let encounter_table_ids = load_encounter_table_ids(content_dir);
    let mut current_map_id = id.to_string();
    let mut start_cursor = None;

    loop {
        let map_path = content_dir
            .join("maps")
            .join(format!("{}.json", current_map_id));
        let map = if map_path.exists() {
            MapFile::load(&map_path).map_err(|err| err.to_string())?
        } else {
            match prompt_new_map(&mut session, &current_map_id)? {
                Some(map) => map,
                None => {
                    session.finish().ok();
                    return Ok(());
                }
            }
        };

        let map_ids = load_map_ids(content_dir);
        let encounter_zone_ids = load_encounter_zone_ids(&map);
        let config = MapEditorConfig {
            map: map_to_ui(map),
            start_cursor,
            map_ids,
            event_ids: event_ids.clone(),
            vehicle_ids: vehicle_ids.clone(),
            npc_ids: npc_ids.clone(),
            item_ids: item_ids.clone(),
            equipment_ids: equipment_ids.clone(),
            currency_ids: currency_ids.clone(),
            campfire_ids: campfire_ids.clone(),
            encounter_zone_ids,
            encounter_table_ids: encounter_table_ids.clone(),
        };

        let outcome = match tui::map_editor::run_map_editor(&mut session, config) {
            Ok(outcome) => outcome,
            Err(err) => {
                session.finish().ok();
                return Err(err.to_string());
            }
        };

        match outcome {
            MapEditorOutcome::Saved(map) => {
                save_map(content_dir, &current_map_id, &ui_to_map(map))?;
                println!("Saved maps/{}.json", current_map_id);
                session.finish().ok();
                return Ok(());
            }
            MapEditorOutcome::Cancelled => {
                println!("Map edit cancelled");
                session.finish().ok();
                return Ok(());
            }
            MapEditorOutcome::FollowTransition(request) => {
                if matches!(request.save_action, FollowSaveAction::Save) {
                    save_map(content_dir, &current_map_id, &ui_to_map(request.map))?;
                    println!("Saved maps/{}.json", current_map_id);
                }

                if !target_map_exists(content_dir, &request.target_map)
                    && !prompt_create_target_map(content_dir, &mut session, &request.target_map)?
                {
                    start_cursor = None;
                    continue;
                }

                current_map_id = request.target_map;
                start_cursor = Some(request.target_pos);
            }
        }
    }
}

fn save_map(content_dir: &Path, id: &str, map: &MapFile) -> Result<(), String> {
    let map_path = content_dir.join("maps").join(format!("{}.json", id));
    if let Some(parent) = map_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    write_json_pretty(&map_path, map)?;
    Ok(())
}

fn target_map_exists(content_dir: &Path, id: &str) -> bool {
    content_dir
        .join("maps")
        .join(format!("{}.json", id))
        .exists()
}

fn prompt_create_target_map(
    content_dir: &Path,
    session: &mut TuiSession,
    id: &str,
) -> Result<bool, String> {
    let options = vec!["Create target map".to_string(), "Cancel jump".to_string()];
    let bindings = InputBindings::default_bindings();
    let choice = tui::dialog::prompt_choice(
        session,
        &bindings,
        "Missing Target Map",
        &format!("Target '{}' does not exist.", id),
        &options,
        0,
    )
    .map_err(|err| err.to_string())?;
    if choice != Some(0) {
        return Ok(false);
    }
    match prompt_new_map(session, id)? {
        Some(map) => {
            save_map(content_dir, id, &map)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

fn prompt_new_map(session: &mut TuiSession, id: &str) -> Result<Option<MapFile>, String> {
    let name_default = title_case_id(id);
    let name = prompt_text(session, "New Map", "Map name:", &name_default, 48)
        .map_err(|err| err.to_string())?;
    let Some(name) = name else {
        return Ok(None);
    };
    let world = prompt_text(session, "New Map", "World id:", "default_world", 32)
        .map_err(|err| err.to_string())?;
    let Some(world) = world else {
        return Ok(None);
    };
    let width =
        prompt_text(session, "New Map", "Width:", "20", 6).map_err(|err| err.to_string())?;
    let Some(width) = width else {
        return Ok(None);
    };
    let height =
        prompt_text(session, "New Map", "Height:", "15", 6).map_err(|err| err.to_string())?;
    let Some(height) = height else {
        return Ok(None);
    };
    let width: u32 = width.trim().parse().unwrap_or(20).max(1);
    let height: u32 = height.trim().parse().unwrap_or(15).max(1);
    let tiles = vec![".".repeat(width as usize); height as usize];
    let legend = std::collections::HashMap::from([(
        ".".to_string(),
        TileLegend {
            tile: "floor".to_string(),
            passable: true,
            palette: Some("green".to_string()),
        },
    )]);
    Ok(Some(MapFile {
        version: 1,
        id: id.to_string(),
        name,
        hide_name: false,
        world,
        width,
        height,
        loop_config: MapLoop::default(),
        tiles,
        legend,
        encounters: Vec::new(),
        encounter_rate: 0.0,
        events: Vec::new(),
        npcs: Vec::new(),
        signs: Vec::new(),
        chests: Vec::new(),
        doors: Vec::new(),
        puzzles: Vec::new(),
        campfires: Vec::new(),
        allow_save: true,
        save_points: Vec::new(),
        transitions: Vec::new(),
        vehicles: Vec::new(),
    }))
}

fn load_map_ids(content_dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    let maps_dir = content_dir.join("maps");
    if let Ok(entries) = fs::read_dir(maps_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Ok(map) = MapFile::load(entry.path()) {
                ids.push(map.id);
            } else if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_string());
            }
        }
    }
    ids
}

fn load_event_ids(content_dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    let events_dir = content_dir.join("events");
    if let Ok(entries) = fs::read_dir(events_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Ok(event) = EventFile::load(entry.path()) {
                ids.push(event.id);
            } else if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_string());
            }
        }
    }
    ids
}

fn load_vehicle_ids(content_dir: &Path) -> Vec<String> {
    let path = content_dir.join("entities").join("vehicles.json");
    if let Ok(file) = load_json::<VehiclesFile>(&path) {
        return file.vehicles.into_iter().map(|entry| entry.id).collect();
    }
    Vec::new()
}

fn load_npc_ids(content_dir: &Path) -> Vec<String> {
    let path = content_dir.join("entities").join("npcs.json");
    if let Ok(file) = load_json::<NpcsFile>(&path) {
        return file.npcs.into_iter().map(|entry| entry.id).collect();
    }
    Vec::new()
}

fn load_item_ids(content_dir: &Path) -> Vec<String> {
    let path = content_dir.join("entities").join("items.json");
    if let Ok(file) = load_json::<ItemsFile>(&path) {
        return file.items.into_iter().map(|entry| entry.id).collect();
    }
    Vec::new()
}

fn load_equipment_ids(content_dir: &Path) -> Vec<String> {
    let path = content_dir.join("entities").join("equipment.json");
    if let Ok(file) = load_json::<EquipmentFile>(&path) {
        return file.equipment.into_iter().map(|entry| entry.id).collect();
    }
    Vec::new()
}

fn load_currency_ids(content_dir: &Path) -> Vec<String> {
    let path = content_dir.join("rules.json");
    if let Ok(file) = load_json::<RulesFile>(&path) {
        return file
            .game
            .currencies
            .into_iter()
            .map(|entry| entry.id)
            .collect();
    }
    Vec::new()
}

fn load_campfire_ids(content_dir: &Path) -> Vec<String> {
    let path = content_dir.join("cooking.json");
    if let Ok(file) = load_json::<CookingFile>(&path) {
        return file.campfires.into_iter().map(|entry| entry.id).collect();
    }
    Vec::new()
}

fn load_encounter_zone_ids(map: &MapFile) -> Vec<String> {
    map.encounters
        .iter()
        .map(|entry| entry.zone_id.clone())
        .collect()
}

fn load_encounter_table_ids(content_dir: &Path) -> Vec<String> {
    let path = content_dir.join("entities").join("encounters.json");
    if let Ok(file) = load_json::<EncountersFile>(&path) {
        return file.tables.into_iter().map(|entry| entry.id).collect();
    }
    Vec::new()
}

fn map_to_ui(map: MapFile) -> UiMapData {
    let tiles = map
        .tiles
        .iter()
        .map(|row| row.chars().collect::<Vec<_>>())
        .collect();
    let legend = map
        .legend
        .iter()
        .filter_map(|(glyph, entry)| {
            let ch = glyph.chars().next()?;
            Some(UiLegendEntry {
                glyph: ch,
                tile: entry.tile.clone(),
                passable: entry.passable,
                palette: entry.palette.clone(),
            })
        })
        .collect();
    UiMapData {
        version: map.version,
        id: map.id,
        name: map.name,
        hide_name: map.hide_name,
        world: map.world,
        width: map.width,
        height: map.height,
        loop_x: map.loop_config.x,
        loop_y: map.loop_config.y,
        tiles,
        legend,
        encounters: map
            .encounters
            .into_iter()
            .map(|entry| UiEncounterZone {
                zone_id: entry.zone_id,
                rect: entry.rect,
                table: entry.table,
            })
            .collect(),
        encounter_rate: map.encounter_rate,
        events: map
            .events
            .into_iter()
            .map(|event| UiMapEvent {
                id: event.id,
                trigger: event.trigger,
                script: event.script,
                zone: event.zone,
                pos: event.pos,
            })
            .collect(),
        npcs: map
            .npcs
            .into_iter()
            .map(|npc| UiMapNpc {
                id: npc.id,
                pos: npc.pos,
                script: npc.script,
                requires_flags: npc.requires_flags,
            })
            .collect(),
        signs: map
            .signs
            .into_iter()
            .map(|sign| UiMapSign {
                id: sign.id,
                pos: sign.pos,
                glyph: sign.glyph,
                palette: sign.palette,
                text: sign.text,
            })
            .collect(),
        chests: map
            .chests
            .into_iter()
            .map(|chest| UiMapChest {
                id: chest.id,
                pos: chest.pos,
                glyph_closed: chest.glyph_closed,
                glyph_open: chest.glyph_open,
                palette: chest.palette,
                opened_flag: chest.opened_flag,
                loot: UiMapChestLoot {
                    items: chest
                        .loot
                        .items
                        .into_iter()
                        .map(|item| UiInventoryStack {
                            id: item.id,
                            qty: item.qty,
                        })
                        .collect(),
                    equipment: chest
                        .loot
                        .equipment
                        .into_iter()
                        .map(|item| UiInventoryStack {
                            id: item.id,
                            qty: item.qty,
                        })
                        .collect(),
                    currency: chest
                        .loot
                        .currency
                        .into_iter()
                        .map(|item| UiCurrencyStack {
                            id: item.id,
                            amount: item.amount,
                        })
                        .collect(),
                },
            })
            .collect(),
        doors: map
            .doors
            .into_iter()
            .map(|door| UiMapDoor {
                id: door.id,
                pos: door.pos,
                requires_flag: door.requires_flag,
                locked_text: door.locked_text,
                locked_event: door.locked_event,
                target_map: door.target_map,
                target_pos: door.target_pos,
                return_to_last: door.return_to_last,
                glyph: door.glyph,
                palette: door.palette,
            })
            .collect(),
        puzzles: map
            .puzzles
            .into_iter()
            .map(|puzzle| UiMapPuzzle {
                id: puzzle.id,
                pos: puzzle.pos,
                requires_flags: puzzle.requires_flags,
                text: puzzle.text,
                event: puzzle.event,
                set_flag: puzzle.set_flag,
                glyph: puzzle.glyph,
                palette: puzzle.palette,
            })
            .collect(),
        campfires: map
            .campfires
            .into_iter()
            .map(|campfire| UiMapCampfire {
                id: campfire.id,
                pos: campfire.pos,
                campfire_id: campfire.campfire_id,
                requires_flags: campfire.requires_flags,
                glyph: campfire.glyph,
                palette: campfire.palette,
            })
            .collect(),
        allow_save: map.allow_save,
        save_points: map.save_points,
        transitions: map
            .transitions
            .into_iter()
            .map(|transition| UiMapTransition {
                id: transition.id,
                pos: transition.pos,
                target_map: transition.target_map,
                target_pos: transition.target_pos,
                label: transition.label,
                requires_flag: transition.requires_flag,
                cost: transition.cost.map(|cost| UiCurrencyStack {
                    id: cost.id,
                    amount: cost.amount,
                }),
                return_to_last: transition.return_to_last,
                glyph: transition.glyph,
                palette: transition.palette,
            })
            .collect(),
        vehicles: map
            .vehicles
            .into_iter()
            .map(|vehicle| UiMapVehicle {
                vehicle_id: vehicle.vehicle_id,
                pos: vehicle.pos,
                requires_flags: vehicle.requires_flags,
            })
            .collect(),
    }
}

fn ui_to_map(map: UiMapData) -> MapFile {
    let tiles = map
        .tiles
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>();
    let legend = map
        .legend
        .iter()
        .map(|entry| {
            (
                entry.glyph.to_string(),
                TileLegend {
                    tile: entry.tile.clone(),
                    passable: entry.passable,
                    palette: entry.palette.clone(),
                },
            )
        })
        .collect();
    MapFile {
        version: map.version,
        id: map.id,
        name: map.name,
        hide_name: map.hide_name,
        world: map.world,
        width: map.width,
        height: map.height,
        loop_config: MapLoop {
            x: map.loop_x,
            y: map.loop_y,
        },
        tiles,
        legend,
        encounters: map
            .encounters
            .into_iter()
            .map(|entry| EncounterZone {
                zone_id: entry.zone_id,
                rect: entry.rect,
                table: entry.table,
            })
            .collect(),
        encounter_rate: map.encounter_rate,
        events: map
            .events
            .into_iter()
            .map(|event| MapEvent {
                id: event.id,
                trigger: event.trigger,
                script: event.script,
                zone: event.zone,
                pos: event.pos,
            })
            .collect(),
        npcs: map
            .npcs
            .into_iter()
            .map(|npc| MapNpc {
                id: npc.id,
                pos: npc.pos,
                script: npc.script,
                requires_flags: npc.requires_flags,
            })
            .collect(),
        signs: map
            .signs
            .into_iter()
            .map(|sign| MapSign {
                id: sign.id,
                pos: sign.pos,
                glyph: sign.glyph,
                palette: sign.palette,
                text: sign.text,
            })
            .collect(),
        chests: map
            .chests
            .into_iter()
            .map(|chest| MapChest {
                id: chest.id,
                pos: chest.pos,
                glyph_closed: chest.glyph_closed,
                glyph_open: chest.glyph_open,
                palette: chest.palette,
                opened_flag: chest.opened_flag,
                loot: MapChestLoot {
                    items: chest
                        .loot
                        .items
                        .into_iter()
                        .map(|item| InventoryStack {
                            id: item.id,
                            qty: item.qty,
                        })
                        .collect(),
                    equipment: chest
                        .loot
                        .equipment
                        .into_iter()
                        .map(|item| InventoryStack {
                            id: item.id,
                            qty: item.qty,
                        })
                        .collect(),
                    currency: chest
                        .loot
                        .currency
                        .into_iter()
                        .map(|item| MapCurrencyStack {
                            id: item.id,
                            amount: item.amount,
                        })
                        .collect(),
                },
            })
            .collect(),
        doors: map
            .doors
            .into_iter()
            .map(|door| MapDoor {
                id: door.id,
                pos: door.pos,
                requires_flag: door.requires_flag,
                locked_text: door.locked_text,
                locked_event: door.locked_event,
                target_map: door.target_map,
                target_pos: door.target_pos,
                return_to_last: door.return_to_last,
                glyph: door.glyph,
                palette: door.palette,
            })
            .collect(),
        puzzles: map
            .puzzles
            .into_iter()
            .map(|puzzle| MapPuzzle {
                id: puzzle.id,
                pos: puzzle.pos,
                requires_flags: puzzle.requires_flags,
                text: puzzle.text,
                event: puzzle.event,
                set_flag: puzzle.set_flag,
                glyph: puzzle.glyph,
                palette: puzzle.palette,
            })
            .collect(),
        campfires: map
            .campfires
            .into_iter()
            .map(|campfire| MapCampfire {
                id: campfire.id,
                pos: campfire.pos,
                campfire_id: campfire.campfire_id,
                requires_flags: campfire.requires_flags,
                glyph: campfire.glyph,
                palette: campfire.palette,
            })
            .collect(),
        allow_save: map.allow_save,
        save_points: map.save_points,
        transitions: map
            .transitions
            .into_iter()
            .map(|transition| MapTransition {
                id: transition.id,
                pos: transition.pos,
                target_map: transition.target_map,
                target_pos: transition.target_pos,
                label: transition.label,
                requires_flag: transition.requires_flag,
                cost: transition.cost.map(|cost| MapCurrencyStack {
                    id: cost.id,
                    amount: cost.amount,
                }),
                return_to_last: transition.return_to_last,
                glyph: transition.glyph,
                palette: transition.palette,
            })
            .collect(),
        vehicles: map
            .vehicles
            .into_iter()
            .map(|vehicle| MapVehicle {
                vehicle_id: vehicle.vehicle_id,
                pos: vehicle.pos,
                requires_flags: vehicle.requires_flags,
            })
            .collect(),
    }
}

fn title_case_id(id: &str) -> String {
    id.split(|ch| ch == '_' || ch == '-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
