use tui::input::{Action, InputBindings};

pub fn read_action(bindings: &InputBindings) -> Option<Action> {
    if let crossterm::event::Event::Key(key) = crossterm::event::read().ok()? {
        return bindings.action_for(key.code);
    }
    None
}
