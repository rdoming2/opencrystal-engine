use std::collections::{HashMap, HashSet};

use super::ValidationContext;

const BATTLE_POS_MAX_X: i32 = 9;
const BATTLE_POS_MAX_Y: i32 = 5;

pub(crate) fn validate_strings(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(strings) = context.strings else {
        return;
    };
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

pub(crate) fn validate_stats(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(stats) = context.stats else {
        return;
    };
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

pub(crate) fn validate_jobs_spells_abilities(
    context: &ValidationContext,
    errors: &mut Vec<String>,
) {
    let (Some(spells), Some(abilities), Some(jobs)) =
        (context.spells, context.abilities, context.jobs)
    else {
        return;
    };

    let default_jobs = jobs.jobs.iter().filter(|job| job.is_default).count();
    if default_jobs == 0 {
        errors.push("jobs.json: at least one job must be marked is_default".to_string());
    } else if default_jobs > 1 {
        errors.push("jobs.json: only one job can be marked is_default".to_string());
    }

    let command_ids: HashSet<&str> = context
        .rules
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
    if let Some(rules) = context.rules {
        for command in &rules.battle.commands {
            if let Some(ability_id) = command.ability_id.as_deref() {
                if !context.ids.ability_ids.contains(ability_id) {
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
                for stat in &context.ids.base_stat_ids {
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
                for stat in &context.ids.base_stat_ids {
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
            if !context.ids.spell_ids.contains(spell.id.as_str()) {
                errors.push(format!(
                    "jobs.json: job '{}' references unknown spell '{}'",
                    job.id, spell.id
                ));
            }
        }
        for ability in &job.abilities {
            if !context.ids.ability_ids.contains(ability.id.as_str()) {
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
            if !context.ids.effect_ids.contains(effect_id.as_str()) {
                errors.push(format!(
                    "spells.json: spell '{}' references unknown effect '{}'",
                    spell.id, effect_id
                ));
            }
        }
        if let Some(element) = spell.effect.element.as_deref() {
            if !context.ids.element_ids.is_empty() && !context.ids.element_ids.contains(element) {
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
            if !context.ids.effect_ids.contains(effect_id.as_str()) {
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
        if ability.effect.r#type == "charge" {
            if ability.effect.windup_turns == 0 {
                errors.push(format!(
                    "abilities.json: ability '{}' charge windup_turns must be >= 1",
                    ability.id
                ));
            }
            if ability.target_mode != "single" {
                errors.push(format!(
                    "abilities.json: ability '{}' charge target_mode must be 'single'",
                    ability.id
                ));
            }
            let enemy_only = !ability.allowed_targets.is_empty()
                && ability
                    .allowed_targets
                    .iter()
                    .all(|target| target.as_str() == "enemy");
            if ability.default_target != "enemy" || !enemy_only {
                errors.push(format!(
                    "abilities.json: ability '{}' charge must target enemies only",
                    ability.id
                ));
            }
        }
        if let Some(cost) = &ability.cost {
            if cost.r#type == "currency" {
                match cost.currency_id.as_deref() {
                    Some(id) if context.ids.currency_ids.contains(id) => {}
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

pub(crate) fn validate_party(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(party) = context.party else {
        return;
    };

    let roster_ids: HashSet<&str> = party.roster.iter().map(|actor| actor.id.as_str()).collect();

    for actor in &party.roster {
        if !context.ids.job_ids.is_empty() && !context.ids.job_ids.contains(actor.job_id.as_str()) {
            errors.push(format!(
                "party.json: actor '{}' references unknown job '{}'",
                actor.id, actor.job_id
            ));
        }
        for stat in actor.base_stats.keys() {
            if !context.ids.base_stat_ids.is_empty()
                && !context.ids.base_stat_ids.contains(stat.as_str())
            {
                errors.push(format!(
                    "party.json: actor '{}' references unknown stat '{}'",
                    actor.id, stat
                ));
            }
        }
        for spell in &actor.spells {
            if !context.ids.spell_ids.is_empty() && !context.ids.spell_ids.contains(spell.as_str())
            {
                errors.push(format!(
                    "party.json: actor '{}' references unknown spell '{}'",
                    actor.id, spell
                ));
            }
        }
        for (slot, item_id) in &actor.starting_equipment {
            if !context.ids.equipment_ids.is_empty()
                && !context.ids.equipment_ids.contains(item_id.as_str())
            {
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

pub(crate) fn validate_jobs_exist(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(jobs) = context.jobs else {
        return;
    };
    if jobs.jobs.is_empty() {
        errors.push("jobs.json: must define at least one job".to_string());
    }
}

pub(crate) fn validate_create_mode_default_job(
    context: &ValidationContext,
    errors: &mut Vec<String>,
) {
    let (Some(rules), Some(jobs)) = (context.rules, context.jobs) else {
        return;
    };
    if rules.party_mode == crate::rules::PartyMode::Create {
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

pub(crate) fn validate_items_equipment_shops(
    context: &ValidationContext,
    errors: &mut Vec<String>,
) {
    let (Some(items), Some(equipment), Some(shops)) =
        (context.items, context.equipment, context.shops)
    else {
        return;
    };
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
            validate_price_map(
                errors,
                "items.json: item",
                &item.id,
                prices,
                &context.ids.currency_ids,
            );
        }
    }
    for shop in &shops.shops {
        if shop.currency.trim().is_empty() {
            errors.push(format!("shops.json: shop '{}' missing currency", shop.id));
        } else if !context.ids.currency_ids.contains(shop.currency.as_str()) {
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

pub(crate) fn validate_items_effects(context: &ValidationContext, errors: &mut Vec<String>) {
    let (Some(items), Some(_effects)) = (context.items, context.effects) else {
        return;
    };
    for item in &items.items {
        for effect_id in &item.effect.effects {
            if !context.ids.effect_ids.contains(effect_id.as_str()) {
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
                if !context.ids.status_ids.contains(status_id.as_str()) {
                    errors.push(format!(
                        "items.json: item '{}' references unknown status '{}'",
                        item.id, status_id
                    ));
                }
            }
        }
    }
}

pub(crate) fn validate_items_warp(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(items) = context.items else {
        return;
    };
    for item in &items.items {
        if item.effect.r#type != "warp" {
            continue;
        }
        if let Some(destination) = &item.effect.destination {
            if !context.ids.map_ids.contains(destination.map.as_str()) {
                errors.push(format!(
                    "items.json: item '{}' warp destination '{}' not found",
                    item.id, destination.map
                ));
                continue;
            }
            if let Some((width, height)) = context.ids.map_dims.get(destination.map.as_str()) {
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

pub(crate) fn validate_encounters_enemies(context: &ValidationContext, errors: &mut Vec<String>) {
    let (Some(enemies), Some(encounters)) = (context.enemies, context.encounters) else {
        return;
    };
    let enemy_ids: HashSet<&str> = enemies
        .enemies
        .iter()
        .map(|enemy| enemy.id.as_str())
        .collect();
    let trait_ids = &context.ids.trait_ids;
    for table in &encounters.tables {
        for entry in &table.entries {
            if let Some(tile) = entry.tile.as_ref() {
                if tile.trim().is_empty() {
                    errors.push(format!(
                        "encounters.json: table '{}' entry has empty tile filter",
                        table.id
                    ));
                }
            }
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
            if !context.ids.currency_ids.contains(currency.id.as_str()) {
                errors.push(format!(
                    "enemies.json: enemy '{}' references unknown currency '{}'",
                    enemy.id, currency.id
                ));
            }
        }
    }
}

pub(crate) fn validate_world_vehicles(context: &ValidationContext, errors: &mut Vec<String>) {
    let (Some(_vehicles), Some(worlds)) = (context.vehicles, context.worlds) else {
        return;
    };
    for world in &worlds.worlds {
        for vehicle in &world.vehicles {
            if !context.ids.vehicle_ids.contains(vehicle.as_str()) {
                errors.push(format!(
                    "worlds.json: world '{}' references unknown vehicle '{}'",
                    world.id, vehicle
                ));
            }
        }
    }
}

pub(crate) fn validate_npcs(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(npcs) = context.npcs else {
        return;
    };
    let npc_ids: HashSet<&str> = npcs.npcs.iter().map(|npc| npc.id.as_str()).collect();
    for map in context.maps {
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

pub(crate) fn validate_equipment_traits_prices(
    context: &ValidationContext,
    errors: &mut Vec<String>,
) {
    let (Some(equipment), Some(effects)) = (context.equipment, context.effects) else {
        return;
    };
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
            validate_price_map(
                errors,
                "equipment.json: equipment",
                &item.id,
                prices,
                &context.ids.currency_ids,
            );
        }
    }
}

fn validate_price_map(
    errors: &mut Vec<String>,
    label: &str,
    entry_id: &str,
    prices: &HashMap<String, i32>,
    currency_ids: &HashSet<&str>,
) {
    for (currency, amount) in prices {
        if !currency_ids.contains(currency.as_str()) {
            errors.push(format!(
                "{} '{}' has unknown currency '{}'",
                label, entry_id, currency
            ));
        }
        if *amount < 0 {
            errors.push(format!(
                "{} '{}' has negative price {} for '{}'",
                label, entry_id, amount, currency
            ));
        }
    }
}
