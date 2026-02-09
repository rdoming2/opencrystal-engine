use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use engine::encounters::{EncounterEntry, EncounterTable, EncountersFile};
use engine::entities::{
    AbilitiesFile, AbilityDefinition, AbilityEffect, EffectsFile, EnemiesFile, EnemyDefinition,
    EnemySprite, EquipmentDefinition, EquipmentFile, ItemDefinition, ItemEffect, ItemUsage,
    ItemsFile, JobDefinition, JobEquipment, JobSprite, JobsFile, MagicSchool, NpcBehavior,
    NpcDefinition, NpcsFile, ShopDefinition, ShopsFile, SpellCost, SpellDefinition, SpellEffect,
    SpellsFile, VehicleDefinition, VehiclesFile,
};
use engine::io::{load_json, write_json_pretty};
use engine::maps::{MapFile, TileLegend};
use engine::quests::QuestsFile;
use engine::rules::{
    AbilityAcquisition, BattleMode, Currency, ExpCurveRules, GameRules, JobProgressionMode,
    JobSystemRules, JpMode, MagicAcquisition, MagicSystem, PartyCreateRules, PartyMode,
    RenderRules, RulesFile, SaveRules, SettingsRules, StatsRules,
};
use engine::stats::{StatEntry, StatsDefinition, StatsFile};
use engine::world::{FastTravelConfig, OverviewConfig, WorldDefinition, WorldsFile};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Deserializer;
use tui::input::InputFile;
use tui::ui::{
    AbilitiesMenu, AttackMenu, BattleAnimation, BattleLayout, BattleLog, BattleMenus, BattlePanels,
    BattleUiFile, Breakpoint, BreakpointBehavior, ColumnSpec, CommandPanel, CommandRow,
    DialogUiFile, EnemyPanel, FooterConfig, HighlightRules, ItemsMenu, MagicMenu, MenuColumn,
    MenuEntry, MenuItem, MenuLayout, MenuPanel, MenuUiFile, PanelAnchor, PartyGrid, PartyPanel,
    ProgressItem, ProgressPanel, ProgressUiFile, SelectionRules, TitleLogo, TitleUiFile,
};

pub fn run_build(args: Vec<String>) {
    if args.is_empty() {
        print_build_usage();
        return;
    }
    match args[0].as_str() {
        "new" => run_build_new(&args[1..]),
        "upgrade" => run_build_upgrade(&args[1..]),
        "new-project" => run_build_new_project(&args[1..]),
        other => {
            eprintln!("Unknown build command: {}", other);
            print_build_usage();
        }
    }
}

fn run_build_new(args: &[String]) {
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

fn run_build_upgrade(args: &[String]) {
    let options = BuildUpgradeOptions::from_args(args);
    let content_dir = match resolve_content_dir(options.content_dir.as_deref()) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    let mut updated = 0;
    let mut skipped_unknown = 0;
    let mut errors = 0;

    let mut targets = Vec::new();
    targets.extend(base_upgrade_targets(&content_dir));
    targets.extend(dir_upgrade_targets(&content_dir.join("maps"), "map"));
    targets.extend(dir_upgrade_targets(&content_dir.join("events"), "event"));
    targets.extend(dir_upgrade_targets(&content_dir.join("dialog"), "dialog"));
    targets.extend(dir_upgrade_targets(&content_dir.join("quests"), "quest"));

    for target in targets {
        match upgrade_file(&target, options.dry_run) {
            Ok(UpgradeOutcome::Written) => updated += 1,
            Ok(UpgradeOutcome::WouldWrite) => updated += 1,
            Ok(UpgradeOutcome::SkippedUnknown(paths)) => {
                skipped_unknown += 1;
                eprintln!("{}: unknown fields detected:", target.path.display());
                for path in paths {
                    eprintln!("- {}", path);
                }
            }
            Ok(UpgradeOutcome::SkippedMissing) => {}
            Err(err) => {
                errors += 1;
                eprintln!("{}", err);
            }
        }
    }

    if options.dry_run {
        println!(
            "Upgrade dry run complete: {} files would update, {} skipped (unknown fields), {} errors",
            updated, skipped_unknown, errors
        );
    } else {
        println!(
            "Upgrade complete: {} updated, {} skipped (unknown fields), {} errors",
            updated, skipped_unknown, errors
        );
    }
}

fn run_build_new_project(args: &[String]) {
    let options = BuildNewProjectOptions::from_args(args);
    let Some(name) = options.name else {
        eprintln!("Missing project name. Example: cryst build new-project my_game");
        return;
    };

    let dir_name = slugify(&name);
    let target_dir = options
        .path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("content").join(dir_name));

    if target_dir.exists() {
        let has_entries = fs::read_dir(&target_dir)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(true);
        if has_entries {
            eprintln!("{} already exists and is not empty", target_dir.display());
            return;
        }
    }

    if let Err(err) = create_project_structure(&target_dir, &name) {
        eprintln!("{}", err);
        return;
    }

    println!("Created new project at {}", target_dir.display());
}

fn create_project_structure(target_dir: &Path, title: &str) -> Result<(), String> {
    fs::create_dir_all(target_dir).map_err(|err| err.to_string())?;
    fs::create_dir_all(target_dir.join("entities")).map_err(|err| err.to_string())?;
    fs::create_dir_all(target_dir.join("maps")).map_err(|err| err.to_string())?;
    fs::create_dir_all(target_dir.join("events")).map_err(|err| err.to_string())?;
    fs::create_dir_all(target_dir.join("dialog")).map_err(|err| err.to_string())?;
    fs::create_dir_all(target_dir.join("quests")).map_err(|err| err.to_string())?;
    fs::create_dir_all(target_dir.join("ui")).map_err(|err| err.to_string())?;

    let rules = RulesFile {
        version: 1,
        game: GameRules {
            title: title.to_string(),
            description: None,
            author: None,
            party_size: 4,
            party_reserve_size: 4,
            battle_mode: BattleMode::Turn,
            magic_system: MagicSystem::Mp,
            magic_acquisition: MagicAcquisition::Level,
            ability_acquisition: AbilityAcquisition::Level,
            start_event: None,
            start_location: engine::rules::StartLocation {
                world: "default_world".to_string(),
                map: "starting_map".to_string(),
                x: 0,
                y: 0,
            },
            currency: Currency {
                id: "gil".to_string(),
                name: "G".to_string(),
                symbol: "G".to_string(),
            },
            readiness_speed: 2.0,
        },
        battle: engine::rules::BattleRules::default(),
        party_mode: PartyMode::Create,
        party_create: PartyCreateRules::default(),
        exp_curve: ExpCurveRules {
            mode: "table".to_string(),
            table: vec![0, 10],
            formula: None,
            max_level: 2,
        },
        inventory: engine::rules::InventoryRules::default(),
        systems: default_systems(),
        save: SaveRules::default(),
        settings: SettingsRules::default(),
        render: RenderRules {
            min_art_width: 110,
            min_art_height: 32,
            palette: "terminal".to_string(),
        },
        stats: StatsRules { track: Vec::new() },
        job_system: JobSystemRules {
            progression_mode: JobProgressionMode::Job,
            secondary_jobs: false,
            jp_mode: JpMode::Earn,
            job_exp_curve: ExpCurveRules {
                mode: "table".to_string(),
                table: vec![0, 10],
                formula: None,
                max_level: 2,
            },
        },
    };

    let effects = EffectsFile {
        version: 1,
        elements: Vec::new(),
        effects: Vec::new(),
        statuses: Vec::new(),
        traits: Vec::new(),
    };

    let worlds = WorldsFile {
        version: 1,
        worlds: vec![WorldDefinition {
            id: "default_world".to_string(),
            name: "Default World".to_string(),
            starting_map: "starting_map".to_string(),
            overworld_map_id: "starting_map".to_string(),
            zoom_levels: vec!["explore".to_string()],
            overview: OverviewConfig {
                enabled: false,
                map_id: "starting_map".to_string(),
            },
            vehicles: Vec::new(),
            fast_travel: FastTravelConfig {
                enabled: false,
                requires_flag: "world.fast_travel_unlocked".to_string(),
            },
            links: Vec::new(),
        }],
    };

    let stats = StatsFile {
        version: 1,
        stats: StatsDefinition {
            base: vec![
                StatEntry {
                    id: "hp".to_string(),
                    name: "HP".to_string(),
                    min: Some(0),
                },
                StatEntry {
                    id: "mp".to_string(),
                    name: "MP".to_string(),
                    min: Some(0),
                },
                StatEntry {
                    id: "str".to_string(),
                    name: "STR".to_string(),
                    min: None,
                },
                StatEntry {
                    id: "int".to_string(),
                    name: "INT".to_string(),
                    min: None,
                },
                StatEntry {
                    id: "vit".to_string(),
                    name: "VIT".to_string(),
                    min: None,
                },
                StatEntry {
                    id: "agi".to_string(),
                    name: "AGI".to_string(),
                    min: None,
                },
                StatEntry {
                    id: "lck".to_string(),
                    name: "LCK".to_string(),
                    min: None,
                },
            ],
            derived: Vec::new(),
            formulas: HashMap::new(),
        },
    };

    let input = InputFile {
        version: 1,
        bindings: default_input_bindings(),
    };

    let jobs = JobsFile {
        version: 1,
        jobs: vec![default_job_definition()],
    };

    let spells = SpellsFile {
        version: 1,
        schools: vec![MagicSchool {
            id: "white".to_string(),
            name: "White".to_string(),
        }],
        spells: Vec::new(),
    };

    let abilities = AbilitiesFile {
        version: 1,
        abilities: Vec::new(),
    };

    let items = ItemsFile {
        version: 1,
        items: Vec::new(),
    };

    let equipment = EquipmentFile {
        version: 1,
        equipment: Vec::new(),
    };

    let enemies = EnemiesFile {
        version: 1,
        enemies: Vec::new(),
    };

    let vehicles = VehiclesFile {
        version: 1,
        vehicles: Vec::new(),
    };

    let shops = ShopsFile {
        version: 1,
        shops: Vec::new(),
    };

    let npcs = NpcsFile {
        version: 1,
        npcs: Vec::new(),
    };

    let encounters = EncountersFile {
        version: 1,
        tables: Vec::new(),
    };

    let starting_map = MapFile {
        version: 1,
        id: "starting_map".to_string(),
        name: "Starting Map".to_string(),
        hide_name: false,
        world: "default_world".to_string(),
        width: 1,
        height: 1,
        tiles: vec![".".to_string()],
        legend: HashMap::from([(
            ".".to_string(),
            TileLegend {
                tile: "floor".to_string(),
                passable: true,
                palette: Some("green".to_string()),
            },
        )]),
        encounters: Vec::new(),
        encounter_rate: 0.0,
        events: Vec::new(),
        npcs: Vec::new(),
        signs: Vec::new(),
        chests: Vec::new(),
        shops: Vec::new(),
        allow_save: true,
        save_points: Vec::new(),
        transitions: Vec::new(),
        vehicles: Vec::new(),
    };

    let title_ui = TitleUiFile {
        version: 1,
        title: title.to_string(),
        logo: TitleLogo {
            lines: vec!["OpenCrystal".to_string()],
            palette: None,
            line_palettes: None,
        },
        menu: vec![
            MenuItem {
                id: "new_game".to_string(),
                label: "New Game".to_string(),
            },
            MenuItem {
                id: "load_game".to_string(),
                label: "Load".to_string(),
            },
            MenuItem {
                id: "settings".to_string(),
                label: "Settings".to_string(),
            },
            MenuItem {
                id: "exit".to_string(),
                label: "Exit".to_string(),
            },
        ],
        footer: FooterConfig {
            left: "A crystal-bound journey".to_string(),
            right: "".to_string(),
        },
    };

    let menu_ui = MenuUiFile {
        version: 1,
        layout: MenuLayout {
            left_width_ratio: 0.4,
            right_width_ratio: 0.6,
        },
        default_panel: "party_status".to_string(),
        menu: vec![
            MenuEntry {
                id: "items".to_string(),
                label: "Items".to_string(),
                action: "items".to_string(),
                enabled: true,
                system: Some("items".to_string()),
                unlock_flag: None,
                locked_behavior: None,
            },
            MenuEntry {
                id: "status".to_string(),
                label: "Status".to_string(),
                action: "status".to_string(),
                enabled: true,
                system: Some("status".to_string()),
                unlock_flag: None,
                locked_behavior: None,
            },
            MenuEntry {
                id: "gameplay_stats".to_string(),
                label: "Gameplay Stats".to_string(),
                action: "gameplay_stats".to_string(),
                enabled: true,
                system: Some("gameplay_stats".to_string()),
                unlock_flag: None,
                locked_behavior: None,
            },
            MenuEntry {
                id: "party".to_string(),
                label: "Party".to_string(),
                action: "party".to_string(),
                enabled: true,
                system: Some("party".to_string()),
                unlock_flag: None,
                locked_behavior: None,
            },
            MenuEntry {
                id: "save".to_string(),
                label: "Save".to_string(),
                action: "save".to_string(),
                enabled: true,
                system: Some("save".to_string()),
                unlock_flag: None,
                locked_behavior: Some("disable".to_string()),
            },
            MenuEntry {
                id: "settings".to_string(),
                label: "Settings".to_string(),
                action: "settings".to_string(),
                enabled: true,
                system: Some("settings".to_string()),
                unlock_flag: None,
                locked_behavior: None,
            },
            MenuEntry {
                id: "exit".to_string(),
                label: "Exit".to_string(),
                action: "exit".to_string(),
                enabled: true,
                system: None,
                unlock_flag: None,
                locked_behavior: None,
            },
        ],
        panels: vec![
            MenuPanel {
                id: "party_status".to_string(),
                title: "Party".to_string(),
                panel_type: "party_summary".to_string(),
                source: None,
            },
            MenuPanel {
                id: "gameplay_stats".to_string(),
                title: "Gameplay Stats".to_string(),
                panel_type: "progress".to_string(),
                source: Some("ui/gameplay_stats.json".to_string()),
            },
        ],
    };

    let battle_ui = BattleUiFile {
        version: 1,
        breakpoints: vec![Breakpoint {
            id: "standard".to_string(),
            min_width: 0,
            min_height: 0,
            behavior: BreakpointBehavior {
                enemy_art: "glyph".to_string(),
                hide_panel_titles: false,
            },
        }],
        layout: BattleLayout {
            battlefield: PanelAnchor {
                anchor: "top".to_string(),
                height_ratio: 0.6,
            },
            command_row: CommandRow {
                anchor: "bottom".to_string(),
                height_ratio: 0.4,
                columns: vec![
                    ColumnSpec {
                        id: "enemies".to_string(),
                        width_ratio: 0.3,
                    },
                    ColumnSpec {
                        id: "commands".to_string(),
                        width_ratio: 0.4,
                    },
                    ColumnSpec {
                        id: "party".to_string(),
                        width_ratio: 0.3,
                    },
                ],
            },
            party_grid: PartyGrid { columns: 1 },
        },
        log: Some(BattleLog {
            position: "top".to_string(),
            height: 2,
            auto_advance_ms: 700,
            allow_skip: true,
        }),
        animation: Some(BattleAnimation {
            flash_ms: 150,
            flash_cycles: 2,
        }),
        panels: BattlePanels {
            enemies: EnemyPanel {
                title: "Enemies".to_string(),
                highlight: HighlightRules {
                    style: "invert".to_string(),
                    link_to_battlefield: true,
                },
            },
            commands: CommandPanel {
                title: "Commands".to_string(),
                items: vec![
                    "Attack".to_string(),
                    "Magic".to_string(),
                    "Abilities".to_string(),
                    "Items".to_string(),
                    "Defend".to_string(),
                    "Run".to_string(),
                ],
                page_size: 6,
            },
            party: PartyPanel {
                title: "Party".to_string(),
                show: vec![
                    "hp".to_string(),
                    "mp".to_string(),
                    "readiness".to_string(),
                    "status".to_string(),
                ],
                highlight: HighlightRules {
                    style: "underline".to_string(),
                    link_to_battlefield: true,
                },
            },
        },
        menus: BattleMenus {
            attack: AttackMenu {
                target: "enemy".to_string(),
            },
            magic: MagicMenu {
                list: "spells".to_string(),
                group_by: "school".to_string(),
                columns: vec![
                    MenuColumn {
                        id: "name".to_string(),
                        label: "Spell".to_string(),
                    },
                    MenuColumn {
                        id: "cost".to_string(),
                        label: "MP".to_string(),
                    },
                ],
                target_from_spell: true,
            },
            abilities: AbilitiesMenu {
                list: "abilities".to_string(),
                columns: vec![MenuColumn {
                    id: "name".to_string(),
                    label: "Ability".to_string(),
                }],
                target_from_ability: true,
            },
            items: ItemsMenu {
                list: "inventory".to_string(),
                columns: vec![
                    MenuColumn {
                        id: "name".to_string(),
                        label: "Item".to_string(),
                    },
                    MenuColumn {
                        id: "qty".to_string(),
                        label: "Qty".to_string(),
                    },
                ],
                target_from_item: true,
            },
        },
        selection: SelectionRules {
            target_cursor: "blink".to_string(),
            battlefield_highlight: "outline".to_string(),
            list_highlight: "invert".to_string(),
        },
    };

    let dialog_ui = DialogUiFile {
        version: 1,
        position: "bottom".to_string(),
        height: 4,
        show_speaker: true,
        continue_marker: ">".to_string(),
    };

    let progress_ui = ProgressUiFile {
        version: 1,
        panels: vec![ProgressPanel {
            id: "gameplay_stats".to_string(),
            title: "Gameplay Stats".to_string(),
            items: vec![ProgressItem {
                label: "Steps".to_string(),
                value: "time_played".to_string(),
                max: None,
            }],
        }],
    };

    write_json_pretty(target_dir.join("rules.json"), &rules)?;
    write_json_pretty(target_dir.join("effects.json"), &effects)?;
    write_json_pretty(target_dir.join("worlds.json"), &worlds)?;
    write_json_pretty(target_dir.join("stats.json"), &stats)?;
    write_json_pretty(target_dir.join("input.json"), &input)?;
    write_json_pretty(target_dir.join("entities/jobs.json"), &jobs)?;
    write_json_pretty(target_dir.join("entities/spells.json"), &spells)?;
    write_json_pretty(target_dir.join("entities/abilities.json"), &abilities)?;
    write_json_pretty(target_dir.join("entities/items.json"), &items)?;
    write_json_pretty(target_dir.join("entities/equipment.json"), &equipment)?;
    write_json_pretty(target_dir.join("entities/enemies.json"), &enemies)?;
    write_json_pretty(target_dir.join("entities/vehicles.json"), &vehicles)?;
    write_json_pretty(target_dir.join("entities/shops.json"), &shops)?;
    write_json_pretty(target_dir.join("entities/npcs.json"), &npcs)?;
    write_json_pretty(target_dir.join("entities/encounters.json"), &encounters)?;
    write_json_pretty(
        target_dir.join("maps").join("starting_map.json"),
        &starting_map,
    )?;
    write_json_pretty(target_dir.join("ui/title.json"), &title_ui)?;
    write_json_pretty(target_dir.join("ui/menu.json"), &menu_ui)?;
    write_json_pretty(target_dir.join("ui/battle.json"), &battle_ui)?;
    write_json_pretty(target_dir.join("ui/dialog.json"), &dialog_ui)?;
    write_json_pretty(target_dir.join("ui/gameplay_stats.json"), &progress_ui)?;

    Ok(())
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
        traits: Vec::new(),
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
        currency: 1,
        jp: 0,
        loot: Vec::new(),
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
        currency: "gil".to_string(),
        inventory: Vec::new(),
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

fn load_or_default<T>(path: &Path, default_value: T) -> Result<T, String>
where
    T: DeserializeOwned + Serialize,
{
    if path.exists() {
        load_json(path)
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        write_json_pretty(path, &default_value)?;
        Ok(default_value)
    }
}

fn resolve_content_dir(explicit: Option<&str>) -> Result<PathBuf, String> {
    if let Some(value) = explicit {
        return Ok(PathBuf::from(value));
    }
    let current = std::env::current_dir().map_err(|err| err.to_string())?;
    if current.join("rules.json").exists() {
        return Ok(current);
    }
    Err("No --content provided and current directory is not a content pack".to_string())
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

fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in value.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    while out.starts_with('_') {
        out.remove(0);
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "opencrystal".to_string()
    } else {
        out
    }
}

fn default_systems() -> HashMap<String, bool> {
    HashMap::from([
        ("items".to_string(), true),
        ("magic".to_string(), true),
        ("equipment".to_string(), true),
        ("status".to_string(), true),
        ("party".to_string(), true),
        ("gameplay_stats".to_string(), true),
        ("jobs".to_string(), true),
        ("journal".to_string(), false),
        ("fast_travel".to_string(), false),
        ("overworld_map".to_string(), false),
        ("save".to_string(), true),
        ("settings".to_string(), true),
    ])
}

fn default_input_bindings() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "move_up".to_string(),
            vec!["Up".to_string(), "W".to_string(), "K".to_string()],
        ),
        (
            "move_down".to_string(),
            vec!["Down".to_string(), "S".to_string(), "J".to_string()],
        ),
        (
            "move_left".to_string(),
            vec!["Left".to_string(), "A".to_string(), "H".to_string()],
        ),
        (
            "move_right".to_string(),
            vec!["Right".to_string(), "D".to_string(), "L".to_string()],
        ),
        (
            "confirm".to_string(),
            vec!["Enter".to_string(), "C".to_string()],
        ),
        ("cancel".to_string(), vec!["X".to_string()]),
        (
            "menu".to_string(),
            vec!["I".to_string(), "Escape".to_string()],
        ),
        ("pause".to_string(), vec!["Space".to_string()]),
        ("quit".to_string(), vec!["Q".to_string()]),
    ])
}

fn default_job_definition() -> JobDefinition {
    JobDefinition {
        id: "adventurer".to_string(),
        name: "Adventurer".to_string(),
        stats: HashMap::from([
            ("hp".to_string(), 30),
            ("mp".to_string(), 5),
            ("str".to_string(), 4),
            ("int".to_string(), 3),
            ("vit".to_string(), 3),
            ("agi".to_string(), 3),
            ("lck".to_string(), 3),
        ]),
        growth: engine::entities::GrowthConfig {
            mode: "formula".to_string(),
            per_level: HashMap::from([
                ("hp".to_string(), "2".to_string()),
                ("mp".to_string(), "1".to_string()),
                ("str".to_string(), "1".to_string()),
                ("int".to_string(), "1".to_string()),
                ("vit".to_string(), "1".to_string()),
                ("agi".to_string(), "1".to_string()),
                ("lck".to_string(), "1".to_string()),
            ]),
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
        is_default: true,
        sort_order: Some(10),
        magic_slots: None,
        magic_equip_progression: None,
        description: Some("A balanced starting job.".to_string()),
        magic_schools: Vec::new(),
        acquisition: None,
    }
}

struct BuildNewOptions {
    kind: Option<String>,
    id: Option<String>,
    name: Option<String>,
    content_dir: Option<String>,
    force: bool,
}

impl BuildNewOptions {
    fn from_args(args: &[String]) -> Self {
        let content_dir = flag_value(args, "--content");
        let name = flag_value(args, "--name");
        let force = has_flag(args, "--force");
        let positionals = collect_positionals(args);
        let kind = positionals.get(0).cloned();
        let id = positionals.get(1).map(|value| slugify(value));
        Self {
            kind,
            id,
            name,
            content_dir,
            force,
        }
    }
}

struct BuildUpgradeOptions {
    content_dir: Option<String>,
    dry_run: bool,
}

impl BuildUpgradeOptions {
    fn from_args(args: &[String]) -> Self {
        Self {
            content_dir: flag_value(args, "--content"),
            dry_run: has_flag(args, "--dry-run"),
        }
    }
}

struct BuildNewProjectOptions {
    name: Option<String>,
    path: Option<String>,
}

impl BuildNewProjectOptions {
    fn from_args(args: &[String]) -> Self {
        let path = flag_value(args, "--path");
        let positionals = collect_positionals(args);
        let name = positionals.get(0).cloned();
        Self { name, path }
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix(&format!("{}=", flag)) {
            return Some(value.to_string());
        }
        if arg == flag {
            return iter.next().cloned();
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn collect_positionals(args: &[String]) -> Vec<String> {
    let mut positionals = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            if arg == "--force" || arg == "--dry-run" {
                continue;
            }
            if arg.contains('=') {
                continue;
            }
            if arg == "--content" || arg == "--name" || arg == "--path" {
                iter.next();
            }
            continue;
        }
        positionals.push(arg.to_string());
    }
    positionals
}

fn print_build_usage() {
    println!(
        "Build usage:\n  cryst build new <kind> <id> [--content path] [--name label] [--force]\n  cryst build upgrade [--content path] [--dry-run]\n  cryst build new-project <name> [--path path]"
    );
}

struct UpgradeTarget {
    path: PathBuf,
    kind: UpgradeKind,
}

enum UpgradeKind {
    Rules,
    Effects,
    Worlds,
    Stats,
    Input,
    Party,
    Jobs,
    Spells,
    Abilities,
    Items,
    Equipment,
    Enemies,
    Vehicles,
    Shops,
    Npcs,
    Encounters,
    Map,
    Event,
    Dialog,
    Quest,
    TitleUi,
    MenuUi,
    BattleUi,
    DialogUi,
    ProgressUi,
}

fn base_upgrade_targets(content_dir: &Path) -> Vec<UpgradeTarget> {
    vec![
        UpgradeTarget {
            path: content_dir.join("rules.json"),
            kind: UpgradeKind::Rules,
        },
        UpgradeTarget {
            path: content_dir.join("effects.json"),
            kind: UpgradeKind::Effects,
        },
        UpgradeTarget {
            path: content_dir.join("worlds.json"),
            kind: UpgradeKind::Worlds,
        },
        UpgradeTarget {
            path: content_dir.join("stats.json"),
            kind: UpgradeKind::Stats,
        },
        UpgradeTarget {
            path: content_dir.join("input.json"),
            kind: UpgradeKind::Input,
        },
        UpgradeTarget {
            path: content_dir.join("party.json"),
            kind: UpgradeKind::Party,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("jobs.json"),
            kind: UpgradeKind::Jobs,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("spells.json"),
            kind: UpgradeKind::Spells,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("abilities.json"),
            kind: UpgradeKind::Abilities,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("items.json"),
            kind: UpgradeKind::Items,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("equipment.json"),
            kind: UpgradeKind::Equipment,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("enemies.json"),
            kind: UpgradeKind::Enemies,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("vehicles.json"),
            kind: UpgradeKind::Vehicles,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("shops.json"),
            kind: UpgradeKind::Shops,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("npcs.json"),
            kind: UpgradeKind::Npcs,
        },
        UpgradeTarget {
            path: content_dir.join("entities").join("encounters.json"),
            kind: UpgradeKind::Encounters,
        },
        UpgradeTarget {
            path: content_dir.join("ui").join("title.json"),
            kind: UpgradeKind::TitleUi,
        },
        UpgradeTarget {
            path: content_dir.join("ui").join("menu.json"),
            kind: UpgradeKind::MenuUi,
        },
        UpgradeTarget {
            path: content_dir.join("ui").join("battle.json"),
            kind: UpgradeKind::BattleUi,
        },
        UpgradeTarget {
            path: content_dir.join("ui").join("dialog.json"),
            kind: UpgradeKind::DialogUi,
        },
        UpgradeTarget {
            path: content_dir.join("ui").join("gameplay_stats.json"),
            kind: UpgradeKind::ProgressUi,
        },
    ]
}

fn dir_upgrade_targets(dir: &Path, kind_label: &str) -> Vec<UpgradeTarget> {
    let mut targets = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return targets,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let kind = match kind_label {
            "map" => UpgradeKind::Map,
            "event" => UpgradeKind::Event,
            "dialog" => UpgradeKind::Dialog,
            "quest" => UpgradeKind::Quest,
            _ => continue,
        };
        targets.push(UpgradeTarget { path, kind });
    }
    targets
}

enum UpgradeOutcome {
    Written,
    WouldWrite,
    SkippedUnknown(Vec<String>),
    SkippedMissing,
}

fn upgrade_file(target: &UpgradeTarget, dry_run: bool) -> Result<UpgradeOutcome, String> {
    if !target.path.exists() {
        return Ok(UpgradeOutcome::SkippedMissing);
    }
    match target.kind {
        UpgradeKind::Rules => upgrade_typed::<RulesFile>(&target.path, dry_run),
        UpgradeKind::Effects => upgrade_typed::<EffectsFile>(&target.path, dry_run),
        UpgradeKind::Worlds => upgrade_typed::<WorldsFile>(&target.path, dry_run),
        UpgradeKind::Stats => upgrade_typed::<StatsFile>(&target.path, dry_run),
        UpgradeKind::Input => upgrade_typed::<InputFile>(&target.path, dry_run),
        UpgradeKind::Party => upgrade_typed::<engine::party::PartyFile>(&target.path, dry_run),
        UpgradeKind::Jobs => upgrade_typed::<JobsFile>(&target.path, dry_run),
        UpgradeKind::Spells => upgrade_typed::<SpellsFile>(&target.path, dry_run),
        UpgradeKind::Abilities => upgrade_typed::<AbilitiesFile>(&target.path, dry_run),
        UpgradeKind::Items => upgrade_typed::<ItemsFile>(&target.path, dry_run),
        UpgradeKind::Equipment => upgrade_typed::<EquipmentFile>(&target.path, dry_run),
        UpgradeKind::Enemies => upgrade_typed::<EnemiesFile>(&target.path, dry_run),
        UpgradeKind::Vehicles => upgrade_typed::<VehiclesFile>(&target.path, dry_run),
        UpgradeKind::Shops => upgrade_typed::<ShopsFile>(&target.path, dry_run),
        UpgradeKind::Npcs => upgrade_typed::<NpcsFile>(&target.path, dry_run),
        UpgradeKind::Encounters => upgrade_typed::<EncountersFile>(&target.path, dry_run),
        UpgradeKind::Map => upgrade_typed::<MapFile>(&target.path, dry_run),
        UpgradeKind::Event => upgrade_typed::<engine::events::EventFile>(&target.path, dry_run),
        UpgradeKind::Dialog => upgrade_typed::<engine::dialog::DialogFile>(&target.path, dry_run),
        UpgradeKind::Quest => upgrade_typed::<QuestsFile>(&target.path, dry_run),
        UpgradeKind::TitleUi => upgrade_typed::<TitleUiFile>(&target.path, dry_run),
        UpgradeKind::MenuUi => upgrade_typed::<MenuUiFile>(&target.path, dry_run),
        UpgradeKind::BattleUi => upgrade_typed::<BattleUiFile>(&target.path, dry_run),
        UpgradeKind::DialogUi => upgrade_typed::<DialogUiFile>(&target.path, dry_run),
        UpgradeKind::ProgressUi => upgrade_typed::<ProgressUiFile>(&target.path, dry_run),
    }
}

fn upgrade_typed<T>(path: &Path, dry_run: bool) -> Result<UpgradeOutcome, String>
where
    T: DeserializeOwned + Serialize,
{
    let content = fs::read_to_string(path).map_err(|err| format!("{}: {}", path.display(), err))?;
    let mut unused = Vec::new();
    let mut deserializer = Deserializer::from_str(&content);
    let value: T = serde_ignored::deserialize(&mut deserializer, |path| {
        unused.push(path.to_string());
    })
    .map_err(|err| format!("{}: {}", path.display(), err))?;

    if !unused.is_empty() {
        return Ok(UpgradeOutcome::SkippedUnknown(unused));
    }

    if dry_run {
        return Ok(UpgradeOutcome::WouldWrite);
    }
    write_json_pretty(path, &value)?;
    Ok(UpgradeOutcome::Written)
}
