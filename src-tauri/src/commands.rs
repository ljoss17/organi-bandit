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
use crate::types::game_time::GameTime;
use crate::types::season::Season;
use crate::types::team::Team;
use crate::types::tournament::TournamentSelection;

#[tauri::command]
pub fn tauri_generate_schedule(
    teams: Vec<Team>,
    start_day_str: String,
    end_day_str: String,
    game_times: Vec<GameTime>,
    number_fields: usize,
    game_days_str: Vec<String>,
) -> Result<Vec<Game>, AppError> {
    let start_day_values = start_day_str.split("-").collect::<Vec<_>>();
    let start_day = Zurich
        .with_ymd_and_hms(
            start_day_values[0].parse()?,
            start_day_values[1].parse()?,
            start_day_values[2].parse()?,
            0,
            0,
            0,
        )
        .single()
        .ok_or(AppError::CreateDateTimeError(
            start_day_values.into_iter().map(String::from).collect(),
        ))?;
    let end_day_values = end_day_str.split("-").collect::<Vec<_>>();
    let end_day = Zurich
        .with_ymd_and_hms(
            end_day_values[0].parse()?,
            end_day_values[1].parse()?,
            end_day_values[2].parse()?,
            0,
            0,
            0,
        )
        .single()
        .ok_or(AppError::CreateDateTimeError(
            end_day_values.into_iter().map(String::from).collect(),
        ))?;
    let season = Season::new(
        start_day,
        end_day,
        game_times,
        number_fields,
        TournamentSelection::new(RoundRobin, SingleElimination::new(true)),
        parse_weekday(game_days_str)?,
    );
    let schedule = season.compute_season_schedule(&teams)?;
    Ok(schedule)
}

#[tauri::command]
pub fn generate_excel_schedule(schedule: Vec<Game>, number_fields: u16) -> Result<(), AppError> {
    // Create a new Excel file object.
    let mut workbook = Workbook::new();

    // Add a worksheet to the workbook.
    let worksheet = workbook.add_worksheet();

    let mut sorted_schedule = schedule.clone();
    sorted_schedule.sort_by_key(|game| *game.get_game_day());

    let mut row = 0;
    let mut current_day = sorted_schedule[sorted_schedule.len() - 1]
        .get_game_day()
        .date_naive();
    let mut current_time = None;
    let mut current_games = 0;

    // Resize "VS" columns
    worksheet.set_column_width(2, 4)?;
    worksheet.set_column_width(7, 4)?;

    for game in sorted_schedule.iter() {
        let game_day = game.get_game_day();
        if game_day.date_naive() != current_day {
            row += 2;
            write_day_row(worksheet, row, game_day)?;
            current_day = game_day.date_naive();
            row += 1;
            write_header_row(worksheet, row, 2)?;
            row += 1;
            current_games = 0;
        } else if current_time != Some(game_day.time()) {
            row += 1;
            current_games = 0;
        }
        current_time = Some(game_day.time());

        if game.get_home_team().get_name() == "Bye" || game.get_away_team().get_name() == "Bye" {
            write_bye_game(worksheet, game, row, number_fields)?;
            continue;
        }

        write_game_row(worksheet, game, row, current_games)?;
        current_games += 1;
    }
    let current_date = chrono::Utc::now();
    let year = current_date.year();

    worksheet.autofit();

    // Save the file to disk.
    workbook.save(format!("calendrier_{year}.xlsx"))?;
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
    let month_str = chrono::Month::try_from(month as u8)?;

    let title_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Red);

    worksheet.merge_range(
        row,
        0,
        row,
        10,
        &format!("Day 1 - {day} {} {year}", month_str.name()),
        &title_format,
    )?;
    worksheet.set_row_height(row, 25)?;
    Ok(())
}

fn write_header_row(
    worksheet: &mut Worksheet,
    row: u32,
    number_fields: u16,
) -> Result<(), AppError> {
    let format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Silver);
    for i in 0..number_fields {
        worksheet.write_with_format(row, 5 * i, "Horaire", &format)?;
        worksheet.write_with_format(row, 5 * i + 1, "Domicile", &format)?;
        worksheet.write_with_format(row, 5 * i + 2, "VS", &format)?;
        worksheet.write_with_format(row, 5 * i + 3, "Visiteur", &format)?;
        worksheet.write_with_format(row, 5 * i + 4, "Arbitre", &format)?;
    }
    worksheet.write_with_format(row, number_fields * 5, "Bye", &format)?;
    worksheet.set_row_height(row, 18)?;
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

    worksheet.write_with_format(
        row,
        offset * 5,
        game.get_game_time()?.to_string(),
        &format_team,
    )?;
    worksheet.write_with_format(
        row,
        1 + offset * 5,
        game.get_home_team().get_name(),
        &format_team,
    )?;
    worksheet.write_with_format(row, 2 + offset * 5, "VS", &format_vs)?;
    worksheet.write_with_format(
        row,
        3 + offset * 5,
        game.get_away_team().get_name(),
        &format_team,
    )?;
    if let Some(referee) = game.get_referee() {
        worksheet.write_with_format(row, 4 + offset * 5, referee.get_name(), &format_team)?;
    }

    worksheet.set_row_height(row, 18)?;
    Ok(())
}

fn write_bye_game(
    worksheet: &mut Worksheet,
    game: &Game,
    row: u32,
    number_fields: u16,
) -> Result<(), AppError> {
    let format_bye = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Silver);

    let bye_team = if game.get_home_team().get_name() != "Bye" {
        game.get_home_team().get_name()
    } else {
        game.get_away_team().get_name()
    };

    worksheet.write_with_format(row, number_fields * 5, bye_team, &format_bye)?;
    Ok(())
}
