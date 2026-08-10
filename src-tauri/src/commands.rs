use chrono::{DateTime, Datelike};
use chrono_tz::Tz;
use rust_i18n::t;
use rust_xlsxwriter::workbook::Workbook;
use rust_xlsxwriter::worksheet::Worksheet;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};

use crate::errors::AppError;
use crate::impls::round_robin::RoundRobin;
use crate::impls::single_elimination::SingleElimination;
use crate::types::game::Game;
use crate::types::season::{Season, SeasonConfig};
use crate::types::team::Team;
use crate::types::tournament_selection::TournamentSelection;

#[tauri::command]
pub fn tauri_generate_schedule(
    teams: Vec<Team>,
    season_config: SeasonConfig,
) -> Result<Vec<Game>, AppError> {
    let season = Season::new(
        season_config,
        TournamentSelection::new(RoundRobin, SingleElimination::new(true)),
    );
    let schedule = season.compute_season_schedule(&teams)?;
    Ok(schedule)
}

#[tauri::command]
pub fn generate_excel_schedule(
    schedule: Vec<Game>,
    number_fields: u16,
    output_directory_path: String,
    language: &str,
) -> Result<(), AppError> {
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

    let mut day_index = 1;

    for game in sorted_schedule.iter() {
        let game_day = game.get_game_day();
        if game_day.date_naive() != current_day {
            row += 2;
            write_day_row(worksheet, row, game_day, day_index, language)?;
            day_index += 1;
            current_day = game_day.date_naive();
            row += 1;
            write_header_row(worksheet, row, 2, language)?;
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

        write_game_row(worksheet, game, row, current_games, language)?;
        current_games += 1;
    }
    let current_date = chrono::Utc::now();
    let year = current_date.year();

    worksheet.autofit();

    // Save the file to disk.
    //workbook.save(format!("calendrier_{year}.xlsx"))?;
    workbook.save(format!(
        "{output_directory_path}/calendrier_{year}_{language}.xlsx"
    ))?;
    Ok(())
}

fn write_day_row(
    worksheet: &mut Worksheet,
    row: u32,
    game_day: &DateTime<Tz>,
    day_index: u32,
    language: &str,
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
        &t!(
            "day",
            locale = language,
            day_index = day_index,
            day = day,
            month = month_str.name(),
            year = year
        ),
        &title_format,
    )?;
    worksheet.set_row_height(row, 25)?;
    Ok(())
}

fn write_header_row(
    worksheet: &mut Worksheet,
    row: u32,
    number_fields: u16,
    language: &str,
) -> Result<(), AppError> {
    let format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Silver);
    for i in 0..number_fields {
        worksheet.write_with_format(row, 5 * i, t!("time", locale = language), &format)?;
        worksheet.write_with_format(row, 5 * i + 1, t!("home", locale = language), &format)?;
        worksheet.write_with_format(row, 5 * i + 2, t!("vs", locale = language), &format)?;
        worksheet.write_with_format(row, 5 * i + 3, t!("away", locale = language), &format)?;
        worksheet.write_with_format(row, 5 * i + 4, t!("referee", locale = language), &format)?;
    }
    worksheet.write_with_format(
        row,
        number_fields * 5,
        t!("bye", locale = language),
        &format,
    )?;
    worksheet.set_row_height(row, 18)?;
    Ok(())
}

fn write_game_row(
    worksheet: &mut Worksheet,
    game: &Game,
    row: u32,
    offset: u16,
    language: &str,
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
    worksheet.write_with_format(row, 2 + offset * 5, t!("vs", locale = language), &format_vs)?;
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
