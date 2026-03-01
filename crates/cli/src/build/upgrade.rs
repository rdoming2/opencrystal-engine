use std::fs;
use std::path::{Path, PathBuf};

use engine::encounters::EncountersFile;
use engine::entities::{
    AbilitiesFile, EffectsFile, EnemiesFile, EquipmentFile, ItemsFile, JobsFile, NpcsFile,
    ShopsFile, SpellsFile, VehiclesFile,
};
use engine::io::write_json_pretty;
use engine::maps::MapFile;
use engine::quests::QuestsFile;
use engine::rules::RulesFile;
use engine::stats::StatsFile;
use engine::world::WorldsFile;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Deserializer;
use tui::input::InputFile;
use tui::ui::{BattleUiFile, DialogUiFile, MenuUiFile, ProgressUiFile, TitleUiFile};

use super::args::BuildUpgradeOptions;
use super::common::resolve_content_dir;

pub(crate) fn run_build_upgrade(args: &[String]) {
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
            path: content_dir.join("entities").join("quests.json"),
            kind: UpgradeKind::Quest,
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
