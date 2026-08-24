use chrono::OutOfRange;
use chrono::ParseWeekdayError;
use rust_xlsxwriter::XlsxError;
use serde::{Serialize, Serializer};
use serde_json::Error as SerdeError;
use std::num::TryFromIntError;
use std::{io::Error as IoError, num::ParseIntError};

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
    #[error("failed to parse string to integer")]
    ParseDateError(#[from] ParseIntError),
    #[error("failed to create date time from {0:?}")]
    CreateDateTimeError(Vec<String>),
    #[error("Xlsx error")]
    XlsxError(#[from] XlsxError),
    #[error("Date out of range")]
    DateOutOfRange(#[from] OutOfRange),
    #[error("invalid time. Hour {0}, minute {0}")]
    InvalidTime(u8, u8),
    #[error("error parsing integer value")]
    ParseIntError(#[from] TryFromIntError),
    #[error("no eligible teams to referee")]
    EmptyEligibleReferees,
    #[error("no game days provided")]
    EmptyGameDays,
    #[error("failed to resolve resource path")]
    ResourceResolveError(#[from] tauri::Error),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
