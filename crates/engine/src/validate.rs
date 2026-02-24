use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::dialog::DialogFile;
use crate::encounters::EncountersFile;
use crate::entities::{
    AbilitiesFile, EffectsFile, EnemiesFile, EquipmentFile, ItemsFile, JobsFile, NpcsFile,
    ShopsFile, SpellsFile, StringsFile, VehiclesFile,
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
const EVENT_TYPES: [&str; 23] = [
    "dialog",
    "narration",
    "set_flag",
    "require_flags",
    "give_item",
    "give_equipment",
    "require_items",
    "remove_item",
    "warp",
    "start_battle",
    "start_dialog",
    "open_shop",
    "npc_show",
    "npc_hide",
    "npc_move",
    "npc_set_sprite",
    "party_add",
    "party_remove",
    "learn_recipe",
    "wait",
    "stat_set",
    "stat_add",
    "stat_max",
];

pub fn validate_content(content_dir: impl AsRef<Path>) -> Vec<String> {
    let content_dir = content_dir.as_ref();
    let mut errors = Vec::new();

    let rules_path = content_dir.join("rules.json");
    let effects_path = content_dir.join("effects.json");
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
    let cooking_path = content_dir.join("cooking.json");
    let strings_path = content_dir.join("ui").join("strings.json");

    let rules = load_single(&rules_path, |path| RulesFile::load(path), &mut errors);
    let effects = load_single(&effects_path, |path| EffectsFile::load(path), &mut errors);
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
    let cooking = load_optional(&cooking_path, |path| {
        crate::content::CookingFile::load(path)
    });
    let strings = load_optional(&strings_path, |path| StringsFile::load(path));
    let currency_ids: HashSet<&str> = rules
        .as_ref()
        .map(|rules| {
            rules
                .game
                .currencies
                .iter()
                .map(|currency| currency.id.as_str())
                .collect()
        })
        .unwrap_or_default();

    if let Some(rules) = &rules {
        if rules.game.party_size > 4 {
            errors.push("rules.json: party_size must be <= 4".to_string());
        }
        if rules.game.currencies.is_empty() {
            errors
                .push("rules.json: game.currencies must define at least one currency".to_string());
        }
        let mut seen_currency_ids = HashSet::new();
        for currency in &rules.game.currencies {
            if currency.id.trim().is_empty() {
                errors.push("rules.json: game.currencies has currency with empty id".to_string());
                continue;
            }
            if !seen_currency_ids.insert(currency.id.as_str()) {
                errors.push(format!(
                    "rules.json: game.currencies has duplicate currency id '{}'",
                    currency.id
                ));
            }
            if currency.name.trim().is_empty() {
                errors.push(format!(
                    "rules.json: game.currencies '{}' missing name",
                    currency.id
                ));
            }
        }
        if rules.save.slots_max == 0 {
            errors.push("rules.json: save.slots_max must be > 0".to_string());
        }
        if let Some(readiness) = rules.settings.readiness_speed.as_ref() {
            if readiness.step <= 0.0 {
                errors.push("rules.json: settings.readiness_speed.step must be > 0".to_string());
            }
            if readiness.min > readiness.max {
                errors.push("rules.json: settings.readiness_speed.min must be <= max".to_string());
            }
            if readiness.value < readiness.min || readiness.value > readiness.max {
                errors.push(
                    "rules.json: settings.readiness_speed.value must be within min/max".to_string(),
                );
            }
        }
        if let Some(difficulty) = rules.settings.difficulty_scale.as_ref() {
            if difficulty.step <= 0.0 {
                errors.push("rules.json: settings.difficulty_scale.step must be > 0".to_string());
            }
            if difficulty.min > difficulty.max {
                errors.push("rules.json: settings.difficulty_scale.min must be <= max".to_string());
            }
            if difficulty.value < difficulty.min || difficulty.value > difficulty.max {
                errors.push(
                    "rules.json: settings.difficulty_scale.value must be within min/max"
                        .to_string(),
                );
            }
        }
        if let Some(battle_mode) = rules.settings.battle_mode.as_ref() {
            if !battle_mode.options.is_empty() && !battle_mode.options.contains(&battle_mode.value)
            {
                errors.push(
                    "rules.json: settings.battle_mode.value must be listed in options".to_string(),
                );
            }
        }
        if rules.game.magic_acquisition == crate::rules::MagicAcquisition::Jp
            && rules.progression_mode != crate::rules::ProgressionMode::JobPoints
        {
            errors.push(
                "rules.json: magic_acquisition 'jp' requires progression_mode 'job_points'"
                    .to_string(),
            );
        }
        if rules.game.ability_acquisition == crate::rules::AbilityAcquisition::Jp
            && rules.progression_mode != crate::rules::ProgressionMode::JobPoints
        {
            errors.push(
                "rules.json: ability_acquisition 'jp' requires progression_mode 'job_points'"
                    .to_string(),
            );
        }
        if rules.progression_mode == crate::rules::ProgressionMode::Activity {
            if rules.activity_progression.ranks.is_empty() {
                errors.push("rules.json: activity_progression.ranks must not be empty".to_string());
            }
            let weapon_gain = &rules.activity_progression.weapon_gain;
            let magic_gain = &rules.activity_progression.magic_gain;
            for (label, value) in [
                ("weapon_gain.attack", weapon_gain.attack),
                ("weapon_gain.ability", weapon_gain.ability),
                ("weapon_gain.cast", weapon_gain.cast),
                ("magic_gain.attack", magic_gain.attack),
                ("magic_gain.ability", magic_gain.ability),
                ("magic_gain.cast", magic_gain.cast),
            ] {
                if value < 0.0 {
                    errors.push(format!(
                        "rules.json: activity_progression.{} must be >= 0",
                        label
                    ));
                }
                if value > 1.0 {
                    errors.push(format!(
                        "rules.json: activity_progression.{} must be <= 1",
                        label
                    ));
                }
            }
            if rules.activity_progression.effects.damage_scale < 0.0 {
                errors.push(
                    "rules.json: activity_progression.effects.damage_scale must be >= 0"
                        .to_string(),
                );
            }
            if rules.activity_progression.effects.hit_bonus < 0.0 {
                errors.push(
                    "rules.json: activity_progression.effects.hit_bonus must be >= 0".to_string(),
                );
            }
            for rank in &rules.activity_progression.ranks {
                if !(0.0..=1.0).contains(&rank.min) {
                    errors.push(
                        "rules.json: activity_progression.ranks min must be within 0-1".to_string(),
                    );
                }
                if rank.label.trim().is_empty() {
                    errors.push(
                        "rules.json: activity_progression.ranks label must not be empty"
                            .to_string(),
                    );
                }
            }
            if rules.activity_growth.base_rate < 0.0 {
                errors.push("rules.json: activity_growth.base_rate must be >= 0".to_string());
            }
            if !(0.0..=1.0).contains(&rules.activity_growth.min_gain_threshold) {
                errors.push(
                    "rules.json: activity_growth.min_gain_threshold must be within 0-1".to_string(),
                );
            }
            if rules.activity_growth.danger_factor_min <= 0.0 {
                errors
                    .push("rules.json: activity_growth.danger_factor_min must be > 0".to_string());
            }
            if rules.activity_growth.danger_factor_max < rules.activity_growth.danger_factor_min {
                errors.push(
                    "rules.json: activity_growth.danger_factor_max must be >= danger_factor_min"
                        .to_string(),
                );
            }
            if rules.activity_growth.floor_depth_exponent < 0.0 {
                errors.push(
                    "rules.json: activity_growth.floor_depth_exponent must be >= 0".to_string(),
                );
            }
            for (stat, cap) in &rules.activity_growth.soft_caps {
                if *cap <= 0.0 {
                    errors.push(format!(
                        "rules.json: activity_growth.soft_caps.{stat} must be > 0"
                    ));
                }
            }
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
                "attack"
                    | "magic"
                    | "abilities"
                    | "items"
                    | "run"
                    | "defend"
                    | "abilities_group"
                    | "row"
            );
            if !valid_kind {
                errors.push(format!(
                    "rules.json: battle.commands '{}' has unknown kind '{}'",
                    command.id, command.kind
                ));
            }
            if command.ability_id.is_some() && kind != "abilities" {
                errors.push(format!(
                    "rules.json: battle.commands '{}' ability_id requires kind 'abilities'",
                    command.id
                ));
            }
            if command.ability_id.is_some() && command.ability_group.is_some() {
                errors.push(format!(
                    "rules.json: battle.commands '{}' cannot set both ability_id and ability_group",
                    command.id
                ));
            }
            if kind == "abilities" {
                if command
                    .ability_id
                    .as_ref()
                    .map(|id| id.trim().is_empty())
                    .unwrap_or(false)
                {
                    errors.push(format!(
                        "rules.json: battle.commands '{}' ability_id cannot be empty",
                        command.id
                    ));
                }
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
        if rules.battle.rows.enabled {
            if rules.battle.rows.back_row_attack_multiplier <= 0.0
                || rules.battle.rows.back_row_attack_multiplier > 1.0
            {
                errors.push(
                    "rules.json: battle.rows.back_row_attack_multiplier must be between 0 and 1"
                        .to_string(),
                );
            }
            if rules.battle.rows.back_row_defense_multiplier <= 0.0
                || rules.battle.rows.back_row_defense_multiplier > 1.0
            {
                errors.push(
                    "rules.json: battle.rows.back_row_defense_multiplier must be between 0 and 1"
                        .to_string(),
                );
            }
            if rules.battle.rows.battle_shift < 0 {
                errors.push("rules.json: battle.rows.battle_shift must be >= 0".to_string());
            }
        }
        if rules.systems.get("cooking").copied().unwrap_or(false) && cooking.is_none() {
            errors.push("cooking.json: cooking system enabled but file not found".to_string());
        }
        if let Some(formulas) = rules.battle.formulas.physical.as_deref() {
            if formulas.trim().is_empty() {
                errors.push("rules.json: battle.formulas.physical must not be empty".to_string());
            }
        }
        if let Some(formulas) = rules.battle.formulas.magic.as_deref() {
            if formulas.trim().is_empty() {
                errors.push("rules.json: battle.formulas.magic must not be empty".to_string());
            }
        }
        if let Some(formulas) = rules.battle.formulas.hit.as_deref() {
            if formulas.trim().is_empty() {
                errors.push("rules.json: battle.formulas.hit must not be empty".to_string());
            }
        }
        if let Some(formulas) = rules.battle.formulas.crit.as_deref() {
            if formulas.trim().is_empty() {
                errors.push("rules.json: battle.formulas.crit must not be empty".to_string());
            }
        }
        if rules.battle.formulas.crit_multiplier <= 0.0 {
            errors.push("rules.json: battle.formulas.crit_multiplier must be > 0".to_string());
        }
        if rules.battle.boss_scaling.enabled {
            if rules.battle.boss_scaling.hp_multiplier <= 0.0 {
                errors
                    .push("rules.json: battle.boss_scaling.hp_multiplier must be > 0".to_string());
            }
            if rules.battle.boss_scaling.stat_multiplier <= 0.0 {
                errors.push(
                    "rules.json: battle.boss_scaling.stat_multiplier must be > 0".to_string(),
                );
            }
        }
    }

    if let Some(strings) = &strings {
        for (key, value) in &strings.strings {
            if key.trim().is_empty() {
                errors.push("ui/strings.json: string key must not be empty".to_string());
            }
            if value.trim().is_empty() {
                errors.push(format!(
                    "ui/strings.json: value for key '{}' must not be empty",
                    key
                ));
            }
        }
    }

    if let Some(effects) = &effects {
        let mut effect_ids = HashSet::new();
        for effect in &effects.effects {
            if !effect_ids.insert(effect.id.as_str()) {
                errors.push(format!("effects.json: duplicate effect id '{}'", effect.id));
            }
            let kind = effect.kind.as_str();
            let valid_kind = matches!(
                kind,
                "apply_status"
                    | "poison_tick"
                    | "skip_turn"
                    | "immobile"
                    | "damage_multiplier"
                    | "element_multiplier"
                    | "healing_inversion"
            );
            if !valid_kind {
                errors.push(format!(
                    "effects.json: effect '{}' has unknown kind '{}'",
                    effect.id, effect.kind
                ));
            }
            if kind == "apply_status" && effect.status.as_deref().unwrap_or("").is_empty() {
                errors.push(format!(
                    "effects.json: effect '{}' apply_status requires status",
                    effect.id
                ));
            }
            if kind == "damage_multiplier" {
                let damage_kind = effect.damage_kind.as_deref().unwrap_or("");
                if !matches!(damage_kind, "physical" | "magic" | "all") {
                    errors.push(format!(
                        "effects.json: effect '{}' damage_multiplier requires damage_kind",
                        effect.id
                    ));
                }
                if effect.multiplier.is_none() {
                    errors.push(format!(
                        "effects.json: effect '{}' damage_multiplier requires multiplier",
                        effect.id
                    ));
                }
            }
            if kind == "element_multiplier" {
                if effect.element.as_deref().unwrap_or("").is_empty() {
                    errors.push(format!(
                        "effects.json: effect '{}' element_multiplier requires element",
                        effect.id
                    ));
                }
                if effect.multiplier.is_none() {
                    errors.push(format!(
                        "effects.json: effect '{}' element_multiplier requires multiplier",
                        effect.id
                    ));
                }
            }
            if kind == "skip_turn" && effect.chance.is_none() {
                errors.push(format!(
                    "effects.json: effect '{}' skip_turn requires chance",
                    effect.id
                ));
            }
            if kind == "poison_tick" && effect.power.is_none() && effect.percent.is_none() {
                errors.push(format!(
                    "effects.json: effect '{}' poison_tick requires power or percent",
                    effect.id
                ));
            }
        }

        let element_ids: HashSet<&str> = effects
            .elements
            .iter()
            .map(|element| element.id.as_str())
            .collect();

        let mut status_ids = HashSet::new();
        for status in &effects.statuses {
            if !status_ids.insert(status.id.as_str()) {
                errors.push(format!("effects.json: duplicate status id '{}'", status.id));
            }
            for effect_id in &status.effects {
                if !effect_ids.contains(effect_id.as_str()) {
                    errors.push(format!(
                        "effects.json: status '{}' references unknown effect '{}'",
                        status.id, effect_id
                    ));
                }
            }
        }

        let mut trait_ids = HashSet::new();
        for trait_entry in &effects.traits {
            if !trait_ids.insert(trait_entry.id.as_str()) {
                errors.push(format!(
                    "effects.json: duplicate trait id '{}'",
                    trait_entry.id
                ));
            }
            for effect_id in &trait_entry.effects {
                if !effect_ids.contains(effect_id.as_str()) {
                    errors.push(format!(
                        "effects.json: trait '{}' references unknown effect '{}'",
                        trait_entry.id, effect_id
                    ));
                }
            }
        }

        for effect in &effects.effects {
            if effect.kind.as_str() == "element_multiplier" {
                if let Some(element) = effect.element.as_deref() {
                    if !element_ids.contains(element) {
                        errors.push(format!(
                            "effects.json: effect '{}' references unknown element '{}'",
                            effect.id, element
                        ));
                    }
                }
            }
            if effect.kind.as_str() == "apply_status" {
                if let Some(status) = effect.status.as_deref() {
                    if !status_ids.contains(status) {
                        errors.push(format!(
                            "effects.json: effect '{}' references unknown status '{}'",
                            effect.id, status
                        ));
                    }
                }
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
    let quests = load_quest_files(
        content_dir.join("entities").join("quests.json"),
        &mut errors,
    );

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
            if !map_ids.contains(&world.overworld_map_id) {
                errors.push(format!(
                    "worlds.json: world '{}' overworld_map_id '{}' not found",
                    world.id, world.overworld_map_id
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
        for door in &map.doors {
            if door.pos[0] < 0 || door.pos[1] < 0 {
                errors.push(format!(
                    "maps/{}: door '{}' has negative position",
                    map.id, door.id
                ));
                continue;
            }
            if door.pos[0] >= map.width as i32 || door.pos[1] >= map.height as i32 {
                errors.push(format!(
                    "maps/{}: door '{}' position {:?} out of bounds",
                    map.id, door.id, door.pos
                ));
            }
            if let Some(flag) = door.requires_flag.as_ref() {
                if flag.trim().is_empty() {
                    errors.push(format!(
                        "maps/{}: door '{}' has empty requires_flag",
                        map.id, door.id
                    ));
                }
            }
            if let Some(event) = door.locked_event.as_ref() {
                if !event_ids.contains(event) {
                    errors.push(format!(
                        "maps/{}: door '{}' locked_event '{}' not found",
                        map.id, door.id, event
                    ));
                }
            }
            if door.target_map.is_some() ^ door.target_pos.is_some() {
                errors.push(format!(
                    "maps/{}: door '{}' requires both target_map and target_pos",
                    map.id, door.id
                ));
            }
            if let Some(target_map) = door.target_map.as_ref() {
                if !map_ids.contains(target_map) {
                    errors.push(format!(
                        "maps/{}: door '{}' target '{}' not found",
                        map.id, door.id, target_map
                    ));
                }
                if let Some(target_pos) = door.target_pos.as_ref() {
                    if let Some((width, height)) = map_dims.get(target_map.as_str()) {
                        if target_pos[0] < 0
                            || target_pos[1] < 0
                            || target_pos[0] >= *width as i32
                            || target_pos[1] >= *height as i32
                        {
                            errors.push(format!(
                                "maps/{}: door '{}' target_pos {:?} out of bounds",
                                map.id, door.id, target_pos
                            ));
                        }
                    }
                }
            }
        }
        for puzzle in &map.puzzles {
            if puzzle.pos[0] < 0 || puzzle.pos[1] < 0 {
                errors.push(format!(
                    "maps/{}: puzzle '{}' has negative position",
                    map.id, puzzle.id
                ));
                continue;
            }
            if puzzle.pos[0] >= map.width as i32 || puzzle.pos[1] >= map.height as i32 {
                errors.push(format!(
                    "maps/{}: puzzle '{}' position {:?} out of bounds",
                    map.id, puzzle.id, puzzle.pos
                ));
            }
            if let Some(flags) = puzzle.requires_flags.as_ref() {
                if flags.iter().any(|flag| flag.trim().is_empty()) {
                    errors.push(format!(
                        "maps/{}: puzzle '{}' has empty requires_flags entry",
                        map.id, puzzle.id
                    ));
                }
            }
            if let Some(event) = puzzle.event.as_ref() {
                if !event_ids.contains(event) {
                    errors.push(format!(
                        "maps/{}: puzzle '{}' event '{}' not found",
                        map.id, puzzle.id, event
                    ));
                }
            }
            if puzzle.text.as_deref().unwrap_or("").trim().is_empty() && puzzle.event.is_none() {
                errors.push(format!(
                    "maps/{}: puzzle '{}' requires text or event",
                    map.id, puzzle.id
                ));
            }
            if let Some(flag) = puzzle.set_flag.as_ref() {
                if flag.trim().is_empty() {
                    errors.push(format!(
                        "maps/{}: puzzle '{}' has empty set_flag",
                        map.id, puzzle.id
                    ));
                }
            }
        }
        for campfire in &map.campfires {
            if campfire.pos[0] < 0 || campfire.pos[1] < 0 {
                errors.push(format!(
                    "maps/{}: campfire '{}' has negative position",
                    map.id, campfire.id
                ));
                continue;
            }
            if campfire.pos[0] >= map.width as i32 || campfire.pos[1] >= map.height as i32 {
                errors.push(format!(
                    "maps/{}: campfire '{}' position {:?} out of bounds",
                    map.id, campfire.id, campfire.pos
                ));
            }
            if campfire.campfire_id.trim().is_empty() {
                errors.push(format!(
                    "maps/{}: campfire '{}' has empty campfire_id",
                    map.id, campfire.id
                ));
            }
            if let Some(flags) = campfire.requires_flags.as_ref() {
                if flags.iter().any(|flag| flag.trim().is_empty()) {
                    errors.push(format!(
                        "maps/{}: campfire '{}' has empty requires_flags entry",
                        map.id, campfire.id
                    ));
                }
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
            if let Some(label) = transition.label.as_ref() {
                if label.trim().is_empty() {
                    errors.push(format!(
                        "maps/{}: transition '{}' has empty label",
                        map.id, transition.id
                    ));
                }
            }
            if let Some(flag) = transition.requires_flag.as_ref() {
                if flag.trim().is_empty() {
                    errors.push(format!(
                        "maps/{}: transition '{}' has empty requires_flag",
                        map.id, transition.id
                    ));
                }
            }
            if let Some(cost) = transition.cost.as_ref() {
                if cost.id.trim().is_empty() {
                    errors.push(format!(
                        "maps/{}: transition '{}' has cost with empty id",
                        map.id, transition.id
                    ));
                }
                if cost.amount <= 0 {
                    errors.push(format!(
                        "maps/{}: transition '{}' has non-positive cost",
                        map.id, transition.id
                    ));
                }
                if !cost.id.trim().is_empty() && !currency_ids.contains(cost.id.as_str()) {
                    errors.push(format!(
                        "maps/{}: transition '{}' references unknown currency '{}'",
                        map.id, transition.id, cost.id
                    ));
                }
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
    let item_ids: HashSet<&str> = items
        .as_ref()
        .map(|items| items.items.iter().map(|item| item.id.as_str()).collect())
        .unwrap_or_default();
    for event in &events {
        for step in &event.steps {
            if !event_types.contains(step.r#type.as_str()) {
                errors.push(format!(
                    "events/{}: unknown step type '{}'",
                    event.id, step.r#type
                ));
            }
            if step.r#type == "require_items" || step.r#type == "remove_item" {
                let item_id = step.item.as_deref().unwrap_or("");
                if item_id.trim().is_empty() {
                    errors.push(format!(
                        "events/{}: {} step missing item",
                        event.id, step.r#type
                    ));
                } else if !item_ids.contains(item_id) {
                    errors.push(format!(
                        "events/{}: {} step references unknown item '{}'",
                        event.id, step.r#type, item_id
                    ));
                }
                if step.qty.unwrap_or(1) <= 0 {
                    errors.push(format!(
                        "events/{}: {} step requires qty > 0",
                        event.id, step.r#type
                    ));
                }
            }
            if step.r#type == "warp" {
                let Some(target) = &step.target else {
                    errors.push(format!("events/{}: warp step missing target", event.id));
                    continue;
                };
                if target.map != "last_overworld" {
                    if !map_ids.contains(&target.map) {
                        errors.push(format!(
                            "events/{}: warp target '{}' not found",
                            event.id, target.map
                        ));
                        continue;
                    }
                    if let Some((width, height)) = map_dims.get(target.map.as_str()) {
                        if target.pos[0] < 0
                            || target.pos[1] < 0
                            || target.pos[0] >= *width as i32
                            || target.pos[1] >= *height as i32
                        {
                            errors.push(format!(
                                "events/{}: warp target_pos {:?} out of bounds",
                                event.id, target.pos
                            ));
                        }
                    }
                }
            }
            if step.r#type == "stat_set" || step.r#type == "stat_max" {
                if step.stat.as_deref().unwrap_or("").is_empty() {
                    errors.push(format!(
                        "events/{}: {} step missing stat",
                        event.id, step.r#type
                    ));
                }
                if step.value.is_none() {
                    errors.push(format!(
                        "events/{}: {} step missing value",
                        event.id, step.r#type
                    ));
                }
            }
            if step.r#type == "stat_add" {
                if step.stat.as_deref().unwrap_or("").is_empty() {
                    errors.push(format!("events/{}: stat_add step missing stat", event.id));
                }
            }
            if step.r#type == "party_add" {
                let member_id = step.member.as_deref().unwrap_or("");
                if member_id.trim().is_empty() {
                    errors.push(format!(
                        "events/{}: party_add step missing member",
                        event.id
                    ));
                }
                let Some(party) = party.as_ref() else {
                    errors.push(format!(
                        "events/{}: party_add '{}' requires party.json",
                        event.id, member_id
                    ));
                    continue;
                };
                if !member_id.trim().is_empty()
                    && !party.roster.iter().any(|actor| actor.id == member_id)
                {
                    errors.push(format!(
                        "events/{}: party_add '{}' not found in party roster",
                        event.id, member_id
                    ));
                }
            }
            if step.r#type == "party_remove" {
                if step.member.as_deref().unwrap_or("").trim().is_empty() {
                    errors.push(format!(
                        "events/{}: party_remove step missing member",
                        event.id
                    ));
                }
            }
            if step.r#type == "learn_recipe" {
                let Some(recipe_id) = step.recipe.as_deref() else {
                    errors.push(format!(
                        "events/{}: learn_recipe step missing recipe",
                        event.id
                    ));
                    continue;
                };
                let Some(cooking) = cooking.as_ref() else {
                    errors.push(format!(
                        "events/{}: learn_recipe '{}' requires cooking.json",
                        event.id, recipe_id
                    ));
                    continue;
                };
                let Some(recipe) = cooking.recipes.iter().find(|recipe| recipe.id == recipe_id)
                else {
                    errors.push(format!(
                        "events/{}: learn_recipe '{}' not found",
                        event.id, recipe_id
                    ));
                    continue;
                };
                if recipe
                    .unlock_flag
                    .as_deref()
                    .map(|flag| flag.trim().is_empty())
                    .unwrap_or(true)
                {
                    errors.push(format!(
                        "events/{}: learn_recipe '{}' requires recipe unlock_flag",
                        event.id, recipe_id
                    ));
                }
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
        let effect_ids: HashSet<&str> = effects
            .as_ref()
            .map(|file| {
                file.effects
                    .iter()
                    .map(|effect| effect.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let element_ids: HashSet<&str> = effects
            .as_ref()
            .map(|file| {
                file.elements
                    .iter()
                    .map(|element| element.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
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
        if let Some(rules) = rules.as_ref() {
            for command in &rules.battle.commands {
                if let Some(ability_id) = command.ability_id.as_deref() {
                    if !ability_ids.contains(ability_id) {
                        errors.push(format!(
                            "rules.json: battle.commands '{}' references unknown ability '{}'",
                            command.id, ability_id
                        ));
                    }
                }
            }
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

        for spell in &spells.spells {
            for effect_id in &spell.effect.effects {
                if !effect_ids.contains(effect_id.as_str()) {
                    errors.push(format!(
                        "spells.json: spell '{}' references unknown effect '{}'",
                        spell.id, effect_id
                    ));
                }
            }
            if let Some(element) = spell.effect.element.as_deref() {
                if !element_ids.is_empty() && !element_ids.contains(element) {
                    errors.push(format!(
                        "spells.json: spell '{}' references unknown element '{}'",
                        spell.id, element
                    ));
                }
            }
            if !matches!(spell.target_mode.as_str(), "single" | "multi" | "both") {
                errors.push(format!(
                    "spells.json: spell '{}' has invalid target_mode '{}'",
                    spell.id, spell.target_mode
                ));
            }
            if let Some(multiplier) = spell.multi_attenuation {
                if !(0.1..=1.0).contains(&multiplier) {
                    errors.push(format!(
                        "spells.json: spell '{}' multi_attenuation must be 0.1..=1.0",
                        spell.id
                    ));
                }
            }
        }

        for ability in &abilities.abilities {
            for effect_id in &ability.effect.effects {
                if !effect_ids.contains(effect_id.as_str()) {
                    errors.push(format!(
                        "abilities.json: ability '{}' references unknown effect '{}'",
                        ability.id, effect_id
                    ));
                }
            }
            if !matches!(ability.target_mode.as_str(), "single" | "multi" | "both") {
                errors.push(format!(
                    "abilities.json: ability '{}' has invalid target_mode '{}'",
                    ability.id, ability.target_mode
                ));
            }
            if let Some(multiplier) = ability.multi_attenuation {
                if !(0.1..=1.0).contains(&multiplier) {
                    errors.push(format!(
                        "abilities.json: ability '{}' multi_attenuation must be 0.1..=1.0",
                        ability.id
                    ));
                }
            }
            if let Some(cost) = &ability.cost {
                if cost.r#type == "currency" {
                    match cost.currency_id.as_deref() {
                        Some(id) if currency_ids.contains(id) => {}
                        Some(id) => errors.push(format!(
                            "abilities.json: ability '{}' references unknown currency '{}'",
                            ability.id, id
                        )),
                        None => errors.push(format!(
                            "abilities.json: ability '{}' currency cost missing currency_id",
                            ability.id
                        )),
                    }
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
            if let Some(prices) = &item.price {
                for (currency, amount) in prices {
                    if !currency_ids.contains(currency.as_str()) {
                        errors.push(format!(
                            "items.json: item '{}' has unknown currency '{}'",
                            item.id, currency
                        ));
                    }
                    if *amount < 0 {
                        errors.push(format!(
                            "items.json: item '{}' has negative price {} for '{}'",
                            item.id, amount, currency
                        ));
                    }
                }
            }
        }
        for shop in &shops.shops {
            if shop.currency.trim().is_empty() {
                errors.push(format!("shops.json: shop '{}' missing currency", shop.id));
            } else if !currency_ids.contains(shop.currency.as_str()) {
                errors.push(format!(
                    "shops.json: shop '{}' references unknown currency '{}'",
                    shop.id, shop.currency
                ));
            }
            if shop.buy_price_multiplier < 0.0 {
                errors.push(format!(
                    "shops.json: shop '{}' has negative buy_price_multiplier",
                    shop.id
                ));
            }
            if shop.sell_price_multiplier < 0.0 {
                errors.push(format!(
                    "shops.json: shop '{}' has negative sell_price_multiplier",
                    shop.id
                ));
            }
            if shop.sell_behavior != "disappear" && shop.sell_behavior != "stock" {
                errors.push(format!(
                    "shops.json: shop '{}' has invalid sell_behavior '{}'",
                    shop.id, shop.sell_behavior
                ));
            }
            if shop.currency_pool != "infinite" && shop.currency_pool != "tracked" {
                errors.push(format!(
                    "shops.json: shop '{}' has invalid currency_pool '{}'",
                    shop.id, shop.currency_pool
                ));
            }
            if let Some(amount) = shop.currency_amount {
                if amount < 0 {
                    errors.push(format!(
                        "shops.json: shop '{}' has negative currency_amount",
                        shop.id
                    ));
                }
            }
            for entry in &shop.inventory {
                if !item_ids.contains(entry.item.as_str())
                    && !equipment_ids.contains(entry.item.as_str())
                {
                    errors.push(format!(
                        "shops.json: shop '{}' references unknown item '{}'",
                        shop.id, entry.item
                    ));
                }
                if entry.price < 0 {
                    errors.push(format!(
                        "shops.json: shop '{}' entry '{}' has negative price",
                        shop.id, entry.item
                    ));
                }
                if let Some(stock) = entry.stock {
                    if stock < 0 {
                        errors.push(format!(
                            "shops.json: shop '{}' entry '{}' has negative stock",
                            shop.id, entry.item
                        ));
                    }
                }
                if let Some(price) = entry.sell_price {
                    if price < 0 {
                        errors.push(format!(
                            "shops.json: shop '{}' entry '{}' has negative sell_price",
                            shop.id, entry.item
                        ));
                    }
                }
            }
        }
    }

    if let (Some(items), Some(effects)) = (&items, &effects) {
        let effect_ids: HashSet<&str> = effects
            .effects
            .iter()
            .map(|effect| effect.id.as_str())
            .collect();
        let status_ids: HashSet<&str> = effects
            .statuses
            .iter()
            .map(|status| status.id.as_str())
            .collect();
        for item in &items.items {
            for effect_id in &item.effect.effects {
                if !effect_ids.contains(effect_id.as_str()) {
                    errors.push(format!(
                        "items.json: item '{}' references unknown effect '{}'",
                        item.id, effect_id
                    ));
                }
            }
            if item.effect.r#type == "cure_status" {
                if item.effect.statuses.is_empty() {
                    errors.push(format!(
                        "items.json: item '{}' cure_status requires statuses",
                        item.id
                    ));
                }
                for status_id in &item.effect.statuses {
                    if !status_ids.contains(status_id.as_str()) {
                        errors.push(format!(
                            "items.json: item '{}' references unknown status '{}'",
                            item.id, status_id
                        ));
                    }
                }
            }
        }
    }

    if let Some(items) = &items {
        for item in &items.items {
            if item.effect.r#type != "warp" {
                continue;
            }
            if let Some(destination) = &item.effect.destination {
                if !map_ids.contains(&destination.map) {
                    errors.push(format!(
                        "items.json: item '{}' warp destination '{}' not found",
                        item.id, destination.map
                    ));
                    continue;
                }
                if let Some((width, height)) = map_dims.get(destination.map.as_str()) {
                    if destination.pos[0] < 0
                        || destination.pos[1] < 0
                        || destination.pos[0] >= *width as i32
                        || destination.pos[1] >= *height as i32
                    {
                        errors.push(format!(
                            "items.json: item '{}' warp destination {:?} out of bounds",
                            item.id, destination.pos
                        ));
                    }
                }
            } else if item.effect.target.as_deref() != Some("last_overworld") {
                errors.push(format!(
                    "items.json: item '{}' warp requires destination or target last_overworld",
                    item.id
                ));
            }
        }
    }

    if let Some(items) = &items {
        for item in &items.items {
            if item.effect.r#type != "learn_recipe" {
                continue;
            }
            let Some(recipe_id) = item.effect.target.as_deref() else {
                errors.push(format!(
                    "items.json: item '{}' learn_recipe requires target recipe id",
                    item.id
                ));
                continue;
            };
            let Some(cooking) = cooking.as_ref() else {
                errors.push(format!(
                    "items.json: item '{}' learn_recipe requires cooking.json",
                    item.id
                ));
                continue;
            };
            let Some(recipe) = cooking.recipes.iter().find(|recipe| recipe.id == recipe_id) else {
                errors.push(format!(
                    "items.json: item '{}' learn_recipe references unknown recipe '{}'",
                    item.id, recipe_id
                ));
                continue;
            };
            if recipe
                .unlock_flag
                .as_deref()
                .map(|flag| flag.trim().is_empty())
                .unwrap_or(true)
            {
                errors.push(format!(
                    "items.json: item '{}' learn_recipe requires recipe unlock_flag",
                    item.id
                ));
            }
        }
    }

    if let Some(cooking) = &cooking {
        let mut recipe_ids = HashSet::new();
        let mut campfire_ids = HashSet::new();
        let item_ids: HashSet<&str> = items
            .as_ref()
            .map(|items| items.items.iter().map(|item| item.id.as_str()).collect())
            .unwrap_or_default();
        let equipment_ids: HashSet<&str> = equipment
            .as_ref()
            .map(|equipment| {
                equipment
                    .equipment
                    .iter()
                    .map(|item| item.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        for recipe in &cooking.recipes {
            if !recipe_ids.insert(recipe.id.as_str()) {
                errors.push(format!("cooking.json: duplicate recipe id '{}'", recipe.id));
            }
            if recipe.name.trim().is_empty() {
                errors.push(format!(
                    "cooking.json: recipe '{}' requires name",
                    recipe.id
                ));
            }
            if recipe
                .unlock_flag
                .as_deref()
                .map(|flag| flag.trim().is_empty())
                .unwrap_or(false)
            {
                errors.push(format!(
                    "cooking.json: recipe '{}' has empty unlock_flag",
                    recipe.id
                ));
            }
            if recipe.ingredients.is_empty() {
                errors.push(format!(
                    "cooking.json: recipe '{}' requires ingredients",
                    recipe.id
                ));
            }
            for ingredient in &recipe.ingredients {
                if ingredient.qty <= 0 {
                    errors.push(format!(
                        "cooking.json: recipe '{}' ingredient '{}' must have qty > 0",
                        recipe.id, ingredient.id
                    ));
                }
                if !item_ids.contains(ingredient.id.as_str()) {
                    errors.push(format!(
                        "cooking.json: recipe '{}' references unknown item '{}'",
                        recipe.id, ingredient.id
                    ));
                }
            }
            for item in &recipe.results.items {
                if item.qty <= 0 {
                    errors.push(format!(
                        "cooking.json: recipe '{}' result item '{}' must have qty > 0",
                        recipe.id, item.id
                    ));
                }
                if !item_ids.contains(item.id.as_str()) {
                    errors.push(format!(
                        "cooking.json: recipe '{}' result item '{}' not found in items.json",
                        recipe.id, item.id
                    ));
                }
            }
            for item in &recipe.results.equipment {
                if item.qty <= 0 {
                    errors.push(format!(
                        "cooking.json: recipe '{}' result equipment '{}' must have qty > 0",
                        recipe.id, item.id
                    ));
                }
                if !equipment_ids.contains(item.id.as_str()) {
                    errors.push(format!(
                        "cooking.json: recipe '{}' result equipment '{}' not found in equipment.json",
                        recipe.id, item.id
                    ));
                }
            }
            for currency in &recipe.results.currency {
                if currency.amount <= 0 {
                    errors.push(format!(
                        "cooking.json: recipe '{}' result currency '{}' must have amount > 0",
                        recipe.id, currency.id
                    ));
                }
                if !currency_ids.contains(currency.id.as_str()) {
                    errors.push(format!(
                        "cooking.json: recipe '{}' result currency '{}' not found in rules.json",
                        recipe.id, currency.id
                    ));
                }
            }
        }

        for campfire in &cooking.campfires {
            if !campfire_ids.insert(campfire.id.as_str()) {
                errors.push(format!(
                    "cooking.json: duplicate campfire id '{}'",
                    campfire.id
                ));
            }
            if campfire.label.trim().is_empty() {
                errors.push(format!(
                    "cooking.json: campfire '{}' requires label",
                    campfire.id
                ));
            }
            if campfire.recipes.is_empty() {
                errors.push(format!(
                    "cooking.json: campfire '{}' requires recipes",
                    campfire.id
                ));
            }
            for recipe_id in &campfire.recipes {
                if !recipe_ids.contains(recipe_id.as_str()) {
                    errors.push(format!(
                        "cooking.json: campfire '{}' references unknown recipe '{}'",
                        campfire.id, recipe_id
                    ));
                }
            }
        }
    }

    if cooking.is_none() {
        for map in &maps {
            if !map.campfires.is_empty() {
                errors.push(format!(
                    "maps/{}: campfires defined but cooking.json is missing",
                    map.id
                ));
            }
        }
    } else if let Some(cooking) = &cooking {
        let campfire_ids: HashSet<&str> = cooking
            .campfires
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        for map in &maps {
            for campfire in &map.campfires {
                if !campfire_ids.contains(campfire.campfire_id.as_str()) {
                    errors.push(format!(
                        "maps/{}: campfire '{}' references unknown campfire_id '{}'",
                        map.id, campfire.id, campfire.campfire_id
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
        let trait_ids: HashSet<&str> = effects
            .as_ref()
            .map(|file| file.traits.iter().map(|entry| entry.id.as_str()).collect())
            .unwrap_or_default();
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
        for enemy in &enemies.enemies {
            for trait_id in &enemy.traits {
                if !trait_ids.is_empty() && !trait_ids.contains(trait_id.as_str()) {
                    errors.push(format!(
                        "enemies.json: enemy '{}' references unknown trait '{}'",
                        enemy.id, trait_id
                    ));
                }
            }
            for currency in &enemy.currency {
                if currency.id.trim().is_empty() {
                    errors.push(format!(
                        "enemies.json: enemy '{}' has currency with empty id",
                        enemy.id
                    ));
                    continue;
                }
                if currency.amount <= 0 {
                    errors.push(format!(
                        "enemies.json: enemy '{}' currency '{}' must have amount > 0",
                        enemy.id, currency.id
                    ));
                }
                if !currency_ids.contains(currency.id.as_str()) {
                    errors.push(format!(
                        "enemies.json: enemy '{}' references unknown currency '{}'",
                        enemy.id, currency.id
                    ));
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

    if let Some(vehicles) = &vehicles {
        let vehicle_ids: HashSet<&str> = vehicles
            .vehicles
            .iter()
            .map(|vehicle| vehicle.id.as_str())
            .collect();
        for map in &maps {
            for vehicle in &map.vehicles {
                if !vehicle_ids.contains(vehicle.vehicle_id.as_str()) {
                    errors.push(format!(
                        "maps/{}: vehicle '{}' not found",
                        map.id, vehicle.vehicle_id
                    ));
                }
                if vehicle.pos[0] < 0
                    || vehicle.pos[1] < 0
                    || vehicle.pos[0] >= map.width as i32
                    || vehicle.pos[1] >= map.height as i32
                {
                    errors.push(format!(
                        "maps/{}: vehicle '{}' position {:?} out of bounds",
                        map.id, vehicle.vehicle_id, vehicle.pos
                    ));
                }
            }
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

    if let (Some(equipment), Some(effects)) = (&equipment, &effects) {
        let trait_ids: HashSet<&str> = effects
            .traits
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        for item in &equipment.equipment {
            for trait_id in &item.traits {
                if !trait_ids.contains(trait_id.as_str()) {
                    errors.push(format!(
                        "equipment.json: equipment '{}' references unknown trait '{}'",
                        item.id, trait_id
                    ));
                }
            }
            if let Some(prices) = &item.price {
                for (currency, amount) in prices {
                    if !currency_ids.contains(currency.as_str()) {
                        errors.push(format!(
                            "equipment.json: equipment '{}' has unknown currency '{}'",
                            item.id, currency
                        ));
                    }
                    if *amount < 0 {
                        errors.push(format!(
                            "equipment.json: equipment '{}' has negative price {} for '{}'",
                            item.id, amount, currency
                        ));
                    }
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
                    if !stack.id.trim().is_empty() && !currency_ids.contains(stack.id.as_str()) {
                        errors.push(format!(
                            "maps/{}: chest '{}' has unknown currency '{}'",
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
                            "learn_recipe" => {
                                let Some(recipe_id) = action.recipe.as_deref() else {
                                    errors.push(format!(
                                        "dialog/{}: learn_recipe missing recipe id",
                                        dialog.id
                                    ));
                                    continue;
                                };
                                let Some(cooking) = cooking.as_ref() else {
                                    errors.push(format!(
                                        "dialog/{}: learn_recipe '{}' requires cooking.json",
                                        dialog.id, recipe_id
                                    ));
                                    continue;
                                };
                                let Some(recipe) =
                                    cooking.recipes.iter().find(|recipe| recipe.id == recipe_id)
                                else {
                                    errors.push(format!(
                                        "dialog/{}: learn_recipe '{}' not found",
                                        dialog.id, recipe_id
                                    ));
                                    continue;
                                };
                                if recipe
                                    .unlock_flag
                                    .as_deref()
                                    .map(|flag| flag.trim().is_empty())
                                    .unwrap_or(true)
                                {
                                    errors.push(format!(
                                        "dialog/{}: learn_recipe '{}' requires recipe unlock_flag",
                                        dialog.id, recipe_id
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
                if let Some(choices) = &node.choices {
                    for choice in choices {
                        if let Some(flags) = &choice.requires_flags {
                            if flags.iter().any(|flag| flag.trim().is_empty()) {
                                errors.push(format!(
                                    "dialog/{}: choice '{}' has empty requires_flags entry",
                                    dialog.id, choice.label
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

    if let Some(show_flag) = &step.show_flag {
        if show_flag.trim().is_empty() {
            errors.push(format!(
                "quests: quest '{}' step '{}' has empty show_flag",
                quest_id, step.id
            ));
        }
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

fn load_quest_files(path: PathBuf, errors: &mut Vec<String>) -> Vec<QuestsFile> {
    load_single(&path, |path| QuestsFile::load(path), errors)
        .map(|file| vec![file])
        .unwrap_or_default()
}
