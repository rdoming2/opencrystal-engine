use std::collections::HashMap;
use std::path::Path;

use engine::encounters::{EncounterEntry, EncounterTable, EncountersFile};
use engine::entities::{
    AbilitiesFile, AbilityDefinition, AbilityEffect, EnemiesFile, EnemyDefinition, EnemySprite,
    EquipmentDefinition, EquipmentFile, ItemDefinition, ItemEffect, ItemUsage, ItemsFile,
    JobDefinition, JobEquipment, JobSprite, JobsFile, NpcBehavior, NpcDefinition, NpcsFile,
    ShopDefinition, ShopsFile, SpellCost, SpellDefinition, SpellEffect, SpellsFile,
    VehicleDefinition, VehiclesFile,
};
use engine::io::{load_json, write_json_pretty};
use engine::stats::StatsFile;

use super::args::BuildNewOptions;
use super::common::{load_or_default, resolve_content_dir, title_case_id};

pub(crate) fn run_build_new(args: &[String]) {
    let options = BuildNewOptions::from_args(args);
    let Some(kind) = options.kind else {
        eprintln!("Missing kind. Example: cryst build new spell cure");
        return;
    };
    let Some(id) = options.id else {
        eprintln!("Missing id. Example: cryst build new spell cure");
        return;
    };

    let content_dir = match resolve_content_dir(options.content_dir.as_deref()) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    let display_name = options
        .name
        .as_deref()
        .map(|name| name.to_string())
        .unwrap_or_else(|| title_case_id(&id));

    let result = match kind.as_str() {
        "spell" => build_new_spell(&content_dir, &id, &display_name, options.force),
        "ability" => build_new_ability(&content_dir, &id, &display_name, options.force),
        "item" => build_new_item(&content_dir, &id, &display_name, options.force),
        "equipment" => build_new_equipment(&content_dir, &id, &display_name, options.force),
        "enemy" => build_new_enemy(&content_dir, &id, &display_name, options.force),
        "vehicle" => build_new_vehicle(&content_dir, &id, &display_name, options.force),
        "shop" => build_new_shop(&content_dir, &id, &display_name, options.force),
        "npc" => build_new_npc(&content_dir, &id, &display_name, options.force),
        "encounter" => build_new_encounter(&content_dir, &id, options.force),
        "job" => build_new_job(&content_dir, &id, &display_name, options.force),
        other => Err(format!(
            "Unknown kind '{}'. Try spell, ability, item, equipment, enemy, vehicle, shop, npc, encounter, job",
            other
        )),
    };

    match result {
        Ok(message) => {
            println!("{}", message);
            if kind == "npc" {
                println!(
                    "Note: create dialog/{0}_dialog.json for the new NPC dialog.",
                    id
                );
            }
        }
        Err(err) => eprintln!("{}", err),
    }
}

fn build_new_spell(
    content_dir: &Path,
    id: &str,
    name: &str,
    force: bool,
) -> Result<String, String> {
    let path = content_dir.join("entities").join("spells.json");
    if !path.exists() {
        return Err("entities/spells.json not found. Define spell schools first.".to_string());
    }
    let mut file: SpellsFile = load_json(&path)?;
    if file.schools.is_empty() {
        return Err(
            "entities/spells.json has no schools. Define at least one school before adding spells."
                .to_string(),
        );
    }
    if let Some(index) = file.spells.iter().position(|spell| spell.id == id) {
        if !force {
            return Err(format!(
                "Spell '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.spells.remove(index);
    }

    let school_id = file
        .schools
        .first()
        .map(|school| school.id.clone())
        .unwrap_or_else(|| "white".to_string());
    file.spells.push(SpellDefinition {
        id: id.to_string(),
        name: name.to_string(),
        school: school_id,
        tier: 1,
        cost: SpellCost {
            r#type: "mp".to_string(),
            value: 3,
        },
        default_target: "ally".to_string(),
        allowed_targets: vec!["ally".to_string()],
        target_mode: "single".to_string(),
        multi_attenuation: None,
        effect: SpellEffect {
            r#type: "heal".to_string(),
            power: 10,
            element: None,
            effects: Vec::new(),
        },
    });

    write_json_pretty(path, &file)?;
    Ok(format!("Added spell '{}'", id))
}

fn build_new_ability(
    content_dir: &Path,
    id: &str,
    name: &str,
    force: bool,
) -> Result<String, String> {
    let path = content_dir.join("entities").join("abilities.json");
    let mut file = load_or_default(
        &path,
        AbilitiesFile {
            version: 1,
            abilities: Vec::new(),
        },
    )?;
    if let Some(index) = file.abilities.iter().position(|ability| ability.id == id) {
        if !force {
            return Err(format!(
                "Ability '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.abilities.remove(index);
    }
    file.abilities.push(AbilityDefinition {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        command_group: None,
        default_target: "enemy".to_string(),
        allowed_targets: vec!["enemy".to_string()],
        target_mode: "single".to_string(),
        multi_attenuation: None,
        effect: AbilityEffect {
            r#type: "damage".to_string(),
            power: 5,
            windup_turns: 0,
            vanish_during_windup: false,
            effects: Vec::new(),
        },
        cost: None,
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added ability '{}'", id))
}

fn build_new_item(content_dir: &Path, id: &str, name: &str, force: bool) -> Result<String, String> {
    let path = content_dir.join("entities").join("items.json");
    let mut file = load_or_default(
        &path,
        ItemsFile {
            version: 1,
            items: Vec::new(),
        },
    )?;
    if let Some(index) = file.items.iter().position(|item| item.id == id) {
        if !force {
            return Err(format!(
                "Item '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.items.remove(index);
    }
    file.items.push(ItemDefinition {
        id: id.to_string(),
        name: name.to_string(),
        r#type: "consumable".to_string(),
        unique: false,
        description: None,
        usage: ItemUsage {
            context: "field".to_string(),
            target: "ally".to_string(),
        },
        effect: ItemEffect {
            r#type: "heal_hp".to_string(),
            power: Some(30),
            target: None,
            destination: None,
            effects: Vec::new(),
            statuses: Vec::new(),
        },
        price: None,
        sellable: None,
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added item '{}'", id))
}

fn build_new_equipment(
    content_dir: &Path,
    id: &str,
    name: &str,
    force: bool,
) -> Result<String, String> {
    let path = content_dir.join("entities").join("equipment.json");
    let mut file = load_or_default(
        &path,
        EquipmentFile {
            version: 1,
            equipment: Vec::new(),
        },
    )?;
    if let Some(index) = file.equipment.iter().position(|item| item.id == id) {
        if !force {
            return Err(format!(
                "Equipment '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.equipment.remove(index);
    }
    file.equipment.push(EquipmentDefinition {
        id: id.to_string(),
        name: name.to_string(),
        category: "weapon".to_string(),
        slot: "weapon".to_string(),
        allowed_jobs: None,
        stats: HashMap::new(),
        spells: Vec::new(),
        abilities: Vec::new(),
        traits: Vec::new(),
        price: None,
        sellable: None,
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added equipment '{}'", id))
}

fn build_new_enemy(
    content_dir: &Path,
    id: &str,
    name: &str,
    force: bool,
) -> Result<String, String> {
    let path = content_dir.join("entities").join("enemies.json");
    let mut file = load_or_default(
        &path,
        EnemiesFile {
            version: 1,
            enemies: Vec::new(),
        },
    )?;
    if let Some(index) = file.enemies.iter().position(|enemy| enemy.id == id) {
        if !force {
            return Err(format!(
                "Enemy '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.enemies.remove(index);
    }
    file.enemies.push(EnemyDefinition {
        id: id.to_string(),
        name: name.to_string(),
        stats: HashMap::from([
            ("hp".to_string(), 10),
            ("mp".to_string(), 0),
            ("str".to_string(), 2),
            ("int".to_string(), 1),
            ("vit".to_string(), 1),
            ("agi".to_string(), 1),
            ("lck".to_string(), 1),
        ]),
        traits: Vec::new(),
        sprite: EnemySprite {
            glyph: "e".to_string(),
            palette: "enemy".to_string(),
        },
        art: None,
        exp: 1,
        currency: vec![engine::maps::MapCurrencyStack {
            id: "gold".to_string(),
            amount: 1,
        }],
        jp: 0,
        loot: Vec::new(),
        spells: Vec::new(),
        abilities: Vec::new(),
        mp_pool: "limited".to_string(),
        ai: engine::entities::EnemyAiConfig::default(),
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added enemy '{}'", id))
}

fn build_new_vehicle(
    content_dir: &Path,
    id: &str,
    name: &str,
    force: bool,
) -> Result<String, String> {
    let path = content_dir.join("entities").join("vehicles.json");
    let mut file = load_or_default(
        &path,
        VehiclesFile {
            version: 1,
            vehicles: Vec::new(),
        },
    )?;
    if let Some(index) = file.vehicles.iter().position(|vehicle| vehicle.id == id) {
        if !force {
            return Err(format!(
                "Vehicle '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.vehicles.remove(index);
    }
    file.vehicles.push(VehicleDefinition {
        id: id.to_string(),
        name: name.to_string(),
        speed: 1,
        allowed_tiles: vec!["ground".to_string()],
        unlock_flag: format!("vehicle.{}_unlocked", id),
        glyph: None,
        palette: None,
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added vehicle '{}'", id))
}

fn build_new_shop(content_dir: &Path, id: &str, name: &str, force: bool) -> Result<String, String> {
    let path = content_dir.join("entities").join("shops.json");
    let mut file = load_or_default(
        &path,
        ShopsFile {
            version: 1,
            shops: Vec::new(),
        },
    )?;
    if let Some(index) = file.shops.iter().position(|shop| shop.id == id) {
        if !force {
            return Err(format!(
                "Shop '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.shops.remove(index);
    }
    file.shops.push(ShopDefinition {
        id: id.to_string(),
        name: name.to_string(),
        currency: "gold".to_string(),
        inventory: Vec::new(),
        buy_price_multiplier: 1.0,
        sell_price_multiplier: 0.5,
        sell_behavior: "disappear".to_string(),
        currency_pool: "infinite".to_string(),
        currency_amount: None,
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added shop '{}'", id))
}

fn build_new_npc(content_dir: &Path, id: &str, name: &str, force: bool) -> Result<String, String> {
    let path = content_dir.join("entities").join("npcs.json");
    let mut file = load_or_default(
        &path,
        NpcsFile {
            version: 1,
            npcs: Vec::new(),
        },
    )?;
    if let Some(index) = file.npcs.iter().position(|npc| npc.id == id) {
        if !force {
            return Err(format!(
                "NPC '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.npcs.remove(index);
    }
    file.npcs.push(NpcDefinition {
        id: id.to_string(),
        name: name.to_string(),
        sprite: "npc".to_string(),
        palette: None,
        dialog: format!("{}_dialog", id),
        behavior: NpcBehavior {
            r#type: "static".to_string(),
            radius: None,
            path: None,
            idle_chance: 0.0,
            persist: None,
        },
        interaction_range: None,
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added NPC '{}'", id))
}

fn build_new_encounter(content_dir: &Path, id: &str, force: bool) -> Result<String, String> {
    let path = content_dir.join("entities").join("encounters.json");
    let mut file = load_or_default(
        &path,
        EncountersFile {
            version: 1,
            tables: Vec::new(),
        },
    )?;
    if let Some(index) = file.tables.iter().position(|table| table.id == id) {
        if !force {
            return Err(format!(
                "Encounter '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.tables.remove(index);
    }
    file.tables.push(EncounterTable {
        id: id.to_string(),
        entries: vec![EncounterEntry {
            weight: 1,
            tile: None,
            formation: Vec::new(),
        }],
    });
    write_json_pretty(path, &file)?;
    Ok(format!("Added encounter '{}'", id))
}

fn build_new_job(content_dir: &Path, id: &str, name: &str, force: bool) -> Result<String, String> {
    let path = content_dir.join("entities").join("jobs.json");
    let stats_path = content_dir.join("stats.json");
    let stats: StatsFile = load_json(&stats_path)?;
    let base_stats = stats.stats.base;

    let mut file = load_or_default(
        &path,
        JobsFile {
            version: 1,
            jobs: Vec::new(),
        },
    )?;
    if let Some(index) = file.jobs.iter().position(|job| job.id == id) {
        if !force {
            return Err(format!(
                "Job '{}' already exists. Use --force to overwrite.",
                id
            ));
        }
        file.jobs.remove(index);
    }

    let mut stats_map = HashMap::new();
    let mut growth_map = HashMap::new();
    for stat in &base_stats {
        let value = match stat.id.as_str() {
            "hp" => 30,
            "mp" => 5,
            _ => 3,
        };
        stats_map.insert(stat.id.clone(), value);
        growth_map.insert(stat.id.clone(), "1".to_string());
    }

    file.jobs.push(JobDefinition {
        id: id.to_string(),
        name: name.to_string(),
        stats: stats_map,
        growth: engine::entities::GrowthConfig {
            mode: "formula".to_string(),
            per_level: growth_map,
            tables: HashMap::new(),
        },
        equipment: JobEquipment {
            weapons: Vec::new(),
            armor: Vec::new(),
        },
        equipment_slots: vec!["weapon".to_string(), "armor".to_string()],
        accessory_slots: 1,
        can_dual_wield: false,
        stat_modifiers: HashMap::new(),
        spells: Vec::new(),
        abilities: Vec::new(),
        commands: Vec::new(),
        starting_equipment: HashMap::new(),
        sprite: JobSprite {
            glyph: "@".to_string(),
            palette: "bright_cyan".to_string(),
        },
        art: None,
        unlock_flag: None,
        is_default: false,
        sort_order: None,
        magic_slots: None,
        magic_equip_progression: None,
        description: None,
        magic_schools: Vec::new(),
        acquisition: None,
    });

    write_json_pretty(path, &file)?;
    Ok(format!("Added job '{}'", id))
}
