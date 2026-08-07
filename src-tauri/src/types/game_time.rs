use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Add;

use serde::Deserialize;
use serde::Serialize;

use crate::errors::AppError;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GameTime {
    hour: u8,
    minute: u8,
}

impl Display for GameTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl Add for GameTime {
    type Output = GameTime;

    // The addition does not take into account day changes
    fn add(self, rhs: Self) -> Self::Output {
        let mut hour = (self.hour() + rhs.hour()) % 23;
        let total_minutes = self.minute() + rhs.minute();
        let minute = if total_minutes > 59 {
            hour = (hour + 1) % 23;
            total_minutes % 60
        } else {
            total_minutes
        };
        GameTime::new(hour, minute).expect("safe")
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

    pub fn increment(&mut self, amount: GameTime) {
        self.hour += amount.hour();
        self.minute += amount.minute();
    }
}
