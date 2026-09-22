use chrono::NaiveDate;
use chrono::OutOfRange;
use rust_xlsxwriter::XlsxError;
use serde::{Serialize, Serializer};
use serde_json::Error as SerdeError;
use std::io::Error as IoError;
use std::num::TryFromIntError;

use thiserror::Error;

use crate::types::game_time::GameTime;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to read file")]
    ReadError(#[from] IoError),
    #[error("failed to deserialise data: {0}")]
    DeserializeError(#[from] SerdeError),
    #[error("failed to find game")]
    MissingGame,
    #[error("team name '{0}' is invalid")]
    InvalidTeamName(String),
    #[error("not enough teams. Got {0}, require at least {1}")]
    NotEnoughTeams(usize, usize),
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
    #[error("start time needs to be configured before start break. Start time '{0}', start break: '{1}'")]
    StartTimeAfterStartBreak(GameTime, GameTime),
    #[error(
        "start break needs to be configured after end break. Start break '{0}', end break: '{1}'"
    )]
    StartBreakAfterEndBreak(GameTime, GameTime),
    #[error("failed to resolve resource path")]
    ResourceResolveError(#[from] tauri::Error),
    #[error("{0} at {1} is not a valid local time (daylight saving transition)")]
    InvalidGameDay(NaiveDate, GameTime),
    #[error("number of fields must be at least 1, got {0}")]
    InvalidNumberOfFields(u32),
    #[error(
        "game duration must be longer than zero (the time between games may be zero, but a game itself cannot take no time)"
    )]
    ZeroGameDuration,
    #[error(
        "cannot give every team two distinct opponents per match day with only {0} team(s); at least 4 are required"
    )]
    InfeasibleDailyDoubleRoundRobin(usize),
    #[error(
        "one leg of the daily double round-robin needs {0} game slots, but only {1} are available"
    )]
    InsufficientDailyCapacity(u32, u32),
    #[error("cannot subtract {1} from {0}: {1} is later in the day than {0}")]
    GameTimeSubtractionUnderflow(GameTime, GameTime),
    #[error(
        "the games configured for each day do not fit before midnight when play starts at {0}; start earlier, shorten the games or the break, add fields, or reduce the time between games"
    )]
    ScheduleRunsPastMidnight(GameTime),
    #[error("the next 10 dates starting from {0} are all excluded")]
    NoStartDate(NaiveDate),
    #[error(
        "team \"{0}\" is listed more than once with different seeds ({1} and {2}); give it a single seed or remove the duplicate entry"
    )]
    ConflictingTeamSeeds(String, u32, u32),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
