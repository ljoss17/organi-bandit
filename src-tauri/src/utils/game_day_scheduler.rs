use chrono::{Datelike, Days, NaiveDate, Weekday};

use crate::errors::AppError;
use crate::types::configurations::date::DateConfiguration;
use crate::utils::game_time_scheduler::GameTimeScheduler;

pub struct GameDayScheduler<'a> {
    date_configuration: &'a DateConfiguration,
    current_day: NaiveDate,
    // The weekly slot the season settled into, kept separate from
    // `current_day` so that moving a single week's game off an excluded date
    // doesn't drag every following week onto that new weekday.
    anchor_day: NaiveDate,
}

impl<'a> GameDayScheduler<'a> {
    pub fn new(
        start_day: &'a NaiveDate,
        date_configuration: &'a DateConfiguration,
    ) -> Result<Self, AppError> {
        if date_configuration.game_days().is_empty() {
            return Err(AppError::EmptyGameDays);
        }
        let current_day = Self::next_date(
            start_day,
            date_configuration.game_days(),
            date_configuration.excluded_dates(),
        )?;
        Ok(Self {
            date_configuration,
            current_day,
            anchor_day: current_day,
        })
    }

    pub fn current_day(&self) -> &NaiveDate {
        &self.current_day
    }

    // Advance the day if needed
    pub fn advance(&mut self) -> Result<(), AppError> {
        if self.date_configuration.single_game_per_week() {
            self.advance_to_next_week()
        } else {
            self.advance_to_next_game_day()
        }
    }

    fn advance_to_next_game_day(&mut self) -> Result<(), AppError> {
        let next_day = self
            .current_day
            .checked_add_days(Days::new(1))
            .expect("a single day cannot overflow NaiveDate's range");
        self.current_day = Self::next_date(
            &next_day,
            self.date_configuration.game_days(),
            self.date_configuration.excluded_dates(),
        )?;
        self.anchor_day = self.current_day;
        Ok(())
    }

    // Steps a week on from the season's weekly slot rather than from the day
    // actually played. When a week's usual day is excluded the game shifts to
    // another configured weekday for that week alone.
    fn advance_to_next_week(&mut self) -> Result<(), AppError> {
        while self.anchor_day <= self.current_day {
            self.anchor_day = self
                .anchor_day
                .checked_add_days(Days::new(7))
                .expect("a week cannot overflow NaiveDate's range");
        }

        self.current_day = Self::next_date(
            &self.anchor_day,
            self.date_configuration.game_days(),
            self.date_configuration.excluded_dates(),
        )?;
        Ok(())
    }

    // Move on to the next day if the time scheduler has run out of room for
    // today, instead of letting the time wrap back around and collide with
    // a time slot already used earlier today.
    pub fn advance_if_past_hard_stop(
        &mut self,
        game_time_scheduler: &mut GameTimeScheduler,
    ) -> Result<(), AppError> {
        if game_time_scheduler.is_past_hard_stop() {
            self.advance()?;
            game_time_scheduler.reset();
        }
        Ok(())
    }

    fn next_date(
        current_day: &NaiveDate,
        game_days: &'a [Weekday],
        excluded_dates: &'a [NaiveDate],
    ) -> Result<NaiveDate, AppError> {
        let mut current_day_internal = *current_day;
        // Retry enough times to cover excluded dates
        for _ in 0..excluded_dates.len() + 2 {
            if game_days.contains(&current_day_internal.weekday())
                && !excluded_dates.contains(&current_day_internal)
            {
                return Ok(current_day_internal);
            }

            let current_weekday = current_day_internal.weekday();
            let offset = game_days
                .iter()
                .map(|weekday| match weekday.days_since(current_weekday) {
                    0 => 7,
                    days => days,
                })
                .min()
                .expect("game_days is non-empty, checked above");
            current_day_internal = current_day_internal
                .checked_add_days(Days::new(offset as u64))
                .expect("offset is at most 7 days, cannot overflow NaiveDate's range");
        }
        Err(AppError::NoStartDate(*current_day))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::TimeZone;
    use chrono_tz::Europe::Zurich;

    #[test]
    fn new_rejects_empty_game_days() {
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let date_configuration = DateConfiguration::new(start_day, vec![], vec![], false);
        let result = GameDayScheduler::new(&start_day, &date_configuration);
        assert!(matches!(result, Err(AppError::EmptyGameDays)));
    }

    #[test]
    fn new_accepts_non_empty_game_days() {
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let date_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Tue, Weekday::Sat], vec![], false);
        let result = GameDayScheduler::new(&start_day, &date_configuration);
        assert!(result.is_ok());
    }

    #[test]
    fn new_finds_next_game_day_across_year_boundary() {
        // Sunday, Jan 1 2023 falls in ISO week 52 of 2022, not 2023.
        let start_day = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let date_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Mon], vec![], false);
        let game_day_scheduler = GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        let next_monday = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_monday);
    }

    #[test]
    fn test_advance() {
        // Wednesday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let date_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Tue, Weekday::Sat], vec![], false);
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        assert_eq!(game_day_scheduler.current_day(), &start_day);

        // Next Saturday
        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();

        game_day_scheduler.advance().unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_saturday);

        // Next Tuesday
        let next_tuesday = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();

        game_day_scheduler.advance().unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_tuesday);
    }

    #[test]
    fn test_advance_from_other_weekday() {
        // Wednesday
        let start_day = Zurich
            .with_ymd_and_hms(2026, 5, 13, 8, 45, 0)
            .unwrap()
            .date_naive();
        let date_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Tue, Weekday::Sat], vec![], false);
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        // Next Saturday
        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();

        assert_eq!(game_day_scheduler.current_day(), &next_saturday);

        // Next Tuesday
        let next_tuesday = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();

        game_day_scheduler.advance().unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_tuesday);

        // Next Tuesday
        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();

        game_day_scheduler.advance().unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_saturday);
    }

    #[test]
    fn test_advance_on_excluded() {
        // Wednesday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let excluded_dates = vec![NaiveDate::from_ymd_opt(2026, 5, 16).unwrap()];
        let date_configuration = DateConfiguration::new(
            start_day,
            vec![Weekday::Tue, Weekday::Sat],
            excluded_dates,
            false,
        );
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        assert_eq!(game_day_scheduler.current_day(), &start_day);

        // Next Saturday is the 16/05/2026 which is excluded so the advance should return next Tuesday
        let next_tuesday = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();

        game_day_scheduler.advance().unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_tuesday);
    }

    #[test]
    fn new_skips_an_excluded_start_day() {
        // Tuesday, and a configured game day.
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let excluded_dates = vec![start_day];

        let date_configuration = DateConfiguration::new(
            start_day,
            vec![Weekday::Tue, Weekday::Sat],
            excluded_dates,
            false,
        );
        let game_day_scheduler = GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_saturday);
    }

    #[test]
    fn advance_with_one_game_per_week_skips_the_rest_of_the_week() {
        // Saturday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();
        let date_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Sat, Weekday::Sun], vec![], true);
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        assert_eq!(game_day_scheduler.current_day(), &start_day);

        game_day_scheduler.advance().unwrap();

        let following_saturday = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();
        let skipped_sunday = NaiveDate::from_ymd_opt(2026, 5, 17).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &following_saturday);
        assert_ne!(game_day_scheduler.current_day(), &skipped_sunday);
    }

    #[test]
    fn advance_with_one_game_per_week_keeps_the_starting_weekday() {
        // Sunday, the later of the two configured weekdays.
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 17).unwrap();
        let date_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Sat, Weekday::Sun], vec![], true);
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        game_day_scheduler.advance().unwrap();

        // The Saturday of the following week (23/05) comes first on the
        // calendar, but it belongs to the same week's allowance.
        let following_sunday = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &following_sunday);
    }

    #[test]
    fn advance_with_one_game_per_week_stays_weekly_across_several_advances() {
        // Saturday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();
        let date_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Sat, Weekday::Sun], vec![], true);
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        let expected = [
            NaiveDate::from_ymd_opt(2026, 5, 23).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 6).unwrap(),
        ];
        for expected_day in expected {
            game_day_scheduler.advance().unwrap();
            assert_eq!(game_day_scheduler.current_day(), &expected_day);
        }
    }

    #[test]
    fn advance_with_one_game_per_week_matches_the_default_for_a_single_weekday() {
        // Saturday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();

        let weekly_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Sat], vec![], true);
        let mut weekly = GameDayScheduler::new(&start_day, &weekly_configuration).unwrap();
        let every_game_day_configuration =
            DateConfiguration::new(start_day, vec![Weekday::Sat], vec![], false);
        let mut every_game_day =
            GameDayScheduler::new(&start_day, &every_game_day_configuration).unwrap();

        for _ in 0..3 {
            weekly.advance().unwrap();
            every_game_day.advance().unwrap();
            assert_eq!(weekly.current_day(), every_game_day.current_day());
        }
    }

    #[test]
    fn advance_with_one_game_per_week_falls_back_within_the_same_week() {
        // Saturday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();
        // The Saturday a week later is unavailable.
        let excluded_dates = vec![NaiveDate::from_ymd_opt(2026, 5, 23).unwrap()];
        let date_configuration = DateConfiguration::new(
            start_day,
            vec![Weekday::Sat, Weekday::Sun],
            excluded_dates,
            true,
        );
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        game_day_scheduler.advance().unwrap();

        let same_week_sunday = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &same_week_sunday);
    }

    #[test]
    fn advance_with_one_game_per_week_returns_to_its_weekday_after_an_exclusion() {
        // Saturday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();
        // Only the second Saturday of the season is unavailable.
        let excluded_dates = vec![NaiveDate::from_ymd_opt(2026, 5, 23).unwrap()];
        let date_configuration = DateConfiguration::new(
            start_day,
            vec![Weekday::Sat, Weekday::Sun],
            excluded_dates,
            true,
        );
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        let expected = [
            // The excluded Saturday pushes this one week onto the Sunday.
            NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            // Back to Saturdays from here on.
            NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 6).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 13).unwrap(),
        ];
        for expected_day in expected {
            game_day_scheduler.advance().unwrap();
            assert_eq!(game_day_scheduler.current_day(), &expected_day);
        }
    }

    #[test]
    fn advance_with_one_game_per_week_never_repeats_a_day_after_a_lost_week() {
        // Saturday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();
        // Both days of the following weekend are unavailable.
        let excluded_dates = vec![
            NaiveDate::from_ymd_opt(2026, 5, 23).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
        ];
        let date_configuration = DateConfiguration::new(
            start_day,
            vec![Weekday::Sat, Weekday::Sun],
            excluded_dates,
            true,
        );
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &date_configuration).unwrap();

        let mut days = vec![*game_day_scheduler.current_day()];
        for _ in 0..4 {
            game_day_scheduler.advance().unwrap();
            days.push(*game_day_scheduler.current_day());
        }

        let mut deduplicated = days.clone();
        deduplicated.dedup();
        assert_eq!(days, deduplicated, "a day was scheduled twice: {days:?}");

        assert_eq!(
            days,
            [
                NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
                // The lost weekend skips straight to the week after it.
                NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
                NaiveDate::from_ymd_opt(2026, 6, 6).unwrap(),
                NaiveDate::from_ymd_opt(2026, 6, 13).unwrap(),
                NaiveDate::from_ymd_opt(2026, 6, 20).unwrap(),
            ]
        );
    }
}
