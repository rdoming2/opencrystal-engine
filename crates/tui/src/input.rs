use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Confirm,
    Cancel,
    Menu,
    Pause,
    Quit,
    Learn,
}

#[derive(Clone, Debug)]
pub struct InputBindings {
    key_map: HashMap<KeyCode, Action>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InputFile {
    pub version: u32,
    pub bindings: HashMap<String, Vec<String>>,
}

impl InputFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|err| format!("{}: {}", path.display(), err))?;
        serde_json::from_reader(file).map_err(|err| format!("{}: {}", path.display(), err))
    }
}

impl InputBindings {
    pub fn from_file(file: InputFile) -> Result<Self, String> {
        let mut key_map = HashMap::new();
        for (action_id, keys) in file.bindings {
            let action = action_from_id(&action_id)
                .ok_or_else(|| format!("unknown action '{}'", action_id))?;
            for key in keys {
                let code = parse_key(&key)?;
                key_map.insert(code, action.clone());
            }
        }
        Ok(Self { key_map })
    }

    pub fn action_for(&self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char(c) => {
                let lower = c.to_ascii_lowercase();
                self.key_map.get(&KeyCode::Char(lower)).cloned()
            }
            _ => self.key_map.get(&code).cloned(),
        }
    }

    pub fn default_bindings() -> Self {
        let mut key_map = HashMap::new();
        key_map.insert(KeyCode::Up, Action::MoveUp);
        key_map.insert(KeyCode::Down, Action::MoveDown);
        key_map.insert(KeyCode::Left, Action::MoveLeft);
        key_map.insert(KeyCode::Right, Action::MoveRight);
        key_map.insert(KeyCode::Char('w'), Action::MoveUp);
        key_map.insert(KeyCode::Char('a'), Action::MoveLeft);
        key_map.insert(KeyCode::Char('s'), Action::MoveDown);
        key_map.insert(KeyCode::Char('d'), Action::MoveRight);
        key_map.insert(KeyCode::Char('k'), Action::MoveUp);
        key_map.insert(KeyCode::Char('h'), Action::MoveLeft);
        key_map.insert(KeyCode::Char('j'), Action::MoveDown);
        key_map.insert(KeyCode::Char('l'), Action::MoveRight);
        key_map.insert(KeyCode::Enter, Action::Confirm);
        key_map.insert(KeyCode::Char('c'), Action::Confirm);
        key_map.insert(KeyCode::Char('x'), Action::Cancel);
        key_map.insert(KeyCode::Char('i'), Action::Menu);
        key_map.insert(KeyCode::Esc, Action::Menu);
        key_map.insert(KeyCode::Char(' '), Action::Pause);
        key_map.insert(KeyCode::Char('q'), Action::Quit);
        key_map.insert(KeyCode::Char('l'), Action::Learn);
        Self { key_map }
    }
}

fn action_from_id(id: &str) -> Option<Action> {
    match id {
        "move_up" => Some(Action::MoveUp),
        "move_down" => Some(Action::MoveDown),
        "move_left" => Some(Action::MoveLeft),
        "move_right" => Some(Action::MoveRight),
        "confirm" => Some(Action::Confirm),
        "cancel" => Some(Action::Cancel),
        "menu" => Some(Action::Menu),
        "pause" => Some(Action::Pause),
        "quit" => Some(Action::Quit),
        "learn" => Some(Action::Learn),
        _ => None,
    }
}

fn parse_key(key: &str) -> Result<KeyCode, String> {
    match key {
        "Up" => Ok(KeyCode::Up),
        "Down" => Ok(KeyCode::Down),
        "Left" => Ok(KeyCode::Left),
        "Right" => Ok(KeyCode::Right),
        "Enter" | "Return" => Ok(KeyCode::Enter),
        "Escape" | "Esc" => Ok(KeyCode::Esc),
        "Space" => Ok(KeyCode::Char(' ')),
        "Backspace" => Ok(KeyCode::Backspace),
        _ => {
            if key.chars().count() == 1 {
                let ch = key.chars().next().unwrap().to_ascii_lowercase();
                Ok(KeyCode::Char(ch))
            } else {
                Err(format!("unknown key '{}'", key))
            }
        }
    }
}
