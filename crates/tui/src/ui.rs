use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TitleUiFile {
    pub version: u32,
    pub title: String,
    pub logo: TitleLogo,
    pub menu: Vec<MenuItem>,
    pub footer: FooterConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProgressUiFile {
    pub version: u32,
    pub panels: Vec<ProgressPanel>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MenuUiFile {
    pub version: u32,
    pub layout: MenuLayout,
    pub default_panel: String,
    pub menu: Vec<MenuEntry>,
    pub panels: Vec<MenuPanel>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MenuLayout {
    #[serde(default = "default_menu_left_ratio")]
    pub left_width_ratio: f32,
    #[serde(default = "default_menu_right_ratio")]
    pub right_width_ratio: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MenuEntry {
    pub id: String,
    pub label: String,
    pub action: String,
    #[serde(default = "default_menu_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub unlock_flag: Option<String>,
    #[serde(default)]
    pub locked_behavior: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MenuPanel {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub panel_type: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProgressPanel {
    pub id: String,
    pub title: String,
    pub items: Vec<ProgressItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProgressItem {
    pub label: String,
    pub value: String,
    pub max: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TitleLogo {
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FooterConfig {
    pub left: String,
    pub right: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleUiFile {
    pub version: u32,
    pub breakpoints: Vec<Breakpoint>,
    pub layout: BattleLayout,
    pub log: Option<BattleLog>,
    pub dialog: Option<BattleDialog>,
    pub animation: Option<BattleAnimation>,
    pub panels: BattlePanels,
    pub menus: BattleMenus,
    pub selection: SelectionRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleLog {
    pub position: String,
    pub height: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleDialog {
    pub position: String,
    pub height: u16,
    #[serde(default = "default_battle_dialog_auto_advance_ms")]
    pub auto_advance_ms: u64,
    #[serde(default = "default_battle_dialog_allow_skip")]
    pub allow_skip: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleAnimation {
    #[serde(default = "default_battle_flash_ms")]
    pub flash_ms: u64,
    #[serde(default = "default_battle_flash_cycles")]
    pub flash_cycles: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Breakpoint {
    pub id: String,
    pub min_width: u16,
    pub min_height: u16,
    pub behavior: BreakpointBehavior,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BreakpointBehavior {
    pub enemy_art: String,
    pub hide_panel_titles: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleLayout {
    pub battlefield: PanelAnchor,
    pub command_row: CommandRow,
    #[serde(default = "default_party_grid")]
    pub party_grid: PartyGrid,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanelAnchor {
    pub anchor: String,
    pub height_ratio: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandRow {
    pub anchor: String,
    pub height_ratio: f32,
    pub columns: Vec<ColumnSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyGrid {
    #[serde(default = "default_party_grid_rows")]
    pub rows: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ColumnSpec {
    pub id: String,
    pub width_ratio: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattlePanels {
    pub enemies: EnemyPanel,
    pub commands: CommandPanel,
    pub party: PartyPanel,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyPanel {
    pub title: String,
    pub highlight: HighlightRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandPanel {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyPanel {
    pub title: String,
    pub show: Vec<String>,
    pub highlight: HighlightRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HighlightRules {
    pub style: String,
    pub link_to_battlefield: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleMenus {
    pub attack: AttackMenu,
    pub magic: MagicMenu,
    pub abilities: AbilitiesMenu,
    pub items: ItemsMenu,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AttackMenu {
    pub target: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MagicMenu {
    pub list: String,
    pub group_by: String,
    pub columns: Vec<MenuColumn>,
    pub target_from_spell: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbilitiesMenu {
    pub list: String,
    pub columns: Vec<MenuColumn>,
    pub target_from_ability: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemsMenu {
    pub list: String,
    pub columns: Vec<MenuColumn>,
    pub target_from_item: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MenuColumn {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelectionRules {
    pub target_cursor: String,
    pub battlefield_highlight: String,
    pub list_highlight: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogUiFile {
    pub version: u32,
    pub position: String,
    pub height: u16,
    pub show_speaker: bool,
    pub continue_marker: String,
}

impl TitleUiFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        load_json(path)
    }
}

impl BattleUiFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        load_json(path)
    }
}

impl DialogUiFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        load_json(path)
    }
}

impl ProgressUiFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        load_json(path)
    }
}

impl MenuUiFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        load_json(path)
    }
}

fn default_menu_left_ratio() -> f32 {
    0.4
}

fn default_menu_right_ratio() -> f32 {
    0.6
}

fn default_battle_dialog_auto_advance_ms() -> u64 {
    700
}

fn default_battle_dialog_allow_skip() -> bool {
    true
}

fn default_battle_flash_ms() -> u64 {
    150
}

fn default_battle_flash_cycles() -> u16 {
    2
}

fn default_party_grid() -> PartyGrid {
    PartyGrid { rows: 2 }
}

fn default_party_grid_rows() -> u16 {
    2
}

fn default_menu_enabled() -> bool {
    true
}

fn load_json<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(|err| format!("{}: {}", path.display(), err))?;
    serde_json::from_reader(file).map_err(|err| format!("{}: {}", path.display(), err))
}
