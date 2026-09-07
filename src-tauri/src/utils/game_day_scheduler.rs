use chrono::{Datelike, Days, NaiveDate, Weekday};

use crate::errors::AppError;
use crate::utils::game_time_scheduler::GameTimeScheduler;

pub struct GameDayScheduler<'a> {
    game_days: &'a [Weekday],
    current_day: NaiveDate,
    excluded_dates: &'a [NaiveDate],
}

impl<'a> GameDayScheduler<'a> {
    pub fn new(
        start_day: &'a NaiveDate,
        game_days: &'a [Weekday],
        excluded_dates: &'a [NaiveDate],
    ) -> Result<Self, AppError> {
        if game_days.is_empty() {
            return Err(AppError::EmptyGameDays);
        }
        let current_day = Self::next_date(start_day, game_days, excluded_dates)?;
        Ok(Self {
            game_days,
            current_day,
            excluded_dates,
        })
    }

    pub fn current_day(&self) -> &NaiveDate {
        &self.current_day
    }

    // Advance the day if needed
    pub fn advance(&mut self) -> Result<(), AppError> {
        let next_day = self
            .current_day
            .checked_add_days(Days::new(1))
            .expect("a single day cannot overflow NaiveDate's range");
        self.current_day = Self::next_date(&next_day, self.game_days, self.excluded_dates)?;
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
        let result = GameDayScheduler::new(&start_day, &[], &[]);
        assert!(matches!(result, Err(AppError::EmptyGameDays)));
    }

    #[test]
    fn new_accepts_non_empty_game_days() {
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let result = GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat], &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn new_finds_next_game_day_across_year_boundary() {
        // Sunday, Jan 1 2023 falls in ISO week 52 of 2022, not 2023.
        let start_day = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let game_day_scheduler = GameDayScheduler::new(&start_day, &[Weekday::Mon], &[]).unwrap();

        let next_monday = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_monday);
    }

    #[test]
    fn test_advance() {
        // Wednesday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat], &[]).unwrap();

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
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat], &[]).unwrap();

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
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat], &excluded_dates)
                .unwrap();

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

        let game_day_scheduler =
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat], &excluded_dates)
                .unwrap();

        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_saturday);
    }
}
