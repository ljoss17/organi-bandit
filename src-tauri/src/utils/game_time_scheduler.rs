use crate::types::{configurations::time::TimeConfiguration, game_time::GameTime};

pub struct GameTimeScheduler<'a> {
    time_configuration: &'a TimeConfiguration,
    // Where this leg's clock starts and resets to each day. Distinct from
    // the season's own start_time, since the leg played after the break
    // starts at end_break instead.
    leg_start_time: GameTime,
    number_of_fields: u32,
    current_time: GameTime,
    current_games_per_time: u32,
    hard_stop: GameTime,
}

impl<'a> GameTimeScheduler<'a> {
    pub fn new(
        time_configuration: &'a TimeConfiguration,
        leg_start_time: &GameTime,
        number_of_fields: u32,
    ) -> Self {
        // When this leg starts before the break (start_break -
        // leg_start_time succeeds), mirror that leg's duration onto the
        // other side of the break so both sides offer the same amount of
        // daily room: hard_stop = end_break + (start_break -
        // leg_start_time). When it doesn't (this leg already starts at or
        // after the break), the subtraction underflows and there's nothing
        // to mirror — this side already is the reference window, so it just
        // keeps the real fixed boundary.
        let leg_start_time = *leg_start_time;
        let hard_stop = match *time_configuration.start_break() - leg_start_time {
            Ok(duration) => *time_configuration.end_break() + duration,
            Err(_) => Self::default_hard_stop(),
        };

        Self {
            time_configuration,
            leg_start_time,
            number_of_fields,
            current_time: leg_start_time,
            current_games_per_time: 1,
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

    // A game has to *finish* by the hard stop, not merely kick off before
    // it, so the game's own duration counts against the boundary too.
    pub fn is_past_hard_stop(&self) -> bool {
        self.current_time + *self.time_configuration.game_duration() > self.hard_stop
    }

    // Advance the time
    pub fn try_advance(&mut self) {
        if self.current_games_per_time == self.number_of_fields {
            let next_time = self.current_time + self.time_configuration.interval_between_games();
            // The next slot clashes with the break when the game played in
            // it would still be running once the break starts — judged on
            // when the game ends, not just when it kicks off, so a game
            // can't overrun into the break by its own duration.
            let starts_before_break_ends = next_time < *self.time_configuration.end_break();
            let runs_past_break_start = next_time + *self.time_configuration.game_duration()
                > *self.time_configuration.start_break();
            if starts_before_break_ends && runs_past_break_start {
                self.current_time = *self.time_configuration.end_break();
            } else {
                self.current_time = next_time;
            }
            self.current_games_per_time = 1;
        } else {
            self.current_games_per_time += 1;
        }
    }

    // Reset game time to initial values
    pub fn reset(&mut self) {
        self.current_time = self.leg_start_time;
        self.current_games_per_time = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_advance() {
        let time_configuration = TimeConfiguration::new(
            GameTime::new(9, 30).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
        );

        let mut game_time_scheduler =
            GameTimeScheduler::new(&time_configuration, time_configuration.start_time(), 2);

        // With 2 fields, only the second try_advance() call will change the time
        assert_eq!(
            game_time_scheduler.current_time(),
            time_configuration.start_time()
        );

        game_time_scheduler.try_advance();
        assert_eq!(
            game_time_scheduler.current_time(),
            time_configuration.start_time()
        );

        let expected_next_time =
            *time_configuration.start_time() + time_configuration.interval_between_games();

        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &expected_next_time);
    }

    #[test]
    fn test_try_advance_during_break() {
        let time_configuration = TimeConfiguration::new(
            GameTime::new(11, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
        );

        let mut game_time_scheduler =
            GameTimeScheduler::new(&time_configuration, time_configuration.start_time(), 2);

        // With 2 fields, only the second try_advance() call will change the time
        assert_eq!(
            game_time_scheduler.current_time(),
            time_configuration.start_time()
        );

        game_time_scheduler.try_advance();
        assert_eq!(
            game_time_scheduler.current_time(),
            time_configuration.start_time()
        );

        // Since the next time will happen during the break,
        // the current_time is set to the end of the break
        game_time_scheduler.try_advance();
        assert_eq!(
            game_time_scheduler.current_time(),
            time_configuration.end_break()
        );
    }

    #[test]
    fn test_reset_then_advance() {
        let time_configuration = TimeConfiguration::new(
            GameTime::new(9, 30).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
        );

        let mut game_time_scheduler =
            GameTimeScheduler::new(&time_configuration, time_configuration.start_time(), 2);

        // Move state away from its initial values first, so reset() is actually exercised
        game_time_scheduler.try_advance();
        game_time_scheduler.try_advance();
        game_time_scheduler.try_advance();

        game_time_scheduler.reset();

        // Expect current time to be the start time after reset
        assert_eq!(
            game_time_scheduler.current_time(),
            time_configuration.start_time()
        );

        game_time_scheduler.try_advance();
        assert_eq!(
            game_time_scheduler.current_time(),
            time_configuration.start_time()
        );

        let expected_next_time =
            *time_configuration.start_time() + time_configuration.interval_between_games();

        game_time_scheduler.try_advance();
        assert_eq!(game_time_scheduler.current_time(), &expected_next_time);
    }

    #[test]
    fn is_past_hard_stop_false_before_17_00() {
        let time_configuration = TimeConfiguration::new(
            GameTime::new(16, 30).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(0, 0).unwrap(),
        );

        let game_time_scheduler =
            GameTimeScheduler::new(&time_configuration, time_configuration.start_time(), 1);

        assert!(!game_time_scheduler.is_past_hard_stop());
    }

    #[test]
    fn is_past_hard_stop_true_after_17_00() {
        let time_configuration = TimeConfiguration::new(
            GameTime::new(17, 30).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(0, 0).unwrap(),
        );

        let game_time_scheduler =
            GameTimeScheduler::new(&time_configuration, time_configuration.start_time(), 1);

        assert!(game_time_scheduler.is_past_hard_stop());
    }
}
