use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::traits::tournament::Tournament;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
use crate::types::team::Team;
use crate::types::tournament_selection::TournamentSelection;
use crate::utils::game_day_scheduler::GameDayScheduler;

#[derive(Debug, Serialize, Deserialize)]
pub struct Season<G: Tournament, P: Tournament> {
    season_config: SeasonConfig,
    tournament: TournamentSelection<G, P>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonConfig {
    start_date: NaiveDate,
    start_time: GameTime,
    start_break: GameTime,
    end_break: GameTime,
    time_between_games: GameTime,
    number_fields: u32,
    game_days: Vec<Weekday>,
}

impl SeasonConfig {
    pub fn new(
        start_date: NaiveDate,
        start_time: GameTime,
        start_break: GameTime,
        end_break: GameTime,
        time_between_games: GameTime,
        number_fields: u32,
        game_days: Vec<Weekday>,
    ) -> Self {
        Self {
            start_date,
            start_time,
            start_break,
            end_break,
            time_between_games,
            number_fields,
            game_days,
        }
    }

    pub fn start_date(&self) -> &NaiveDate {
        &self.start_date
    }

    pub fn start_time(&self) -> &GameTime {
        &self.start_time
    }

    pub fn time_between_games(&self) -> &GameTime {
        &self.time_between_games
    }

    pub fn start_break(&self) -> &GameTime {
        &self.start_break
    }

    pub fn end_break(&self) -> &GameTime {
        &self.end_break
    }

    pub fn number_fields(&self) -> u32 {
        self.number_fields
    }

    pub fn game_days(&self) -> &[Weekday] {
        &self.game_days
    }
}

impl<G, P> Season<G, P>
where
    G: Tournament,
    P: Tournament,
{
    pub fn new(season_config: SeasonConfig, tournament: TournamentSelection<G, P>) -> Self {
        Self {
            season_config,
            tournament,
        }
    }

    pub fn season_config(&self) -> &SeasonConfig {
        &self.season_config
    }

    pub fn tournament(&self) -> &TournamentSelection<G, P> {
        &self.tournament
    }

    pub fn compute_season_schedule(&self, teams: &[Team]) -> Result<Vec<Game>, AppError> {
        let group_stage_schedule = self.tournament().group_stage().compute_schedule(
            teams,
            self.season_config().start_date(),
            self.season_config(),
            true,
        )?;
        let last_group_stage_day = group_stage_schedule
            .iter()
            .max_by_key(|game| game.get_game_day())
            .ok_or(AppError::MissingGame)?
            .get_game_day()
            .date_naive();

        let mut game_day_scheduler =
            GameDayScheduler::new(&last_group_stage_day, self.season_config().game_days());
        game_day_scheduler.advance();

        // Note: Currently playoffs are fixed to quarter finales -> finals
        let playoff_teams = teams.iter().take(8).cloned().collect::<Vec<_>>();

        // Referees are not automatically set since this will depend on the group stage results
        let playoff_schedule = self.tournament().playoff().compute_schedule(
            &playoff_teams,
            game_day_scheduler.current_day(),
            self.season_config(),
            false,
        )?;

        let mut full_schedule = group_stage_schedule;
        full_schedule.extend(playoff_schedule);
        Ok(full_schedule)
    }
}

#[cfg(test)]
mod tests {
    use crate::impls::round_robin::RoundRobin;
    use crate::impls::single_elimination::SingleElimination;

    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let season_config = SeasonConfig::new(
            NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );
        let season = Season::new(
            season_config,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
        );

        let json = serde_json::to_string(&season).expect("serialization should succeed");
        let deserialized: Season<RoundRobin, SingleElimination> =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(season.season_config(), deserialized.season_config());
        assert_eq!(season.tournament(), deserialized.tournament());
    }
}
