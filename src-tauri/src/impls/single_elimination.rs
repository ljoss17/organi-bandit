use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::traits::tournament::Tournament;
use crate::types::game::Game;
use crate::types::team::Team;
use crate::utils::game_day_scheduler::GameDayScheduler;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SingleElimination {
    // Define if schedule should contain team names or not
    anonymous: bool,
}

impl Tournament for SingleElimination {
    fn validate_parameters(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        max_games_per_day: usize,
    ) -> bool {
        if teams.len() < 2 {
            // 1 team is considered as invalid
            return false;
        }
        let total_number_games = teams.len() - 1;
        (game_days.len() * max_games_per_day) >= total_number_games
    }

    fn compute_schedule(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        max_games_per_day: usize,
    ) -> Vec<Game> {
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

        let mut schedule = vec![];
        let mut game_day_scheduler = GameDayScheduler::new(game_days, max_games_per_day);

        for i in 0..number_of_byes {
            let home_team = inner_teams[i].clone();
            let bye_team = Team::new("Bye", None);

            let game = Game::new_with_game_day(
                home_team,
                bye_team.clone(),
                *game_day_scheduler.current_day(),
            );

            schedule.push(game);
            inner_teams.push(bye_team);
        }

        // Compute the first round of single elimination, giving higher seed teams a bye week
        for offset in 0..(number_of_teams - number_of_byes) / 2 {
            let home_team = inner_teams[number_of_byes + offset].clone();
            let away_team = inner_teams[number_of_teams - 1 - offset].clone();
            let game =
                Game::new_with_game_day(home_team, away_team, *game_day_scheduler.current_day());
            schedule.push(game);

            game_day_scheduler.try_advance();
        }

        game_day_scheduler.try_force_advance();

        let mut second_round_schedule = vec![];
        let second_round_byes = number_of_byes / 2;
        // Compute the second round of single elimination, taking into account first round bye weeks
        for offset in 0..second_round_byes {
            let home_team = inner_teams[second_round_byes + offset].clone();
            let away_team = inner_teams[number_of_teams - 3 - offset].clone();
            let game =
                Game::new_with_game_day(home_team, away_team, *game_day_scheduler.current_day());
            second_round_schedule.push(game);

            game_day_scheduler.try_advance();
        }
        let remaining_second_round_slots = bracket_size / 4 - second_round_schedule.len();

        for team in inner_teams.iter().take(remaining_second_round_slots) {
            let home_team = team.clone();
            let away_team = Team::new("WinnerPrevious", None);
            let game =
                Game::new_with_game_day(home_team, away_team, *game_day_scheduler.current_day());
            second_round_schedule.push(game);

            game_day_scheduler.try_advance();
        }

        schedule.append(&mut second_round_schedule);

        game_day_scheduler.try_force_advance();

        // Compute all remaining rounds as there is no side effect from bye weeks
        for round in 3..=number_of_round {
            let number_of_games = bracket_size / 2usize.pow(round);
            for _ in 0..number_of_games {
                let home_team = Team::new("WinnerA", None);
                let away_team = Team::new("WinnerB", None);
                let game = Game::new_with_game_day(
                    home_team,
                    away_team,
                    *game_day_scheduler.current_day(),
                );
                schedule.push(game);

                game_day_scheduler.try_advance();
            }
        }

        schedule
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
    use super::*;
    use chrono::{DateTime, TimeZone, Weekday};
    use chrono_tz::Europe::Zurich;
    use chrono_tz::Tz;

    use crate::impls::round_robin::RoundRobin;
    use crate::types::season::Season;
    use crate::types::tournament::TournamentSelection;

    fn start_day() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap()
    }

    fn end_day() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 6, 4, 18, 0, 0).unwrap()
    }

    fn end_day_later() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 8, 4, 18, 0, 0).unwrap()
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
        let season = Season::new(
            start_day(),
            end_day(),
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(
            &teams(),
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert!(result, "passed parameters should be valid");
    }

    #[test]
    fn test_single_elimination_valid_parameter_validation_2() {
        let season = Season::new(
            start_day(),
            end_day_later(),
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(
            &teams_bigger(),
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert!(result, "passed parameters should be valid");
    }

    #[test]
    fn test_single_elimination_invalid_parameter_validation_1() {
        let season = Season::new(
            start_day(),
            end_day(),
            1,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(
            &teams(),
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert!(!result, "passed parameters should not be valid");
    }

    #[test]
    fn test_single_elimination_schedule_3_rounds() {
        let season = Season::new(
            start_day(),
            end_day(),
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(false);

        let schedule = single_elimination.compute_schedule(
            &teams(),
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert_schedule(&schedule, &teams(), season.get_all_game_days(), 2)
    }

    #[test]
    fn test_single_elimination_schedule_4_rounds() {
        let season = Season::new(
            start_day(),
            end_day_later(),
            2,
            TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(false);

        let schedule = single_elimination.compute_schedule(
            &teams_bigger(),
            &season.get_all_game_days(),
            season.max_games_per_day(),
        );

        assert_schedule(
            &schedule,
            &teams_bigger(),
            season.get_all_game_days(),
            season.max_games_per_day(),
        )
    }

    fn assert_schedule(
        schedule: &[Game],
        teams: &[Team],
        game_days: Vec<NaiveDate>,
        max_games_per_day: usize,
    ) {
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
        let mut game_day_index = 0;
        let mut current_game_day = game_days[game_day_index];
        let mut game_counter = max_games_per_day;
        let mut games_per_round = 0;
        let mut max_games_per_round = rounds[0];
        let mut round = 1;
        for game in schedule.iter() {
            assert_eq!(game.get_game_day().date_naive(), current_game_day);
            games_per_round += 1;
            if game.get_home_team().get_name() != "Bye" && game.get_away_team().get_name() != "Bye"
            {
                game_counter -= 1;
            }
            if games_per_round == max_games_per_round && max_games_per_round != 1 {
                game_day_index += 1;
                current_game_day = game_days[game_day_index];
                game_counter = max_games_per_day;
                max_games_per_round = rounds[round];
                round += 1;
                games_per_round = 0;
            } else if game_counter == 0 {
                game_day_index += 1;
                current_game_day = game_days[game_day_index];
                game_counter = max_games_per_day;
            }
        }
    }
}
