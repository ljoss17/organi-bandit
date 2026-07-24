use chrono::{Datelike, NaiveDate};
use rand::seq::SliceRandom;

use crate::traits::tournament::Tournament;
use crate::types::game::Game;
use crate::types::team::Team;

pub struct RoundRobin;

impl Tournament for RoundRobin {
    fn validate_parameters(
        teams: &[Team],
        game_days: Vec<NaiveDate>,
        max_games_per_day: usize,
    ) -> bool {
        let mut inner_teams = teams.to_vec();
        if !inner_teams.len().is_multiple_of(2) {
            inner_teams.push("Bye".to_owned());
        }
        let number_teams = inner_teams.len();
        let total_number_games = (number_teams * (number_teams - 1)) / 2;

        total_number_games <= max_games_per_day * game_days.len()
    }

    fn compute_schedule(
        teams: &[Team],
        game_days: Vec<NaiveDate>,
        max_games_per_day: usize,
    ) -> Vec<Game> {
        let mut rng = rand::rng();
        let mut inner_teams = teams.to_vec();
        if !inner_teams.len().is_multiple_of(2) {
            inner_teams.push("Bye".to_owned());
        }
        inner_teams.shuffle(&mut rng);
        let number_teams = inner_teams.len();

        let mut schedule = vec![];

        let mut remaining_games_per_day = max_games_per_day;
        let mut game_day = game_days[0];
        let mut day_index = 1;
        for _ in 0..number_teams - 1 {
            for i in 0..(number_teams / 2) {
                let home_team = inner_teams[i].clone();
                let away_team = inner_teams[number_teams - 1 - i].clone();
                let game = Game::new(
                    home_team.clone(),
                    away_team.clone(),
                    game_day.day(),
                    game_day.month(),
                    game_day.year(),
                    0,
                    0,
                );
                schedule.push(game);
                if home_team.as_str() != "Bye" && away_team.as_str() != "Bye" {
                    remaining_games_per_day -= 1;
                    if remaining_games_per_day == 0 {
                        remaining_games_per_day = max_games_per_day;
                        game_day = game_days[day_index];
                        day_index += 1;
                    }
                }
            }
            inner_teams = Self::rotate_teams(inner_teams);
        }

        schedule
    }
}

impl RoundRobin {
    fn rotate_teams(teams: Vec<Team>) -> Vec<Team> {
        let number_teams = teams.len();
        let mut resulting_teams = vec!["".to_owned(); number_teams];
        let mut i = number_teams - 1;
        while i >= 2 {
            resulting_teams[i] = teams[i - 1].clone();
            i -= 1;
        }
        resulting_teams[0] = teams[0].clone();
        resulting_teams[1] = teams[number_teams - 1].clone();
        resulting_teams
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use chrono::{DateTime, TimeZone, Weekday};
    use chrono_tz::Europe::Zurich;
    use chrono_tz::Tz;

    use crate::types::season::Season;
    use crate::types::tournament::{TournamentSelection, TournamentType};

    fn start_day() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap()
    }

    fn end_day_1() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 9, 1, 18, 0, 0).unwrap()
    }

    fn end_day_2() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 7, 13, 18, 0, 0).unwrap()
    }

    #[test]
    fn test_round_robin_parameter_validation_1() {
        let teams = [
            "Morges Bandits".to_owned(),
            "Yverdon Ducs".to_owned(),
            "Lausanne Rockets".to_owned(),
            "Team A".to_owned(),
            "Team B".to_owned(),
        ];
        let season = Season::new(
            start_day(),
            end_day_1(),
            TournamentSelection::new(
                TournamentType::RoundRobin,
                TournamentType::SingleElimination,
            ),
            vec![Weekday::Sat],
        );

        let result = RoundRobin::validate_parameters(&teams, season.get_all_game_days(), 1);

        assert!(result, "passed parameters are not valid");
    }

    #[test]
    fn test_round_robin_parameter_validation_2() {
        let teams = [
            "Morges Bandits".to_owned(),
            "Yverdon Ducs".to_owned(),
            "Lausanne Rockets".to_owned(),
            "Team A".to_owned(),
            "Team B".to_owned(),
        ];
        let season = Season::new(
            start_day(),
            end_day_2(),
            TournamentSelection::new(
                TournamentType::RoundRobin,
                TournamentType::SingleElimination,
            ),
            vec![Weekday::Sat],
        );

        let result = RoundRobin::validate_parameters(&teams, season.get_all_game_days(), 2);

        assert!(result, "passed parameters are not valid");
    }

    #[test]
    fn test_round_robin_schedule_1() {
        let teams = [
            "Morges Bandits".to_owned(),
            "Yverdon Ducs".to_owned(),
            "Lausanne Rockets".to_owned(),
            "Team A".to_owned(),
            "Team B".to_owned(),
        ];
        // Total number of games is computed using:
        // N * (N - 1) / 2
        // But since the number of teams is odd a bye team is added for bye weeks so we compute:
        // (N + 1) * N / 2
        let expecte_total_number_games = (teams.len() * (teams.len() + 1)) / 2;
        let season = Season::new(
            start_day(),
            end_day_1(),
            TournamentSelection::new(
                TournamentType::RoundRobin,
                TournamentType::SingleElimination,
            ),
            vec![Weekday::Sat],
        );

        let result = RoundRobin::compute_schedule(&teams, season.get_all_game_days(), 1);

        assert_eq!(result.len(), expecte_total_number_games);

        assert_schedule(&result, &teams, season.get_all_game_days(), 1);
    }

    #[test]
    fn test_round_robin_schedule_2() {
        let teams = [
            "Morges Bandits".to_owned(),
            "Yverdon Ducs".to_owned(),
            "Lausanne Rockets".to_owned(),
            "Team A".to_owned(),
            "Team B".to_owned(),
        ];
        // Total number of games is computed using:
        // N * (N - 1) / 2
        // But since the number of teams is odd a bye team is added for bye weeks so we compute:
        // (N + 1) * N / 2
        let expecte_total_number_games = (teams.len() * (teams.len() + 1)) / 2;
        let season = Season::new(
            start_day(),
            end_day_2(),
            TournamentSelection::new(
                TournamentType::RoundRobin,
                TournamentType::SingleElimination,
            ),
            vec![Weekday::Sat],
        );

        let result = RoundRobin::compute_schedule(&teams, season.get_all_game_days(), 2);

        assert_eq!(result.len(), expecte_total_number_games);

        assert_schedule(&result, &teams, season.get_all_game_days(), 2);
    }

    fn assert_schedule(
        schedule: &[Game],
        teams: &[Team],
        game_days: Vec<NaiveDate>,
        max_games_per_day: usize,
    ) {
        let all_unique_teams = HashSet::<String>::from_iter(teams.iter().cloned());
        assert_eq!(teams.len(), all_unique_teams.len());
        let only_unique_teams = all_unique_teams
            .iter()
            .filter(|&team| team != "Bye")
            .collect::<Vec<_>>();

        let mut bye_weeks = HashMap::new();
        let mut computed_game_days = HashMap::new();

        for game in schedule.iter() {
            let home_team = game.get_home_team();
            let away_team = game.get_away_team();
            let game_day = game.get_game_day();
            assert!(home_team != "Bye" || away_team != "Bye");

            // Record only non bye week games
            if home_team != "Bye" && away_team != "Bye" {
                match computed_game_days.get(game_day) {
                    Some(count) => {
                        computed_game_days.insert(game_day, count + 1);
                    }
                    None => {
                        computed_game_days.insert(game_day, 1);
                    }
                }
            }

            if home_team == "Bye" {
                match bye_weeks.get(away_team) {
                    Some(count) => bye_weeks.insert(away_team, count + 1),
                    None => bye_weeks.insert(away_team, 1),
                };
            }

            if away_team == "Bye" {
                match bye_weeks.get(home_team) {
                    Some(count) => bye_weeks.insert(home_team, count + 1),
                    None => bye_weeks.insert(home_team, 1),
                };
            }
        }

        assert_eq!(bye_weeks.keys().len(), only_unique_teams.len());
        assert!(
            bye_weeks.values().all(|&value| value == 1),
            "All teams should only have 1 bye week"
        );
        assert!(
            computed_game_days
                .keys()
                .all(|day| game_days.contains(&day.date_naive())),
            "Scheduled games should all be in passed game days"
        );
        assert!(
            computed_game_days
                .values()
                .all(|&value| value <= max_games_per_day),
            "All game days should have a maximum of '{max_games_per_day}' games"
        );
    }
}
