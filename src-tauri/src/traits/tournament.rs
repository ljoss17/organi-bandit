use chrono::NaiveDate;

use crate::errors::AppError;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
use crate::types::team::Team;

pub trait Tournament {
    fn name(&self) -> String;

    fn validate_parameters(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        max_games_per_day: usize,
    ) -> Result<(), AppError>;

    fn compute_schedule(
        &self,
        teams: &[Team],
        game_days: &[NaiveDate],
        game_times: &[GameTime],
        number_fields: usize,
        with_referees: bool,
    ) -> Result<Vec<Game>, AppError>;
}
