use std::io;

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::InputBindings;
use crate::session::TuiSession;

pub(super) fn prompt_glyph(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<char>> {
    let value = prompt_text(session, title, prompt, default, 2)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let mut chars = value.trim().chars();
    let glyph = chars.next().unwrap_or('.');
    Ok(Some(glyph))
}

pub(super) fn prompt_optional_text(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
    max_len: usize,
) -> io::Result<Option<String>> {
    let value = prompt_text(session, title, prompt, default, max_len)?;
    Ok(value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }))
}

pub(super) fn prompt_optional_glyph_string(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<String>> {
    let value = prompt_text(session, title, prompt, default, 2)?;
    Ok(value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.chars().next().map(|ch| ch.to_string())
        }
    }))
}

pub(super) fn prompt_yes_no(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: bool,
) -> io::Result<bool> {
    let options = vec!["No".to_string(), "Yes".to_string()];
    let default_index = if default { 1 } else { 0 };
    let selection = prompt_choice(
        session,
        &InputBindings::default_bindings(),
        title,
        prompt,
        &options,
        default_index,
    )?;
    Ok(matches!(selection, Some(1)))
}

pub(super) fn prompt_pos(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<[i32; 2]>> {
    let value = prompt_text(session, title, prompt, default, 16)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Ok(None);
    }
    let x: i32 = parts[0].trim().parse().unwrap_or(0);
    let y: i32 = parts[1].trim().parse().unwrap_or(0);
    Ok(Some([x, y]))
}

pub(super) fn prompt_flags(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<Vec<String>>> {
    let value = prompt_text(session, title, prompt, default, 128)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let flags = value
        .split(',')
        .map(|flag| flag.trim())
        .filter(|flag| !flag.is_empty())
        .map(|flag| flag.to_string())
        .collect::<Vec<_>>();
    if flags.is_empty() {
        Ok(None)
    } else {
        Ok(Some(flags))
    }
}

pub(super) fn flags_to_string(flags: Option<&Vec<String>>) -> String {
    flags.map(|items| items.join(", ")).unwrap_or_default()
}

pub(super) fn choose_from_list_or_custom(
    session: &mut TuiSession,
    bindings: &InputBindings,
    title: &str,
    prompt: &str,
    options: &[String],
    default: &str,
) -> io::Result<Option<String>> {
    if options.is_empty() {
        return prompt_text(session, title, prompt, default, 48);
    }
    let mut choices = vec!["<custom>".to_string()];
    choices.extend(options.iter().cloned());
    let selected = prompt_choice(session, bindings, title, prompt, &choices, 1)?;
    match selected {
        Some(0) => prompt_text(session, title, prompt, default, 48),
        Some(index) => Ok(choices.get(index).cloned()),
        None => Ok(None),
    }
}

pub(super) fn choose_optional_from_list_or_custom(
    session: &mut TuiSession,
    bindings: &InputBindings,
    title: &str,
    prompt: &str,
    options: &[String],
    default: &str,
) -> io::Result<Option<String>> {
    if options.is_empty() {
        let value = prompt_text(session, title, prompt, default, 48)?;
        return Ok(value.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }));
    }
    let mut choices = vec!["<none>".to_string(), "<custom>".to_string()];
    choices.extend(options.iter().cloned());
    let selected = prompt_choice(session, bindings, title, prompt, &choices, 0)?;
    match selected {
        Some(0) | None => Ok(None),
        Some(1) => {
            let value = prompt_text(session, title, prompt, default, 48)?;
            Ok(value.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }))
        }
        Some(index) => Ok(choices.get(index).cloned()),
    }
}
