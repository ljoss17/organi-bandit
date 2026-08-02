use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;

use crate::errors::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameTime {
    hour: u8,
    minute: u8,
}

impl Display for GameTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl GameTime {
    pub fn new(hour: u8, minute: u8) -> Result<Self, AppError> {
        if hour > 23 || minute > 59 {
            return Err(AppError::InvalidTime(hour, minute));
        }
        Ok(Self { hour, minute })
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }
}
