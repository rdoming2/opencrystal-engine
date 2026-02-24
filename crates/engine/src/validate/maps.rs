use std::collections::HashSet;

use super::helpers::check_non_empty;
use super::ValidationContext;

pub(crate) fn validate_start_locations(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(rules) = context.rules else {
        return;
    };
    if !context
        .ids
        .map_ids
        .contains(rules.game.start_location.map.as_str())
    {
        errors.push(format!(
            "rules.json: start_location.map '{}' not found in maps",
            rules.game.start_location.map
        ));
    }
    if let Some(worlds) = context.worlds {
        let world_ids: HashSet<&str> = worlds.worlds.iter().map(|w| w.id.as_str()).collect();
        if !world_ids.contains(rules.game.start_location.world.as_str()) {
            errors.push(format!(
                "rules.json: start_location.world '{}' not found in worlds.json",
                rules.game.start_location.world
            ));
        }
    }
}

pub(crate) fn validate_world_maps(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(worlds) = context.worlds else {
        return;
    };
    for world in &worlds.worlds {
        if !context.ids.map_ids.contains(world.starting_map.as_str()) {
            errors.push(format!(
                "worlds.json: world '{}' starting_map '{}' not found",
                world.id, world.starting_map
            ));
        }
        if !context
            .ids
            .map_ids
            .contains(world.overworld_map_id.as_str())
        {
            errors.push(format!(
                "worlds.json: world '{}' overworld_map_id '{}' not found",
                world.id, world.overworld_map_id
            ));
        }
    }
}

pub(crate) fn validate_encounter_tables(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(encounters) = context.encounters else {
        return;
    };
    let tables: HashSet<&str> = encounters
        .tables
        .iter()
        .map(|table| table.id.as_str())
        .collect();
    for map in context.maps {
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

pub(crate) fn validate_maps(context: &ValidationContext, errors: &mut Vec<String>) {
    for map in context.maps {
        if !(0.0..=1.0).contains(&map.encounter_rate) {
            errors.push(format!(
                "maps/{}: encounter_rate {} must be between 0.0 and 1.0",
                map.id, map.encounter_rate
            ));
        }
        for event in &map.events {
            if !context.ids.event_ids.contains(event.script.as_str()) {
                errors.push(format!(
                    "maps/{}: event script '{}' not found",
                    map.id, event.script
                ));
            }
        }
        for npc in &map.npcs {
            if let Some(script) = &npc.script {
                if !context.ids.event_ids.contains(script.as_str()) {
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
                check_non_empty(errors, flag, || {
                    format!(
                        "maps/{}: door '{}' has empty requires_flag",
                        map.id, door.id
                    )
                });
            }
            if let Some(event) = door.locked_event.as_ref() {
                if !context.ids.event_ids.contains(event.as_str()) {
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
                if !context.ids.map_ids.contains(target_map.as_str()) {
                    errors.push(format!(
                        "maps/{}: door '{}' target '{}' not found",
                        map.id, door.id, target_map
                    ));
                }
                if let Some(target_pos) = door.target_pos.as_ref() {
                    if let Some((width, height)) = context.ids.map_dims.get(target_map.as_str()) {
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
                if !context.ids.event_ids.contains(event.as_str()) {
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
                check_non_empty(errors, flag, || {
                    format!("maps/{}: puzzle '{}' has empty set_flag", map.id, puzzle.id)
                });
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
            check_non_empty(errors, campfire.campfire_id.as_str(), || {
                format!(
                    "maps/{}: campfire '{}' has empty campfire_id",
                    map.id, campfire.id
                )
            });
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
            check_non_empty(errors, chest.opened_flag.as_str(), || {
                format!("maps/{}: chest '{}' missing opened_flag", map.id, chest.id)
            });
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
            if !context.ids.map_ids.contains(transition.target_map.as_str()) {
                errors.push(format!(
                    "maps/{}: transition '{}' target '{}' not found",
                    map.id, transition.id, transition.target_map
                ));
            }
            if let Some(label) = transition.label.as_ref() {
                check_non_empty(errors, label, || {
                    format!(
                        "maps/{}: transition '{}' has empty label",
                        map.id, transition.id
                    )
                });
            }
            if let Some(flag) = transition.requires_flag.as_ref() {
                check_non_empty(errors, flag, || {
                    format!(
                        "maps/{}: transition '{}' has empty requires_flag",
                        map.id, transition.id
                    )
                });
            }
            if let Some(cost) = transition.cost.as_ref() {
                check_non_empty(errors, cost.id.as_str(), || {
                    format!(
                        "maps/{}: transition '{}' has cost with empty id",
                        map.id, transition.id
                    )
                });
                if cost.amount <= 0 {
                    errors.push(format!(
                        "maps/{}: transition '{}' has non-positive cost",
                        map.id, transition.id
                    ));
                }
                if !cost.id.trim().is_empty()
                    && !context.ids.currency_ids.contains(cost.id.as_str())
                {
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
            if let Some((width, height)) = context.ids.map_dims.get(transition.target_map.as_str())
            {
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
}

pub(crate) fn validate_map_vehicles(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(_vehicles) = context.vehicles else {
        return;
    };
    for map in context.maps {
        for vehicle in &map.vehicles {
            if !context
                .ids
                .vehicle_ids
                .contains(vehicle.vehicle_id.as_str())
            {
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

pub(crate) fn validate_chests(context: &ValidationContext, errors: &mut Vec<String>) {
    let (Some(items), Some(equipment)) = (context.items, context.equipment) else {
        return;
    };
    let item_ids: HashSet<&str> = items.items.iter().map(|item| item.id.as_str()).collect();
    let equipment_ids: HashSet<&str> = equipment
        .equipment
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    for map in context.maps {
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
                if !stack.id.trim().is_empty()
                    && !context.ids.currency_ids.contains(stack.id.as_str())
                {
                    errors.push(format!(
                        "maps/{}: chest '{}' has unknown currency '{}'",
                        map.id, chest.id, stack.id
                    ));
                }
            }
        }
    }
}
