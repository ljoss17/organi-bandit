use serde::de;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::errors::AppError;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Team {
    name: String,
    seed: Option<u32>,
}

// A bye is written as null: it has no identity of its own, and keeping the
// reserved name off the wire is what lets the deserialiser below refuse it.
impl Serialize for Team {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_bye() {
            return serializer.serialize_none();
        }
        let mut team = serializer.serialize_struct("Team", 2)?;
        team.serialize_field("name", &self.name)?;
        team.serialize_field("seed", &self.seed)?;
        team.end()
    }
}

// Use Team::new() to validate data before deserializing
impl<'de> Deserialize<'de> for Team {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Fields {
            name: String,
            seed: Option<u32>,
        }

        match Option::<Fields>::deserialize(deserializer)? {
            None => Ok(Self::bye()),
            Some(fields) => Self::new(&fields.name, fields.seed).map_err(de::Error::custom),
        }
    }
}

impl Team {
    pub const BYE_NAME: &'static str = "Bye";

    pub fn new(name: &str, seed: Option<u32>) -> Result<Self, AppError> {
        Ok(Self {
            name: Self::parse_name(name)?,
            seed,
        })
    }

    // Title-cases the name and refuses the one reserved for byes.
    fn parse_name(name: &str) -> Result<String, AppError> {
        let parsed_name = name
            .to_lowercase()
            .split(' ')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => String::new(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ");

        if parsed_name == Self::BYE_NAME {
            return Err(AppError::InvalidTeamName(name.to_string()));
        }
        Ok(parsed_name)
    }

    pub fn bye() -> Self {
        Self {
            name: Self::BYE_NAME.to_string(),
            seed: None,
        }
    }

    pub fn is_bye(&self) -> bool {
        self.name == Self::BYE_NAME
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_seed(&self) -> u32 {
        self.seed.unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_case_team_name() {
        let team_1 = Team::new("morges bandits", None).unwrap();
        assert_eq!(team_1.get_name(), "Morges Bandits");
        let team_2 = Team::new("mOrges BANDITS", None).unwrap();
        assert_eq!(team_2.get_name(), "Morges Bandits");
    }

    #[test]
    fn test_reject_bye_named_teams() {
        let team_1 = Team::new("bye", None);
        assert!(team_1.is_err(), "team name 'bye' should be rejected");
        let team_2 = Team::new("Bye", None);
        assert!(team_2.is_err(), "team name 'Bye' should be rejected");
        let team_3 = Team::new("bYe", None);
        assert!(team_3.is_err(), "team name 'bYe' should be rejected");
        let team_4 = Team::new("byE", None);
        assert!(team_4.is_err(), "team name 'byE' should be rejected");
        let team_5 = Team::new("BYe", None);
        assert!(team_5.is_err(), "team name 'BYe' should be rejected");
        let team_6 = Team::new("ByE", None);
        assert!(team_6.is_err(), "team name 'ByE' should be rejected");
        let team_7 = Team::new("bYE", None);
        assert!(team_7.is_err(), "team name 'bYE' should be rejected");
        let team_8 = Team::new("BYE", None);
        assert!(team_8.is_err(), "team name 'BYE' should be rejected");
    }

    #[test]
    fn test_is_bye_team() {
        let bye_team = Team::bye();

        assert!(bye_team.is_bye());
    }
}
