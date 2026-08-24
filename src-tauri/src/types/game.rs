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
    ) -> Result<Self, AppError> {
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
            .ok_or(AppError::InvalidGameDay(naive_game_day, game_time))?;
        Ok(Self {
            home_team,
            away_team,
            game_day,
            referee,
        })
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

    pub fn get_referee(&self) -> &Option<Team> {
        &self.referee
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
        )
        .unwrap();

        let json = serde_json::to_string(&game).expect("serialization should succeed");
        let deserialized: Game =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(game.get_home_team(), deserialized.get_home_team());
        assert_eq!(game.get_away_team(), deserialized.get_away_team());
        assert_eq!(game.get_game_day(), deserialized.get_game_day());
    }

    #[test]
    fn new_with_game_day_combines_date_and_time() {
        let game = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            NaiveDate::from_ymd_opt(2026, 7, 22).unwrap(),
            GameTime::new(20, 30).unwrap(),
            None,
        )
        .unwrap();

        let game_day = game.get_game_day();
        assert_eq!(game_day.year(), 2026);
        assert_eq!(game_day.month(), 7);
        assert_eq!(game_day.day(), 22);
        assert_eq!(game_day.hour(), 20);
        assert_eq!(game_day.minute(), 30);
    }

    #[test]
    fn get_game_time_round_trips_the_stored_time() {
        let game = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            NaiveDate::from_ymd_opt(2026, 7, 22).unwrap(),
            GameTime::new(9, 15).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(game.get_game_time().unwrap(), GameTime::new(9, 15).unwrap());
    }

    #[test]
    fn get_referee_returns_the_assigned_referee() {
        let referee = Team::new("Referee Team", None);
        let game = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            NaiveDate::from_ymd_opt(2026, 7, 22).unwrap(),
            GameTime::new(9, 15).unwrap(),
            Some(referee.clone()),
        )
        .unwrap();

        assert_eq!(game.get_referee(), &Some(referee));
    }

    #[test]
    fn new_with_game_day_errors_in_dst_spring_forward_gap() {
        // 2026-03-29 02:30 in Europe/Zurich falls in the DST "spring forward"
        // gap (clocks jump 02:00 -> 03:00), so this time does not exist.
        let result = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            NaiveDate::from_ymd_opt(2026, 3, 29).unwrap(),
            GameTime::new(2, 30).unwrap(),
            None,
        );

        assert!(matches!(result, Err(AppError::InvalidGameDay(_, _))));
    }

    #[test]
    fn new_with_game_day_errors_in_dst_fall_back_ambiguity() {
        // 2026-10-25 02:30 in Europe/Zurich is ambiguous (clocks jump
        // 03:00 -> 02:00, so this time occurs twice).
        let result = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            NaiveDate::from_ymd_opt(2026, 10, 25).unwrap(),
            GameTime::new(2, 30).unwrap(),
            None,
        );

        assert!(matches!(result, Err(AppError::InvalidGameDay(_, _))));
    }
}
