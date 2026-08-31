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
        teams: &[Team],
        season_config: &SeasonConfig,
    ) -> Result<(), AppError> {
        if season_config.number_fields() < 1 {
            return Err(AppError::InvalidNumberOfFields(
                season_config.number_fields(),
            ));
        }

        // Below 4 teams, once one team is on bye, too few opponents remain
        // to give every team two *different* opponents on a shared match
        // day — mathematically impossible regardless of how much time is
        // available.
        if teams.len() < 4 {
            return Err(AppError::NotEnoughTeams(teams.len(), 4));
        }

        // Both legs are structurally identical single-leg schedules (same
        // circle-method logic, just a different shuffle), so they always
        // need the same number of real games per round: teams.len() / 2
        // (integer division holds whether the count is even or odd, the
        // odd case's bye simply removes one team from that round before
        // halving).
        let games_per_leg = teams.len() as u32 / 2;
        let slots_per_leg = games_per_leg.div_ceil(season_config.number_fields());

        // Each leg needs to fit entirely on its own side of the break — one
        // leg plays out before start_break, the other starts fresh at
        // end_break — rather than the two legs packing together across the
        // break however capacity happens to allow. So the binding
        // constraint is whichever side has less room, not the day's total
        // capacity (a config where both sides combined have enough slots
        // but one side alone doesn't must still be rejected).
        let slots_before_break = self.available_slots_before_break(season_config);
        let slots_after_break = self.available_slots_after_break(season_config);
        let available_slots = slots_before_break.min(slots_after_break);
        if slots_per_leg > available_slots {
            return Err(AppError::InsufficientDailyCapacity(
                slots_per_leg,
                available_slots,
            ));
        }

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
        self.validate_parameters(teams, season_config)?;

        let mut maybe_schedule = None;
        'outer: for _ in 0..100 {
            let pass_a = self.generate_single_game_schedule(
                teams,
                start_date,
                season_config,
                season_config.start_time(),
            )?;
            for _ in 0..100 {
                let pass_b = self.generate_single_game_schedule(
                    teams,
                    start_date,
                    season_config,
                    season_config.end_break(),
                )?;
                if let Some(schedule) = self.merge_schedules(pass_a.clone(), pass_b) {
                    maybe_schedule = Some(schedule);
                    break 'outer;
                }
            }
        }

        let schedule =
            maybe_schedule.ok_or(AppError::InfeasibleDailyDoubleRoundRobin(teams.len()))?;

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

    // Same eligibility rules as `add_referees` (not busy playing, not on
    // bye that day, not already refereeing another game at that exact
    // time), but where `add_referees` commits to a single greedy pass and
    // stops, this also rebalances afterward: a purely greedy, myopic
    // "assign whoever's currently least-used" pass can leave some team
    // under-assigned overall even when a perfectly even split exists,
    // since it has no look-ahead into which teams are about to become
    // ineligible for a long stretch. The rebalancing pass repeatedly
    // reassigns one game from the most-used team to the least-used team
    // (whenever the least-used team happens to be eligible for one of the
    // most-used team's games) until the spread is at most 1, or no more
    // such swaps can be found (best-effort local search, not a globally
    // optimal assignment).
    fn add_referees(&self, schedule: Vec<Game>, teams: &[Team]) -> Result<Vec<Game>, AppError> {
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
            busy_teams_set
                .entry(game_day)
                .or_default()
                .extend([game.get_home_team(), game.get_away_team()]);
        }

        let real_game_indices: Vec<usize> = schedule
            .iter()
            .enumerate()
            .filter(|(_, game)| {
                game.get_home_team().get_name() != "Bye" && game.get_away_team().get_name() != "Bye"
            })
            .map(|(index, _)| index)
            .collect();

        // Initial pass: identical in spirit to `add_referees`, assign each
        // real game to the currently least-used eligible team.
        let mut referee_count: HashMap<&Team, u32> = teams.iter().map(|team| (team, 0)).collect();
        let mut referees_at_time: HashMap<&DateTime<Tz>, Vec<&Team>> = HashMap::new();
        let mut assigned_referee: HashMap<usize, &Team> = HashMap::new();

        for &index in &real_game_indices {
            let game = &schedule[index];
            let busy_teams = busy_teams_set
                .get(game.get_game_day())
                .expect("busy_teams_set was built from this same schedule, so every game's day is already a key");
            let already_refereeing = referees_at_time
                .get(game.get_game_day())
                .cloned()
                .unwrap_or_default();
            let day = game.get_game_day().date_naive();
            let eligible_teams = teams
                .iter()
                .filter(|team| {
                    !busy_teams.contains(team)
                        && !already_refereeing.contains(team)
                        && bye_team_by_day.get(&day) != Some(team)
                })
                .collect::<Vec<_>>();

            if eligible_teams.is_empty() {
                return Err(AppError::EmptyEligibleReferees);
            }
            let referee = *eligible_teams
                .iter()
                .min_by_key(|team| referee_count[**team])
                .expect("eligible_teams is non-empty, checked above");

            referees_at_time
                .entry(game.get_game_day())
                .or_default()
                .push(referee);
            *referee_count.entry(referee).or_insert(0) += 1;
            assigned_referee.insert(index, referee);
        }

        // Rebalancing pass. Bounded by schedule length: each successful
        // swap strictly reduces the max-min spread by 2, so this can
        // never run longer than that, and it stops early the moment no
        // beneficial swap is found.
        for _ in 0..real_game_indices.len() {
            let (&max_team, &max_count) = referee_count
                .iter()
                .max_by_key(|(_, &count)| count)
                .expect("teams is non-empty, checked in validate_parameters");
            let (&min_team, &min_count) = referee_count
                .iter()
                .min_by_key(|(_, &count)| count)
                .expect("teams is non-empty, checked in validate_parameters");

            if max_count - min_count <= 1 {
                break;
            }

            let swappable_index = real_game_indices.iter().copied().find(|index| {
                if *assigned_referee
                    .get(index)
                    .expect("every real game was assigned a referee above")
                    != max_team
                {
                    return false;
                }
                let game = &schedule[*index];
                let day = game.get_game_day().date_naive();
                !busy_teams_set[game.get_game_day()].contains(&min_team)
                    && bye_team_by_day.get(&day) != Some(&min_team)
                    && !referees_at_time[game.get_game_day()].contains(&min_team)
            });

            let Some(index) = swappable_index else {
                // No game currently refereed by the most-used team can be
                // handed to the least-used team without violating an
                // eligibility rule. Stop rather than loop on a pair that
                // can never be rebalanced.
                break;
            };

            let game_day = schedule[index].get_game_day();
            referees_at_time
                .get_mut(game_day)
                .expect("this game's day is already a key, populated during the initial pass")
                .retain(|&team| team != max_team);
            referees_at_time.entry(game_day).or_default().push(min_team);
            *referee_count.entry(max_team).or_insert(0) -= 1;
            *referee_count.entry(min_team).or_insert(0) += 1;
            assigned_referee.insert(index, min_team);
        }

        let mut schedule_with_referee = Vec::with_capacity(schedule.len());
        for (index, game) in schedule.into_iter().enumerate() {
            let Some(&referee) = assigned_referee.get(&index) else {
                // Bye entries were never assigned a referee above.
                schedule_with_referee.push(game);
                continue;
            };
            let game = Game::new_with_game_day(
                game.get_home_team().clone(),
                game.get_away_team().clone(),
                game.get_game_day().date_naive(),
                game.get_game_time()?,
                Some(referee.clone()),
            )?;
            schedule_with_referee.push(game);
        }

        Ok(schedule_with_referee)
    }

    // Counts how many distinct game-time slots fit in a single day for the
    // configured start time, break window, and time-between-games spacing,
    // stopping at the same fixed hard-stop boundary GameTimeScheduler itself
    // enforces. Uses a probe scheduler with 1 field, since this counts
    // distinct TIME VALUES only; field capacity is factored in separately by
    // the caller.
    // Counts the distinct game-time slots strictly before the configured
    // break, using a probe scheduler with 1 field since this counts
    // distinct TIME VALUES only; field capacity is factored in separately
    // by the caller.
    fn available_slots_before_break(&self, season_config: &SeasonConfig) -> u32 {
        let mut probe = GameTimeScheduler::new(
            season_config.start_time(),
            season_config.time_between_games(),
            1,
            season_config.start_break(),
            season_config.end_break(),
        );

        let mut slots = 0u32;
        // Defensive iteration cap: nothing currently validates
        // time_between_games > 0 anywhere in the codebase (pre-existing gap,
        // out of scope here), and a zero duration would make try_advance
        // never change current_time, which would otherwise loop forever.
        for _ in 0..(24 * 60) {
            if probe.is_past_hard_stop() || probe.current_time() >= season_config.start_break() {
                break;
            }
            slots += 1;
            probe.try_advance();
        }

        slots
    }

    // Counts the distinct game-time slots at or after the configured break,
    // up to the same hard-stop boundary. The probe's clock starts directly
    // at end_break rather than at the season's start_time, since a leg
    // placed after the break always begins there, independent of how much
    // room the pre-break leg actually used.
    fn available_slots_after_break(&self, season_config: &SeasonConfig) -> u32 {
        let mut probe = GameTimeScheduler::new(
            season_config.end_break(),
            season_config.time_between_games(),
            1,
            season_config.start_break(),
            season_config.end_break(),
        );

        let mut slots = 0u32;
        for _ in 0..(24 * 60) {
            if probe.is_past_hard_stop() {
                break;
            }
            slots += 1;
            probe.try_advance();
        }

        slots
    }

    // Generates a complete single round-robin: every pair of teams meets
    // exactly once, and each team plays at most one game per day (one bye
    // if the team count is odd), since only one leg's worth of games is
    // scheduled per day here. This is the proven-correct core the
    // eventual double round-robin is built from, by generating two of
    // these (different team orders) and merging them.
    // `leg_start_time` is the clock each day's schedule for this leg
    // resets to — the season's actual start_time for the leg that plays
    // before the break, or end_break for the leg that plays after it, so
    // the two legs land in disjoint, non-adjacent windows every day rather
    // than one leg continuing wherever the other's clock happened to stop.
    fn generate_single_game_schedule(
        &self,
        teams: &[Team],
        start_date: &NaiveDate,
        season_config: &SeasonConfig,
        leg_start_time: &GameTime,
    ) -> Result<Vec<Game>, AppError> {
        let mut rng = rand::rng();
        let mut inner_teams = teams.to_vec();
        if !inner_teams.len().is_multiple_of(2) {
            inner_teams.push(Team::new("Bye", None));
        }
        inner_teams.shuffle(&mut rng);
        let number_teams = inner_teams.len();

        let mut schedule = Vec::with_capacity((number_teams - 1) * (number_teams / 2));

        let mut game_day_scheduler = GameDayScheduler::new(start_date, season_config.game_days())?;
        let mut game_time_scheduler = GameTimeScheduler::new(
            leg_start_time,
            season_config.time_between_games(),
            season_config.number_fields(),
            season_config.start_break(),
            season_config.end_break(),
        );

        for _ in 0..number_teams - 1 {
            game_time_scheduler.reset();

            for i in 0..(number_teams / 2) {
                let home_team = inner_teams[i].clone();
                let away_team = inner_teams[number_teams - 1 - i].clone();
                let is_bye = home_team.get_name() == "Bye" || away_team.get_name() == "Bye";
                // A bye never advances the clock, so it can never legitimately
                // need to spill onto a new day either — skipping the check
                // here avoids the round's harmless bye slot getting stranded
                // on a fresh day purely because the clock's last real-game
                // advance happened to tick past hard_stop with nothing left
                // that actually needed the room.
                if !is_bye {
                    game_day_scheduler.advance_if_past_hard_stop(&mut game_time_scheduler);
                }
                let game_day = *game_day_scheduler.current_day();
                let game_time = *game_time_scheduler.current_time();
                let game =
                    Game::new_with_game_day(home_team, away_team, game_day, game_time, None)?;
                schedule.push(game);
                if !is_bye {
                    game_time_scheduler.try_advance();
                }
            }

            game_day_scheduler.advance();
            Self::rotate_teams(&mut inner_teams);
        }

        Ok(schedule)
    }

    // Merges two single-round-robin schedules (see `generate_single_game_schedule`)
    // into one combined schedule where every active team plays twice per
    // day. Rounds are matched by which team has the bye that day (or, if
    // there's no bye at all, by the calendar day directly, since both
    // passes then share the exact same day sequence with no team ever
    // idle), so a team's bye absorbs both passes' idle round into one
    // true day off rather than two separate ones. Returns None if any
    // merged day would repeat the same pair in both halves — the two
    // passes don't combine cleanly and the caller should try a fresh pair
    // of schedules.
    fn merge_schedules(&self, pass_a: Vec<Game>, pass_b: Vec<Game>) -> Option<Vec<Game>> {
        let has_bye = pass_a.iter().any(is_bye_game);

        let pass_a_days = group_by_day(pass_a);
        let pass_b_days = group_by_day(pass_b);

        let mut pass_b_by_bye: HashMap<String, Vec<Game>> = HashMap::new();
        let mut pass_b_by_day: HashMap<NaiveDate, Vec<Game>> = HashMap::new();
        if has_bye {
            for (_, games) in pass_b_days {
                let bye_name = bye_team_name(&games)
                    .expect("every round has a bye team when the padded team count is odd")
                    .to_owned();
                pass_b_by_bye.insert(bye_name, games);
            }
        } else {
            for (day, games) in pass_b_days {
                pass_b_by_day.insert(day, games);
            }
        }

        let mut merged = Vec::new();

        for (day, games_a) in pass_a_days {
            let games_b = if has_bye {
                let bye_name = bye_team_name(&games_a)
                    .expect("every round has a bye team when the padded team count is odd");
                pass_b_by_bye.remove(bye_name).expect(
                    "pass_b is a complete single round-robin, so every team has exactly one bye round",
                )
            } else {
                pass_b_by_day.remove(&day).expect(
                    "pass_a and pass_b share the same start date and game days, so their day sequences align exactly",
                )
            };

            // A same-day rematch means these two passes don't combine cleanly.
            let mut opponent_in_a: HashMap<&str, &str> = HashMap::new();
            for game in games_a.iter().filter(|game| !is_bye_game(game)) {
                opponent_in_a.insert(
                    game.get_home_team().get_name(),
                    game.get_away_team().get_name(),
                );
                opponent_in_a.insert(
                    game.get_away_team().get_name(),
                    game.get_home_team().get_name(),
                );
            }
            for game in games_b.iter().filter(|game| !is_bye_game(game)) {
                if opponent_in_a.get(game.get_home_team().get_name())
                    == Some(&game.get_away_team().get_name())
                {
                    return None;
                }
            }

            merged.extend(games_a);
            // pass_a's own bye entry (kept above) already represents this
            // day's bye, so pass_b's bye entry (if any) is redundant here.
            // pass_b's own games already carry the correct time for their
            // side of the break (each leg was generated with its own
            // leg_start_time — start_time for pass_a, end_break for
            // pass_b — and a matching mirrored hard_stop), so they're used
            // as generated rather than replayed through a continuation
            // scheduler. Only the calendar day is re-stamped to pass_a's,
            // as a cheap safety net in case the two passes' day sequences
            // ever drift.
            for game in games_b.into_iter().filter(|game| !is_bye_game(game)) {
                let game_time = game.get_game_time().ok()?;
                let updated_game = Game::new_with_game_day(
                    game.get_home_team().clone(),
                    game.get_away_team().clone(),
                    day,
                    game_time,
                    game.get_referee().clone(),
                )
                .ok()?;
                merged.push(updated_game);
            }
        }

        Some(merged)
    }
}

fn is_bye_game(game: &Game) -> bool {
    game.get_home_team().get_name() == "Bye" || game.get_away_team().get_name() == "Bye"
}

fn bye_team_name(games: &[Game]) -> Option<&str> {
    games.iter().find_map(|game| {
        let home = game.get_home_team().get_name();
        let away = game.get_away_team().get_name();
        if home == "Bye" {
            Some(away)
        } else if away == "Bye" {
            Some(home)
        } else {
            None
        }
    })
}

// Groups games into calendar-day buckets, preserving day order. Relies on
// games already being contiguous by day (true for anything produced by
// `generate_single_game_schedule`, which only ever advances forward).
fn group_by_day(games: Vec<Game>) -> Vec<(NaiveDate, Vec<Game>)> {
    let mut groups: Vec<(NaiveDate, Vec<Game>)> = Vec::new();
    for game in games {
        let day = game.get_game_day().date_naive();
        match groups.last_mut() {
            Some((last_day, group)) if *last_day == day => group.push(game),
            _ => groups.push((day, vec![game])),
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use chrono::{Datelike, NaiveDate, Weekday};

    use crate::types::game_time::GameTime;

    fn start_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()
    }

    fn many_teams(count: usize) -> Vec<Team> {
        (0..count)
            .map(|i| Team::new(&format!("T{i}"), None))
            .collect()
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

        let result = round_robin.validate_parameters(&many_teams(5), &season_config);

        assert!(result.is_ok(), "passed parameters are not valid");
    }

    // Test case: fewer than 2 teams is rejected.
    #[test]
    fn compute_schedule_rejects_fewer_than_two_teams() {
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let result =
            RoundRobin.compute_schedule(&many_teams(1), &start_date(), &season_config, false);

        assert!(matches!(result, Err(AppError::NotEnoughTeams(1, 4))));
    }

    // Test case: zero fields is rejected.
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

        let result =
            RoundRobin.compute_schedule(&many_teams(5), &start_date(), &season_config, false);

        assert!(matches!(result, Err(AppError::InvalidNumberOfFields(0))));
    }

    // Test case: an otherwise ordinary season config (a normal morning
    // start, a standard lunch break, one field) simply doesn't have enough
    // pre-lunch room for this many teams — 8 teams need 4 morning slots,
    // but a 9:00 start with a 12:00 lunch only offers 3 (9:00, 10:00,
    // 11:00) an hour apart.
    #[test]
    fn compute_schedule_rejects_insufficient_daily_capacity() {
        let teams = many_teams(8);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let result = RoundRobin.compute_schedule(&teams, &start_date(), &season_config, false);

        assert!(matches!(
            result,
            Err(AppError::InsufficientDailyCapacity(4, 3))
        ));
    }

    // Test case: referees requested with too few eligible teams to cover a
    // game. With the smallest valid team count (4) and 2 fields, both of a
    // leg's games always land in the same time slot, meaning all 4 teams
    // are playing simultaneously and nobody is left to referee either one.
    #[test]
    fn compute_schedule_rejects_when_no_eligible_referees_remain() {
        let teams = many_teams(4);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let result = RoundRobin.compute_schedule(&teams, &start_date(), &season_config, true);

        assert!(matches!(result, Err(AppError::EmptyEligibleReferees)));
    }

    // Test case: the smallest valid team count (4) — the current minimum
    // below which validate_parameters rejects for pairing infeasibility.
    #[test]
    fn test_smallest_valid_team_count() {
        let teams = many_teams(4);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            1,
            vec![Weekday::Sat],
        );

        let schedule = RoundRobin
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, &start_date(), &season_config, false);
    }

    // Test case: exactly 5 teams. Proven earlier (an exhaustive search over
    // every possible pairing) to be mathematically impossible for this
    // two-pass construction to merge without a same-day rematch, no matter
    // how many shuffles are tried, so this should deterministically exhaust
    // the retry budget and fail, rather than just being unlikely to succeed.
    #[test]
    fn compute_schedule_rejects_infeasible_five_teams() {
        let teams = many_teams(5);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            2,
            vec![Weekday::Wed, Weekday::Sat],
        );

        let result = RoundRobin.compute_schedule(&teams, &start_date(), &season_config, false);

        assert!(matches!(
            result,
            Err(AppError::InfeasibleDailyDoubleRoundRobin(_))
        ));
    }

    // Test case: an even number of teams.
    #[test]
    fn test_even_team_count() {
        let teams = many_teams(6);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 30).unwrap(),
            3,
            vec![Weekday::Wed, Weekday::Sat],
        );

        let schedule = RoundRobin
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, &start_date(), &season_config, false);
    }

    // Test case: referees requested (also covers an odd team count — this
    // config is otherwise identical to what a dedicated odd-team-count
    // test would use, and with_referees: true exercises everything a
    // referee-less run would check, plus the referee assertions).
    #[test]
    fn test_referees_requested() {
        let teams = many_teams(9);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(14, 0).unwrap(),
            GameTime::new(2, 0).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let schedule = RoundRobin
            .compute_schedule(&teams, &start_date(), &season_config, true)
            .unwrap();

        assert_schedule(&schedule, &teams, &start_date(), &season_config, true);
    }

    // Test case: a single field configured, the tightest possible
    // field-capacity pressure (no two games anywhere can share a time slot).
    #[test]
    fn test_single_field() {
        let teams = many_teams(8);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(13, 0).unwrap(),
            GameTime::new(14, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            1,
            vec![Weekday::Wed, Weekday::Sat],
        );

        let schedule = RoundRobin
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, &start_date(), &season_config, false);
    }

    // Test case: 2 fields configured.
    #[test]
    fn test_two_fields() {
        let teams = many_teams(8);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(14, 0).unwrap(),
            GameTime::new(2, 0).unwrap(),
            2,
            vec![Weekday::Sat],
        );

        let schedule = RoundRobin
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        assert_schedule(&schedule, &teams, &start_date(), &season_config, false);
    }

    // Test case: multiple game days configured (more than one weekday
    // allowed), so the schedule actually has to rotate between them.
    #[test]
    fn test_multiple_game_days() {
        let teams = many_teams(7);
        let season_config = SeasonConfig::new(
            start_date(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(2, 0).unwrap(),
            2,
            vec![Weekday::Wed, Weekday::Sat, Weekday::Sun],
        );

        let schedule = RoundRobin
            .compute_schedule(&teams, &start_date(), &season_config, false)
            .unwrap();

        let days_used: HashSet<Weekday> = schedule
            .iter()
            .map(|game| game.get_game_day().weekday())
            .collect();
        assert!(
            days_used.len() > 1,
            "expected the schedule to actually use more than one configured weekday"
        );

        assert_schedule(&schedule, &teams, &start_date(), &season_config, false);
    }

    fn assert_schedule(
        schedule: &[Game],
        teams: &[Team],
        start_date: &NaiveDate,
        season_config: &SeasonConfig,
        with_referees: bool,
    ) {
        let is_odd = !teams.len().is_multiple_of(2);

        let input_names: HashSet<&str> = teams.iter().map(Team::get_name).collect();
        assert_eq!(
            teams.len(),
            input_names.len(),
            "input teams have duplicate names"
        );
        let input_team_set: HashSet<&Team> = teams.iter().collect();

        // The total game count matches the expected formula for the given
        // team count: N * (N - 1) real games (every pair meets twice), plus
        // one bye game per team when the count is odd.
        let expected_total = if is_odd {
            teams.len() * teams.len()
        } else {
            teams.len() * (teams.len() - 1)
        };
        assert_eq!(
            schedule.len(),
            expected_total,
            "unexpected total game count"
        );

        let schedule_team_names: HashSet<&str> = schedule
            .iter()
            .flat_map(|game| [game.get_home_team(), game.get_away_team()])
            .map(Team::get_name)
            .filter(|&name| name != "Bye")
            .collect();

        // Every input team appears in the generated schedule.
        assert_eq!(
            schedule_team_names, input_names,
            "not every input team appears in the schedule"
        );

        // Team identities (names/seeds) are preserved unchanged from input
        // to output.
        for game in schedule.iter() {
            for team in [game.get_home_team(), game.get_away_team()] {
                if team.get_name() != "Bye" {
                    assert!(
                        input_team_set.contains(team),
                        "team {team:?} in the schedule doesn't match any input team exactly"
                    );
                }
            }
        }

        let mut bye_weeks: HashMap<&str, u32> = HashMap::new();
        let mut computed_game_days: HashMap<&DateTime<Tz>, u32> = HashMap::new();
        let mut team_real_game_days: HashMap<&str, HashSet<NaiveDate>> = HashMap::new();
        let mut team_bye_days: HashMap<&str, NaiveDate> = HashMap::new();
        let mut times_by_day: HashMap<NaiveDate, HashSet<GameTime>> = HashMap::new();
        let mut team_daily_times: HashMap<(&str, NaiveDate), Vec<GameTime>> = HashMap::new();
        let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();
        let mut pair_days: HashMap<(String, String), HashSet<NaiveDate>> = HashMap::new();

        for game in schedule.iter() {
            let home_team = game.get_home_team();
            let away_team = game.get_away_team();
            let game_day = game.get_game_day();
            let day = game_day.date_naive();

            // No team is ever scheduled to play itself.
            assert_ne!(home_team, away_team, "a team was scheduled against itself");

            // Games only fall on the configured days of the week.
            assert!(
                season_config.game_days().contains(&game_day.weekday()),
                "game on {day} falls on a day of the week not in the configured game days"
            );

            // No game is scheduled before the configured season start date.
            assert!(
                day >= *start_date,
                "game on {day} is scheduled before the season start date {start_date}"
            );

            let is_bye = home_team.get_name() == "Bye" || away_team.get_name() == "Bye";
            if !is_bye {
                *computed_game_days.entry(game_day).or_insert(0) += 1;
                team_real_game_days
                    .entry(home_team.get_name())
                    .or_default()
                    .insert(day);
                team_real_game_days
                    .entry(away_team.get_name())
                    .or_default()
                    .insert(day);

                let game_time = game
                    .get_game_time()
                    .expect("a real game's time should always be extractable");

                // No game is scheduled inside the configured break window.
                assert!(
                    !(game_time > *season_config.start_break()
                        && game_time < *season_config.end_break()),
                    "game at {game_time} on {day} falls inside the break window"
                );

                times_by_day.entry(day).or_default().insert(game_time);
                team_daily_times
                    .entry((home_team.get_name(), day))
                    .or_default()
                    .push(game_time);
                team_daily_times
                    .entry((away_team.get_name(), day))
                    .or_default()
                    .push(game_time);

                let mut names = [
                    home_team.get_name().to_owned(),
                    away_team.get_name().to_owned(),
                ];
                names.sort();
                let key = (names[0].clone(), names[1].clone());
                *pair_counts.entry(key.clone()).or_insert(0) += 1;
                pair_days.entry(key).or_default().insert(day);

                if with_referees {
                    assert!(
                        game.get_referee().is_some(),
                        "real game on {day} at {game_time} has no referee assigned"
                    );
                    let referee = game
                        .get_referee()
                        .as_ref()
                        .expect("checked above that a referee is present");
                    // A referee is never assigned to a game one of their own
                    // teams is playing in.
                    assert_ne!(referee, home_team, "referee is playing in their own game");
                    assert_ne!(referee, away_team, "referee is playing in their own game");
                }
            } else {
                let bye_team_name = if home_team.get_name() == "Bye" {
                    away_team.get_name()
                } else {
                    home_team.get_name()
                };
                *bye_weeks.entry(bye_team_name).or_insert(0) += 1;
                team_bye_days.insert(bye_team_name, day);
            }
        }

        // An odd number of teams gives every team exactly one bye; an even
        // number of teams never produces a bye.
        if is_odd {
            assert_eq!(bye_weeks.len(), teams.len(), "not every team has a bye");
            assert!(
                bye_weeks.values().all(|&count| count == 1),
                "every team should have exactly one bye"
            );
        } else {
            assert!(
                bye_weeks.is_empty(),
                "an even team count should never produce a bye"
            );
        }

        // No team plays a real game on the same day as its own bye.
        for (team, bye_day) in team_bye_days.iter() {
            assert!(
                !team_real_game_days
                    .get(team)
                    .is_some_and(|days| days.contains(bye_day)),
                "team {team} has a real game scheduled on its bye day {bye_day}"
            );
        }

        // No two games share the same date and time beyond the number of
        // available fields.
        assert!(
            computed_game_days
                .values()
                .all(|&count| count <= season_config.number_fields()),
            "some time slot has more games than the configured number of fields"
        );

        // Consecutive games on the same day respect the configured spacing
        // between games (the gap can be larger, e.g. when it jumps over the
        // break window, but never smaller).
        for (day, times) in times_by_day.iter() {
            let mut sorted_times: Vec<GameTime> = times.iter().copied().collect();
            sorted_times.sort();
            for pair in sorted_times.windows(2) {
                assert!(
                    pair[0] + *season_config.time_between_games() <= pair[1],
                    "games on {day} at {} and {} are closer together than the configured spacing",
                    pair[0],
                    pair[1]
                );
            }
        }

        // A team's two games on a given day are never back-to-back: the
        // season's break exists to give teams rest, so one game must fall
        // strictly before the break starts and the other at or after it
        // ends, never both on the same side of it.
        for ((team, day), mut times) in team_daily_times {
            assert_eq!(
                times.len(),
                2,
                "team {team} should play exactly 2 games on {day}, played {}",
                times.len()
            );
            times.sort();
            assert!(
                times[0] < *season_config.start_break() && times[1] >= *season_config.end_break(),
                "team {team}'s games on {day} at {} and {} aren't separated by the break window",
                times[0],
                times[1]
            );
        }

        // Every unique real (non-bye) pair should face each other exactly
        // twice across the season, no pair skipped, none repeated unevenly,
        // and on two different calendar days, not an immediate same-day
        // rematch.
        for i in 0..teams.len() {
            for j in (i + 1)..teams.len() {
                let mut names = [
                    teams[i].get_name().to_owned(),
                    teams[j].get_name().to_owned(),
                ];
                names.sort();
                let key = (names[0].clone(), names[1].clone());
                assert_eq!(
                    pair_counts.get(&key).copied().unwrap_or(0),
                    2,
                    "pair {key:?} should meet exactly twice, met {:?} times",
                    pair_counts.get(&key)
                );
                assert_eq!(
                    pair_days.get(&key).map(HashSet::len).unwrap_or(0),
                    2,
                    "pair {key:?} should meet on two different days, met on {:?}",
                    pair_days.get(&key)
                );
            }
        }

        // The season fits within a reasonable number of calendar days, a
        // flat, deliberately generous sanity bound (not scaled to team
        // count or field count, since low field counts can legitimately
        // stretch a season a long way) just to catch a genuine
        // runaway/infinite scheduling bug (e.g. a date-arithmetic bug
        // producing a wildly wrong jump), not a tight timing requirement.
        if let (Some(first), Some(last)) = (
            schedule
                .iter()
                .map(|game| game.get_game_day().date_naive())
                .min(),
            schedule
                .iter()
                .map(|game| game.get_game_day().date_naive())
                .max(),
        ) {
            let span_days = (last - first).num_days();
            assert!(
                span_days <= 3650,
                "season spans an unreasonable number of days ({span_days}) for {} teams",
                teams.len()
            );
        }

        // Referee assignments are spread as evenly as possible across
        // eligible teams over the season, and a referee is never scheduled
        // to referee two games happening at the same time.
        if with_referees {
            let mut referee_counts: HashMap<&str, u32> =
                teams.iter().map(|team| (team.get_name(), 0)).collect();
            let mut referees_at_time: HashMap<&DateTime<Tz>, HashSet<&str>> = HashMap::new();
            for game in schedule.iter() {
                if let Some(referee) = game.get_referee() {
                    *referee_counts.entry(referee.get_name()).or_insert(0) += 1;

                    let already_refereeing = !referees_at_time
                        .entry(game.get_game_day())
                        .or_default()
                        .insert(referee.get_name());
                    assert!(
                        !already_refereeing,
                        "{} was assigned as referee for two games at {}",
                        referee.get_name(),
                        game.get_game_day()
                    );
                }
            }
            let min_count = referee_counts.values().min().copied().unwrap_or(0);
            let max_count = referee_counts.values().max().copied().unwrap_or(0);
            assert!(
                max_count - min_count <= 2,
                "referee assignments are not evenly spread: counts range from {min_count} to {max_count}"
            );
        }
    }
}
