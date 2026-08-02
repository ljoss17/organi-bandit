use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::traits::tournament::Tournament;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
use crate::types::team::Team;
use crate::types::tournament::TournamentSelection;
use crate::utils::serde_datetime;

#[derive(Debug, Serialize, Deserialize)]
pub struct Season<G: Tournament, P: Tournament> {
    #[serde(with = "serde_datetime")]
    start_day: DateTime<Tz>,
    #[serde(with = "serde_datetime")]
    end_day: DateTime<Tz>,
    game_times: Vec<GameTime>,
    number_fields: usize,
    tournament: TournamentSelection<G, P>,
    game_days: Vec<Weekday>,
}

impl<G, P> Season<G, P>
where
    G: Tournament,
    P: Tournament,
{
    pub fn new(
        start_day: DateTime<Tz>,
        end_day: DateTime<Tz>,
        mut game_times: Vec<GameTime>,
        number_fields: usize,
        tournament: TournamentSelection<G, P>,
        game_days: Vec<Weekday>,
    ) -> Self {
        game_times.sort();
        Self {
            start_day,
            end_day,
            game_times,
            number_fields,
            tournament,
            game_days,
        }
    }

    pub fn start_day(&self) -> &DateTime<Tz> {
        &self.start_day
    }

    pub fn end_day(&self) -> &DateTime<Tz> {
        &self.end_day
    }

    pub fn game_times(&self) -> &Vec<GameTime> {
        &self.game_times
    }

    pub fn number_fields(&self) -> usize {
        self.number_fields
    }

    pub fn max_games_per_day(&self) -> usize {
        self.number_fields * self.game_times.len()
    }

    pub fn tournament(&self) -> &TournamentSelection<G, P> {
        &self.tournament
    }

    pub fn is_game_day_in_range(&self, game_day: DateTime<Tz>) -> bool {
        game_day.ge(&self.start_day) && game_day.le(&self.end_day)
    }

    pub fn get_all_game_days(&self) -> Vec<NaiveDate> {
        let end_day = self.end_day.date_naive();
        self.start_day
            .date_naive()
            .iter_days()
            .take_while(|day| day <= &end_day)
            .filter(|day| self.game_days.contains(&day.weekday()))
            .collect::<Vec<_>>()
    }

    pub fn compute_season_schedule(&self, teams: &[Team]) -> Result<Vec<Game>, AppError> {
        let all_game_days = self.get_all_game_days();
        self.tournament().group_stage().validate_parameters(
            teams,
            &all_game_days,
            self.max_games_per_day(),
        )?;
        let group_stage_schedule = self.tournament().group_stage().compute_schedule(
            teams,
            &all_game_days,
            self.game_times(),
            self.number_fields(),
            true,
        )?;
        let last_group_stage_day = group_stage_schedule
            .iter()
            .max_by_key(|game| game.get_game_day())
            .ok_or(AppError::MissingGame)?
            .get_game_day();

        let playoff_game_days = all_game_days
            .iter()
            .filter(|game| {
                game.signed_duration_since(last_group_stage_day.date_naive()) > TimeDelta::zero()
            })
            .cloned()
            .collect::<Vec<_>>();
        let playoff_teams = teams.iter().take(8).cloned().collect::<Vec<_>>();

        self.tournament().playoff().validate_parameters(
            &playoff_teams,
            &playoff_game_days,
            self.max_games_per_day(),
        )?;

        let playoff_schedule = self.tournament().playoff().compute_schedule(
            &playoff_teams,
            &playoff_game_days,
            self.game_times(),
            self.number_fields(),
            false,
        )?;

        Ok([&group_stage_schedule[..], &playoff_schedule[..]].concat())
    }
}

#[cfg(test)]
mod tests {
    use crate::impls::round_robin::RoundRobin;
    use crate::impls::single_elimination::SingleElimination;

    use super::*;
    use chrono::NaiveDate;
    use chrono::TimeZone;
    use chrono_tz::Europe::Zurich;

    #[test]
    fn test_correct_game_day() {
        let season = Season::new(
            Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap(),
            Zurich.with_ymd_and_hms(2026, 9, 22, 18, 0, 0).unwrap(),
            vec![GameTime::new(9, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
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
            vec![GameTime::new(9, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
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
            vec![GameTime::new(9, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let json = serde_json::to_string(&season).expect("serialization should succeed");
        let deserialized: Season<RoundRobin, SingleElimination> =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(season.start_day(), deserialized.start_day());
        assert_eq!(season.end_day(), deserialized.end_day());
        assert_eq!(season.tournament(), deserialized.tournament());
    }

    #[test]
    fn test_get_all_game_days() {
        let season = Season::new(
            Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap(),
            Zurich.with_ymd_and_hms(2026, 7, 1, 18, 0, 0).unwrap(),
            vec![GameTime::new(9, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let expected_days = [
            NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 23).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 6).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 13).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 20).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(),
        ];

        let computed_days = season.get_all_game_days();

        assert_eq!(expected_days.len(), computed_days.len());

        for i in 0..expected_days.len() {
            assert_eq!(expected_days[i], computed_days[i]);
        }
    }

    #[test]
    fn test_empty_game_days() {
        let season = Season::new(
            Zurich.with_ymd_and_hms(2026, 5, 4, 8, 45, 0).unwrap(),
            Zurich.with_ymd_and_hms(2026, 5, 8, 18, 0, 0).unwrap(),
            vec![GameTime::new(9, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let computed_days = season.get_all_game_days();

        assert!(
            computed_days.is_empty(),
            "expected game days to be an empty list"
        );
    }
}
