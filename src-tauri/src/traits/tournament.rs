use chrono::NaiveDate;

use crate::types::game::Game;
use crate::types::team::Team;

pub trait Tournament {
    fn validate_parameters(
        teams: &[Team],
        game_days: Vec<NaiveDate>,
        max_games_per_day: usize,
    ) -> bool;

    fn compute_schedule(
        teams: &[Team],
        game_days: Vec<NaiveDate>,
        max_games_per_day: usize,
    ) -> Vec<Game>;
}
