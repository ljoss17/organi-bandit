use std::collections::HashMap;

use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::traits::tournament::Tournament;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
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
        teams: &[Team],
        game_days: &[NaiveDate],
        max_games_per_day: usize,
    ) -> Result<(), AppError> {
        self.validate_tournament_duration(teams, game_days, max_games_per_day)
    }

    fn compute_schedule(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        game_times: &[GameTime],
        number_fields: usize,
        with_referees: bool,
    ) -> Result<Vec<Game>, AppError> {
        // Validate parameters
        self.validate_tournament_duration(teams, game_days, number_fields * game_times.len())?;

        let mut rng = rand::rng();
        let mut inner_teams = teams.to_vec();
        if !inner_teams.len().is_multiple_of(2) {
            inner_teams.push(Team::new("Bye", None));
        }
        inner_teams.shuffle(&mut rng);
        let number_teams = inner_teams.len();

        let mut schedule = vec![];

        let mut game_day_scheduler =
            GameDayScheduler::new(game_days, game_times.len() * number_fields);
        let mut game_time_scheduler = GameTimeScheduler::new(game_times);

        for _ in 0..number_teams - 1 {
            let round_day = *game_day_scheduler.current_day();
            for i in 0..(number_teams / 2) {
                let game_time = *game_time_scheduler.current_time();
                let home_team = inner_teams[i].clone();
                let away_team = inner_teams[number_teams - 1 - i].clone();
                let is_bye = home_team.get_name() == "Bye" || away_team.get_name() == "Bye";
                let game_day = if is_bye {
                    round_day
                } else {
                    *game_day_scheduler.current_day()
                };
                let game = Game::new_with_game_day(home_team, away_team, game_day, game_time, None);
                schedule.push(game);
                if !is_bye {
                    game_day_scheduler.try_advance();
                    game_time_scheduler.advance();
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

    fn validate_tournament_duration(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        max_games_per_day: usize,
    ) -> Result<(), AppError> {
        if teams.is_empty() {
            return Err(AppError::NotEnoughTeams(0, 2));
        }
        let number_teams = teams.len() + teams.len() % 2;
        let total_number_games = (number_teams * (number_teams - 1)) / 2;

        if total_number_games > max_games_per_day * game_days.len() {
            return Err(AppError::TournamentTooShort(
                self.name(),
                max_games_per_day * game_days.len(),
                total_number_games,
            ));
        }

        Ok(())
    }

    fn add_referees(&self, schedule: Vec<Game>, teams: &[Team]) -> Result<Vec<Game>, AppError> {
        let mut schedule_with_referee = vec![];
        let mut referee_count = HashMap::new();
        for team in teams.iter() {
            referee_count.insert(team, 0);
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
            let eligible_teams = teams
                .iter()
                .filter(|team| !busy_teams.contains(team))
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
    use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Weekday};
    use chrono_tz::Europe::Zurich;
    use chrono_tz::Tz;

    use crate::impls::single_elimination::SingleElimination;
    use crate::types::game_time::GameTime;
    use crate::types::season::Season;
    use crate::types::tournament_selection::TournamentSelection;

    fn start_day() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap()
    }

    fn end_day_1() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 9, 1, 18, 0, 0).unwrap()
    }

    fn end_day_2() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 7, 13, 18, 0, 0).unwrap()
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
        let season = Season::new(
            start_day(),
            end_day_1(),
            vec![GameTime::new(9, 0).unwrap()],
            1,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let result = round_robin.validate_parameters(
            &teams(),
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert!(result.is_ok(), "passed parameters are not valid");
    }

    #[test]
    fn test_round_robin_parameter_validation_2() {
        let season = Season::new(
            start_day(),
            end_day_2(),
            vec![GameTime::new(9, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let result = round_robin.validate_parameters(
            &teams(),
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert!(result.is_ok(), "passed parameters are not valid");
    }

    #[test]
    fn test_validate_real_scenario_parameters() {
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
        let season = Season::new(
            start_day(),
            end_day_1(),
            vec![GameTime::new(9, 0).unwrap(), GameTime::new(14, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let result = round_robin.validate_parameters(
            &teams,
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert!(result.is_ok(), "passed parameters are not valid");
    }

    #[test]
    fn test_round_robin_schedule() {
        // Total number of games is computed using:
        // N * (N - 1) / 2
        // But since the number of teams is odd a bye team is added for bye weeks so we compute:
        // (N + 1) * N / 2
        let expecte_total_number_games = (teams().len() * (teams().len() + 1)) / 2;
        let season = Season::new(
            start_day(),
            end_day_1(),
            vec![GameTime::new(9, 0).unwrap()],
            1,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let maybe_schedule = round_robin.compute_schedule(
            &teams(),
            &season.get_all_game_days(),
            season.game_times(),
            season.number_fields(),
            true,
        );

        assert!(maybe_schedule.is_ok(), "{maybe_schedule:#?}");

        let schedule = maybe_schedule.unwrap();

        assert_eq!(schedule.len(), expecte_total_number_games);

        assert_schedule(
            &schedule,
            &teams(),
            season.get_all_game_days(),
            season.game_times(),
            season.number_fields(),
        );
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
        let season = Season::new(
            start_day(),
            end_day_1(),
            vec![GameTime::new(9, 0).unwrap(), GameTime::new(14, 0).unwrap()],
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let round_robin = RoundRobin;

        let maybe_schedule = round_robin.compute_schedule(
            &teams,
            &season.get_all_game_days(),
            &[GameTime::new(9, 0).unwrap(), GameTime::new(14, 0).unwrap()],
            2,
            true,
        );

        assert!(maybe_schedule.is_ok());

        let schedule = maybe_schedule.unwrap();

        //assert_eq!(schedule.len(), expecte_total_number_games);

        assert_schedule(
            &schedule,
            &teams,
            season.get_all_game_days(),
            season.game_times(),
            season.number_fields(),
        );
    }

    fn assert_schedule(
        schedule: &[Game],
        teams: &[Team],
        game_days: Vec<NaiveDate>,
        game_times: &[GameTime],
        number_fields: usize,
    ) {
        let mut inner_game_days = vec![];
        for game_time in game_times.iter() {
            for day in game_days.iter() {
                let game_day = Zurich
                    .with_ymd_and_hms(
                        day.year(),
                        day.month(),
                        day.day(),
                        game_time.hour().into(),
                        game_time.minute().into(),
                        0,
                    )
                    .single()
                    .expect("Game day should be an exact date and time");
                inner_game_days.push(game_day);
            }
        }
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
                .keys()
                .all(|day| inner_game_days.contains(day)),
            "Scheduled games should all be in passed game days"
        );
        assert!(
            computed_game_days
                .values()
                .all(|&value| value <= number_fields),
            "All game days should have a maximum of '{}' games per date/time",
            number_fields
        );
    }
}
