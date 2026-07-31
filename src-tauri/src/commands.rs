use chrono::{DateTime, Datelike, TimeZone, Weekday};
use chrono_tz::Europe::Zurich;
use chrono_tz::Tz;
use rust_xlsxwriter::workbook::Workbook;
use rust_xlsxwriter::worksheet::Worksheet;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};

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
        TournamentSelection::new(RoundRobin, SingleElimination::new(true)),
        parse_weekday(game_days_str)?,
    );
    let schedule = season.compute_season_schedule(&teams)?;
    Ok(schedule)
}

#[tauri::command]
pub fn generate_excel_schedule(schedule: Vec<Game>) -> Result<(), AppError> {
    // Create a new Excel file object.
    let mut workbook = Workbook::new();

    // Add a worksheet to the workbook.
    let worksheet = workbook.add_worksheet();

    let mut row = 0;
    let mut current_day = schedule[schedule.len() - 1].get_game_day();
    let mut current_games = 0;

    // Resize "VS" columns
    worksheet.set_column_width(2, 4).unwrap();
    worksheet.set_column_width(7, 4).unwrap();

    for game in schedule.iter() {
        let game_day = game.get_game_day();
        if game_day != current_day {
            row += 2;
            write_day_row(worksheet, row, game_day)?;
            current_day = game_day;
            row += 1;
            write_header_row(worksheet, row, 2).unwrap();
            row += 1;
            current_games = 0;
        }
        if current_games == 2 {
            row += 1;
            current_games = 0;
        }
        write_game_row(worksheet, game, row, current_games).unwrap();
        current_games += 1;
    }
    let current_date = chrono::Utc::now();
    let year = current_date.year();

    worksheet.autofit();

    // Save the file to disk.
    workbook.save(format!("calendrier_{year}.xlsx")).unwrap();
    Ok(())
}

fn parse_weekday(days_str: Vec<String>) -> Result<Vec<Weekday>, AppError> {
    days_str
        .iter()
        .map(|day| day.parse().map_err(AppError::WeekdayParseError))
        .collect()
}

fn write_day_row(
    worksheet: &mut Worksheet,
    row: u32,
    game_day: &DateTime<Tz>,
) -> Result<(), AppError> {
    let day = game_day.day();
    let month = game_day.month();
    let year = game_day.year();
    let month_str = chrono::Month::try_from(month as u8).unwrap();

    let title_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Red);

    worksheet
        .merge_range(
            row,
            0,
            row,
            9,
            &format!("Day 1 - {day} {} {year}", month_str.name()),
            &title_format,
        )
        .unwrap();
    worksheet.set_row_height(row, 25).unwrap();
    Ok(())
}

fn write_header_row(
    worksheet: &mut Worksheet,
    row: u32,
    max_games_per_day: u16,
) -> Result<(), AppError> {
    let format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Silver);
    for i in 0..max_games_per_day {
        worksheet
            .write_with_format(row, 5 * i, "Horaire", &format)
            .unwrap();
        worksheet
            .write_with_format(row, 5 * i + 1, "Domicile", &format)
            .unwrap();
        worksheet
            .write_with_format(row, 5 * i + 2, "VS", &format)
            .unwrap();
        worksheet
            .write_with_format(row, 5 * i + 3, "Visiteur", &format)
            .unwrap();
        worksheet
            .write_with_format(row, 5 * i + 4, "Arbitre", &format)
            .unwrap();
    }
    worksheet.set_row_height(row, 18).unwrap();
    Ok(())
}

fn write_game_row(
    worksheet: &mut Worksheet,
    game: &Game,
    row: u32,
    offset: u16,
) -> Result<(), AppError> {
    let format_team = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let format_vs = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Silver);
    worksheet
        .write_with_format(
            row,
            1 + offset * 5,
            game.get_home_team().get_name(),
            &format_team,
        )
        .unwrap();
    worksheet
        .write_with_format(row, 2 + offset * 5, "VS", &format_vs)
        .unwrap();
    worksheet
        .write_with_format(
            row,
            3 + offset * 5,
            game.get_away_team().get_name(),
            &format_team,
        )
        .unwrap();

    worksheet.set_row_height(row, 18).unwrap();
    Ok(())
}
