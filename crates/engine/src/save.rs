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
    pub flags: HashSet<String>,
    pub map_states: HashMap<String, MapState>,
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
    pub active: Vec<String>,
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
    pub unlocked_abilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveJobProgress {
    pub level: u32,
    pub exp: i32,
    pub jp_earned: i32,
    pub jp_spent: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveInventory {
    pub items: HashMap<String, i32>,
    pub equipment: HashMap<String, i32>,
    pub currency: HashMap<String, i32>,
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
            flags: runtime.flags.clone(),
            map_states: runtime.map_states.clone(),
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
        runtime.inventory = self.inventory.to_inventory();
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
            unlocked_abilities,
        }
    }

    fn to_actor(&self) -> Actor {
        let job_progress = self
            .job_progress
            .iter()
            .map(|(id, progress)| (id.clone(), progress.to_progress()))
            .collect();
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
            magic_tier_charges: self.magic_tier_charges.clone(),
            secondary_job_id: self.secondary_job_id.clone(),
            job_progress,
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
        Self {
            level: progress.level,
            exp: progress.exp,
            jp_earned: progress.jp_earned,
            jp_spent: progress.jp_spent,
        }
    }

    fn to_progress(&self) -> JobProgress {
        JobProgress {
            level: self.level,
            exp: self.exp,
            jp_earned: self.jp_earned,
            jp_spent: self.jp_spent,
        }
    }
}

impl SaveInventory {
    fn from_inventory(inventory: &crate::inventory::InventoryState) -> Self {
        Self {
            items: inventory.items.clone(),
            equipment: inventory.equipment.clone(),
            currency: inventory.currency.clone(),
        }
    }

    fn to_inventory(&self) -> crate::inventory::InventoryState {
        crate::inventory::InventoryState {
            items: self.items.clone(),
            equipment: self.equipment.clone(),
            currency: self.currency.clone(),
        }
    }
}
