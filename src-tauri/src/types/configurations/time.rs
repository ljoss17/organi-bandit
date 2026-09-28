use serde::{de, Deserialize, Deserializer, Serialize};

use crate::{errors::AppError, types::game_time::GameTime};

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeConfiguration {
    start_time: GameTime,
    start_break: GameTime,
    end_break: GameTime,
    game_duration: GameTime,
    time_between_games: GameTime,
    hard_stop: GameTime,
}

// Use TimeConfiguration::new() to validate data before deserializing.
impl<'de> Deserialize<'de> for TimeConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fields {
            start_time: GameTime,
            start_break: GameTime,
            end_break: GameTime,
            game_duration: GameTime,
            time_between_games: GameTime,
            hard_stop: GameTime,
        }

        let fields = Fields::deserialize(deserializer)?;
        Self::new(
            fields.start_time,
            fields.start_break,
            fields.end_break,
            fields.game_duration,
            fields.time_between_games,
            fields.hard_stop,
        )
        .map_err(de::Error::custom)
    }
}

impl TimeConfiguration {
    pub fn new(
        start_time: GameTime,
        start_break: GameTime,
        end_break: GameTime,
        game_duration: GameTime,
        time_between_games: GameTime,
        hard_stop: GameTime,
    ) -> Result<Self, AppError> {
        if hard_stop.as_minutes() + game_duration.as_minutes() + time_between_games.as_minutes()
            >= (24 * 60)
        {
            return Err(AppError::HardStopTooCloseToMidnight(
                hard_stop,
                game_duration,
                time_between_games,
            ));
        }
        if start_time.as_minutes() + game_duration.as_minutes() > hard_stop.as_minutes() {
            return Err(AppError::FirstGameEndsAfterHardStop(start_time, hard_stop));
        }
        // Only a break with some length has a first game after it; with
        // matching break times there is no break to place.
        if start_break != end_break
            && end_break.as_minutes() + game_duration.as_minutes() > hard_stop.as_minutes()
        {
            return Err(AppError::FirstGameAfterBreakEndsAfterHardStop(
                end_break, hard_stop,
            ));
        }
        Ok(Self {
            start_time,
            start_break,
            end_break,
            game_duration,
            time_between_games,
            hard_stop,
        })
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

    pub fn has_break(&self) -> bool {
        self.start_break != self.end_break
    }

    pub fn hard_stop(&self) -> &GameTime {
        &self.hard_stop
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
            GameTime::new(17, 0).unwrap(),
        )
        .unwrap()
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
            "timeBetweenGames": {"hour": 0, "minute": 30},
            "hardStop": {"hour": 20, "minute": 15}
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
        assert_eq!(configuration.hard_stop(), &GameTime::new(20, 15).unwrap());
    }

    // An out-of-range time is refused where the payload is read, rather
    // than being carried into the schedule as an impossible kick-off.
    #[test]
    fn refuses_a_payload_carrying_an_out_of_range_time() {
        let payload = r#"{
            "startTime": {"hour": 99, "minute": 0},
            "startBreak": {"hour": 12, "minute": 0},
            "endBreak": {"hour": 13, "minute": 0},
            "gameDuration": {"hour": 1, "minute": 0},
            "timeBetweenGames": {"hour": 0, "minute": 30}
        }"#;

        let result = serde_json::from_str::<TimeConfiguration>(payload);

        assert!(result.is_err(), "hour 99 should be rejected");
    }

    // The hard stop rules apply to what the frontend sends too, not only to
    // configurations built through new().
    #[test]
    fn refuses_a_payload_whose_first_game_ends_after_the_hard_stop() {
        let payload = r#"{
            "startTime": {"hour": 9, "minute": 0},
            "startBreak": {"hour": 12, "minute": 0},
            "endBreak": {"hour": 13, "minute": 0},
            "gameDuration": {"hour": 1, "minute": 0},
            "timeBetweenGames": {"hour": 0, "minute": 30},
            "hardStop": {"hour": 9, "minute": 30}
        }"#;

        let result = serde_json::from_str::<TimeConfiguration>(payload);

        assert!(
            result.is_err(),
            "a 9:30 hard stop should be rejected for a 9:00 start with 1h games"
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
            "hardStop",
        ] {
            assert!(json.contains(key), "expected key {key} in {json}");
        }
    }

    // With a 23:00 hard stop, 30 minute games and a 30 minute gap, the slot
    // after the last game (22:30) is 23:30 and its game would end at 24:00,
    // which GameTime addition wraps to 00:00 — before the hard stop.
    #[test]
    fn new_rejects_a_hard_stop_whose_next_slot_reaches_midnight() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(23, 0).unwrap(),
        );

        assert!(
            matches!(result, Err(AppError::HardStopTooCloseToMidnight(hard_stop, duration, gap))
                if hard_stop == GameTime::new(23, 0).unwrap()
                    && duration == GameTime::new(0, 30).unwrap()
                    && gap == GameTime::new(0, 30).unwrap()),
            "{result:?}"
        );
    }

    // One minute earlier and the slot after the last game ends at 23:59, so
    // nothing reaches midnight.
    #[test]
    fn new_accepts_a_hard_stop_whose_next_slot_ends_before_midnight() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(22, 59).unwrap(),
        );

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn new_rejects_a_first_game_ending_after_the_hard_stop() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(9, 59).unwrap(),
        );

        assert!(
            matches!(result, Err(AppError::FirstGameEndsAfterHardStop(start, hard_stop))
                if start == GameTime::new(9, 0).unwrap()
                    && hard_stop == GameTime::new(9, 59).unwrap()),
            "{result:?}"
        );
    }

    // A hard stop earlier than the start time is refused the same way, rather
    // than every game being pushed to a new day and played after it anyway.
    #[test]
    fn new_rejects_a_hard_stop_before_the_start_time() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(8, 0).unwrap(),
        );

        assert!(
            matches!(result, Err(AppError::FirstGameEndsAfterHardStop(..))),
            "{result:?}"
        );
    }

    // A game has to finish by the hard stop, so one ending exactly on it fits.
    #[test]
    fn new_accepts_a_first_game_ending_exactly_at_the_hard_stop() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(9, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(10, 0).unwrap(),
        );

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn new_rejects_a_first_game_after_the_break_ending_after_the_hard_stop() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(13, 59).unwrap(),
        );

        assert!(
            matches!(result, Err(AppError::FirstGameAfterBreakEndsAfterHardStop(end_break, hard_stop))
                if end_break == GameTime::new(13, 0).unwrap()
                    && hard_stop == GameTime::new(13, 59).unwrap()),
            "{result:?}"
        );
    }

    // Matching break times mean there is no break, so where they sit in the
    // day says nothing about the hard stop — even past it.
    #[test]
    fn new_ignores_break_times_after_the_hard_stop_when_there_is_no_break() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(20, 0).unwrap(),
            GameTime::new(20, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(17, 0).unwrap(),
        );

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn new_accepts_a_first_game_after_the_break_ending_exactly_at_the_hard_stop() {
        let result = TimeConfiguration::new(
            GameTime::new(9, 0).unwrap(),
            GameTime::new(12, 0).unwrap(),
            GameTime::new(13, 0).unwrap(),
            GameTime::new(1, 0).unwrap(),
            GameTime::new(0, 30).unwrap(),
            GameTime::new(14, 0).unwrap(),
        );

        assert!(result.is_ok(), "{result:?}");
    }
}
