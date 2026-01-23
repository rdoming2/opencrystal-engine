use crate::entities::{EnemyInstance, PartyMember};

#[derive(Clone, Debug)]
pub struct BattleState {
    pub party: Vec<PartyMember>,
    pub enemies: Vec<EnemyInstance>,
}
