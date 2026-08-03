use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, Hash, Serialize, Deserialize, PartialEq)]
pub struct Team {
    name: String,
    seed: Option<u32>,
}

impl Team {
    pub fn new(name: &str, seed: Option<u32>) -> Self {
        Self {
            name: name.to_owned(),
            seed,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_seed(&self) -> u32 {
        self.seed.unwrap_or(0)
    }
}
