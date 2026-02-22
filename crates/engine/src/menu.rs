#[derive(Clone, Debug, PartialEq)]
pub enum MenuFocus {
    List,
    Detail,
}

#[derive(Clone, Debug)]
pub struct MenuState {
    pub focus: MenuFocus,
    pub selected: usize,
    pub active_submenu: Option<String>,
    pub detail_page: usize,
    pub detail_scroll: usize,
    pub detail_selection: usize,
    pub detail_filter: usize,
    pub detail_sort: usize,
    pub detail_actor: usize,
    pub detail_slot: usize,
    pub detail_target: usize,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            focus: MenuFocus::List,
            selected: 0,
            active_submenu: None,
            detail_page: 0,
            detail_scroll: 0,
            detail_selection: 0,
            detail_filter: 0,
            detail_sort: 0,
            detail_actor: 0,
            detail_slot: 0,
            detail_target: 0,
        }
    }
}
