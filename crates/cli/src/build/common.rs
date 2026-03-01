use std::fs;
use std::path::{Path, PathBuf};

use engine::io::{load_json, write_json_pretty};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub(crate) fn load_or_default<T>(path: &Path, default_value: T) -> Result<T, String>
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

pub(crate) fn resolve_content_dir(explicit: Option<&str>) -> Result<PathBuf, String> {
    if let Some(value) = explicit {
        return Ok(PathBuf::from(value));
    }
    let current = std::env::current_dir().map_err(|err| err.to_string())?;
    if current.join("rules.json").exists() {
        return Ok(current);
    }
    Err("No --content provided and current directory is not a content pack".to_string())
}

pub(crate) fn title_case_id(id: &str) -> String {
    id.split(['_', '-'])
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

pub(crate) fn slugify(value: &str) -> String {
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
