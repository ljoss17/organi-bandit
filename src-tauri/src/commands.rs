use std::path::Path;

use chrono::{DateTime, Datelike, NaiveDate};
use chrono_tz::Tz;
use rust_i18n::t;
use rust_xlsxwriter::workbook::Workbook;
use rust_xlsxwriter::worksheet::Worksheet;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};

use crate::errors::AppError;
use crate::impls::round_robin::RoundRobin;
use crate::impls::single_elimination::SingleElimination;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
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
    start_break: GameTime,
    end_break: GameTime,
    number_fields: u16,
    output_directory_path: String,
    language: &str,
) -> Result<(), AppError> {
    // Create a new Excel file object.
    let mut workbook = Workbook::new();

    // Add a worksheet to the workbook.
    let worksheet = workbook.add_worksheet();

    let mut sorted_schedule = schedule;
    sorted_schedule.sort_by_key(|game| *game.get_game_day());

    let mut row = 0;
    let mut current_day: Option<NaiveDate> = None;
    let mut current_time = None;
    let mut current_games = 0;

    // Resize "VS" columns
    for i in 0..number_fields {
        worksheet.set_column_width(5 * i + 2, 4)?;
    }

    let mut day_index = 1;

    for game in sorted_schedule.iter() {
        let game_day = game.get_game_day();
        if current_day != Some(game_day.date_naive()) {
            row += 2;
            write_day_row(worksheet, row, game_day, day_index, number_fields, language)?;
            day_index += 1;
            current_day = Some(game_day.date_naive());
            row += 1;
            write_header_row(worksheet, row, number_fields, language)?;
            row += 1;
            current_games = 0;
        } else if current_time != Some(game_day.time()) {
            if current_time.map(GameTime::try_from).transpose()? < Some(start_break)
                && GameTime::try_from(game_day.time())? >= end_break
            {
                row += 1;
                write_break_row(
                    worksheet,
                    row,
                    start_break,
                    end_break,
                    number_fields,
                    language,
                )?;
            }
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
    let year = sorted_schedule
        .first()
        .map(|game| game.get_game_day().year())
        .unwrap_or_else(|| chrono::Utc::now().year());

    worksheet.autofit();

    // Save the file to disk, versioning the filename instead of overwriting
    // an existing one from a previous generation.
    let mut version = 1;
    let mut path = format!("{output_directory_path}/calendrier_{year}_{language}.xlsx");
    while Path::new(&path).exists() {
        version += 1;
        path = format!("{output_directory_path}/calendrier_{year}_{language}_v{version}.xlsx");
    }
    workbook.save(path)?;
    Ok(())
}

fn write_break_row(
    worksheet: &mut Worksheet,
    row: u32,
    start_break: GameTime,
    end_break: GameTime,
    number_fields: u16,
    language: &str,
) -> Result<(), AppError> {
    let break_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Gray);

    worksheet.merge_range(
        row,
        0,
        row,
        number_fields * 5,
        &t!(
            "break_row",
            locale = language,
            start_break = start_break,
            end_break = end_break,
        ),
        &break_format,
    )?;
    worksheet.set_row_height(row, 25)?;
    Ok(())
}

fn write_day_row(
    worksheet: &mut Worksheet,
    row: u32,
    game_day: &DateTime<Tz>,
    day_index: u32,
    number_fields: u16,
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
        number_fields * 5,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, remove_dir_all};
    use std::path::PathBuf;

    fn temp_output_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("organi-bandit-test-{name}"));
        create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn generate_excel_schedule_does_not_error_on_an_empty_schedule() {
        let output_dir = temp_output_dir("empty-schedule");

        let result = generate_excel_schedule(
            vec![],
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        );

        assert!(result.is_ok());
        remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn generate_excel_schedule_succeeds_for_a_single_day_schedule() {
        use crate::types::game_time::GameTime;

        let game = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            chrono::NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            GameTime::new(9, 0).unwrap(),
            None,
        )
        .unwrap();
        let output_dir = temp_output_dir("single-day-schedule");

        let result = generate_excel_schedule(
            vec![game],
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        );

        assert!(result.is_ok());
        remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn generate_excel_schedule_names_the_file_after_the_season_year_not_today() {
        use crate::types::game_time::GameTime;

        // The season's game is in 2030, deliberately far from whatever year
        // the test actually runs in, so the assertion can't accidentally
        // pass by coincidence.
        let game = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            chrono::NaiveDate::from_ymd_opt(2030, 5, 13).unwrap(),
            GameTime::new(9, 0).unwrap(),
            None,
        )
        .unwrap();
        let output_dir = temp_output_dir("season-year-filename");

        generate_excel_schedule(
            vec![game],
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        )
        .unwrap();

        assert!(output_dir.join("calendrier_2030_en.xlsx").exists());
        remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn generate_excel_schedule_versions_the_filename_instead_of_overwriting() {
        use crate::types::game_time::GameTime;

        fn game(home: &str, away: &str) -> Game {
            Game::new_with_game_day(
                Team::new(home, None),
                Team::new(away, None),
                chrono::NaiveDate::from_ymd_opt(2030, 5, 13).unwrap(),
                GameTime::new(9, 0).unwrap(),
                None,
            )
            .unwrap()
        }

        let output_dir = temp_output_dir("versioned-schedule");

        // First generation: plain filename, no suffix.
        generate_excel_schedule(
            vec![game("Home", "Away")],
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        )
        .unwrap();
        let first_path = output_dir.join("calendrier_2030_en.xlsx");
        assert!(first_path.exists());
        let first_bytes = std::fs::read(&first_path).unwrap();

        // Second generation with the same year/language/output dir: should
        // not touch the first file, should create a "_v2" file instead.
        generate_excel_schedule(
            vec![game("Other Home", "Other Away")],
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        )
        .unwrap();
        let second_path = output_dir.join("calendrier_2030_en_v2.xlsx");
        assert!(second_path.exists(), "expected a versioned v2 file");
        assert_eq!(
            first_bytes,
            std::fs::read(&first_path).unwrap(),
            "original file should not have been modified"
        );
        let second_bytes = std::fs::read(&second_path).unwrap();

        // Third generation: should create a "_v3" file, leaving the first
        // two untouched.
        generate_excel_schedule(
            vec![game("Third Home", "Third Away")],
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        )
        .unwrap();
        let third_path = output_dir.join("calendrier_2030_en_v3.xlsx");
        assert!(third_path.exists(), "expected a versioned v3 file");
        assert_eq!(first_bytes, std::fs::read(&first_path).unwrap());
        assert_eq!(second_bytes, std::fs::read(&second_path).unwrap());

        remove_dir_all(&output_dir).unwrap();
    }
}
