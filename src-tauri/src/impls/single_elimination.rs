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

    fn validate_parameters(
        &self,
        teams: &[Team],
        season_config: &SeasonConfig,
    ) -> Result<(), AppError> {
        if teams.len() < 2 {
            return Err(AppError::NotEnoughTeams(teams.len(), 2));
        }
        let number_fields = season_config.number_fields();
        if number_fields < 1 {
            return Err(AppError::InvalidNumberOfFields(number_fields));
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
        self.validate_parameters(teams, season_config)?;
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
        let first_real_round_games = (number_of_teams - number_of_byes) / 2;
        for offset in 0..first_real_round_games {
            game_day_scheduler.advance_if_past_hard_stop(&mut game_time_scheduler);
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

        // Compute the second round of single elimination, taking into account first round bye weeks.
        // Bye recipients from round 1 are paired against each other two at a time. An odd one out
        // (when number_of_byes is odd) plays the still-undecided winner of a round-1 real game. Any
        // round-1 real games left over after that play each other, using the same WinnerA/WinnerB
        // placeholder names later rounds use, since neither side is known yet.
        let bye_recipients = &inner_teams[..number_of_byes];
        let bye_recipient_pairs = bye_recipients.as_chunks::<2>();
        for pair in &mut bye_recipient_pairs.0.iter() {
            game_day_scheduler.advance_if_past_hard_stop(&mut game_time_scheduler);
            let home_team = pair[0].clone();
            let away_team = pair[1].clone();
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

        let mut winner_previous_slots = first_real_round_games;
        if let [leftover_bye_recipient] = bye_recipient_pairs.1 {
            game_day_scheduler.advance_if_past_hard_stop(&mut game_time_scheduler);
            let home_team = leftover_bye_recipient.clone();
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
            winner_previous_slots -= 1;
        }

        for _ in 0..winner_previous_slots / 2 {
            game_day_scheduler.advance_if_past_hard_stop(&mut game_time_scheduler);
            let home_team = Team::new("WinnerA", None);
            let away_team = Team::new("WinnerB", None);
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

        // Compute all remaining rounds as there is no side effect from bye weeks.
        // Each round is a new day, since the previous round's winners aren't
        // decided yet.
        for round in 3..=number_of_round {
            game_day_scheduler.advance();
            game_time_scheduler.reset();

            let number_of_games = bracket_size / 2usize.pow(round);
            for _ in 0..number_of_games {
                game_day_scheduler.advance_if_past_hard_stop(&mut game_time_scheduler);
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
    use std::collections::{HashMap, HashSet};

    use super::*;
    use chrono::{Datelike, Weekday};

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
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );
        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(&teams(), &season_config);

        assert!(result.is_ok(), "passed parameters should be valid");
    }

    #[test]
    fn test_single_elimination_parameter_validation_rejects_too_few_teams() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );
        let single_elimination = SingleElimination::new(false);

        let result =
            single_elimination.validate_parameters(&[Team::new("Solo Team", None)], &season_config);

        assert!(matches!(result, Err(AppError::NotEnoughTeams(1, 2))));
    }

    #[test]
    fn test_single_elimination_parameter_validation_rejects_zero_fields() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            0,
            vec![Weekday::Sat],
        );
        let single_elimination = SingleElimination::new(false);

        let result = single_elimination.validate_parameters(&teams(), &season_config);

        assert!(matches!(result, Err(AppError::InvalidNumberOfFields(0))));
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
    fn compute_schedule_rejects_zero_fields() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            0,
            vec![Weekday::Sat],
        );

        let result = SingleElimination::new(false).compute_schedule(
            &teams(),
            &start_date(),
            &season_config,
            false,
        );

        assert!(matches!(result, Err(AppError::InvalidNumberOfFields(0))));
    }

    // No game days configured means GameDayScheduler can never find a
    // valid starting day.
    #[test]
    fn compute_schedule_rejects_empty_game_days() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![],
        );

        let result = SingleElimination::new(false).compute_schedule(
            &teams(),
            &start_date(),
            &season_config,
            false,
        );

        assert!(matches!(result, Err(AppError::EmptyGameDays)));
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

    #[test]
    fn test_single_elimination_later_rounds_advance_the_day() {
        // 9 teams needs 4 rounds (bracket of 16), so rounds 3 and 4 both
        // exist, letting this check that the day actually advances between
        // them rather than both landing on the same calendar day.
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let teams = teams_bigger();
        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        let bracket_size = teams.len().next_power_of_two();
        let number_of_byes = bracket_size - teams.len();
        let first_real_round_games = (teams.len() - number_of_byes) / 2;
        let round_1_games = number_of_byes + first_real_round_games;
        let round_2_games = bracket_size / 4;
        let round_3_games = bracket_size / 8;
        let round_3_start = round_1_games + round_2_games;
        let round_4_start = round_3_start + round_3_games;

        let round_3_day = schedule[round_3_start].get_game_day().date_naive();
        let round_4_day = schedule[round_4_start].get_game_day().date_naive();

        assert!(
            round_4_day > round_3_day,
            "round 4 ({round_4_day}) should be scheduled after round 3 ({round_3_day}), not the same day"
        );
    }

    #[test]
    fn test_large_bracket_respects_single_field_capacity() {
        // With only 1 field, no two games should ever share the exact same
        // date and time. A large enough round (round 1 of a 32-team
        // power-of-two bracket needs 16 games) can need more distinct time
        // slots in a single day than the day's time window provides before
        // wrapping back around, colliding with a time already used earlier
        // that same day, the same failure mode found in round_robin.rs.
        let teams: Vec<Team> = (0..32).map(|i| Team::new(&format!("T{i}"), None)).collect();
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Wed, Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        // The bracket spans several rounds, each advancing the day, so it
        // should actually rotate across both configured weekdays rather
        // than only ever landing on one of them.
        let days_used: HashSet<Weekday> = schedule
            .iter()
            .map(|game| game.get_game_day().weekday())
            .collect();
        assert!(
            days_used.len() > 1,
            "expected the schedule to actually use more than one configured weekday"
        );

        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    // The large-bracket capacity test above happens to use a power-of-two
    // team count (32, zero byes), so it never exercises round 2's
    // bye-recipient-pairing loop (which also calls
    // advance_if_past_hard_stop) under real field-capacity pressure. 20
    // teams needs a bracket of 32 with 12 byes, giving round 2 real work
    // to do under the same tight, 1-field window.
    #[test]
    fn test_round_2_respects_single_field_capacity() {
        let teams: Vec<Team> = (0..20).map(|i| Team::new(&format!("T{i}"), None)).collect();
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(20, 0).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Wed, Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    // A break window that actually falls within the game-time range, so
    // at least one time slot has to jump over it instead of landing
    // inside it.
    #[test]
    fn test_break_window_within_game_time_range() {
        let teams = teams_bigger();
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(11, 0).unwrap(),
            GameTime::new(12, 30).unwrap(),
            GameTime::new(1, 0).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    #[test]
    fn test_single_elimination_anonymous_mode() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let single_elimination = SingleElimination::new(true);

        let schedule = single_elimination
            .compute_schedule(&teams(), &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams(), season_config.number_fields());

        // Anonymous mode should never leak the real team names into round 1.
        let input_teams = teams();
        let real_names: HashSet<&str> = input_teams.iter().map(|team| team.get_name()).collect();
        for game in schedule.iter() {
            assert!(!real_names.contains(game.get_home_team().get_name()));
            assert!(!real_names.contains(game.get_away_team().get_name()));
        }
    }

    #[test]
    fn test_single_elimination_gives_byes_to_lowest_seed_numbers() {
        // Seed 1 is the top seed by convention; the lowest seed numbers
        // should be the ones receiving byes.
        let teams = [
            Team::new("Seed 1", Some(1)),
            Team::new("Seed 2", Some(2)),
            Team::new("Seed 3", Some(3)),
            Team::new("Seed 4", Some(4)),
            Team::new("Seed 5", Some(5)),
        ];
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        // 5 teams -> bracket of 8 -> 3 byes, expected for seeds 1, 2, 3.
        let bye_recipients: HashSet<&str> = schedule[..3]
            .iter()
            .flat_map(|game| {
                [
                    game.get_home_team().get_name(),
                    game.get_away_team().get_name(),
                ]
            })
            .filter(|&name| name != "Bye")
            .collect();

        assert_eq!(
            bye_recipients,
            HashSet::from(["Seed 1", "Seed 2", "Seed 3"]),
            "byes should go to the lowest (best) seed numbers"
        );
    }

    #[test]
    fn test_single_elimination_power_of_two_team_count_needs_no_byes() {
        let teams = [
            Team::new("A", None),
            Team::new("B", None),
            Team::new("C", None),
            Team::new("D", None),
        ];
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert!(
            schedule
                .iter()
                .all(|game| game.get_home_team().get_name() != "Bye"
                    && game.get_away_team().get_name() != "Bye"),
            "a power-of-two team count should never need a bye"
        );
        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    #[test]
    fn test_single_elimination_two_teams_minimal_bracket() {
        let teams = [Team::new("A", None), Team::new("B", None)];
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    #[test]
    fn test_single_elimination_three_teams_smallest_bye_case() {
        let teams = [
            Team::new("A", None),
            Team::new("B", None),
            Team::new("C", None),
        ];
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    #[test]
    fn test_single_elimination_even_number_of_byes() {
        let teams = [
            Team::new("A", None),
            Team::new("B", None),
            Team::new("C", None),
            Team::new("D", None),
            Team::new("E", None),
            Team::new("F", None),
        ];
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let schedule = SingleElimination::new(false)
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        // 6 teams -> bracket of 8 -> 2 byes (even).
        assert_eq!(
            schedule
                .iter()
                .filter(|game| game.get_home_team().get_name() == "Bye"
                    || game.get_away_team().get_name() == "Bye")
                .count(),
            2
        );
        assert_schedule(&schedule, &teams, season_config.number_fields());
    }

    #[test]
    fn test_is_anonymous_reflects_constructor_argument() {
        assert!(SingleElimination::new(true).is_anonymous());
        assert!(!SingleElimination::new(false).is_anonymous());
    }

    fn assert_schedule(schedule: &[Game], teams: &[Team], number_of_fields: u32) {
        let bracket_size = teams.len().next_power_of_two();
        assert_eq!(bracket_size - 1, schedule.len());
        let number_of_byes = bracket_size - teams.len();
        let first_real_round_games = (teams.len() - number_of_byes) / 2;

        for (index, game) in schedule.iter().enumerate() {
            assert_ne!(
                game.get_home_team(),
                game.get_away_team(),
                "home and away team should be different"
            );
            assert_eq!(
                game.get_referee(),
                &None,
                "single elimination does not support referees"
            );
            let is_bye = game.get_home_team().get_name() == "Bye"
                || game.get_away_team().get_name() == "Bye";
            assert_eq!(
                is_bye,
                index < number_of_byes,
                "bye games should be exactly the first {number_of_byes} games, found one at index {index}"
            );
        }

        // Round 1 (the byes plus the first real pairing round) should
        // account for every input team exactly once, none dropped, none
        // duplicated. Later rounds use placeholder names (e.g. "WinnerA"),
        // since the actual winners aren't known yet, so this only checks
        // round 1. Names aren't compared against the input teams directly,
        // since anonymous mode renames teams to "1".."N", so this only
        // checks that round 1 has exactly as many distinct participants as
        // there are input teams.
        let round_1_games = number_of_byes + first_real_round_games;
        let round_1_team_names: Vec<&str> = schedule[..round_1_games]
            .iter()
            .flat_map(|game| {
                [
                    game.get_home_team().get_name(),
                    game.get_away_team().get_name(),
                ]
            })
            .filter(|&name| name != "Bye")
            .collect();
        let unique_round_1_names: HashSet<&str> = round_1_team_names.iter().copied().collect();
        assert_eq!(
            round_1_team_names.len(),
            teams.len(),
            "round 1 should account for every input team exactly once"
        );
        assert_eq!(
            unique_round_1_names.len(),
            teams.len(),
            "round 1 should not have any duplicate teams"
        );

        // Round 2 should seat every round-1 bye recipient exactly once
        // (identified directly from round 1's bye games), plus one
        // "WinnerPrevious" placeholder per round-1 real game (the winner
        // isn't known yet). No bye recipient should be missing, and none
        // should be paired against another bye recipient more than once.
        // A 2-team bracket has no round 2 at all (round 1's single game
        // already decides the champion), so this only applies once a
        // round 2 genuinely exists.
        let round_2_games = bracket_size / 4;
        if round_2_games > 0 {
            let bye_recipients: Vec<&str> = schedule[..number_of_byes]
                .iter()
                .flat_map(|game| {
                    [
                        game.get_home_team().get_name(),
                        game.get_away_team().get_name(),
                    ]
                })
                .filter(|&name| name != "Bye")
                .collect();
            let round_2_slice = &schedule[round_1_games..round_1_games + round_2_games];
            let round_2_names: Vec<&str> = round_2_slice
                .iter()
                .flat_map(|game| {
                    [
                        game.get_home_team().get_name(),
                        game.get_away_team().get_name(),
                    ]
                })
                .collect();
            let round_2_bye_recipient_names: Vec<&str> = round_2_names
                .iter()
                .copied()
                .filter(|name| bye_recipients.contains(name))
                .collect();
            let unique_round_2_bye_recipient_names: HashSet<&str> =
                round_2_bye_recipient_names.iter().copied().collect();
            assert_eq!(
                round_2_bye_recipient_names.len(),
                number_of_byes,
                "every round-1 bye recipient should appear in round 2 exactly once"
            );
            assert_eq!(
                unique_round_2_bye_recipient_names.len(),
                number_of_byes,
                "round 2 should not pair the same bye recipient more than once"
            );
            // A round-1 real game's still-undecided winner shows up in round
            // 2 as either "WinnerPrevious" (paired against a known bye
            // recipient) or "WinnerA"/"WinnerB" (paired against another
            // undecided winner), depending on how it's slotted.
            let unresolved_winner_count = round_2_names
                .iter()
                .filter(|&&name| name == "WinnerPrevious" || name == "WinnerA" || name == "WinnerB")
                .count();
            assert_eq!(
                unresolved_winner_count, first_real_round_games,
                "round 2 should have one unresolved-winner slot per round-1 real game"
            );
        }

        let mut games_per_time = HashMap::new();
        for game in schedule.iter() {
            if game.get_home_team().get_name() != "Bye" && game.get_away_team().get_name() != "Bye"
            {
                let game_time = game.get_game_time().unwrap();
                let game_date = game.get_game_day();
                let date_identifier = (game_time, *game_date);
                let value = games_per_time.entry(date_identifier).or_insert(0);
                *value += 1;
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
