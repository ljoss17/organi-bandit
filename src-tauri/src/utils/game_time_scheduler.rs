use crate::types::game_time::GameTime;

pub struct GameTimeScheduler<'a> {
    game_times: &'a [GameTime],
    time_index: usize,
    current_time: GameTime,
}

impl<'a> GameTimeScheduler<'a> {
    pub fn new(game_times: &'a [GameTime]) -> Self {
        Self {
            game_times,
            time_index: 0,
            current_time: game_times[0],
        }
    }

    pub fn current_time(&self) -> &GameTime {
        &self.current_time
    }

    // Advance the time
    pub fn advance(&mut self) {
        self.time_index = (self.time_index + 1) % self.game_times.len();
        self.current_time = self.game_times[self.time_index];
    }

    // Reset game time to initial values
    pub fn reset(&mut self) {
        *self = Self::new(self.game_times);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advance() {
        let game_times = [GameTime::new(9, 30).unwrap(), GameTime::new(2, 30).unwrap()];

        let mut game_time_scheduler = GameTimeScheduler::new(&game_times);

        assert_eq!(game_time_scheduler.current_time(), &game_times[0]);

        game_time_scheduler.advance();
        assert_eq!(game_time_scheduler.current_time(), &game_times[1]);

        game_time_scheduler.advance();
        assert_eq!(game_time_scheduler.current_time(), &game_times[0]);
    }
}
