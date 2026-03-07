use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::maps::MapState;
use crate::party::{Actor, JobProgress, PartyState};
use crate::runtime::GameRuntime;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveFile {
    pub version: u32,
    pub encoding: String,
    pub metadata: SaveMetadata,
    pub world: SaveWorld,
    pub party: SaveParty,
    pub inventory: SaveInventory,
    #[serde(default)]
    pub shops: HashMap<String, crate::runtime::ShopState>,
    pub flags: HashSet<String>,
    pub map_states: HashMap<String, MapState>,
    #[serde(default)]
    pub stats: HashMap<String, i32>,
    #[serde(default)]
    pub vehicles: HashMap<String, SaveVehicleState>,
    #[serde(default)]
    pub settings: Option<crate::runtime::SettingsState>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveMetadata {
    pub slot: u8,
    pub title: String,
    pub play_time_seconds: u64,
    pub timestamp_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveWorld {
    pub world_id: String,
    pub map_id: String,
    pub pos: [i32; 2],
    #[serde(default)]
    pub vehicle: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveParty {
    pub active: Vec<Option<String>>,
    pub reserve: Vec<String>,
    pub roster: HashMap<String, SaveActor>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveActor {
    pub id: String,
    pub name: String,
    pub job_id: String,
    pub level: u32,
    pub exp: i32,
    #[serde(default = "default_battle_row")]
    pub row: crate::party::BattleRow,
    pub current_hp: i32,
    pub current_mp: i32,
    pub base_stats: HashMap<String, i32>,
    pub derived_stats: HashMap<String, i32>,
    pub equipment: HashMap<String, String>,
    pub spells: Vec<String>,
    pub equipped_spells: Vec<String>,
    pub magic_tier_charges: HashMap<u32, i32>,
    pub secondary_job_id: Option<String>,
    pub job_progress: HashMap<String, SaveJobProgress>,
    #[serde(default)]
    pub weapon_proficiencies: HashMap<String, f32>,
    #[serde(default)]
    pub magic_proficiencies: HashMap<String, f32>,
    pub unlocked_abilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveJobProgress {
    pub level: u32,
    pub exp: i32,
    pub jp_earned: i32,
    pub jp_spent: i32,
    #[serde(default)]
    pub learned_spells: Vec<String>,
    #[serde(default)]
    pub learned_abilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveInventory {
    pub items: HashMap<String, i32>,
    pub equipment: HashMap<String, i32>,
    pub currency: HashMap<String, i32>,
    #[serde(default)]
    pub items_order: Vec<String>,
    #[serde(default)]
    pub equipment_order: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveVehicleState {
    pub map_id: String,
    pub pos: [i32; 2],
}

impl SaveFile {
    pub fn from_runtime(runtime: &GameRuntime, slot: u8) -> Self {
        let play_time_seconds = runtime.playtime + runtime.start_time.elapsed().as_secs();
        let timestamp_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            version: 1,
            encoding: "plain".to_string(),
            metadata: SaveMetadata {
                slot,
                title: runtime.content.rules.game.title.clone(),
                play_time_seconds,
                timestamp_seconds,
            },
            world: SaveWorld {
                world_id: runtime.world.world_id.clone(),
                map_id: runtime.world.map_id.clone(),
                pos: [runtime.world.position.0, runtime.world.position.1],
                vehicle: runtime.active_vehicle.clone(),
            },
            party: SaveParty::from_party(&runtime.party),
            inventory: SaveInventory::from_inventory(&runtime.inventory),
            shops: runtime.shop_states.clone(),
            flags: runtime.flags.clone(),
            map_states: prune_map_states(runtime),
            stats: runtime.stats_for_save(),
            vehicles: runtime
                .vehicle_positions
                .iter()
                .map(|(id, entry)| {
                    (
                        id.clone(),
                        SaveVehicleState {
                            map_id: entry.map_id.clone(),
                            pos: [entry.pos.0, entry.pos.1],
                        },
                    )
                })
                .collect(),
            settings: Some(runtime.settings.clone()),
        }
    }

    pub fn apply_to_runtime(&self, runtime: &mut GameRuntime) {
        runtime.flags = self.flags.clone();
        runtime.map_states = self.map_states.clone();
        runtime.world.world_id = self.world.world_id.clone();
        runtime.world.map_id = self.world.map_id.clone();
        runtime.world.position = (self.world.pos[0], self.world.pos[1]);
        if runtime.is_overworld_map(&runtime.world.map_id) {
            runtime.last_overworld = Some(crate::runtime::LastOverworld {
                world_id: runtime.world.world_id.clone(),
                map_id: runtime.world.map_id.clone(),
                pos: runtime.world.position,
            });
        } else {
            runtime.last_overworld = None;
        }
        runtime.active_vehicle = self.world.vehicle.clone();
        runtime.vehicle_positions = self
            .vehicles
            .iter()
            .map(|(id, entry)| {
                (
                    id.clone(),
                    crate::runtime::VehiclePosition {
                        map_id: entry.map_id.clone(),
                        pos: (entry.pos[0], entry.pos[1]),
                    },
                )
            })
            .collect();
        runtime.vehicle_slow_mode = false;
        runtime.party = self.party.to_party();
        for actor in runtime.party.roster.values_mut() {
            crate::party::sanitize_job_learned_sets(&runtime.content, actor);
            crate::party::reconcile_job_learned_state(&runtime.content, actor);
            crate::party::update_equipped_spells(&runtime.content, actor);
            crate::party::update_equipped_abilities(&runtime.content, actor);
        }
        runtime.inventory = self.inventory.to_inventory();
        runtime.shop_states = self.shops.clone();
        let initial_shops = crate::runtime::initial_shop_states(&runtime.content);
        for (id, state) in initial_shops {
            runtime.shop_states.entry(id).or_insert(state);
        }
        let mut stats = self.stats.clone();
        stats.remove("time_played");
        runtime.stats = stats;
        runtime.ensure_tracked_stats();
        runtime.settings = self
            .settings
            .clone()
            .unwrap_or_else(|| crate::runtime::SettingsState::from_rules(&runtime.content.rules));
        runtime.playtime = self.metadata.play_time_seconds;
        runtime.start_time = std::time::Instant::now();
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }

    pub fn write(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let path = path.as_ref();
        let file =
            std::fs::File::create(path).map_err(|err| format!("{}: {}", path.display(), err))?;
        serde_json::to_writer_pretty(file, self)
            .map_err(|err| format!("{}: {}", path.display(), err))
    }
}

fn prune_map_states(runtime: &GameRuntime) -> HashMap<String, MapState> {
    let mut map_states = runtime.map_states.clone();

    for (map_id, map_state) in map_states.iter_mut() {
        let map_index = match runtime.content.map_index.get(map_id) {
            Some(index) => *index,
            None => continue,
        };
        let map = &runtime.content.maps[map_index];

        for npc in &map.npcs {
            let persist = runtime
                .content
                .npcs
                .npcs
                .iter()
                .find(|entry| entry.id == npc.id)
                .and_then(|entry| entry.behavior.persist)
                .unwrap_or(false);
            if persist {
                continue;
            }
            if let Some(state) = map_state.entities.get_mut(&npc.id) {
                let is_roam = state.state.as_deref() == Some("roam");
                let is_patrol = state
                    .state
                    .as_deref()
                    .map(|value| value.starts_with("patrol:"))
                    .unwrap_or(false);
                if is_roam || is_patrol {
                    state.pos = None;
                    state.state = None;
                }
            }
        }

        map_state.entities.retain(|_, state| {
            state.pos.is_some()
                || state.state.is_some()
                || state.visible.is_some()
                || state.sprite.is_some()
        });
    }

    map_states
}

impl SaveParty {
    fn from_party(party: &PartyState) -> Self {
        let roster = party
            .roster
            .iter()
            .map(|(id, actor)| (id.clone(), SaveActor::from_actor(actor)))
            .collect();
        Self {
            active: party.active.clone(),
            reserve: party.reserve.clone(),
            roster,
        }
    }

    fn to_party(&self) -> PartyState {
        let roster = self
            .roster
            .iter()
            .map(|(id, actor)| (id.clone(), actor.to_actor()))
            .collect();
        PartyState {
            roster,
            active: self.active.clone(),
            reserve: self.reserve.clone(),
        }
    }
}

impl SaveActor {
    fn from_actor(actor: &Actor) -> Self {
        let mut unlocked_abilities = actor.unlocked_abilities.iter().cloned().collect::<Vec<_>>();
        unlocked_abilities.sort();
        let job_progress = actor
            .job_progress
            .iter()
            .map(|(id, progress)| (id.clone(), SaveJobProgress::from_progress(progress)))
            .collect();
        Self {
            id: actor.id.clone(),
            name: actor.name.clone(),
            job_id: actor.job_id.clone(),
            level: actor.level,
            exp: actor.exp,
            row: actor.row,
            current_hp: actor.current_hp,
            current_mp: actor.current_mp,
            base_stats: actor.base_stats.clone(),
            derived_stats: actor.derived_stats.clone(),
            equipment: actor.equipment.clone(),
            spells: actor.spells.clone(),
            equipped_spells: actor.equipped_spells.clone(),
            magic_tier_charges: actor.magic_tier_charges.clone(),
            secondary_job_id: actor.secondary_job_id.clone(),
            job_progress,
            weapon_proficiencies: actor.weapon_proficiencies.clone(),
            magic_proficiencies: actor.magic_proficiencies.clone(),
            unlocked_abilities,
        }
    }

    fn to_actor(&self) -> Actor {
        let mut job_progress: HashMap<String, JobProgress> = self
            .job_progress
            .iter()
            .map(|(id, progress)| (id.clone(), progress.to_progress()))
            .collect();
        let has_job_learned_data = job_progress.values().any(|progress| {
            !progress.learned_spells.is_empty() || !progress.learned_abilities.is_empty()
        });
        if !has_job_learned_data && (!self.spells.is_empty() || !self.unlocked_abilities.is_empty())
        {
            let progress = job_progress
                .entry(self.job_id.clone())
                .or_insert_with(JobProgress::default);
            for spell_id in &self.spells {
                progress.learned_spells.insert(spell_id.clone());
            }
            for ability_id in &self.unlocked_abilities {
                progress.learned_abilities.insert(ability_id.clone());
            }
        }
        Actor {
            id: self.id.clone(),
            name: self.name.clone(),
            job_id: self.job_id.clone(),
            level: self.level,
            exp: self.exp,
            row: self.row,
            current_hp: self.current_hp,
            current_mp: self.current_mp,
            base_stats: self.base_stats.clone(),
            derived_stats: self.derived_stats.clone(),
            equipment: self.equipment.clone(),
            spells: self.spells.clone(),
            equipped_spells: self.equipped_spells.clone(),
            equipped_abilities: Vec::new(),
            magic_tier_charges: self.magic_tier_charges.clone(),
            secondary_job_id: self.secondary_job_id.clone(),
            job_progress,
            weapon_proficiencies: self.weapon_proficiencies.clone(),
            magic_proficiencies: self.magic_proficiencies.clone(),
            unlocked_abilities: self.unlocked_abilities.iter().cloned().collect(),
            statuses: Vec::new(),
        }
    }
}

fn default_battle_row() -> crate::party::BattleRow {
    crate::party::BattleRow::Front
}

impl SaveJobProgress {
    fn from_progress(progress: &JobProgress) -> Self {
        let mut learned_spells = progress.learned_spells.iter().cloned().collect::<Vec<_>>();
        learned_spells.sort();
        let mut learned_abilities = progress
            .learned_abilities
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        learned_abilities.sort();
        Self {
            level: progress.level,
            exp: progress.exp,
            jp_earned: progress.jp_earned,
            jp_spent: progress.jp_spent,
            learned_spells,
            learned_abilities,
        }
    }

    fn to_progress(&self) -> JobProgress {
        JobProgress {
            level: self.level,
            exp: self.exp,
            jp_earned: self.jp_earned,
            jp_spent: self.jp_spent,
            learned_spells: self.learned_spells.iter().cloned().collect(),
            learned_abilities: self.learned_abilities.iter().cloned().collect(),
        }
    }
}

impl SaveInventory {
    fn from_inventory(inventory: &crate::inventory::InventoryState) -> Self {
        Self {
            items: inventory.items.clone(),
            equipment: inventory.equipment.clone(),
            currency: inventory.currency.clone(),
            items_order: inventory.items_order.clone(),
            equipment_order: inventory.equipment_order.clone(),
        }
    }

    fn to_inventory(&self) -> crate::inventory::InventoryState {
        let mut inventory = crate::inventory::InventoryState {
            items: self.items.clone(),
            equipment: self.equipment.clone(),
            currency: self.currency.clone(),
            items_order: self.items_order.clone(),
            equipment_order: self.equipment_order.clone(),
        };
        inventory.normalize_orders();
        inventory
    }
}

#[cfg(test)]
mod tests {
    use super::{SaveActor, SaveFile, SaveInventory, SaveJobProgress};
    use crate::content::Content;
    use crate::party::{BattleRow, JobProgress};
    use crate::runtime::GameRuntime;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn save_job_progress_sorts_learned_collections() {
        let progress = JobProgress {
            level: 2,
            exp: 50,
            jp_earned: 20,
            jp_spent: 4,
            learned_spells: ["zeta".to_string(), "alpha".to_string()]
                .into_iter()
                .collect(),
            learned_abilities: ["guard".to_string(), "bash".to_string()]
                .into_iter()
                .collect(),
        };

        let saved = SaveJobProgress::from_progress(&progress);
        assert_eq!(
            saved.learned_spells,
            vec!["alpha".to_string(), "zeta".to_string()]
        );
        assert_eq!(
            saved.learned_abilities,
            vec!["bash".to_string(), "guard".to_string()]
        );
    }

    #[test]
    fn save_actor_to_actor_migrates_legacy_learned_data() {
        let actor = SaveActor {
            id: "a1".to_string(),
            name: "Hero".to_string(),
            job_id: "knight".to_string(),
            level: 1,
            exp: 0,
            row: BattleRow::Front,
            current_hp: 10,
            current_mp: 5,
            base_stats: HashMap::new(),
            derived_stats: HashMap::new(),
            equipment: HashMap::new(),
            spells: vec!["fire".to_string()],
            equipped_spells: Vec::new(),
            magic_tier_charges: HashMap::new(),
            secondary_job_id: None,
            job_progress: HashMap::new(),
            weapon_proficiencies: HashMap::new(),
            magic_proficiencies: HashMap::new(),
            unlocked_abilities: vec!["focus".to_string()],
        };

        let migrated = actor.to_actor();
        let progress = migrated
            .job_progress
            .get("knight")
            .expect("job progress should be created");
        assert!(progress.learned_spells.contains("fire"));
        assert!(progress.learned_abilities.contains("focus"));
    }

    #[test]
    fn save_inventory_to_inventory_normalizes_orders() {
        let inventory = SaveInventory {
            items: [
                ("potion".to_string(), 1),
                ("ether".to_string(), 2),
                ("zero".to_string(), 0),
            ]
            .into_iter()
            .collect(),
            equipment: [("sword".to_string(), 1)].into_iter().collect(),
            currency: HashMap::new(),
            items_order: vec!["zero".to_string(), "unknown".to_string()],
            equipment_order: Vec::new(),
        };

        let normalized = inventory.to_inventory();
        assert_eq!(
            normalized.items_order,
            vec!["ether".to_string(), "potion".to_string()]
        );
        assert_eq!(normalized.equipment_order, vec!["sword".to_string()]);
    }

    #[test]
    #[ignore = "depends on local content bundle"]
    fn apply_to_runtime_excludes_time_played_from_stats_map() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/opencrystal-peak");
        let content = Content::load(dir).expect("content should load for tests");
        let mut runtime = GameRuntime::new(content);
        runtime.stats.insert("wins".to_string(), 3);

        let mut save = SaveFile::from_runtime(&runtime, 1);
        save.stats.insert("time_played".to_string(), 9999);
        save.stats.insert("wins".to_string(), 42);
        save.metadata.play_time_seconds = 555;

        save.apply_to_runtime(&mut runtime);

        assert_eq!(runtime.stats.get("wins"), Some(&42));
        assert!(!runtime.stats.contains_key("time_played"));
        assert_eq!(runtime.playtime, 555);
    }
}
