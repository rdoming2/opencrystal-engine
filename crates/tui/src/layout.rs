#[derive(Clone, Debug)]
pub struct LayoutConfig {
    pub min_art_width: u16,
    pub min_art_height: u16,
}

impl LayoutConfig {
    pub fn default_wide() -> Self {
        Self {
            min_art_width: 110,
            min_art_height: 32,
        }
    }
}
