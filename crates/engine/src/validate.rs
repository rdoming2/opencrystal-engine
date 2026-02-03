use std::collections::{HashMap, HashSet};
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
use crate::quests::QuestsFile;
use crate::rules::{PartyMode, RulesFile};
use crate::stats::StatsFile;
use crate::world::WorldsFile;

const BATTLE_POS_MAX_X: i32 = 9;
const BATTLE_POS_MAX_Y: i32 = 5;
const EVENT_TYPES: [&str; 15] = [
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
    "wait",
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
    let abilities_path = content_dir.join("entities").join("abilities.json");
    let items_path = content_dir.join("entities").join("items.json");
    let equipment_path = content_dir.join("entities").join("equipment.json");
    let enemies_path = content_dir.join("entities").join("enemies.json");
    let vehicles_path = content_dir.join("entities").join("vehicles.json");
    let shops_path = content_dir.join("entities").join("shops.json");

    let rules = load_single(&rules_path, |path| RulesFile::load(path), &mut errors);
    let party = match rules.as_ref().map(|file| &file.party_mode) {
        Some(PartyMode::Preset) | Some(PartyMode::PresetRename) => load_single(
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
    let abilities = load_single(
        &abilities_path,
        |path| AbilitiesFile::load(path),
        &mut errors,
    );
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
        if rules.save.slots_max == 0 {
            errors.push("rules.json: save.slots_max must be > 0".to_string());
        }
        if rules.game.magic_acquisition == crate::rules::MagicAcquisition::Jp
            && rules.job_system.progression_mode != crate::rules::JobProgressionMode::JobPoints
        {
            errors.push(
                "rules.json: magic_acquisition 'jp' requires job_system.progression_mode 'job_points'"
                    .to_string(),
            );
        }
        if rules.game.ability_acquisition == crate::rules::AbilityAcquisition::Jp
            && rules.job_system.progression_mode != crate::rules::JobProgressionMode::JobPoints
        {
            errors.push(
                "rules.json: ability_acquisition 'jp' requires job_system.progression_mode 'job_points'"
                    .to_string(),
            );
        }
        match rules.exp_curve.mode.as_str() {
            "table" => {
                if rules.exp_curve.table.is_empty() {
                    errors.push("rules.json: exp_curve.table must not be empty".to_string());
                }
                if rules.exp_curve.max_level == 0 {
                    errors.push("rules.json: exp_curve.max_level must be > 0".to_string());
                }
                if rules.exp_curve.table.len() < rules.exp_curve.max_level as usize {
                    errors.push("rules.json: exp_curve.table must cover max_level".to_string());
                }
            }
            "formula" => {
                if rules
                    .exp_curve
                    .formula
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
                {
                    errors.push(
                        "rules.json: exp_curve.formula required for formula mode".to_string(),
                    );
                }
                if rules.exp_curve.max_level == 0 {
                    errors.push("rules.json: exp_curve.max_level must be > 0".to_string());
                }
            }
            other => {
                errors.push(format!(
                    "rules.json: exp_curve has unknown mode '{}'",
                    other
                ));
            }
        }
        if rules.battle.commands.is_empty() {
            errors.push("rules.json: battle.commands must define at least one command".to_string());
        }
        let mut command_ids = HashSet::new();
        for command in &rules.battle.commands {
            let id = command.id.trim();
            if id.is_empty() {
                errors.push("rules.json: battle.commands requires non-empty id".to_string());
            }
            if !id
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
            {
                errors.push(format!(
                    "rules.json: battle.commands '{}' must be lowercase snake_case",
                    command.id
                ));
            }
            if !command_ids.insert(id.to_string()) {
                errors.push(format!(
                    "rules.json: battle.commands duplicate id '{}'",
                    command.id
                ));
            }
            let kind = command.kind.as_str();
            let valid_kind = matches!(
                kind,
                "attack" | "magic" | "abilities" | "items" | "run" | "defend" | "abilities_group"
            );
            if !valid_kind {
                errors.push(format!(
                    "rules.json: battle.commands '{}' has unknown kind '{}'",
                    command.id, command.kind
                ));
            }
            if kind == "abilities_group"
                && command
                    .ability_group
                    .as_ref()
                    .map(|group| group.trim().is_empty())
                    .unwrap_or(true)
            {
                errors.push(format!(
                    "rules.json: battle.commands '{}' abilities_group requires ability_group",
                    command.id
                ));
            }
        }
        for command_id in &rules.battle.global_commands {
            if !command_ids.contains(command_id) {
                errors.push(format!(
                    "rules.json: battle.global_commands references unknown command '{}'",
                    command_id
                ));
            }
        }
        let mut global_ids = HashSet::new();
        for command_id in &rules.battle.global_commands {
            if !global_ids.insert(command_id.as_str()) {
                errors.push(format!(
                    "rules.json: battle.global_commands duplicate command '{}'",
                    command_id
                ));
            }
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
    let quests = load_quest_files(content_dir.join("quests"), &mut errors);

    let map_ids: HashSet<String> = maps.iter().map(|map| map.id.clone()).collect();
    let map_dims: HashMap<&str, (u32, u32)> = maps
        .iter()
        .map(|map| (map.id.as_str(), (map.width, map.height)))
        .collect();

    if let Some(rules) = &rules {
        if !map_ids.contains(&rules.game.start_location.map) {
            errors.push(format!(
                "rules.json: start_location.map '{}' not found in maps",
                rules.game.start_location.map
            ));
        }
        if let Some(worlds) = &worlds {
            let world_ids: HashSet<&str> = worlds.worlds.iter().map(|w| w.id.as_str()).collect();
            if !world_ids.contains(rules.game.start_location.world.as_str()) {
                errors.push(format!(
                    "rules.json: start_location.world '{}' not found in worlds.json",
                    rules.game.start_location.world
                ));
            }
        }
    }

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
        if !(0.0..=1.0).contains(&map.encounter_rate) {
            errors.push(format!(
                "maps/{}: encounter_rate {} must be between 0.0 and 1.0",
                map.id, map.encounter_rate
            ));
        }
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
        for chest in &map.chests {
            if chest.opened_flag.trim().is_empty() {
                errors.push(format!(
                    "maps/{}: chest '{}' missing opened_flag",
                    map.id, chest.id
                ));
            }
            if chest.pos[0] < 0 || chest.pos[1] < 0 {
                errors.push(format!(
                    "maps/{}: chest '{}' has negative position",
                    map.id, chest.id
                ));
                continue;
            }
            if chest.pos[0] >= map.width as i32 || chest.pos[1] >= map.height as i32 {
                errors.push(format!(
                    "maps/{}: chest '{}' position {:?} out of bounds",
                    map.id, chest.id, chest.pos
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

    if let (Some(spells), Some(abilities), Some(jobs)) = (&spells, &abilities, &jobs) {
        let spell_ids: HashSet<&str> = spells
            .spells
            .iter()
            .map(|spell| spell.id.as_str())
            .collect();
        let ability_ids: HashSet<&str> = abilities
            .abilities
            .iter()
            .map(|ability| ability.id.as_str())
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
        let command_ids: HashSet<&str> = rules
            .as_ref()
            .map(|rules| {
                rules
                    .battle
                    .commands
                    .iter()
                    .map(|command| command.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
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
                        match job.growth.tables.get(*stat) {
                            Some(values) if !values.is_empty() => {}
                            _ => errors.push(format!(
                                "jobs.json: job '{}' table growth missing stat '{}'",
                                job.id, stat
                            )),
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
                    for stat in &base_stats {
                        if !job.growth.per_level.contains_key(*stat) {
                            errors.push(format!(
                                "jobs.json: job '{}' formula growth missing stat '{}'",
                                job.id, stat
                            ));
                        }
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
            for ability in &job.abilities {
                if !ability_ids.contains(ability.id.as_str()) {
                    errors.push(format!(
                        "jobs.json: job '{}' references unknown ability '{}'",
                        job.id, ability.id
                    ));
                }
            }
            for command_id in &job.commands {
                if !command_ids.is_empty() && !command_ids.contains(command_id.as_str()) {
                    errors.push(format!(
                        "jobs.json: job '{}' references unknown command '{}'",
                        job.id, command_id
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

    if let Some(jobs) = &jobs {
        if jobs.jobs.is_empty() {
            errors.push("jobs.json: must define at least one job".to_string());
        }
    }

    if let (Some(rules), Some(jobs)) = (&rules, &jobs) {
        if rules.party_mode == PartyMode::Create {
            let default_jobs: Vec<_> = jobs.jobs.iter().filter(|job| job.is_default).collect();
            if default_jobs.is_empty() {
                errors.push("jobs.json: create mode requires a default job".to_string());
            } else if default_jobs.len() > 1 {
                errors.push("jobs.json: create mode requires a single default job".to_string());
            } else if let Some(default_job) = default_jobs.first() {
                if default_job
                    .unlock_flag
                    .as_ref()
                    .map(|flag| !flag.trim().is_empty())
                    .unwrap_or(false)
                {
                    errors.push(format!(
                        "jobs.json: default job '{}' cannot be gated by unlock_flag",
                        default_job.id
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
        let valid_contexts: HashSet<&str> = ["field", "battle", "both"].into_iter().collect();
        let valid_targets: HashSet<&str> = ["self", "ally", "party", "enemy"].into_iter().collect();
        for item in &items.items {
            if !valid_contexts.contains(item.usage.context.as_str()) {
                errors.push(format!(
                    "items.json: item '{}' has invalid usage context '{}'",
                    item.id, item.usage.context
                ));
            }
            if !valid_targets.contains(item.usage.target.as_str()) {
                errors.push(format!(
                    "items.json: item '{}' has invalid usage target '{}'",
                    item.id, item.usage.target
                ));
            }
        }
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

    if let (Some(rules), Some(items), Some(equipment)) = (&rules, &items, &equipment) {
        let item_ids: HashSet<&str> = items.items.iter().map(|item| item.id.as_str()).collect();
        let equipment_ids: HashSet<&str> = equipment
            .equipment
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        if rules.inventory.max_stack <= 0 {
            errors.push("rules.json: inventory.max_stack must be > 0".to_string());
        }
        for stack in &rules.inventory.items {
            if stack.qty <= 0 {
                errors.push(format!(
                    "rules.json: inventory item '{}' must have qty > 0",
                    stack.id
                ));
            }
            if !item_ids.contains(stack.id.as_str()) {
                errors.push(format!(
                    "rules.json: inventory item '{}' not found in items.json",
                    stack.id
                ));
            }
        }
        for stack in &rules.inventory.equipment {
            if stack.qty <= 0 {
                errors.push(format!(
                    "rules.json: inventory equipment '{}' must have qty > 0",
                    stack.id
                ));
            }
            if !equipment_ids.contains(stack.id.as_str()) {
                errors.push(format!(
                    "rules.json: inventory equipment '{}' not found in equipment.json",
                    stack.id
                ));
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
        for npc in &npcs.npcs {
            if let Some(range) = npc.interaction_range {
                if range < 1 {
                    errors.push(format!(
                        "npcs.json: npc '{}' has interaction_range {} which must be >= 1",
                        npc.id, range
                    ));
                }
            }
        }
    }

    if let (Some(items), Some(equipment)) = (&items, &equipment) {
        let item_ids: HashSet<&str> = items.items.iter().map(|item| item.id.as_str()).collect();
        let equipment_ids: HashSet<&str> = equipment
            .equipment
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        for map in &maps {
            for chest in &map.chests {
                for stack in &chest.loot.items {
                    if stack.qty <= 0 {
                        errors.push(format!(
                            "maps/{}: chest '{}' has item '{}' with non-positive qty",
                            map.id, chest.id, stack.id
                        ));
                    }
                    if !item_ids.contains(stack.id.as_str()) {
                        errors.push(format!(
                            "maps/{}: chest '{}' references unknown item '{}'",
                            map.id, chest.id, stack.id
                        ));
                    }
                }
                for stack in &chest.loot.equipment {
                    if stack.qty <= 0 {
                        errors.push(format!(
                            "maps/{}: chest '{}' has equipment '{}' with non-positive qty",
                            map.id, chest.id, stack.id
                        ));
                    }
                    if !equipment_ids.contains(stack.id.as_str()) {
                        errors.push(format!(
                            "maps/{}: chest '{}' references unknown equipment '{}'",
                            map.id, chest.id, stack.id
                        ));
                    }
                }
                for stack in &chest.loot.currency {
                    if stack.id.trim().is_empty() {
                        errors.push(format!(
                            "maps/{}: chest '{}' has currency with empty id",
                            map.id, chest.id
                        ));
                    }
                    if stack.amount <= 0 {
                        errors.push(format!(
                            "maps/{}: chest '{}' has currency '{}' with non-positive amount",
                            map.id, chest.id, stack.id
                        ));
                    }
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
                            "rest_party" => {}
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

    // Validate quest files
    for quest_file in &quests {
        let category_ids: HashSet<&str> = quest_file
            .categories
            .iter()
            .map(|cat| cat.id.as_str())
            .collect();

        // Check for duplicate category IDs
        let mut seen_categories = HashSet::new();
        for category in &quest_file.categories {
            if !seen_categories.insert(&category.id) {
                errors.push(format!("quests: duplicate category id '{}'", category.id));
            }
        }

        // Check for duplicate quest IDs
        let mut seen_quests = HashSet::new();
        for quest in &quest_file.quests {
            if !seen_quests.insert(&quest.id) {
                errors.push(format!("quests: duplicate quest id '{}'", quest.id));
            }
        }

        // Validate quest category references
        for quest in &quest_file.quests {
            if !category_ids.contains(quest.category_id.as_str()) {
                errors.push(format!(
                    "quests: quest '{}' references unknown category '{}'",
                    quest.id, quest.category_id
                ));
            }
        }

        // Validate step flag format
        for quest in &quest_file.quests {
            for step in &quest.steps {
                validate_step_flags(&mut errors, &quest.id, step, 0);
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

fn validate_step_flags(
    errors: &mut Vec<String>,
    quest_id: &str,
    step: &crate::quests::QuestStep,
    depth: usize,
) {
    // Check flag format: should be quest.<quest_id>.<step_id>
    if !step.flag.starts_with("quest.") {
        errors.push(format!(
            "quests: quest '{}' step '{}' has invalid flag format '{}', should be 'quest.<quest_id>.<step_id>'",
            quest_id, step.id, step.flag
        ));
    }

    // Check for duplicate step IDs within the same quest
    let mut seen_step_ids = HashSet::new();
    if !seen_step_ids.insert(&step.id) {
        errors.push(format!(
            "quests: quest '{}' has duplicate step id '{}'",
            quest_id, step.id
        ));
    }

    // Validate substeps recursively
    for substep in &step.substeps {
        validate_step_flags(errors, quest_id, substep, depth + 1);
    }
}

fn load_quest_files(dir: PathBuf, errors: &mut Vec<String>) -> Vec<QuestsFile> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("{}: {}", dir.display(), err));
            }
            return files;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match QuestsFile::load(&path) {
            Ok(file) => files.push(file),
            Err(err) => errors.push(err),
        }
    }

    files
}
