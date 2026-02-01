use rand::Rng;
use std::collections::HashMap;

use crate::content::Content;
use crate::encounters::EncounterMember;
use crate::entities::{EnemyArt, EnemyDefinition, EnemyLoot, EnemySprite};
use crate::party::{Actor, PartyState};

#[derive(Clone, Debug, PartialEq)]
pub enum BattleMode {
    Turn,
    Dynamic,
    DynamicWait,
}

#[derive(Clone, Debug)]
pub struct BattleState {
    pub party_order: Vec<String>,
    pub enemies: Vec<BattleEnemy>,
    pub active_index: usize,
    pub log: Vec<String>,
    pub atb_party: HashMap<String, f32>,
    pub atb_enemy: Vec<f32>,
    pub mode: BattleMode,
}

#[derive(Clone, Debug)]
pub struct BattleEnemy {
    pub id: String,
    pub name: String,
    pub stats: HashMap<String, i32>,
    pub traits: Vec<String>,
    pub sprite: EnemySprite,
    pub art: Option<EnemyArt>,
    pub loot: Vec<EnemyLoot>,
    pub exp: i32,
    pub currency: i32,
    pub jp: i32,
    pub pos: (i32, i32),
    pub current_hp: i32,
    pub current_mp: i32,
    pub scanned: bool,
}

#[derive(Clone, Debug, Default)]
pub struct BattleRewards {
    pub exp: i32,
    pub currency: i32,
    pub jp: i32,
    pub items: HashMap<String, i32>,
}

#[derive(Clone, Debug, Default)]
pub struct LevelUpDiff {
    pub actor_name: String,
    pub old_level: u32,
    pub new_level: u32,
    pub stat_changes: HashMap<String, (i32, i32)>,
}

#[derive(Clone, Debug, Default)]
pub struct BattleResult {
    pub rewards: BattleRewards,
    pub level_ups: Vec<LevelUpDiff>,
}

impl BattleEnemy {
    pub fn max_hp(&self) -> i32 {
        stat_value(&self.stats, "hp")
    }

    pub fn max_mp(&self) -> i32 {
        stat_value(&self.stats, "mp")
    }

    pub fn atk(&self) -> i32 {
        if let Some(atk) = self.stats.get("atk") {
            return *atk;
        }
        stat_value(&self.stats, "str") * 2
    }

    pub fn def(&self) -> i32 {
        if let Some(def) = self.stats.get("def") {
            return *def;
        }
        stat_value(&self.stats, "vit") * 2
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }
}

pub fn build_battle_state(
    content: &Content,
    party: &PartyState,
    formation: &[EncounterMember],
) -> BattleState {
    let party_order = party.active.clone();
    let enemies = formation
        .iter()
        .filter_map(|member| {
            let enemy = content
                .enemies
                .enemies
                .iter()
                .find(|enemy| enemy.id == member.enemy)?;
            Some(build_enemy(enemy, member.pos))
        })
        .collect::<Vec<_>>();

    let mode = match content.rules.game.battle_mode {
        crate::rules::BattleMode::Turn => BattleMode::Turn,
        crate::rules::BattleMode::Dynamic => BattleMode::Dynamic,
        crate::rules::BattleMode::DynamicWait => BattleMode::DynamicWait,
    };

    let mut rng = rand::thread_rng();
    let atb_party = party_order
        .iter()
        .map(|id| (id.clone(), rng.gen_range(0.0..10.0)))
        .collect();
    let atb_enemy = (0..enemies.len())
        .map(|_| rng.gen_range(0.0..10.0))
        .collect();

    BattleState {
        party_order,
        enemies,
        active_index: 0,
        log: Vec::new(),
        atb_party,
        atb_enemy,
        mode,
    }
}

pub fn is_party_defeated(party: &PartyState, order: &[String]) -> bool {
    order.iter().all(|id| {
        party
            .roster
            .get(id)
            .map(|actor| actor.current_hp <= 0)
            .unwrap_or(true)
    })
}

pub fn is_enemies_defeated(enemies: &[BattleEnemy]) -> bool {
    enemies.iter().all(|enemy| enemy.current_hp <= 0)
}

pub fn next_living_party_index(
    party: &PartyState,
    order: &[String],
    start: usize,
) -> Option<usize> {
    if order.is_empty() {
        return None;
    }
    for offset in 0..order.len() {
        let index = (start + offset) % order.len();
        if let Some(actor) = party.roster.get(&order[index]) {
            if actor.current_hp > 0 {
                return Some(index);
            }
        }
    }
    None
}

pub fn apply_damage_to_actor(actor: &mut Actor, amount: i32) {
    actor.current_hp = (actor.current_hp - amount).max(0);
}

pub fn apply_damage_to_enemy(enemy: &mut BattleEnemy, amount: i32) {
    enemy.current_hp = (enemy.current_hp - amount).max(0);
}

pub fn physical_damage(attacker_atk: i32, defender_def: i32, rng: &mut impl Rng) -> i32 {
    let base = attacker_atk.saturating_sub(defender_def / 2).max(1);
    roll_damage(base, rng)
}

pub fn roll_damage(base: i32, rng: &mut impl Rng) -> i32 {
    if base <= 1 {
        return base.max(1);
    }
    let variance = rng.gen_range(90..=110) as f32 / 100.0;
    ((base as f32) * variance).round().max(1.0) as i32
}

pub fn collect_rewards(enemies: &[BattleEnemy], rng: &mut impl Rng) -> BattleRewards {
    let mut rewards = BattleRewards::default();
    for enemy in enemies {
        rewards.exp += enemy.exp.max(0);
        rewards.currency += enemy.currency.max(0);
        rewards.jp += enemy.jp.max(0);
        for loot in &enemy.loot {
            if loot.chance <= 0.0 {
                continue;
            }
            if rng.r#gen::<f32>() <= loot.chance {
                let entry = rewards.items.entry(loot.item.clone()).or_insert(0);
                *entry += 1;
            }
        }
    }
    rewards
}

fn build_enemy(enemy: &EnemyDefinition, pos: [i32; 2]) -> BattleEnemy {
    let max_hp = stat_value(&enemy.stats, "hp");
    let max_mp = stat_value(&enemy.stats, "mp");
    BattleEnemy {
        id: enemy.id.clone(),
        name: enemy.name.clone(),
        stats: enemy.stats.clone(),
        traits: enemy.traits.clone(),
        sprite: enemy.sprite.clone(),
        art: enemy.art.clone(),
        loot: enemy.loot.clone(),
        exp: enemy.exp,
        currency: enemy.currency,
        jp: enemy.jp,
        pos: (pos[0], pos[1]),
        current_hp: max_hp.max(1),
        current_mp: max_mp.max(0),
        scanned: false,
    }
}

fn stat_value(stats: &HashMap<String, i32>, key: &str) -> i32 {
    stats.get(key).copied().unwrap_or(0)
}
