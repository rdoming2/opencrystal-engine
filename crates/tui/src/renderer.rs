#[derive(Clone, Debug)]
pub enum RenderMode {
    Auto,
    Wide,
    Modern,
}

impl RenderMode {
    pub fn from_arg(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "wide" => Some(Self::Wide),
            "modern" => Some(Self::Modern),
            _ => None,
        }
    }
}
