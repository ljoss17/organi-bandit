use chrono::{Datelike, Days, NaiveDate, Weekday};

pub struct GameDayScheduler<'a> {
    game_days: &'a [Weekday],
    current_day: NaiveDate,
}

impl<'a> GameDayScheduler<'a> {
    pub fn new(start_day: &'a NaiveDate, game_days: &'a [Weekday]) -> Self {
        let start_weekday = start_day.weekday();
        let current_day = if !game_days.contains(&start_weekday) {
            let next_weekday = game_days
                .iter()
                .min_by_key(|weekday| weekday.days_since(start_weekday))
                .unwrap();
            NaiveDate::from_isoywd_opt(start_day.year(), start_day.iso_week().week(), *next_weekday)
                .unwrap()
        } else {
            *start_day
        };
        Self {
            game_days,
            current_day,
        }
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
            .unwrap();
        self.current_day = self
            .current_day
            .checked_add_days(Days::new(days_to_next as u64))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::TimeZone;
    use chrono_tz::Europe::Zurich;

    #[test]
    fn test_advance() {
        // Wednesday
        let start_day = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let mut game_day_scheduler =
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat]);

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
            GameDayScheduler::new(&start_day, &[Weekday::Tue, Weekday::Sat]);

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
