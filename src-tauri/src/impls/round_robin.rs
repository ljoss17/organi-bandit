use std::collections::HashMap;

use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;
use rand::seq::SliceRandom;
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
pub struct RoundRobin;

impl Tournament for RoundRobin {
    fn name(&self) -> String {
        "Round Robin".to_owned()
    }

    fn validate_parameters(
        &self,
        _teams: &[Team],
        _start_date: &NaiveDate,
        _start_time: &GameTime,
        _time_between_games: &GameTime,
    ) -> Result<(), AppError> {
        //self.validate_tournament_duration(teams, game_days, max_games_per_day)
        Ok(())
    }

    fn compute_schedule(
        &self,
        teams: &[Team],
        start_date: &NaiveDate,
        season_config: &SeasonConfig,
        with_referees: bool,
    ) -> Result<Vec<Game>, AppError> {
        // Validate parameters
        self.validate_parameters(
            teams,
            start_date,
            season_config.start_time(),
            season_config.time_between_games(),
        )?;

        let mut rng = rand::rng();
        let mut inner_teams = teams.to_vec();
        if !inner_teams.len().is_multiple_of(2) {
            inner_teams.push(Team::new("Bye", None));
        }
        inner_teams.shuffle(&mut rng);
        let number_teams = inner_teams.len();

        let mut schedule = vec![];

        let mut game_day_scheduler = GameDayScheduler::new(start_date, season_config.game_days())?;
        let mut game_time_scheduler = GameTimeScheduler::new(
            season_config.start_time(),
            season_config.time_between_games(),
            season_config.number_fields(),
            season_config.start_break(),
            season_config.end_break(),
        );

        for _ in 0..number_teams - 1 {
            let game_day = *game_day_scheduler.current_day();
            game_day_scheduler.advance();
            game_time_scheduler.reset();

            let mut bye_pair = None;

            for i in 0..(number_teams / 2) {
                let game_time = *game_time_scheduler.current_time();
                let home_team = inner_teams[i].clone();
                let away_team = inner_teams[number_teams - 1 - i].clone();
                let is_bye = home_team.get_name() == "Bye" || away_team.get_name() == "Bye";
                if is_bye {
                    bye_pair = Some((home_team.clone(), away_team.clone()));
                }
                let game = Game::new_with_game_day(home_team, away_team, game_day, game_time, None);
                schedule.push(game);
                if !is_bye {
                    game_time_scheduler.try_advance();
                }
            }

            let mut leg2_teams = inner_teams
                .iter()
                .filter(|team| match &bye_pair {
                    Some((a, b)) => *team != a && *team != b,
                    None => true,
                })
                .cloned()
                .collect::<Vec<_>>();

            if leg2_teams.len() >= 2 {
                Self::rotate_teams(&mut leg2_teams);
                let leg2_count = leg2_teams.len();
                for i in 0..(leg2_count / 2) {
                    let game_time = *game_time_scheduler.current_time();
                    let home_team = leg2_teams[i].clone();
                    let away_team = leg2_teams[leg2_count - 1 - i].clone();
                    let game =
                        Game::new_with_game_day(home_team, away_team, game_day, game_time, None);
                    schedule.push(game);
                    game_time_scheduler.try_advance();
                }
            }

            Self::rotate_teams(&mut inner_teams);
        }

        if with_referees {
            return self.add_referees(schedule, teams);
        }

        Ok(schedule)
    }
}

impl RoundRobin {
    fn rotate_teams(teams: &mut [Team]) {
        teams[1..].rotate_right(1);
    }

    fn add_referees(&self, schedule: Vec<Game>, teams: &[Team]) -> Result<Vec<Game>, AppError> {
        let mut schedule_with_referee = vec![];
        let mut referee_count = HashMap::new();
        for team in teams.iter() {
            referee_count.insert(team, 0);
        }
        let mut bye_team_by_day: HashMap<NaiveDate, &Team> = HashMap::new();
        for game in schedule.iter() {
            let day = game.get_game_day().date_naive();
            if game.get_home_team().get_name() == "Bye" {
                bye_team_by_day.insert(day, game.get_away_team());
            } else if game.get_away_team().get_name() == "Bye" {
                bye_team_by_day.insert(day, game.get_home_team());
            }
        }
        let mut busy_teams_set: HashMap<&DateTime<Tz>, Vec<&Team>> = HashMap::new();
        for game in schedule.iter() {
            let game_day = game.get_game_day();
            // Teams are only busy for specific game time, not entire day
            busy_teams_set
                .entry(game_day)
                .or_default()
                .extend([game.get_home_team(), game.get_away_team()]);
        }

        for game in schedule.iter() {
            if game.get_home_team().get_name() == "Bye" || game.get_away_team().get_name() == "Bye"
            {
                schedule_with_referee.push(game.clone());
                continue;
            }
            let busy_teams = busy_teams_set.get(game.get_game_day()).unwrap();
            let day = game.get_game_day().date_naive();
            let eligible_teams = teams
                .iter()
                .filter(|team| {
                    !busy_teams.contains(team) && bye_team_by_day.get(&day) != Some(team)
                })
                .collect::<Vec<_>>();

            if eligible_teams.is_empty() {
                return Err(AppError::EmptyEligibleReferees);
            }
            let referee = *eligible_teams
                .iter()
                .min_by_key(|team| referee_count[*team])
                .unwrap();

            *referee_count.entry(referee).or_insert(0) += 1;

            let game = Game::new_with_game_day(
                game.get_home_team().clone(),
                game.get_away_team().clone(),
                game.get_game_day().date_naive(),
                game.get_game_time()?,
                Some(referee.clone()),
            );
            schedule_with_referee.push(game);
        }

        Ok(schedule_with_referee)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use chrono::{NaiveDate, Weekday};

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

    #[test]
    fn test_round_robin_parameter_validation_1() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let result = round_robin.validate_parameters(
            &teams(),
            season_config.start_date(),
            season_config.start_time(),
            season_config.time_between_games(),
        );

        assert!(result.is_ok(), "passed parameters are not valid");
    }

    #[test]
    fn test_round_robin_parameter_validation_2() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let result = round_robin.validate_parameters(
            &teams(),
            season_config.start_date(),
            season_config.start_time(),
            season_config.time_between_games(),
        );

        assert!(result.is_ok(), "passed parameters are not valid");
    }

    #[test]
    fn test_round_robin_schedule() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let maybe_schedule = round_robin.compute_schedule(
            &teams(),
            season_config.start_date(),
            &season_config,
            true,
        );

        assert!(maybe_schedule.is_ok(), "{maybe_schedule:#?}");

        let schedule = maybe_schedule.unwrap();

        // Total number of real games is computed using:
        // N * (N - 1)
        // Plus 1 bye game per teams which results in:
        // N * N
        let expecte_total_number_games = teams().len() * teams().len();

        assert_eq!(schedule.len(), expecte_total_number_games);

        assert_schedule(&schedule, &teams(), season_config.number_fields());
    }

    #[test]
    fn test_real_scenario() {
        let teams = [
            Team::new("Riviera Saints", None),
            Team::new("Fribourg Cardinals", None),
            Team::new("Suzerains", None),
            Team::new("Geneva Whoppers", None),
            Team::new("Lausanne Owls", None),
            Team::new("Monthey Rhinos", None),
            Team::new("Morges Bandits", None),
            Team::new("Yverdon Ducs", None),
            Team::new("Lausanne Rockets", None),
        ];
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(14, 0).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let maybe_schedule =
            round_robin.compute_schedule(&teams, season_config.start_date(), &season_config, true);

        assert!(maybe_schedule.is_ok());

        let schedule = maybe_schedule.unwrap();

        // Total number of games is computed using:
        // N * (N - 1)
        // Plus 1 bye game per teams which results in:
        // N * N
        let expecte_total_number_games = teams.len() * teams.len();

        assert_eq!(schedule.len(), expecte_total_number_games);

        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    fn assert_schedule(schedule: &[Game], teams: &[Team], number_of_fields: u32) {
        let all_unique_teams = HashSet::<String>::from_iter(
            teams
                .iter()
                .map(|team| team.get_name().to_owned())
                .collect::<Vec<String>>(),
        );
        assert_eq!(teams.len(), all_unique_teams.len());
        let only_unique_teams = all_unique_teams
            .iter()
            .filter(|&team| team != "Bye")
            .collect::<Vec<_>>();

        let mut bye_weeks = HashMap::new();
        let mut computed_game_days = HashMap::new();
        let mut team_real_game_days: HashMap<&str, HashSet<NaiveDate>> = HashMap::new();
        let mut team_bye_days: HashMap<&str, NaiveDate> = HashMap::new();

        for game in schedule.iter() {
            let home_team = game.get_home_team();
            let away_team = game.get_away_team();
            let game_day = game.get_game_day();
            assert!(home_team.get_name() != "Bye" || away_team.get_name() != "Bye");

            // Record only non bye week games
            if home_team.get_name() != "Bye" && away_team.get_name() != "Bye" {
                match computed_game_days.get(game_day) {
                    Some(count) => {
                        computed_game_days.insert(game_day, count + 1);
                    }
                    None => {
                        computed_game_days.insert(game_day, 1);
                    }
                }
                team_real_game_days
                    .entry(home_team.get_name())
                    .or_default()
                    .insert(game_day.date_naive());
                team_real_game_days
                    .entry(away_team.get_name())
                    .or_default()
                    .insert(game_day.date_naive());
            }

            if home_team.get_name() == "Bye" {
                match bye_weeks.get(away_team.get_name()) {
                    Some(count) => bye_weeks.insert(away_team.get_name(), count + 1),
                    None => bye_weeks.insert(away_team.get_name(), 1),
                };
                team_bye_days.insert(away_team.get_name(), game_day.date_naive());
            }

            if away_team.get_name() == "Bye" {
                match bye_weeks.get(home_team.get_name()) {
                    Some(count) => bye_weeks.insert(home_team.get_name(), count + 1),
                    None => bye_weeks.insert(home_team.get_name(), 1),
                };
                team_bye_days.insert(home_team.get_name(), game_day.date_naive());
            }
        }

        assert_eq!(bye_weeks.keys().len(), only_unique_teams.len());
        assert!(
            bye_weeks.values().all(|&value| value == 1),
            "All teams should only have 1 bye week"
        );
        for (team, bye_day) in team_bye_days.iter() {
            assert!(
                !team_real_game_days
                    .get(team)
                    .is_some_and(|days| days.contains(bye_day)),
                "team {team} has a real game scheduled on its bye day {bye_day}"
            );
        }
        assert!(
            computed_game_days
                .values()
                .all(|&value| value <= number_of_fields),
            "All game days should have a maximum of '{}' games per date/time",
            number_of_fields
        );
    }
}
