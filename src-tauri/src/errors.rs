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
    #[error("invalid time. Hour {0}, minute {1}")]
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
    #[error(
        "hard stop '{0}' is too close to midnight for {1} games with {2} between them; the slot after the last game would run past midnight"
    )]
    HardStopTooCloseToMidnight(GameTime, GameTime, GameTime),
    #[error(
        "the first game starting at '{0}' would end after the hard stop '{1}'; start earlier, shorten the games or set a later hard stop"
    )]
    FirstGameEndsAfterHardStop(GameTime, GameTime),
    #[error(
        "the first game after the break ending at '{0}' would end after the hard stop '{1}'; end the break earlier, shorten the games or set a later hard stop"
    )]
    FirstGameAfterBreakEndsAfterHardStop(GameTime, GameTime),
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
    #[error("the next 10 dates starting from {0} are all excluded")]
    NoStartDate(NaiveDate),
    #[error(
        "team \"{0}\" is listed more than once with different seeds ({1} and {2}); give it a single seed or remove the duplicate entry"
    )]
    ConflictingTeamSeeds(String, u32, u32),
    #[error("missing team for specified date '{0}'")]
    MissingTeam(String),
    #[error("failed to retrieve eligible team")]
    MissingEligibleTeam,
    #[error("a round on {0} has no team on bye although the schedule has byes")]
    MissingByeTeam(NaiveDate),
    #[error("the second round-robin pass has no round matching the first pass's round on {0}")]
    MismatchedSchedulePasses(NaiveDate),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
