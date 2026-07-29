use chrono::NaiveDate;

pub struct GameDayScheduler<'a> {
    game_days: &'a [NaiveDate],
    day_index: usize,
    current_day: NaiveDate,
    remaining_games_today: usize,
    max_games_per_day: usize,
}

impl<'a> GameDayScheduler<'a> {
    pub fn new(game_days: &'a [NaiveDate], max_games_per_day: usize) -> Self {
        Self {
            game_days,
            day_index: 1,
            current_day: game_days[0],
            remaining_games_today: max_games_per_day,
            max_games_per_day,
        }
    }

    pub fn current_day(&self) -> &NaiveDate {
        &self.current_day
    }

    // Advance the day if needed
    pub fn try_advance(&mut self) {
        self.remaining_games_today -= 1;
        if self.remaining_games_today == 0 {
            self.roll_to_next_day();
        }
    }

    // Force advance the day needed
    pub fn try_force_advance(&mut self) {
        if self.remaining_games_today != self.max_games_per_day {
            self.roll_to_next_day();
        }
    }

    fn roll_to_next_day(&mut self) {
        self.remaining_games_today = self.max_games_per_day;
        self.current_day = self.game_days[self.day_index];
        self.day_index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, Datelike, TimeZone, Weekday};
    use chrono_tz::Europe::Zurich;
    use chrono_tz::Tz;

    const MAX_GAMES_PER_DAY: usize = 3;

    fn start_day() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 5, 13, 8, 45, 0).unwrap()
    }

    fn end_day() -> DateTime<Tz> {
        Zurich.with_ymd_and_hms(2026, 6, 4, 18, 0, 0).unwrap()
    }

    #[test]
    fn test_try_advance() {
        let game_days = start_day()
            .date_naive()
            .iter_days()
            .take_while(|day| day <= &end_day().date_naive())
            .filter(|day| [Weekday::Sat].contains(&day.weekday()))
            .collect::<Vec<_>>();
        let mut game_day_scheduler = GameDayScheduler::new(&game_days, MAX_GAMES_PER_DAY);

        assert_eq!(game_day_scheduler.current_day(), &game_days[0]);

        // First advance should not change current day the first max_games_per_day - 1 calls
        for _ in 0..MAX_GAMES_PER_DAY - 1 {
            game_day_scheduler.try_advance();
            assert_eq!(game_day_scheduler.current_day(), &game_days[0]);
        }

        game_day_scheduler.try_advance();
        assert_eq!(game_day_scheduler.current_day(), &game_days[1]);
    }

    #[test]
    fn test_try_force_advance() {
        let game_days = start_day()
            .date_naive()
            .iter_days()
            .take_while(|day| day <= &end_day().date_naive())
            .filter(|day| [Weekday::Sat].contains(&day.weekday()))
            .collect::<Vec<_>>();
        let mut game_day_scheduler = GameDayScheduler::new(&game_days, 3);

        assert_eq!(game_day_scheduler.current_day(), &game_days[0]);

        game_day_scheduler.try_force_advance();
        assert_eq!(game_day_scheduler.current_day(), &game_days[0]);

        // Set the state of the scheduler such that:
        //  * try_advance() would not advance the day
        //  * try_force_advance will advance the day
        for _ in 0..MAX_GAMES_PER_DAY - 2 {
            game_day_scheduler.try_advance();
            assert_eq!(game_day_scheduler.current_day(), &game_days[0]);
        }

        game_day_scheduler.try_force_advance();
        assert_eq!(game_day_scheduler.current_day(), &game_days[1]);
    }
}
