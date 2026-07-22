use std::fs;
use std::path::Path;

use crate::errors::AppError;
use crate::types::team::Team;

pub fn read_team_list(file_path: &Path) -> Result<Vec<Team>, AppError> {
    let content = fs::read_to_string(file_path).map_err(AppError::ReadError)?;
    let teams: Vec<Team> = serde_json::from_str(&content).map_err(AppError::DeserializeError)?;
    Ok(teams)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_team_list() {
        let teams =
            read_team_list(Path::new("resources/teams.json")).expect("failed to read team file");
        assert_eq!(teams[0], "Morges Bandits".to_owned());
        assert_eq!(teams[1], "Yverdon Ducs".to_owned());
        assert_eq!(teams[2], "Lausanne Rockets".to_owned());
    }
}
