use chrono::{Datelike, Days, NaiveDate, Weekday};

use crate::errors::AppError;

pub struct GameDayScheduler<'a> {
    game_days: &'a [Weekday],
    current_day: NaiveDate,
}

impl<'a> GameDayScheduler<'a> {
    pub fn new(start_day: &'a NaiveDate, game_days: &'a [Weekday]) -> Result<Self, AppError> {
        if game_days.is_empty() {
            return Err(AppError::EmptyGameDays);
        }
        let start_weekday = start_day.weekday();
        let current_day = if !game_days.contains(&start_weekday) {
            let offset = game_days
                .iter()
                .map(|weekday| weekday.days_since(start_weekday))
                .min()
                .expect("game_days is non-empty, checked above");
            start_day
                .checked_add_days(Days::new(offset as u64))
                .expect("offset is at most 6 days, cannot overflow NaiveDate's range")
        } else {
            *start_day
        };
        Ok(Self {
            game_days,
            current_day,
        })
    }

    pub fn current_day(&self) -> &NaiveDate {
        &self.current_day
    }

    // Advance the day if needed
    pub fn advance(&mut self) {
        let current_weekday = self.current_day.weekday();
        let days_to_next = self
            .game_days
            .iter()
            .map(|weekday| {
                let days = weekday.days_since(current_weekday);
                if days == 0 {
                    7
                } else {
                    days
                }
            })
            .min()
            .expect("game_days is non-empty, enforced in new()");
        self.current_day = self
            .current_day
            .checked_add_days(Days::new(days_to_next as u64))
            .expect("offset is at most 7 days, cannot overflow NaiveDate's range");
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
        let result = GameDayScheduler::new(&start_day, &[]);
        assert!(matches!(result, Err(AppError::EmptyGameDays)));
    }

    #[test]
    fn new_accepts_non_empty_game_days() {
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let result = GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat]);
        assert!(result.is_ok());
    }

    #[test]
    fn new_finds_next_game_day_across_year_boundary() {
        // Sunday, Jan 1 2023 falls in ISO week 52 of 2022, not 2023.
        let start_day = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let game_day_scheduler = GameDayScheduler::new(&start_day, &[Weekday::Mon]).unwrap();

        let next_monday = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
        assert_eq!(game_day_scheduler.current_day(), &next_monday);
    }

    #[test]
    fn test_advance() {
        // Wednesday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat]).unwrap();

        assert_eq!(game_day_scheduler.current_day(), &start_day);

        // Next Saturday
        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();

        game_day_scheduler.advance();
        assert_eq!(game_day_scheduler.current_day(), &next_saturday);

        // Next Tuesday
        let next_tuesday = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();

        game_day_scheduler.advance();
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
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat]).unwrap();

        // Next Saturday
        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();

        assert_eq!(game_day_scheduler.current_day(), &next_saturday);

        // Next Tuesday
        let next_tuesday = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();

        game_day_scheduler.advance();
        assert_eq!(game_day_scheduler.current_day(), &next_tuesday);

        // Next Tuesday
        let next_saturday = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();

        game_day_scheduler.advance();
        assert_eq!(game_day_scheduler.current_day(), &next_saturday);
    }
}
