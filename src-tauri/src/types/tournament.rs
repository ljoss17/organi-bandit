use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Tournament {
    group_stage: TournamentType,
    playoff: TournamentType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TournamentType {
    RoundRobin,
    SingleElimination,
}

impl Tournament {
    pub fn new(group_stage: TournamentType, playoff: TournamentType) -> Self {
        Self {
            group_stage,
            playoff,
        }
    }

    pub fn group_stage(&self) -> &TournamentType {
        &self.group_stage
    }

    pub fn playoff(&self) -> &TournamentType {
        &self.playoff
    }
}
