use serde::{Deserialize, Serialize};

use crate::types::game_time::GameTime;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeConfiguration {
    start_time: GameTime,
    start_break: GameTime,
    end_break: GameTime,
    game_duration: GameTime,
    time_between_games: GameTime,
}

impl TimeConfiguration {
    pub fn new(
        start_time: GameTime,
        start_break: GameTime,
        end_break: GameTime,
        game_duration: GameTime,
        time_between_games: GameTime,
    ) -> Self {
        Self {
            start_time,
            start_break,
            end_break,
            game_duration,
            time_between_games,
        }
    }

    pub fn start_time(&self) -> &GameTime {
        &self.start_time
    }

    // How far apart two consecutive games start on the same field
    pub fn interval_between_games(&self) -> GameTime {
        self.game_duration + self.time_between_games
    }

    pub fn game_duration(&self) -> &GameTime {
        &self.game_duration
    }

    pub fn time_between_games(&self) -> &GameTime {
        &self.time_between_games
    }

    pub fn start_break(&self) -> &GameTime {
        &self.start_break
    }

    pub fn end_break(&self) -> &GameTime {
        &self.end_break
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time_configuration(
        game_duration: GameTime,
        time_between_games: GameTime,
    ) -> TimeConfiguration {
        TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 0).unwrap(),
            game_duration,
            time_between_games,
        )
    }

    #[test]
    fn interval_between_games_sums_the_duration_and_the_gap() {
        let configuration =
            time_configuration(GameTime::new(1, 0).unwrap(), GameTime::new(0, 30).unwrap());

        assert_eq!(
            configuration.interval_between_games(),
            GameTime::new(1, 30).unwrap()
        );
    }

    #[test]
    fn interval_between_games_carries_minutes_into_the_hour() {
        let configuration =
            time_configuration(GameTime::new(0, 45).unwrap(), GameTime::new(0, 30).unwrap());

        assert_eq!(
            configuration.interval_between_games(),
            GameTime::new(1, 15).unwrap()
        );
    }

    #[test]
    fn interval_between_games_equals_the_duration_when_there_is_no_gap() {
        let configuration =
            time_configuration(GameTime::new(0, 45).unwrap(), GameTime::new(0, 0).unwrap());

        assert_eq!(
            configuration.interval_between_games(),
            GameTime::new(0, 45).unwrap()
        );
    }

    #[test]
    fn deserializes_the_camel_case_keys_sent_by_the_frontend() {
        let payload = r#"{
            "startTime": {"hour": 9, "minute": 0},
            "startBreak": {"hour": 12, "minute": 0},
            "endBreak": {"hour": 13, "minute": 0},
            "gameDuration": {"hour": 1, "minute": 0},
            "timeBetweenGames": {"hour": 0, "minute": 30}
        }"#;

        let configuration: TimeConfiguration =
            serde_json::from_str(payload).expect("frontend payload should deserialize");

        assert_eq!(configuration.start_time(), &GameTime::new(9, 0).unwrap());
        assert_eq!(configuration.start_break(), &GameTime::new(12, 0).unwrap());
        assert_eq!(configuration.end_break(), &GameTime::new(13, 0).unwrap());
        assert_eq!(configuration.game_duration(), &GameTime::new(1, 0).unwrap());
        assert_eq!(
            configuration.time_between_games(),
            &GameTime::new(0, 30).unwrap()
        );
    }

    #[test]
    fn serializes_back_to_the_same_camel_case_keys() {
        let configuration =
            time_configuration(GameTime::new(1, 0).unwrap(), GameTime::new(0, 30).unwrap());

        let json = serde_json::to_string(&configuration).expect("serialization should succeed");

        for key in [
            "startTime",
            "startBreak",
            "endBreak",
            "gameDuration",
            "timeBetweenGames",
        ] {
            assert!(json.contains(key), "expected key {key} in {json}");
        }
    }
}
