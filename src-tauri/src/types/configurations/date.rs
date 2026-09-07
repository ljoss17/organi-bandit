use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DateConfiguration {
    start_date: NaiveDate,
    game_days: Vec<Weekday>,
    excluded_dates: Vec<NaiveDate>,
}

impl DateConfiguration {
    pub fn new(
        start_date: NaiveDate,
        game_days: Vec<Weekday>,
        excluded_dates: Vec<NaiveDate>,
    ) -> Self {
        Self {
            start_date,
            game_days,
            excluded_dates,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_the_camel_case_keys_sent_by_the_frontend() {
        let payload = r#"{
            "startDate": "2026-05-13",
            "gameDays": ["Wed", "Sat"],
            "excludedDates": ["2026-05-16", "2026-07-04"]
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
        );

        let json = serde_json::to_string(&configuration).expect("serialization should succeed");

        for key in ["startDate", "gameDays", "excludedDates"] {
            assert!(json.contains(key), "expected key {key} in {json}");
        }
        // The values round-trip in the same shapes the frontend sends.
        assert!(json.contains("\"2026-05-13\""), "{json}");
        assert!(json.contains("\"Sat\""), "{json}");
        assert!(json.contains("\"2026-05-16\""), "{json}");
    }
}
