use chrono::{TimeZone, Weekday};
use chrono_tz::Europe::Zurich;

use crate::errors::AppError;
use crate::impls::round_robin::RoundRobin;
use crate::impls::single_elimination::SingleElimination;
use crate::types::game::Game;
use crate::types::season::Season;
use crate::types::team::Team;
use crate::types::tournament::TournamentSelection;

#[tauri::command]
pub fn tauri_generate_schedule(
    teams: Vec<Team>,
    start_day_str: String,
    end_day_str: String,
    max_games_per_day: usize,
    game_days_str: Vec<String>,
) -> Result<Vec<Game>, AppError> {
    let start_day_values = start_day_str.split("-").collect::<Vec<_>>();
    let start_day = Zurich
        .with_ymd_and_hms(
            start_day_values[0].parse().unwrap(),
            start_day_values[1].parse().unwrap(),
            start_day_values[2].parse().unwrap(),
            0,
            0,
            0,
        )
        .unwrap();
    let end_day_values = end_day_str.split("-").collect::<Vec<_>>();
    let end_day = Zurich
        .with_ymd_and_hms(
            end_day_values[0].parse().unwrap(),
            end_day_values[1].parse().unwrap(),
            end_day_values[2].parse().unwrap(),
            0,
            0,
            0,
        )
        .unwrap();
    let season = Season::new(
        start_day,
        end_day,
        max_games_per_day,
        TournamentSelection::new(RoundRobin, SingleElimination::new(false)),
        parse_weekday(game_days_str)?,
    );
    let schedule = season.compute_season_schedule(&teams)?;
    Ok(schedule)
}

fn parse_weekday(days_str: Vec<String>) -> Result<Vec<Weekday>, AppError> {
    days_str
        .iter()
        .map(|day| day.parse().map_err(AppError::WeekdayParseError))
        .collect()
}
