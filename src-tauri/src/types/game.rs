use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Timelike};
use chrono_tz::Europe::Zurich;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::types::game_time::GameTime;
use crate::types::team::Team;
use crate::utils::serde_datetime;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    home_team: Team,
    away_team: Team,
    #[serde(with = "serde_datetime")]
    game_day: DateTime<Tz>,
    referee: Option<Team>,
}

impl Game {
    pub fn new_with_game_day(
        home_team: Team,
        away_team: Team,
        naive_game_day: NaiveDate,
        game_time: GameTime,
        referee: Option<Team>,
    ) -> Self {
        let game_day = Zurich
            .with_ymd_and_hms(
                naive_game_day.year(),
                naive_game_day.month(),
                naive_game_day.day(),
                game_time.hour().into(),
                game_time.minute().into(),
                0,
            )
            .single()
            .expect("Game day should be an exact date and time");
        Self {
            home_team,
            away_team,
            game_day,
            referee,
        }
    }

    pub fn get_home_team(&self) -> &Team {
        &self.home_team
    }

    pub fn get_away_team(&self) -> &Team {
        &self.away_team
    }

    pub fn get_game_day(&self) -> &DateTime<Tz> {
        &self.game_day
    }

    pub fn get_game_time(&self) -> Result<GameTime, AppError> {
        let hour = self.game_day.hour().try_into()?;
        let minute = self.game_day.minute().try_into()?;
        GameTime::new(hour, minute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let game = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            NaiveDate::from_ymd_opt(2026, 7, 22).unwrap(),
            GameTime::new(20, 30).unwrap(),
            None,
        );

        let json = serde_json::to_string(&game).expect("serialization should succeed");
        let deserialized: Game =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(game.get_home_team(), deserialized.get_home_team());
        assert_eq!(game.get_away_team(), deserialized.get_away_team());
        assert_eq!(game.get_game_day(), deserialized.get_game_day());
    }
}
