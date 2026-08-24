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
        let mut hour = (self.hour() + rhs.hour()) % 24;
        let total_minutes = self.minute() + rhs.minute();
        let minute = if total_minutes > 59 {
            hour = (hour + 1) % 24;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_boundary_values() {
        assert!(GameTime::new(0, 0).is_ok());
        assert!(GameTime::new(23, 59).is_ok());
    }

    #[test]
    fn new_rejects_invalid_hour() {
        let result = GameTime::new(24, 0);
        assert!(matches!(result, Err(AppError::InvalidTime(24, 0))));
    }

    #[test]
    fn new_rejects_invalid_minute() {
        let result = GameTime::new(0, 60);
        assert!(matches!(result, Err(AppError::InvalidTime(0, 60))));
    }

    #[test]
    fn hour_and_minute_return_constructed_values() {
        let time = GameTime::new(14, 5).unwrap();
        assert_eq!(time.hour(), 14);
        assert_eq!(time.minute(), 5);
    }

    #[test]
    fn display_formats_with_leading_zeros() {
        let time = GameTime::new(9, 5).unwrap();
        assert_eq!(time.to_string(), "09:05");
    }

    #[test]
    fn add_sums_hours_and_minutes_without_carry() {
        let start = GameTime::new(9, 0).unwrap();
        let duration = GameTime::new(1, 30).unwrap();
        assert_eq!(start + duration, GameTime::new(10, 30).unwrap());
    }

    #[test]
    fn add_carries_minute_overflow_into_hour() {
        let start = GameTime::new(9, 45).unwrap();
        let duration = GameTime::new(0, 30).unwrap();
        assert_eq!(start + duration, GameTime::new(10, 15).unwrap());
    }

    #[test]
    fn add_leaves_hour_23_unchanged_when_adding_zero() {
        let start = GameTime::new(23, 0).unwrap();
        let duration = GameTime::new(0, 0).unwrap();
        assert_eq!(start + duration, GameTime::new(23, 0).unwrap());
    }

    #[test]
    fn add_wraps_past_midnight_at_hour_24() {
        let start = GameTime::new(23, 45).unwrap();
        let duration = GameTime::new(0, 30).unwrap();
        assert_eq!(start + duration, GameTime::new(0, 15).unwrap());
    }
}
