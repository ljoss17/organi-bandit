use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::errors::AppError;
use crate::types::team::Team;

#[tauri::command]
pub fn read_changelog(app_handle: AppHandle) -> Result<String, AppError> {
    let changelog_path = app_handle
        .path()
        .resolve("CHANGELOG.md", BaseDirectory::Resource)?;
    let content = fs::read_to_string(changelog_path)?;
    Ok(content)
}

#[tauri::command]
pub fn read_team_list(file_path: &Path) -> Result<Vec<Team>, AppError> {
    let content = fs::read_to_string(file_path)?;
    let teams: Vec<Team> = serde_json::from_str(&content)?;
    deduplicate_teams(teams)
}

// Drops entries that repeat a team exactly, and refuses a list where one name
// carries two different seeds, since there is no way to tell which the
// organiser meant. An absent seed counts as 0, so `null` and `0` are the same
// team. The first occurrence is the one kept, and input order is preserved.
fn deduplicate_teams(teams: Vec<Team>) -> Result<Vec<Team>, AppError> {
    let mut seeds_by_name: HashMap<String, u32> = HashMap::new();
    let mut unique = Vec::with_capacity(teams.len());

    for team in teams {
        match seeds_by_name.get(team.get_name()).copied() {
            Some(seed) if seed == team.get_seed() => continue,
            Some(seed) => {
                return Err(AppError::ConflictingTeamSeeds(
                    team.get_name().to_owned(),
                    seed,
                    team.get_seed(),
                ));
            }
            None => {
                seeds_by_name.insert(team.get_name().to_owned(), team.get_seed());
                unique.push(team);
            }
        }
    }

    Ok(unique)
}

#[tauri::command]
pub fn write_team_list(file_path: &Path, new_teams: Vec<Team>) -> Result<(), AppError> {
    let serialised_teams = serde_json::to_string(&new_teams)?;
    fs::write(file_path, &serialised_teams)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_team_list() {
        let teams = read_team_list(Path::new("resources/test_teams.json"))
            .expect("failed to read team file");
        assert_eq!(teams[0], Team::new("Morges Bandits", None).unwrap());
        assert_eq!(teams[1], Team::new("Yverdon Ducs", Some(3)).unwrap());
        assert_eq!(teams[2], Team::new("Lausanne Rockets", None).unwrap());
    }

    fn team(name: &str, seed: Option<u32>) -> Team {
        Team::new(name, seed).unwrap()
    }

    #[test]
    fn deduplicate_teams_keeps_a_list_that_has_no_repeats() {
        let teams = vec![
            team("Morges Bandits", None),
            team("Yverdon Ducs", Some(3)),
            team("Lausanne Rockets", Some(1)),
        ];

        let deduplicated = deduplicate_teams(teams.clone()).unwrap();

        assert_eq!(deduplicated, teams);
    }

    #[test]
    fn deduplicate_teams_drops_an_exact_repeat_and_keeps_the_order() {
        let teams = vec![
            team("Morges Bandits", None),
            team("Yverdon Ducs", Some(3)),
            team("Morges Bandits", None),
            team("Lausanne Rockets", Some(1)),
        ];

        let deduplicated = deduplicate_teams(teams).unwrap();

        assert_eq!(
            deduplicated,
            vec![
                team("Morges Bandits", None),
                team("Yverdon Ducs", Some(3)),
                team("Lausanne Rockets", Some(1)),
            ]
        );
    }

    // The names are normalised before they reach here, so an entry that
    // only differs by its padding is an exact repeat and gets dropped
    // rather than scheduled as a second team with the same name.
    #[test]
    fn deduplicate_teams_drops_a_repeat_that_only_differs_by_whitespace() {
        let teams = vec![
            team("Morges Bandits", None),
            team("  morges bandits ", None),
            team("Morges   Bandits", None),
        ];

        let deduplicated = deduplicate_teams(teams).unwrap();

        assert_eq!(deduplicated, vec![team("Morges Bandits", None)]);
    }

    // An absent seed and a seed of 0 mean the same thing, so these are the
    // same team rather than a conflict. The first entry is the one kept.
    #[test]
    fn deduplicate_teams_treats_an_absent_seed_as_zero() {
        let teams = vec![
            team("Morges Bandits", None),
            team("Morges Bandits", Some(0)),
        ];

        let deduplicated = deduplicate_teams(teams).unwrap();

        assert_eq!(deduplicated, vec![team("Morges Bandits", None)]);
    }

    #[test]
    fn deduplicate_teams_rejects_one_name_with_two_seeds() {
        let teams = vec![
            team("Morges Bandits", Some(1)),
            team("Yverdon Ducs", Some(3)),
            team("Morges Bandits", Some(2)),
        ];

        let result = deduplicate_teams(teams);

        let Err(AppError::ConflictingTeamSeeds(name, first, second)) = result else {
            panic!("expected a conflicting-seed error, got {result:?}");
        };
        assert_eq!(name, "Morges Bandits");
        assert_eq!((first, second), (1, 2));
    }

    // A seeded entry and an unseeded one for the same team is still a
    // conflict, since the unseeded one reads as seed 0.
    #[test]
    fn deduplicate_teams_rejects_a_seeded_and_an_unseeded_entry() {
        let teams = vec![
            team("Morges Bandits", Some(2)),
            team("Morges Bandits", None),
        ];

        let result = deduplicate_teams(teams);

        assert!(matches!(result, Err(AppError::ConflictingTeamSeeds(..))));
    }

    #[test]
    fn deduplicate_teams_accepts_an_empty_list() {
        assert!(deduplicate_teams(vec![]).unwrap().is_empty());
    }
}
