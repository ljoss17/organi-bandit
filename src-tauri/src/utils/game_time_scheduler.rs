use crate::types::game_time::GameTime;

pub struct GameTimeScheduler<'a> {
    start_time: &'a GameTime,
    time_between_games: &'a GameTime,
    number_of_fields: u32,
    current_time: GameTime,
    current_games_per_time: u32,
    start_break: GameTime,
    end_break: GameTime,
    hard_stop: GameTime,
}

impl<'a> GameTimeScheduler<'a> {
    pub fn new(
        start_time: &'a GameTime,
        time_between_games: &'a GameTime,
        number_of_fields: u32,
        start_break: &'a GameTime,
        end_break: &'a GameTime,
    ) -> Self {
        // When this instance starts before the break (start_break -
        // start_time succeeds), mirror that leg's duration onto the other
        // side of the break so both sides offer the same amount of daily
        // room: hard_stop = end_break + (start_break - start_time). When it
        // doesn't (this instance already starts at or after the break), the
        // subtraction underflows and there's nothing to mirror — this side
        // already is the reference window, so it just keeps the real fixed
        // boundary.
        let hard_stop = match *start_break - *start_time {
            Ok(duration) => *end_break + duration,
            Err(_) => Self::default_hard_stop(),
        };

        Self {
            start_time,
            time_between_games,
            number_of_fields,
            current_time: *start_time,
            current_games_per_time: 1,
            start_break: *start_break,
            end_break: *end_break,
            hard_stop,
        }
    }

    pub fn current_time(&self) -> &GameTime {
        &self.current_time
    }

    pub fn current_games_per_time(&self) -> u32 {
        self.current_games_per_time
    }

    // No new game should be scheduled past this time of day. A day only has
    // so many reasonable hours to play in, so once this is reached the
    // remaining games for that "round" need to spill onto the next day
    // instead of wrapping the clock back around within the same day.
    fn default_hard_stop() -> GameTime {
        GameTime::new(17, 0).expect("17:00 is always a valid time")
    }

    pub fn is_past_hard_stop(&self) -> bool {
        self.current_time > self.hard_stop
    }

    // Advance the time
    pub fn try_advance(&mut self) {
        if self.current_games_per_time == self.number_of_fields {
            if self.current_time + *self.time_between_games > self.start_break
                && self.current_time + *self.time_between_games < self.end_break
            {
                self.current_time = self.end_break;
            } else {
                self.current_time = self.current_time + *self.time_between_games;
            }
            self.current_games_per_time = 1;
        } else {
            self.current_games_per_time += 1;
        }
    }

    // Reset game time to initial values
    pub fn reset(&mut self) {
        self.current_time = *self.start_time;
        self.current_games_per_time = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_advance() {
        let start_time = GameTime::new(9, 30).unwrap();
        let time_between_games = GameTime::new(1, 30).unwrap();
        let start_break = GameTime::new(12, 0).unwrap();
        let end_break = GameTime::new(13, 30).unwrap();

        let mut game_time_scheduler = GameTimeScheduler::new(
            &start_time,
            &time_between_games,
            2,
            &start_break,
            &end_break,
        );

        // With 2 fields, only the second try_advance() call will change the time
        assert_eq!(game_time_scheduler.current_time(), &start_time);

        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &start_time);

        let expected_next_time = start_time + time_between_games;

        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &expected_next_time);
    }

    #[test]
    fn test_try_advance_during_break() {
        let start_time = GameTime::new(11, 0).unwrap();
        let time_between_games = GameTime::new(1, 30).unwrap();
        let start_break = GameTime::new(12, 0).unwrap();
        let end_break = GameTime::new(13, 30).unwrap();

        let mut game_time_scheduler = GameTimeScheduler::new(
            &start_time,
            &time_between_games,
            2,
            &start_break,
            &end_break,
        );

        // With 2 fields, only the second try_advance() call will change the time
        assert_eq!(game_time_scheduler.current_time(), &start_time);

        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &start_time);

        // Since the next time will happen during the break,
        // the current_time is set to the end of the break
        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &end_break);
    }

    #[test]
    fn test_reset_then_advance() {
        let start_time = GameTime::new(9, 30).unwrap();
        let time_between_games = GameTime::new(1, 30).unwrap();
        let start_break = GameTime::new(12, 0).unwrap();
        let end_break = GameTime::new(13, 30).unwrap();

        let mut game_time_scheduler = GameTimeScheduler::new(
            &start_time,
            &time_between_games,
            2,
            &start_break,
            &end_break,
        );

        // Move state away from its initial values first, so reset() is actually exercised
        game_time_scheduler.try_advance();
        game_time_scheduler.try_advance();
        game_time_scheduler.try_advance();

        game_time_scheduler.reset();

        // Expect current time to be the start time after reset
        assert_eq!(game_time_scheduler.current_time(), &start_time);

        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &start_time);

        let expected_next_time = start_time + time_between_games;

        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &expected_next_time);
    }

    #[test]
    fn is_past_hard_stop_false_before_17_00() {
        let start_time = GameTime::new(16, 30).unwrap();
        let time_between_games = GameTime::new(0, 30).unwrap();
        let start_break = GameTime::new(12, 0).unwrap();
        let end_break = GameTime::new(13, 30).unwrap();

        let game_time_scheduler = GameTimeScheduler::new(
            &start_time,
            &time_between_games,
            1,
            &start_break,
            &end_break,
        );

        assert!(!game_time_scheduler.is_past_hard_stop());
    }

    #[test]
    fn is_past_hard_stop_true_after_17_00() {
        let start_time = GameTime::new(17, 30).unwrap();
        let time_between_games = GameTime::new(0, 30).unwrap();
        let start_break = GameTime::new(12, 0).unwrap();
        let end_break = GameTime::new(13, 30).unwrap();

        let game_time_scheduler = GameTimeScheduler::new(
            &start_time,
            &time_between_games,
            1,
            &start_break,
            &end_break,
        );

        assert!(game_time_scheduler.is_past_hard_stop());
    }
}
