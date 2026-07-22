use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::types::tournament::Tournament;
use crate::utils::serde_datetime;

#[derive(Debug, Serialize, Deserialize)]
pub struct Season {
    #[serde(with = "serde_datetime")]
    start_day: DateTime<Tz>,
    #[serde(with = "serde_datetime")]
    end_day: DateTime<Tz>,
    tournament: Tournament,
}

impl Season {
    pub fn new(start_day: DateTime<Tz>, end_day: DateTime<Tz>, tournament: Tournament) -> Self {
        Self {
            start_day,
            end_day,
            tournament,
        }
    }

    pub fn start_day(&self) -> &DateTime<Tz> {
        &self.start_day
    }

    pub fn end_day(&self) -> &DateTime<Tz> {
        &self.end_day
    }

    pub fn tournament(&self) -> &Tournament {
        &self.tournament
    }

    pub fn is_game_day_in_range(&self, game_day: DateTime<Tz>) -> bool {
        game_day.ge(&self.start_day) && game_day.le(&self.end_day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono_tz::Europe::Zurich;

    use crate::types::tournament::TournamentType;

    #[test]
    fn test_correct_game_day() {
        let season = Season::new(
            Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap(),
            Zurich.with_ymd_and_hms(2026, 9, 22, 18, 0, 0).unwrap(),
            Tournament::new(
                TournamentType::RoundRobin,
                TournamentType::SingleElimination,
            ),
        );
        let game_day = Zurich.with_ymd_and_hms(2026, 6, 2, 8, 45, 0).unwrap();

        assert!(
            season.is_game_day_in_range(game_day),
            "Game day should be between start and end date of the season"
        );
    }

    #[test]
    fn test_game_day_outside_season() {
        let season = Season::new(
            Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap(),
            Zurich.with_ymd_and_hms(2026, 9, 22, 18, 0, 0).unwrap(),
            Tournament::new(
                TournamentType::RoundRobin,
                TournamentType::SingleElimination,
            ),
        );
        let game_day = Zurich.with_ymd_and_hms(2025, 6, 2, 8, 45, 0).unwrap();

        assert!(
            !season.is_game_day_in_range(game_day),
            "Game day should be between start and end date of the season"
        );
    }

    #[test]
    fn test_serialize_deserialize() {
        let season = Season::new(
            Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap(),
            Zurich.with_ymd_and_hms(2026, 9, 22, 18, 0, 0).unwrap(),
            Tournament::new(
                TournamentType::RoundRobin,
                TournamentType::SingleElimination,
            ),
        );

        let json = serde_json::to_string(&season).expect("serialization should succeed");
        let deserialized: Season =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(season.start_day(), deserialized.start_day());
        assert_eq!(season.end_day(), deserialized.end_day());
        assert_eq!(season.tournament(), deserialized.tournament());
    }
}
