use std::collections::HashSet;

use super::ValidationContext;

pub(crate) fn validate_effects(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(effects) = context.effects else {
        return;
    };

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
