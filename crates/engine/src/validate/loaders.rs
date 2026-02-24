use std::fs;
use std::path::{Path, PathBuf};

use crate::dialog::DialogFile;
use crate::events::EventFile;
use crate::maps::MapFile;
use crate::quests::QuestsFile;

pub(crate) fn load_single<T, F>(path: &PathBuf, loader: F, errors: &mut Vec<String>) -> Option<T>
where
    F: FnOnce(&Path) -> Result<T, String>,
{
    if !path.exists() {
        errors.push(format!("{}: file not found", path.display()));
        return None;
    }
    match loader(path) {
        Ok(data) => Some(data),
        Err(err) => {
            errors.push(err);
            None
        }
    }
}

pub(crate) fn load_optional<T, F>(path: &PathBuf, loader: F) -> Option<T>
where
    F: FnOnce(&Path) -> Result<T, String>,
{
    if !path.exists() {
        return None;
    }
    loader(path).ok()
}

pub(crate) fn load_map_files(dir: PathBuf, errors: &mut Vec<String>) -> Vec<MapFile> {
    let mut maps = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(format!("{}: {}", dir.display(), err));
            return maps;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match MapFile::load(&path) {
            Ok(map) => {
                if let Err(err) = validate_map(&map) {
                    errors.push(format!("{}: {}", path.display(), err));
                }
                maps.push(map);
            }
            Err(err) => errors.push(err),
        }
    }

    if maps.is_empty() {
        errors.push(format!("{}: no map files found", dir.display()));
    }

    maps
}

pub(crate) fn load_event_files(dir: PathBuf, errors: &mut Vec<String>) -> Vec<EventFile> {
    let mut events = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(format!("{}: {}", dir.display(), err));
            return events;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match EventFile::load(&path) {
            Ok(event) => events.push(event),
            Err(err) => errors.push(err),
        }
    }

    events
}

pub(crate) fn load_dialog_files(dir: PathBuf, errors: &mut Vec<String>) -> Vec<DialogFile> {
    let mut dialogs = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("{}: {}", dir.display(), err));
            }
            return dialogs;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match DialogFile::load(&path) {
            Ok(dialog) => dialogs.push(dialog),
            Err(err) => errors.push(err),
        }
    }

    dialogs
}

pub(crate) fn load_quest_files(path: PathBuf, errors: &mut Vec<String>) -> Vec<QuestsFile> {
    load_single(&path, |path| QuestsFile::load(path), errors)
        .map(|file| vec![file])
        .unwrap_or_default()
}

fn validate_map(map: &MapFile) -> Result<(), String> {
    if map.tiles.len() != map.height as usize {
        return Err(format!(
            "map '{}' tiles height {} does not match height {}",
            map.id,
            map.tiles.len(),
            map.height
        ));
    }
    for (row_index, row) in map.tiles.iter().enumerate() {
        if row.chars().count() != map.width as usize {
            return Err(format!(
                "map '{}' row {} length {} does not match width {}",
                map.id,
                row_index,
                row.chars().count(),
                map.width
            ));
        }
    }
    Ok(())
}
