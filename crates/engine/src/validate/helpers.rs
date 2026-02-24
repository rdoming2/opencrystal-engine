use crate::content::{CookingFile, CookingRecipe};

pub(crate) fn check_non_empty(errors: &mut Vec<String>, value: &str, err: impl FnOnce() -> String) {
    if value.trim().is_empty() {
        errors.push(err());
    }
}

pub(crate) fn find_recipe<'a>(
    errors: &mut Vec<String>,
    cooking: Option<&'a CookingFile>,
    recipe_id: &str,
    missing_cooking: impl FnOnce() -> String,
    missing_recipe: impl FnOnce() -> String,
) -> Option<&'a CookingRecipe> {
    let Some(cooking) = cooking else {
        errors.push(missing_cooking());
        return None;
    };
    let Some(recipe) = cooking.recipes.iter().find(|recipe| recipe.id == recipe_id) else {
        errors.push(missing_recipe());
        return None;
    };
    Some(recipe)
}
