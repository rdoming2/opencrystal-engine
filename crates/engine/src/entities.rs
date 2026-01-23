#[derive(Clone, Debug)]
pub struct PartyMember {
    pub id: String,
    pub name: String,
    pub job_id: String,
}

#[derive(Clone, Debug)]
pub struct EnemyInstance {
    pub id: String,
    pub name: String,
}
