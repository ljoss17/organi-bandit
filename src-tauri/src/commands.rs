use std::path::Path;

use chrono::{DateTime, Datelike, NaiveDate};
use chrono_tz::Tz;
use rust_i18n::t;
use rust_xlsxwriter::workbook::Workbook;
use rust_xlsxwriter::worksheet::Worksheet;
use rust_xlsxwriter::{Color, DataValidation, Format, FormatAlign, FormatBorder};

use crate::errors::AppError;
use crate::impls::round_robin::RoundRobin;
use crate::impls::single_elimination::SingleElimination;
use crate::types::configurations::date::DateConfiguration;
use crate::types::game::Game;
use crate::types::game_time::GameTime;
use crate::types::season::{Season, SeasonConfig};
use crate::types::team::Team;
use crate::types::tournament_selection::TournamentSelection;

const ROWS_BEFORE_DAY_BLOCK: u32 = 2;

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
    date_configuration: DateConfiguration,
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

    write_excluded_dates_column(
        worksheet,
        date_configuration.excluded_dates(),
        number_fields,
        language,
    )?;

    let mut day_index = 1;

    for game in sorted_schedule.iter() {
        let game_day = game.get_game_day();
        if current_day != Some(game_day.date_naive()) {
            row += ROWS_BEFORE_DAY_BLOCK;
            write_day_row(
                worksheet,
                row,
                game_day,
                day_index,
                number_fields,
                &date_configuration,
                language,
            )?;
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

fn write_excluded_dates_column(
    worksheet: &mut Worksheet,
    excluded_dates: &[NaiveDate],
    number_fields: u16,
    language: &str,
) -> Result<(), AppError> {
    if excluded_dates.is_empty() {
        return Ok(());
    }

    let column = number_fields * 5 + 1;
    let header_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Silver);
    let date_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    worksheet.write_with_format(
        ROWS_BEFORE_DAY_BLOCK,
        column,
        t!("excluded_dates", locale = language),
        &header_format,
    )?;

    for (index, group) in group_consecutive_dates(excluded_dates).iter().enumerate() {
        worksheet.write_with_format(
            ROWS_BEFORE_DAY_BLOCK + 1 + index as u32,
            column,
            group,
            &date_format,
        )?;
    }

    Ok(())
}

// Collapses runs of consecutive days into a single entry, so a week off
// reads as "10/09/2026 - 13/09/2026" rather than four separate lines. Input
// order doesn't matter, and repeated dates collapse into one.
fn group_consecutive_dates(dates: &[NaiveDate]) -> Vec<String> {
    let mut sorted_dates = dates.to_vec();
    sorted_dates.sort();
    sorted_dates.dedup();

    let mut groups = Vec::new();
    let mut index = 0;
    while index < sorted_dates.len() {
        let start = sorted_dates[index];
        let mut end = start;

        // Walk forward while the next date is exactly the day after the
        // one the run currently ends on.
        while let Some(next_day) = end.succ_opt() {
            if sorted_dates.get(index + 1) != Some(&next_day) {
                break;
            }
            index += 1;
            end = next_day;
        }

        groups.push(if start == end {
            start.format("%d/%m/%Y").to_string()
        } else {
            format!("{} - {}", start.format("%d/%m/%Y"), end.format("%d/%m/%Y"))
        });
        index += 1;
    }

    groups
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
    date_configuration: &DateConfiguration,
    language: &str,
) -> Result<(), AppError> {
    let title_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::Red);

    let scheduled_day = game_day.date_naive();
    worksheet.merge_range(
        row,
        0,
        row,
        number_fields * 5,
        &day_label(&scheduled_day, day_index, language)?,
        &title_format,
    )?;

    let choices = weekly_day_choices(&scheduled_day, day_index, date_configuration, language)?;
    if choices.len() > 1 {
        let validation = DataValidation::new().allow_list_strings(&choices)?;
        worksheet.add_data_validation(row, 0, row, 0, &validation)?;
    }

    worksheet.set_row_height(row, 25)?;
    Ok(())
}

fn day_label(day: &NaiveDate, day_index: u32, language: &str) -> Result<String, AppError> {
    let month = chrono::Month::try_from(day.month() as u8)?;
    Ok(t!(
        "day",
        locale = language,
        day_index = day_index,
        day = day.day(),
        month = month.name(),
        year = day.year()
    )
    .to_string())
}

// Every configured weekday in the same ISO week as the scheduled day, minus
// the excluded ones. Returns a single entry when there's nothing to choose
// between
fn weekly_day_choices(
    scheduled_day: &NaiveDate,
    day_index: u32,
    date_configuration: &DateConfiguration,
    language: &str,
) -> Result<Vec<String>, AppError> {
    if !date_configuration.single_game_per_week() {
        return Ok(vec![day_label(scheduled_day, day_index, language)?]);
    }

    let monday =
        *scheduled_day - chrono::Days::new(scheduled_day.weekday().num_days_from_monday() as u64);

    let mut labels = Vec::new();
    for weekday in date_configuration.game_days() {
        let candidate = monday + chrono::Days::new(weekday.num_days_from_monday() as u64);
        if date_configuration.excluded_dates().contains(&candidate) {
            continue;
        }
        labels.push(day_label(&candidate, day_index, language)?);
    }

    // The scheduled day always belongs in the list, even if it ended up
    // outside the configured weekdays somehow.
    let scheduled_label = day_label(scheduled_day, day_index, language)?;
    if !labels.contains(&scheduled_label) {
        labels.insert(0, scheduled_label);
    }

    Ok(labels)
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
    use chrono::Weekday;
    use std::fs::{create_dir_all, remove_dir_all};
    use std::path::PathBuf;

    fn start_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()
    }

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
            DateConfiguration::new(start_date(), vec![Weekday::Sat], vec![], false),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        );

        assert!(result.is_ok());
        remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn generate_excel_schedule_writes_the_excluded_dates() {
        use crate::types::game_time::GameTime;

        let game = Game::new_with_game_day(
            Team::new("Home", None),
            Team::new("Away", None),
            chrono::NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            GameTime::new(9, 0).unwrap(),
            None,
        )
        .unwrap();
        let output_dir = temp_output_dir("excluded-dates-column");

        generate_excel_schedule(
            vec![game],
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 30).unwrap(),
            DateConfiguration::new(
                start_date(),
                vec![Weekday::Sat],
                vec![
                    chrono::NaiveDate::from_ymd_opt(2026, 7, 4).unwrap(),
                    chrono::NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
                ],
                false,
            ),
            1,
            output_dir.to_string_lossy().to_string(),
            "en",
        )
        .unwrap();

        let output = std::process::Command::new("unzip")
            .arg("-p")
            .arg(output_dir.join("calendrier_2026_en.xlsx"))
            .arg("xl/sharedStrings.xml")
            .output()
            .unwrap();
        let contents = String::from_utf8_lossy(&output.stdout);

        assert!(contents.contains("Excluded dates"), "{contents}");

        let first = contents.find("16/05/2026").expect("missing 16/05/2026");
        let second = contents.find("04/07/2026").expect("missing 04/07/2026");
        assert!(first < second, "excluded dates are not sorted: {contents}");

        let sheet = std::process::Command::new("unzip")
            .arg("-p")
            .arg(output_dir.join("calendrier_2026_en.xlsx"))
            .arg("xl/worksheets/sheet1.xml")
            .output()
            .unwrap();
        let sheet = String::from_utf8_lossy(&sheet.stdout);
        assert!(sheet.contains("r=\"G3\""), "no cell at G3: {sheet}");
        assert!(sheet.contains("r=\"G4\""), "no cell at G4: {sheet}");

        remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn weekly_day_choices_offers_every_configured_weekday_in_that_week() {
        let configuration = DateConfiguration::new(
            date(2026, 5, 16),
            vec![Weekday::Sat, Weekday::Sun],
            vec![],
            true,
        );

        // Saturday 16/05; the Sunday of the same week is 17/05.
        let choices = weekly_day_choices(&date(2026, 5, 16), 1, &configuration, "en").unwrap();

        assert_eq!(choices, vec!["Day 1: 16 May 2026", "Day 1: 17 May 2026"]);
    }

    #[test]
    fn weekly_day_choices_offers_only_the_scheduled_day_when_the_option_is_off() {
        let configuration = DateConfiguration::new(
            date(2026, 5, 16),
            vec![Weekday::Sat, Weekday::Sun],
            vec![],
            false,
        );

        let choices = weekly_day_choices(&date(2026, 5, 16), 1, &configuration, "en").unwrap();

        assert_eq!(choices, vec!["Day 1: 16 May 2026"]);
    }

    #[test]
    fn weekly_day_choices_offers_only_the_scheduled_day_for_a_single_weekday() {
        let configuration =
            DateConfiguration::new(date(2026, 5, 16), vec![Weekday::Sat], vec![], true);

        let choices = weekly_day_choices(&date(2026, 5, 16), 1, &configuration, "en").unwrap();

        assert_eq!(choices, vec!["Day 1: 16 May 2026"]);
    }

    #[test]
    fn weekly_day_choices_leaves_out_excluded_alternatives() {
        let configuration = DateConfiguration::new(
            date(2026, 5, 16),
            vec![Weekday::Sat, Weekday::Sun],
            vec![date(2026, 5, 17)],
            true,
        );

        let choices = weekly_day_choices(&date(2026, 5, 16), 1, &configuration, "en").unwrap();

        assert_eq!(choices, vec!["Day 1: 16 May 2026"]);
    }

    #[test]
    fn weekly_day_choices_includes_the_scheduled_day_after_a_fallback() {
        let configuration = DateConfiguration::new(
            date(2026, 5, 16),
            vec![Weekday::Sat, Weekday::Sun],
            vec![date(2026, 5, 23)],
            true,
        );

        // The Saturday was excluded, so this week runs on the Sunday.
        let choices = weekly_day_choices(&date(2026, 5, 24), 1, &configuration, "en").unwrap();

        assert_eq!(choices, vec!["Day 1: 24 May 2026"]);
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn group_consecutive_dates_keeps_a_lone_date_on_its_own() {
        let groups = group_consecutive_dates(&[date(2026, 9, 10)]);

        assert_eq!(groups, vec!["10/09/2026"]);
    }

    #[test]
    fn group_consecutive_dates_collapses_a_run_into_a_range() {
        let groups = group_consecutive_dates(&[
            date(2026, 9, 10),
            date(2026, 9, 11),
            date(2026, 9, 12),
            date(2026, 9, 13),
        ]);

        assert_eq!(groups, vec!["10/09/2026 - 13/09/2026"]);
    }

    #[test]
    fn group_consecutive_dates_separates_runs_with_a_gap_between_them() {
        let groups = group_consecutive_dates(&[
            date(2026, 9, 10),
            date(2026, 9, 11),
            date(2026, 9, 20),
            date(2026, 9, 21),
            date(2026, 9, 25),
        ]);

        assert_eq!(
            groups,
            vec![
                "10/09/2026 - 11/09/2026",
                "20/09/2026 - 21/09/2026",
                "25/09/2026"
            ]
        );
    }

    #[test]
    fn group_consecutive_dates_spans_month_and_year_boundaries() {
        let groups = group_consecutive_dates(&[
            date(2026, 9, 30),
            date(2026, 10, 1),
            date(2026, 12, 31),
            date(2027, 1, 1),
        ]);

        assert_eq!(
            groups,
            vec!["30/09/2026 - 01/10/2026", "31/12/2026 - 01/01/2027"]
        );
    }

    #[test]
    fn group_consecutive_dates_sorts_and_deduplicates_its_input() {
        let groups = group_consecutive_dates(&[
            date(2026, 9, 12),
            date(2026, 9, 10),
            date(2026, 9, 11),
            date(2026, 9, 10),
        ]);

        assert_eq!(groups, vec!["10/09/2026 - 12/09/2026"]);
    }

    #[test]
    fn group_consecutive_dates_returns_nothing_for_no_dates() {
        assert!(group_consecutive_dates(&[]).is_empty());
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
            DateConfiguration::new(start_date(), vec![Weekday::Sat], vec![], false),
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
            DateConfiguration::new(start_date(), vec![Weekday::Sat], vec![], false),
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
            DateConfiguration::new(start_date(), vec![Weekday::Sat], vec![], false),
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
            DateConfiguration::new(start_date(), vec![Weekday::Sat], vec![], false),
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
            DateConfiguration::new(start_date(), vec![Weekday::Sat], vec![], false),
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
