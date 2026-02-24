use std::collections::HashSet;

use crate::rules::{AbilityAcquisition, MagicAcquisition, ProgressionMode};

use super::ValidationContext;

pub(crate) fn validate_rules(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(rules) = context.rules else {
        return;
    };

    if rules.game.party_size > 4 {
        errors.push("rules.json: party_size must be <= 4".to_string());
    }
    if rules.game.currencies.is_empty() {
        errors.push("rules.json: game.currencies must define at least one currency".to_string());
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
                "rules.json: settings.difficulty_scale.value must be within min/max".to_string(),
            );
        }
    }
    if let Some(battle_mode) = rules.settings.battle_mode.as_ref() {
        if !battle_mode.options.is_empty() && !battle_mode.options.contains(&battle_mode.value) {
            errors.push(
                "rules.json: settings.battle_mode.value must be listed in options".to_string(),
            );
        }
    }
    if rules.game.magic_acquisition == MagicAcquisition::Jp
        && rules.progression_mode != ProgressionMode::JobPoints
    {
        errors.push(
            "rules.json: magic_acquisition 'jp' requires progression_mode 'job_points'".to_string(),
        );
    }
    if rules.game.ability_acquisition == AbilityAcquisition::Jp
        && rules.progression_mode != ProgressionMode::JobPoints
    {
        errors.push(
            "rules.json: ability_acquisition 'jp' requires progression_mode 'job_points'"
                .to_string(),
        );
    }
    if rules.progression_mode == ProgressionMode::Activity {
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
                "rules.json: activity_progression.effects.damage_scale must be >= 0".to_string(),
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
                    "rules.json: activity_progression.ranks label must not be empty".to_string(),
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
            errors.push("rules.json: activity_growth.danger_factor_min must be > 0".to_string());
        }
        if rules.activity_growth.danger_factor_max < rules.activity_growth.danger_factor_min {
            errors.push(
                "rules.json: activity_growth.danger_factor_max must be >= danger_factor_min"
                    .to_string(),
            );
        }
        if rules.activity_growth.floor_depth_exponent < 0.0 {
            errors
                .push("rules.json: activity_growth.floor_depth_exponent must be >= 0".to_string());
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
                errors.push("rules.json: exp_curve.formula required for formula mode".to_string());
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
    if rules.systems.get("cooking").copied().unwrap_or(false) && context.cooking.is_none() {
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
            errors.push("rules.json: battle.boss_scaling.hp_multiplier must be > 0".to_string());
        }
        if rules.battle.boss_scaling.stat_multiplier <= 0.0 {
            errors.push("rules.json: battle.boss_scaling.stat_multiplier must be > 0".to_string());
        }
    }
}

pub(crate) fn validate_inventory(context: &ValidationContext, errors: &mut Vec<String>) {
    let (Some(rules), Some(items), Some(equipment)) =
        (context.rules, context.items, context.equipment)
    else {
        return;
    };
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
