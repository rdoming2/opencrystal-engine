use engine::battle::BattleState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BattlePhase {
    Command,
    Magic,
    Abilities,
    Items,
    TargetEnemy,
    TargetParty,
    Victory,
    Defeat,
}

#[derive(Clone, Debug)]
pub enum PendingBattleAction {
    Attack,
    Magic(crate::menu::common::SpellEntry),
    Ability(crate::menu::common::AbilityEntry),
    Item(String),
}

pub struct BattleMenuState {
    pub phase: BattlePhase,
    pub command_index: usize,
    pub enemy_index: usize,
    pub party_index: usize,
    pub magic_index: usize,
    pub ability_index: usize,
    pub item_index: usize,
    pub pending_action: Option<PendingBattleAction>,
    pub paused: bool,
}

impl BattleMenuState {
    pub fn new() -> Self {
        Self {
            phase: BattlePhase::Command,
            command_index: 0,
            enemy_index: 0,
            party_index: 0,
            magic_index: 0,
            ability_index: 0,
            item_index: 0,
            pending_action: None,
            paused: false,
        }
    }

    pub fn reset_for_actor(&mut self) {
        self.phase = BattlePhase::Command;
        self.pending_action = None;
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BattleTurnActor {
    Party(usize),
    Enemy(usize),
}

pub struct BattleTurnState {
    pub order: Vec<BattleTurnActor>,
    pub index: usize,
}

impl BattleTurnState {
    pub fn new(order: Vec<BattleTurnActor>) -> Self {
        Self { order, index: 0 }
    }

    pub fn reset(&mut self, order: Vec<BattleTurnActor>) {
        self.order = order;
        self.index = 0;
    }
}

#[derive(Clone, Copy)]
pub enum TargetRule {
    Alive,
    KnockedOut,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VictoryState {
    Summary,
    LevelUp(usize),
}

pub fn enemy_target_indices(battle_state: &BattleState) -> Vec<usize> {
    battle_state
        .enemies
        .iter()
        .enumerate()
        .filter_map(|(index, enemy)| if enemy.is_alive() { Some(index) } else { None })
        .collect()
}

pub fn party_target_indices(
    runtime: &engine::runtime::GameRuntime,
    battle_state: &BattleState,
    rule: TargetRule,
) -> Vec<usize> {
    battle_state
        .party_order
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let alive = runtime
                .party
                .roster
                .get(id)
                .map(|actor| actor.current_hp > 0)
                .unwrap_or(false);
            match rule {
                TargetRule::Alive if alive => Some(index),
                TargetRule::KnockedOut if !alive => Some(index),
                _ => None,
            }
        })
        .collect()
}

pub fn step_target_index(current: usize, valid: &[usize], direction: i32) -> usize {
    if valid.is_empty() {
        return current;
    }
    let position = valid
        .iter()
        .position(|index| *index == current)
        .unwrap_or(0);
    let len = valid.len();
    let next = if direction >= 0 {
        (position + 1) % len
    } else {
        (position + len - 1) % len
    };
    valid[next]
}

pub fn ensure_valid_index(current: usize, valid: &[usize]) -> Option<usize> {
    if valid.is_empty() {
        return None;
    }
    if valid.contains(&current) {
        Some(current)
    } else {
        valid.first().copied()
    }
}

pub fn set_initial_enemy_target(
    menu_state: &mut BattleMenuState,
    battle_state: &BattleState,
) -> bool {
    let valid = enemy_target_indices(battle_state);
    if let Some(index) = ensure_valid_index(menu_state.enemy_index, &valid) {
        menu_state.enemy_index = index;
        true
    } else {
        false
    }
}

pub fn set_initial_party_target(
    menu_state: &mut BattleMenuState,
    battle_state: &BattleState,
    runtime: &engine::runtime::GameRuntime,
    action: &PendingBattleAction,
) -> bool {
    let rule = party_target_rule(action, runtime);
    let valid = party_target_indices(runtime, battle_state, rule);
    if let Some(index) = ensure_valid_index(menu_state.party_index, &valid) {
        menu_state.party_index = index;
        true
    } else {
        false
    }
}

pub fn party_target_rule(
    action: &PendingBattleAction,
    runtime: &engine::runtime::GameRuntime,
) -> TargetRule {
    match action {
        PendingBattleAction::Magic(entry) => target_rule_for_effect(entry.effect_type.as_str()),
        PendingBattleAction::Ability(entry) => target_rule_for_effect(entry.effect_type.as_str()),
        PendingBattleAction::Item(item_id) => runtime
            .content
            .items
            .items
            .iter()
            .find(|item| item.id == *item_id)
            .map(|item| target_rule_for_effect(item.effect.r#type.as_str()))
            .unwrap_or(TargetRule::Alive),
        PendingBattleAction::Attack => TargetRule::Alive,
    }
}

fn target_rule_for_effect(effect_type: &str) -> TargetRule {
    match effect_type {
        "revive" => TargetRule::KnockedOut,
        _ => TargetRule::Alive,
    }
}
