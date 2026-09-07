use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DateConfiguration {
    start_date: NaiveDate,
    game_days: Vec<Weekday>,
    excluded_dates: Vec<NaiveDate>,
    #[serde(default)]
    single_game_per_week: bool,
}

impl DateConfiguration {
    pub fn new(
        start_date: NaiveDate,
        game_days: Vec<Weekday>,
        excluded_dates: Vec<NaiveDate>,
        single_game_per_week: bool,
    ) -> Self {
        Self {
            start_date,
            game_days,
            excluded_dates,
            single_game_per_week,
        }
    }

    pub fn start_date(&self) -> &NaiveDate {
        &self.start_date
    }

    pub fn game_days(&self) -> &[Weekday] {
        &self.game_days
    }

    pub fn excluded_dates(&self) -> &[NaiveDate] {
        &self.excluded_dates
    }

    pub fn single_game_per_week(&self) -> bool {
        self.single_game_per_week
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_the_camel_case_keys_sent_by_the_frontend() {
        let payload = r#"{
            "startDate": "2026-05-13",
            "gameDays": ["Wed", "Sat"],
            "excludedDates": ["2026-05-16", "2026-07-04"],
            "singleGamePerWeek": true
        }"#;

        let configuration: DateConfiguration =
            serde_json::from_str(payload).expect("frontend payload should deserialize");

        assert_eq!(
            configuration.start_date(),
            &NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()
        );
        assert_eq!(configuration.game_days(), &[Weekday::Wed, Weekday::Sat]);
        assert_eq!(
            configuration.excluded_dates(),
            &[
                NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
                NaiveDate::from_ymd_opt(2026, 7, 4).unwrap(),
            ]
        );
        assert!(configuration.single_game_per_week());
    }

    #[test]
    fn defaults_to_every_game_day_when_the_toggle_is_absent() {
        let payload = r#"{
            "startDate": "2026-05-13",
            "gameDays": ["Sat", "Sun"],
            "excludedDates": []
        }"#;

        let configuration: DateConfiguration =
            serde_json::from_str(payload).expect("payload without the toggle should deserialize");

        assert!(!configuration.single_game_per_week());
    }

    #[test]
    fn deserializes_an_empty_excluded_dates_list() {
        let payload = r#"{
            "startDate": "2026-05-13",
            "gameDays": ["Sat"],
            "excludedDates": []
        }"#;

        let configuration: DateConfiguration =
            serde_json::from_str(payload).expect("frontend payload should deserialize");

        assert!(configuration.excluded_dates().is_empty());
    }

    #[test]
    fn serializes_back_to_the_same_camel_case_keys() {
        let configuration = DateConfiguration::new(
            NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            vec![Weekday::Sat],
            vec![NaiveDate::from_ymd_opt(2026, 5, 16).unwrap()],
            false,
        );

        let json = serde_json::to_string(&configuration).expect("serialization should succeed");

        for key in [
            "startDate",
            "gameDays",
            "excludedDates",
            "singleGamePerWeek",
        ] {
            assert!(json.contains(key), "expected key {key} in {json}");
        }
        // The values round-trip in the same shapes the frontend sends.
        assert!(json.contains("\"2026-05-13\""), "{json}");
        assert!(json.contains("\"Sat\""), "{json}");
        assert!(json.contains("\"2026-05-16\""), "{json}");
    }
}
