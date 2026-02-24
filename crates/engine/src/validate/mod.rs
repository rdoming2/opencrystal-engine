use std::collections::{HashMap, HashSet};
use std::path::Path;

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

mod cooking;
mod dialog;
mod effects;
mod entities;
mod events;
mod helpers;
mod loaders;
mod maps;
mod quests;
mod rules;

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

    let rules = loaders::load_single(&rules_path, |path| RulesFile::load(path), &mut errors);
    let effects = loaders::load_single(&effects_path, |path| EffectsFile::load(path), &mut errors);
    let party = match rules.as_ref().map(|file| &file.party_mode) {
        Some(PartyMode::Preset) | Some(PartyMode::PresetRename) => loaders::load_single(
            &content_dir.join("party.json"),
            |path| PartyFile::load(path),
            &mut errors,
        ),
        Some(PartyMode::Create) => {
            loaders::load_optional(&content_dir.join("party.json"), |path| {
                PartyFile::load(path)
            })
        }
        None => None,
    };

    let worlds = loaders::load_single(&worlds_path, |path| WorldsFile::load(path), &mut errors);
    let stats = loaders::load_single(&stats_path, |path| StatsFile::load(path), &mut errors);
    let encounters = loaders::load_single(
        &encounters_path,
        |path| EncountersFile::load(path),
        &mut errors,
    );
    let jobs = loaders::load_single(&jobs_path, |path| JobsFile::load(path), &mut errors);
    let spells = loaders::load_single(&spells_path, |path| SpellsFile::load(path), &mut errors);
    let abilities = loaders::load_single(
        &abilities_path,
        |path| AbilitiesFile::load(path),
        &mut errors,
    );
    let items = loaders::load_single(&items_path, |path| ItemsFile::load(path), &mut errors);
    let equipment = loaders::load_single(
        &equipment_path,
        |path| EquipmentFile::load(path),
        &mut errors,
    );
    let enemies = loaders::load_single(&enemies_path, |path| EnemiesFile::load(path), &mut errors);
    let vehicles =
        loaders::load_single(&vehicles_path, |path| VehiclesFile::load(path), &mut errors);
    let shops = loaders::load_single(&shops_path, |path| ShopsFile::load(path), &mut errors);
    let npcs = loaders::load_single(&npcs_path, |path| NpcsFile::load(path), &mut errors);
    let cooking = loaders::load_optional(&cooking_path, |path| {
        crate::content::CookingFile::load(path)
    });
    let strings = loaders::load_optional(&strings_path, |path| StringsFile::load(path));

    let maps = loaders::load_map_files(content_dir.join("maps"), &mut errors);
    let events = loaders::load_event_files(content_dir.join("events"), &mut errors);
    let dialogs = loaders::load_dialog_files(content_dir.join("dialog"), &mut errors);
    let quests = loaders::load_quest_files(
        content_dir.join("entities").join("quests.json"),
        &mut errors,
    );

    let ids = ValidationIds::new(
        rules.as_ref(),
        effects.as_ref(),
        jobs.as_ref(),
        spells.as_ref(),
        abilities.as_ref(),
        items.as_ref(),
        equipment.as_ref(),
        vehicles.as_ref(),
        shops.as_ref(),
        stats.as_ref(),
        &maps,
        &events,
        &dialogs,
    );

    let context = ValidationContext {
        rules: rules.as_ref(),
        effects: effects.as_ref(),
        worlds: worlds.as_ref(),
        stats: stats.as_ref(),
        encounters: encounters.as_ref(),
        jobs: jobs.as_ref(),
        spells: spells.as_ref(),
        abilities: abilities.as_ref(),
        items: items.as_ref(),
        equipment: equipment.as_ref(),
        enemies: enemies.as_ref(),
        vehicles: vehicles.as_ref(),
        shops: shops.as_ref(),
        npcs: npcs.as_ref(),
        party: party.as_ref(),
        cooking: cooking.as_ref(),
        strings: strings.as_ref(),
        maps: &maps,
        events: &events,
        dialogs: &dialogs,
        quests: &quests,
        ids,
    };

    rules::validate_rules(&context, &mut errors);
    entities::validate_strings(&context, &mut errors);
    effects::validate_effects(&context, &mut errors);
    entities::validate_stats(&context, &mut errors);

    maps::validate_start_locations(&context, &mut errors);
    maps::validate_world_maps(&context, &mut errors);
    maps::validate_encounter_tables(&context, &mut errors);
    maps::validate_maps(&context, &mut errors);

    events::validate_events(&context, &mut errors);
    entities::validate_jobs_spells_abilities(&context, &mut errors);
    entities::validate_party(&context, &mut errors);
    entities::validate_jobs_exist(&context, &mut errors);
    entities::validate_create_mode_default_job(&context, &mut errors);
    entities::validate_items_equipment_shops(&context, &mut errors);
    entities::validate_items_effects(&context, &mut errors);
    entities::validate_items_warp(&context, &mut errors);
    cooking::validate_items_learn_recipe(&context, &mut errors);
    cooking::validate_cooking(&context, &mut errors);
    cooking::validate_map_campfires(&context, &mut errors);
    rules::validate_inventory(&context, &mut errors);
    entities::validate_encounters_enemies(&context, &mut errors);
    entities::validate_world_vehicles(&context, &mut errors);
    dialog::validate_npc_dialogs(&context, &mut errors);
    maps::validate_map_vehicles(&context, &mut errors);
    entities::validate_npcs(&context, &mut errors);
    entities::validate_equipment_traits_prices(&context, &mut errors);
    maps::validate_chests(&context, &mut errors);
    dialog::validate_dialogs(&context, &mut errors);
    quests::validate_quests(&context, &mut errors);

    errors
}

pub(crate) struct ValidationContext<'a> {
    pub(crate) rules: Option<&'a RulesFile>,
    pub(crate) effects: Option<&'a EffectsFile>,
    pub(crate) worlds: Option<&'a WorldsFile>,
    pub(crate) stats: Option<&'a StatsFile>,
    pub(crate) encounters: Option<&'a EncountersFile>,
    pub(crate) jobs: Option<&'a JobsFile>,
    pub(crate) spells: Option<&'a SpellsFile>,
    pub(crate) abilities: Option<&'a AbilitiesFile>,
    pub(crate) items: Option<&'a ItemsFile>,
    pub(crate) equipment: Option<&'a EquipmentFile>,
    pub(crate) enemies: Option<&'a EnemiesFile>,
    pub(crate) vehicles: Option<&'a VehiclesFile>,
    pub(crate) shops: Option<&'a ShopsFile>,
    pub(crate) npcs: Option<&'a NpcsFile>,
    pub(crate) party: Option<&'a PartyFile>,
    pub(crate) cooking: Option<&'a crate::content::CookingFile>,
    pub(crate) strings: Option<&'a StringsFile>,
    pub(crate) maps: &'a [MapFile],
    pub(crate) events: &'a [EventFile],
    pub(crate) dialogs: &'a [DialogFile],
    pub(crate) quests: &'a [QuestsFile],
    pub(crate) ids: ValidationIds<'a>,
}

pub(crate) struct ValidationIds<'a> {
    pub(crate) currency_ids: HashSet<&'a str>,
    pub(crate) map_ids: HashSet<&'a str>,
    pub(crate) map_dims: HashMap<&'a str, (u32, u32)>,
    pub(crate) event_ids: HashSet<&'a str>,
    pub(crate) dialog_ids: HashSet<&'a str>,
    pub(crate) shop_ids: HashSet<&'a str>,
    pub(crate) item_ids: HashSet<&'a str>,
    pub(crate) equipment_ids: HashSet<&'a str>,
    pub(crate) spell_ids: HashSet<&'a str>,
    pub(crate) ability_ids: HashSet<&'a str>,
    pub(crate) job_ids: HashSet<&'a str>,
    pub(crate) effect_ids: HashSet<&'a str>,
    pub(crate) element_ids: HashSet<&'a str>,
    pub(crate) status_ids: HashSet<&'a str>,
    pub(crate) trait_ids: HashSet<&'a str>,
    pub(crate) vehicle_ids: HashSet<&'a str>,
    pub(crate) base_stat_ids: HashSet<&'a str>,
}

impl<'a> ValidationIds<'a> {
    fn new(
        rules: Option<&'a RulesFile>,
        effects: Option<&'a EffectsFile>,
        jobs: Option<&'a JobsFile>,
        spells: Option<&'a SpellsFile>,
        abilities: Option<&'a AbilitiesFile>,
        items: Option<&'a ItemsFile>,
        equipment: Option<&'a EquipmentFile>,
        vehicles: Option<&'a VehiclesFile>,
        shops: Option<&'a ShopsFile>,
        stats: Option<&'a StatsFile>,
        maps: &'a [MapFile],
        events: &'a [EventFile],
        dialogs: &'a [DialogFile],
    ) -> Self {
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

        let map_ids: HashSet<&str> = maps.iter().map(|map| map.id.as_str()).collect();
        let map_dims: HashMap<&str, (u32, u32)> = maps
            .iter()
            .map(|map| (map.id.as_str(), (map.width, map.height)))
            .collect();
        let event_ids: HashSet<&str> = events.iter().map(|event| event.id.as_str()).collect();
        let dialog_ids: HashSet<&str> = dialogs.iter().map(|dialog| dialog.id.as_str()).collect();
        let shop_ids: HashSet<&str> = shops
            .as_ref()
            .map(|file| file.shops.iter().map(|shop| shop.id.as_str()).collect())
            .unwrap_or_default();

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
        let ability_ids: HashSet<&str> = abilities
            .as_ref()
            .map(|abilities| {
                abilities
                    .abilities
                    .iter()
                    .map(|ability| ability.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let job_ids: HashSet<&str> = jobs
            .as_ref()
            .map(|jobs| jobs.jobs.iter().map(|job| job.id.as_str()).collect())
            .unwrap_or_default();
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
        let status_ids: HashSet<&str> = effects
            .as_ref()
            .map(|file| {
                file.statuses
                    .iter()
                    .map(|status| status.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let trait_ids: HashSet<&str> = effects
            .as_ref()
            .map(|file| file.traits.iter().map(|entry| entry.id.as_str()).collect())
            .unwrap_or_default();
        let vehicle_ids: HashSet<&str> = vehicles
            .as_ref()
            .map(|vehicles| {
                vehicles
                    .vehicles
                    .iter()
                    .map(|vehicle| vehicle.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let base_stat_ids: HashSet<&str> = stats
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

        Self {
            currency_ids,
            map_ids,
            map_dims,
            event_ids,
            dialog_ids,
            shop_ids,
            item_ids,
            equipment_ids,
            spell_ids,
            ability_ids,
            job_ids,
            effect_ids,
            element_ids,
            status_ids,
            trait_ids,
            vehicle_ids,
            base_stat_ids,
        }
    }
}
