use std::fs;
use std::path::Path;

use crate::errors::AppError;
use crate::types::team::Team;

#[tauri::command]
pub fn read_team_list(file_path: &Path) -> Result<Vec<Team>, AppError> {
    let content = fs::read_to_string(file_path)?;
    let teams: Vec<Team> = serde_json::from_str(&content)?;
    Ok(teams)
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
        assert_eq!(teams[0], Team::new("Morges Bandits", None));
        assert_eq!(teams[1], Team::new("Yverdon Ducs", Some(3)));
        assert_eq!(teams[2], Team::new("Lausanne Rockets", None));
    }
}
