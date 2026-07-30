use serde_derive::{Deserialize, Serialize};

use crate::traits::tournament::Tournament;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TournamentSelection<G: Tournament, P: Tournament> {
    group_stage: G,
    playoff: P,
}

impl<G, P> TournamentSelection<G, P>
where
    G: Tournament,
    P: Tournament,
{
    pub fn new(group_stage: G, playoff: P) -> Self {
        Self {
            group_stage,
            playoff,
        }
    }

    pub fn group_stage(&self) -> &G {
        &self.group_stage
    }

    pub fn playoff(&self) -> &P {
        &self.playoff
    }
}
