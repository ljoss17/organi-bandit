use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DateConfiguration {
    start_date: NaiveDate,
    game_days: Vec<Weekday>,
}

impl DateConfiguration {
    pub fn new(start_date: NaiveDate, game_days: Vec<Weekday>) -> Self {
        Self {
            start_date,
            game_days,
        }
    }

    pub fn start_date(&self) -> &NaiveDate {
        &self.start_date
    }

    pub fn game_days(&self) -> &[Weekday] {
        &self.game_days
    }
}
