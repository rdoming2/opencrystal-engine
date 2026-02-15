use tui::input::{is_actionable_key, Action, InputBindings};

pub fn read_action(bindings: &InputBindings) -> Option<Action> {
    if let crossterm::event::Event::Key(key) = crossterm::event::read().ok()? {
        if !is_actionable_key(&key) {
            return None;
        }
        return bindings.action_for(key.code);
    }
    None
}
