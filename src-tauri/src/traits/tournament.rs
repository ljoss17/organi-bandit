use chrono::NaiveDate;

use crate::types::game::Game;
use crate::types::team::Team;

pub trait Tournament {
    fn validate_parameters(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        max_games_per_day: usize,
    ) -> bool;

    fn compute_schedule(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        max_games_per_day: usize,
    ) -> Vec<Game>;
}
