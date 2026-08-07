use chrono::NaiveDate;

use crate::errors::AppError;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
use crate::types::season::SeasonConfig;
use crate::types::team::Team;

pub trait Tournament {
    fn name(&self) -> String;

    fn validate_parameters(
        &self,
        teams: &[Team],
        start_date: &NaiveDate,
        start_time: &GameTime,
        time_between_games: &GameTime,
    ) -> Result<(), AppError>;

    fn compute_schedule(
        &self,
        teams: &[Team],
        start_day: &NaiveDate,
        season_config: &SeasonConfig,
        with_referees: bool,
    ) -> Result<Vec<Game>, AppError>;
}
