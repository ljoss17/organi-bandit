use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::traits::tournament::Tournament;
use crate::types::configurations::date::DateConfiguration;
use crate::types::configurations::time::TimeConfiguration;
use crate::types::game::Game;
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
    #[serde(flatten)]
    time_configuration: TimeConfiguration,
    #[serde(flatten)]
    date_configuration: DateConfiguration,
    number_fields: u32,
}

impl SeasonConfig {
    pub fn new(
        time_configuration: TimeConfiguration,
        date_configuration: DateConfiguration,
        number_fields: u32,
    ) -> Self {
        Self {
            time_configuration,
            date_configuration,
            number_fields,
        }
    }

    pub fn time_configuration(&self) -> &TimeConfiguration {
        &self.time_configuration
    }

    pub fn date_configuration(&self) -> &DateConfiguration {
        &self.date_configuration
    }

    pub fn number_fields(&self) -> u32 {
        self.number_fields
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
            self.season_config().date_configuration().start_date(),
            self.season_config(),
            true,
        )?;
        let last_group_stage_day = group_stage_schedule
            .iter()
            .max_by_key(|game| game.get_game_day())
            .ok_or(AppError::MissingGame)?
            .get_game_day()
            .date_naive();

        let mut game_day_scheduler = GameDayScheduler::new(
            &last_group_stage_day,
            self.season_config().date_configuration().game_days(),
        )?;
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
    use chrono::{NaiveDate, Weekday};

    use crate::impls::round_robin::RoundRobin;
    use crate::impls::single_elimination::SingleElimination;
    use crate::types::game_time::GameTime;

    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let time_configuration = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(0, 45).unwrap(),
            GameTime::new(0, 15).unwrap(),
        );
        let date_configuration = DateConfiguration::new(
            NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            vec![Weekday::Sat],
        );
        let season_config = SeasonConfig::new(time_configuration, date_configuration, 2);
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

    #[test]
    fn deserializes_the_season_config_payload_sent_by_the_frontend() {
        let payload = r#"{
            "startDate": "2026-05-13",
            "startTime": {"hour": 9, "minute": 0},
            "startBreak": {"hour": 12, "minute": 0},
            "endBreak": {"hour": 13, "minute": 0},
            "gameDuration": {"hour": 1, "minute": 0},
            "timeBetweenGames": {"hour": 0, "minute": 30},
            "numberFields": 2,
            "gameDays": ["Sat"]
        }"#;

        let season_config: SeasonConfig =
            serde_json::from_str(payload).expect("frontend payload should deserialize");

        let time_configuration = season_config.time_configuration();

        assert_eq!(
            time_configuration.game_duration(),
            &GameTime::new(1, 0).unwrap()
        );
        assert_eq!(
            time_configuration.time_between_games(),
            &GameTime::new(0, 30).unwrap()
        );
        // A slot spans the game itself plus the gap after it.
        assert_eq!(
            time_configuration.interval_between_games(),
            GameTime::new(1, 30).unwrap()
        );
    }
}
