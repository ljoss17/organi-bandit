use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::traits::tournament::Tournament;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
use crate::types::season::SeasonConfig;
use crate::types::team::Team;
use crate::utils::game_day_scheduler::GameDayScheduler;
use crate::utils::game_time_scheduler::GameTimeScheduler;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SingleElimination {
    // Define if schedule should contain team names or not
    anonymous: bool,
}

impl Tournament for SingleElimination {
    fn name(&self) -> String {
        "Single Elimination".to_owned()
    }

    fn validate_parameters(&self, teams: &[Team]) -> Result<(), AppError> {
        if teams.len() < 2 {
            return Err(AppError::NotEnoughTeams(teams.len(), 2));
        }
        Ok(())
    }

    // Note: Currently Single Elimination does not support referees setup
    fn compute_schedule(
        &self,
        teams: &[Team],
        start_date: &NaiveDate,
        season_config: &SeasonConfig,
        _with_referees: bool,
    ) -> Result<Vec<Game>, AppError> {
        self.validate_parameters(teams)?;
        let number_of_teams = teams.len();
        let bracket_size = number_of_teams.next_power_of_two();
        let number_of_byes = bracket_size - number_of_teams;
        let number_of_round = bracket_size.ilog(2);

        let mut inner_teams = if self.anonymous {
            (1..=number_of_teams)
                .map(|value| {
                    Team::new(
                        &format!("{value}"),
                        Some(number_of_teams as u32 - value as u32),
                    )
                })
                .collect::<Vec<_>>()
        } else {
            let mut inner = teams.to_vec();
            inner.sort_by_key(|team| team.get_seed());
            inner
        };
        inner_teams.reserve(number_of_byes);

        let mut schedule = Vec::with_capacity(bracket_size - 1);
        let mut game_day_scheduler = GameDayScheduler::new(start_date, season_config.game_days())?;
        let mut game_time_scheduler = GameTimeScheduler::new(
            season_config.start_time(),
            season_config.time_between_games(),
            season_config.number_fields(),
            season_config.start_break(),
            season_config.end_break(),
        );

        let bye_time = GameTime::new(0, 0)?;
        for i in 0..number_of_byes {
            let home_team = inner_teams[i].clone();
            let bye_team = Team::new("Bye", None);

            let game = Game::new_with_game_day(
                home_team,
                bye_team.clone(),
                *game_day_scheduler.current_day(),
                bye_time,
                None,
            )?;

            schedule.push(game);
            inner_teams.push(bye_team);
        }

        // Compute the first round of single elimination, giving higher seed teams a bye week
        for offset in 0..(number_of_teams - number_of_byes) / 2 {
            let home_team = inner_teams[number_of_byes + offset].clone();
            let away_team = inner_teams[number_of_teams - 1 - offset].clone();
            let game_time = *game_time_scheduler.current_time();
            let game = Game::new_with_game_day(
                home_team,
                away_team,
                *game_day_scheduler.current_day(),
                game_time,
                None,
            )?;
            schedule.push(game);

            game_time_scheduler.try_advance();
        }

        game_day_scheduler.advance();
        game_time_scheduler.reset();

        let mut second_round_schedule = Vec::with_capacity(bracket_size / 4);
        let second_round_byes = number_of_byes / 2;
        game_time_scheduler.reset();

        // Compute the second round of single elimination, taking into account first round bye weeks
        for offset in 0..second_round_byes {
            let home_team = inner_teams[second_round_byes + offset].clone();
            let away_team = inner_teams[number_of_teams - 3 - offset].clone();
            let game_time = *game_time_scheduler.current_time();
            let game = Game::new_with_game_day(
                home_team,
                away_team,
                *game_day_scheduler.current_day(),
                game_time,
                None,
            )?;
            second_round_schedule.push(game);

            game_time_scheduler.try_advance();
        }
        let remaining_second_round_slots = bracket_size / 4 - second_round_schedule.len();

        for team in inner_teams.iter().take(remaining_second_round_slots) {
            let home_team = team.clone();
            let away_team = Team::new("WinnerPrevious", None);
            let game_time = *game_time_scheduler.current_time();
            let game = Game::new_with_game_day(
                home_team,
                away_team,
                *game_day_scheduler.current_day(),
                game_time,
                None,
            )?;
            second_round_schedule.push(game);

            game_time_scheduler.try_advance();
        }

        schedule.append(&mut second_round_schedule);

        game_day_scheduler.advance();
        game_time_scheduler.reset();

        // Compute all remaining rounds as there is no side effect from bye weeks
        for round in 3..=number_of_round {
            let number_of_games = bracket_size / 2usize.pow(round);
            for _ in 0..number_of_games {
                let game_time = *game_time_scheduler.current_time();
                let home_team = Team::new("WinnerA", None);
                let away_team = Team::new("WinnerB", None);
                let game = Game::new_with_game_day(
                    home_team,
                    away_team,
                    *game_day_scheduler.current_day(),
                    game_time,
                    None,
                )?;
                schedule.push(game);

                game_time_scheduler.try_advance();
            }
        }

        Ok(schedule)
    }
}

impl SingleElimination {
    pub fn new(anonymous: bool) -> Self {
        Self { anonymous }
    }

    pub fn is_anonymous(&self) -> bool {
        self.anonymous
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use chrono::Weekday;

    use crate::types::game_time::GameTime;

    fn start_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()
    }

    fn teams() -> [Team; 5] {
        [
            Team::new("Morges Bandits", None),
            Team::new("Yverdon Ducs", None),
            Team::new("Lausanne Rockets", None),
            Team::new("Team A", None),
            Team::new("Team B", None),
        ]
    }

    fn teams_bigger() -> [Team; 9] {
        [
            Team::new("Morges Bandits", None),
            Team::new("Yverdon Ducs", None),
            Team::new("Lausanne Rockets", None),
            Team::new("Team A", None),
            Team::new("Team B", None),
            Team::new("Team C", None),
            Team::new("Team D", None),
            Team::new("Team E", None),
            Team::new("Team F", None),
        ]
    }

    #[test]
    fn test_single_elimination_valid_parameter_validation_1() {
        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(&teams());

        assert!(result.is_ok(), "passed parameters should be valid");
    }

    #[test]
    fn test_single_elimination_parameter_validation_rejects_too_few_teams() {
        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(&[Team::new("Solo Team", None)]);

        assert!(matches!(result, Err(AppError::NotEnoughTeams(1, 2))));
    }

    #[test]
    fn compute_schedule_rejects_empty_teams() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let result = SingleElimination::new(false).compute_schedule(
            &[],
            &start_date(),
            &season_config,
            false,
        );

        assert!(matches!(result, Err(AppError::NotEnoughTeams(0, 2))));
    }

    #[test]
    fn test_single_elimination_valid_parameter_validation_2() {
        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(&teams());

        assert!(result.is_ok(), "passed parameters should be valid");
    }

    #[test]
    fn test_single_elimination_schedule_3_rounds() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(false);

        let maybe_schedule =
            single_elimination.compute_schedule(&teams(), &start_date(), &season_config, true);

        assert!(maybe_schedule.is_ok());

        let schedule = maybe_schedule.unwrap();

        assert_schedule(&schedule, &teams(), season_config.number_fields())
    }

    #[test]
    fn test_single_elimination_schedule_4_rounds() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(false);

        let maybe_schedule = single_elimination.compute_schedule(
            &teams_bigger(),
            &start_date(),
            &season_config,
            true,
        );

        assert!(maybe_schedule.is_ok());

        let schedule = maybe_schedule.unwrap();

        assert_schedule(&schedule, &teams_bigger(), season_config.number_fields())
    }

    fn assert_schedule(schedule: &[Game], teams: &[Team], number_of_fields: u32) {
        assert_eq!(teams.len().next_power_of_two() - 1, schedule.len());
        let number_of_byes = teams.len().next_power_of_two() - teams.len();
        for game in schedule.iter().take(number_of_byes) {
            assert!(
                game.get_home_team().get_name() == "Bye"
                    || game.get_away_team().get_name() == "Bye",
                "should be a bye week"
            );
        }
        let mut games = teams.len().next_power_of_two() / 2;
        let mut rounds = Vec::new();
        while games >= 1 {
            rounds.push(games);
            games /= 2;
        }
        let mut game_counter = number_of_fields;
        let mut games_per_round = 0;
        let mut max_games_per_round = rounds[0];
        let mut round = 1;

        let mut games_per_time = HashMap::new();

        for game in schedule.iter() {
            assert_ne!(
                game.get_home_team(),
                game.get_away_team(),
                "home and away team should be different"
            );
            if game.get_home_team().get_name() != "Bye" && game.get_away_team().get_name() != "Bye"
            {
                let game_time = game.get_game_time().unwrap();
                let game_date = game.get_game_day();
                let date_identifier = (game_time, *game_date);
                let value = games_per_time.entry(date_identifier).or_insert(0);
                *value += 1;
            }

            games_per_round += 1;
            if game.get_home_team().get_name() != "Bye" && game.get_away_team().get_name() != "Bye"
            {
                game_counter -= 1;
            }
            if games_per_round == max_games_per_round && max_games_per_round != 1 {
                game_counter = number_of_fields;
                max_games_per_round = rounds[round];
                round += 1;
                games_per_round = 0;
            } else if game_counter == 0 {
                game_counter = number_of_fields;
            }
        }

        assert!(
            games_per_time
                .values()
                .all(|value| *value <= number_of_fields),
            "too many games for a specific time"
        );
    }
}
