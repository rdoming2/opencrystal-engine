use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use engine::encounters::EncountersFile;
use engine::entities::{
    AbilitiesFile, EffectsFile, EnemiesFile, EquipmentFile, ItemsFile, JobDefinition, JobEquipment,
    JobSprite, JobsFile, MagicSchool, NpcsFile, ShopsFile, SpellsFile, VehiclesFile,
};
use engine::io::write_json_pretty;
use engine::maps::{MapFile, MapLoop, TileLegend};
use engine::quests::QuestsFile;
use engine::rules::{
    AbilityAcquisition, ActivityGrowthRules, ActivityProgressionRules, BattleMode, Currency,
    ExpCurveRules, GameRules, JobSystemRules, JpMode, MagicAcquisition, MagicSystem,
    NewGamePlusCarryoverRules, NewGamePlusRules, PartyCreateRules, PartyMode, ProgressionMode,
    RenderRules, RulesFile, SaveRules, SettingsRules, StatsRules,
};
use engine::stats::{StatEntry, StatsDefinition, StatsFile};
use engine::world::{FastTravelConfig, WorldDefinition, WorldsFile};
use tui::input::InputFile;
use tui::ui::{
    AbilitiesMenu, AttackMenu, BattleAnimation, BattleLayout, BattleLog, BattleMenus, BattlePanels,
    BattleUiFile, Breakpoint, BreakpointBehavior, ColumnSpec, CommandPanel, CommandRow,
    DialogUiFile, EnemyPanel, FooterConfig, HighlightRules, ItemsMenu, MagicMenu, MenuColumn,
    MenuEntry, MenuItem, MenuLayout, MenuPanel, MenuUiFile, PanelAnchor, PartyGrid, PartyPanel,
    ProgressItem, ProgressPanel, ProgressUiFile, SelectionRules, TitleLogo, TitleUiFile,
};

use super::args::BuildNewProjectOptions;
use super::common::slugify;

pub(crate) fn run_build_new_project(args: &[String]) {
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
    fs::create_dir_all(target_dir.join("ui")).map_err(|err| err.to_string())?;

    let mut rules = RulesFile {
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
            currencies: vec![Currency {
                id: "gold".to_string(),
                name: "Gold".to_string(),
                symbol: "G".to_string(),
            }],
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
        progression_mode: ProgressionMode::Job,
        activity_progression: ActivityProgressionRules::default(),
        activity_growth: ActivityGrowthRules::default(),
        inventory: engine::rules::InventoryRules::default(),
        systems: default_systems(),
        save: SaveRules {
            slots_max: 10,
            new_game_plus: NewGamePlusRules {
                carryover: NewGamePlusCarryoverRules::default(),
            },
        },
        settings: SettingsRules::default(),
        render: RenderRules {
            min_art_width: 110,
            min_art_height: 32,
            palette: "terminal".to_string(),
            death_markers: engine::rules::DeathMarkerRules::default(),
        },
        stats: StatsRules { track: Vec::new() },
        job_system: JobSystemRules {
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
    rules.battle.enemy_ai_defaults.palliative_cooldown_turns = 1;
    rules.battle.enemy_ai_defaults.palliative_reroll_chance = 0.5;

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
            growth_formulas: HashMap::new(),
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
        loop_config: MapLoop::default(),
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
        doors: Vec::new(),
        puzzles: Vec::new(),
        campfires: Vec::new(),
        allow_save: true,
        save_points: Vec::new(),
        transitions: Vec::new(),
        vehicles: Vec::new(),
    };

    let title_ui = TitleUiFile {
        version: 1,
        title: title.to_string(),
        logo: TitleLogo {
            lines: vec![
                "    .   .   /#\\     .".to_string(),
                "  *        /*##\\  .    *".to_string(),
                "'      .  /**###\\    .  '".to_string(),
                "    <    /***####\\      >".to_string(),
                "  .   *  \\***####/  *   .".to_string(),
                "*       . \\**###/   .    *".to_string(),
                "    .      \\*##/  *      .".to_string(),
                "  '      *  \\#/  .    '".to_string(),
                "     OpenCrystal Engine".to_string(),
            ],
            palette: None,
            line_palettes: Some(vec![
                "bright_cyan".to_string(),
                "bright_cyan".to_string(),
                "bright_cyan".to_string(),
                "bright_cyan".to_string(),
                "cyan".to_string(),
                "cyan".to_string(),
                "cyan".to_string(),
                "cyan".to_string(),
                "white".to_string(),
            ]),
        },
        menu: vec![
            MenuItem {
                id: "new_game".to_string(),
                label: "New Game".to_string(),
            },
            MenuItem {
                id: "new_game_plus".to_string(),
                label: "New Game+".to_string(),
            },
            MenuItem {
                id: "load_game".to_string(),
                label: "Load".to_string(),
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
        gameover: None,
        endgame: None,
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
                        width_ratio: 0.3,
                    },
                    ColumnSpec {
                        id: "party".to_string(),
                        width_ratio: 0.4,
                    },
                ],
            },
            party_grid: PartyGrid { columns: 1 },
        },
        log: Some(BattleLog {
            position: "pane_top".to_string(),
            height: 3,
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
                hp_colors: None,
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

    let quests = QuestsFile {
        version: 1,
        categories: Vec::new(),
        quests: Vec::new(),
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
    write_json_pretty(target_dir.join("entities/quests.json"), &quests)?;
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
