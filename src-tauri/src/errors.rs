use chrono::ParseWeekdayError;
use serde::{Serialize, Serializer};
use serde_json::Error as SerdeError;
use std::io::Error as IoError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to read file")]
    ReadError(#[from] IoError),
    #[error("failed to deserialise data")]
    DeserializeError(#[from] SerdeError),
    #[error("failed to parse string to chrono Weekday")]
    WeekdayParseError(#[from] ParseWeekdayError),
    #[error("failed to find game")]
    MissingGame,
    #[error("not enough teams. Got {0}, require at leasts {1}")]
    NotEnoughTeams(usize, usize),
    #[error("{0} is too short. Game days {1}, required game days {2}")]
    TournamentTooShort(String, usize, usize),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
