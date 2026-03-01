use std::path::Path;

use engine::io::write_json_pretty;
use serde_json::{Map, Number, Value};

use super::args::BuildStringsOptions;
use super::common::resolve_content_dir;

pub(crate) fn run_build_strings(args: &[String]) {
    let options = BuildStringsOptions::from_args(args);
    let content_dir = match resolve_content_dir(options.content_dir.as_deref()) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    match build_strings_stub(&content_dir, options.force) {
        Ok(message) => println!("{}", message),
        Err(err) => eprintln!("{}", err),
    }
}

fn build_strings_stub(content_dir: &Path, force: bool) -> Result<String, String> {
    let path = content_dir.join("ui").join("strings.json");
    if path.exists() && !force {
        return Err(format!(
            "{} already exists (use --force to overwrite)",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let mut strings = Map::new();
    for (key, value) in default_strings_entries() {
        strings.insert(key.to_string(), Value::String(value.to_string()));
    }

    let mut root = Map::new();
    root.insert("version".to_string(), Value::Number(Number::from(1)));
    root.insert("strings".to_string(), Value::Object(strings));
    write_json_pretty(&path, &Value::Object(root))?;
    Ok(format!("Wrote {}", path.display()))
}

fn default_strings_entries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("gameover.title", "Game Over"),
        ("gameover.subtitle", "The party has fallen."),
        ("gameover.retry_battle", "Retry Battle"),
        ("gameover.load_latest", "Load Latest Save"),
        ("gameover.load_autosave", "Load Autosave"),
        ("gameover.return_title", "Return to Title"),
        ("gameover.exit", "Exit"),
        ("command.attack", "Attack"),
        ("command.magic", "Magic"),
        ("command.abilities", "Abilities"),
        ("command.items", "Items"),
        ("command.defend", "Defend"),
        ("command.run", "Run"),
        ("battle.start", "A battle begins!"),
        ("battle.victory", "Victory!"),
        ("battle.defeat", "Defeat..."),
        ("battle.victory_title", "Victory!"),
        ("battle.items_found", "Items found:"),
        ("battle.victory_prompt", "Press Confirm to continue."),
        ("battle.level_up", "{actor} reached Level {level}!"),
        ("battle.level_up_prompt", "Press Confirm to continue."),
        ("battle.growth", "{actor} grows stronger."),
        ("battle.growth_prompt", "Press Confirm to continue."),
        ("battle.pause_title", "PAUSED"),
        ("battle.pause_hint", "Press Pause to resume"),
        ("battle.command_unavailable", "Command unavailable."),
        ("battle.no_targets", "No valid targets."),
        ("battle.no_target", "No target."),
        ("battle.no_fallen", "No fallen allies."),
        ("battle.no_spells", "No spells available."),
        ("battle.no_abilities", "No abilities available."),
        ("battle.no_items", "No items available."),
        ("battle.escape_success", "Escaped!"),
        ("battle.escape_fail", "Escape failed!"),
        ("battle.cast_unavailable", "Cannot cast."),
        ("battle.item_unusable", "Item unusable."),
        ("battle.item_none_left", "No items left."),
        ("battle.ability_unavailable", "Cannot use ability."),
        ("battle.cost_failed", "Failed to pay cost."),
        ("battle.nothing_happens", "Nothing happens."),
        (
            "battle.log.attack",
            "{actor} attacks {target} for {damage} HP.",
        ),
        (
            "battle.log.cast",
            "{actor} casts {spell} on {target} for {damage} HP.",
        ),
        (
            "battle.log.cast_party",
            "{actor} casts {spell} on the party.",
        ),
        (
            "battle.log.ability",
            "{actor} uses {ability} on {target} for {damage} HP.",
        ),
        (
            "battle.log.ability_party",
            "{actor} uses {ability} on the party.",
        ),
        ("battle.log.critical", "Critical hit!"),
        ("battle.log.cover", "{coverer} covers {target}!"),
        ("battle.log.parry", "{target} parries the attack!"),
        ("battle.log.brace", "{target} braces for impact!"),
        (
            "battle.log.counter",
            "{actor} counters {target} for {damage} HP.",
        ),
        ("battle.log.defend", "{actor} defends."),
        ("battle.log.row", "{actor} moves to the {row} row."),
        ("battle.log.miss", "{actor} misses {target}."),
        ("battle.log.fall", "{actor} falls!"),
        (
            "battle.log.healing_damage",
            "{target} is harmed by healing for {damage} HP.",
        ),
        ("battle.log.heal", "{target} recovers {amount} HP."),
        (
            "battle.log.scan",
            "{actor} scans {target}: {current}/{max} HP.",
        ),
        ("battle.log.status", "{target} is affected by {status}."),
        ("battle.log.steal_success", "{actor} steals {item}."),
        ("battle.log.steal_fail", "{actor} fails to steal anything."),
        (
            "battle.log.throw",
            "{actor} throws {item} at {target} for {damage} HP.",
        ),
        ("battle.log.parry_ready", "{actor} readies a parry."),
        ("battle.log.counter_ready", "{actor} readies a counter."),
        ("battle.log.cover_ready", "{actor} will cover {target}."),
        (
            "battle.log.pray",
            "{actor} prays and the party recovers {amount} HP.",
        ),
    ]
}
