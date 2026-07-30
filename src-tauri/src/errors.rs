use chrono::NaiveDate;
use chrono::ParseWeekdayError;
use serde::{Serialize, Serializer};
use serde_json::Error as SerdeError;
use std::io::Error as IoError;

use thiserror::Error;

use crate::types::team::Team;

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
    #[error("invalid '{0}' parameters. Teams {1:?}, game_days: {2:?}, max_games_per_day: {3}")]
    InvalidTournamentParameters(String, Vec<Team>, Vec<NaiveDate>, usize),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
