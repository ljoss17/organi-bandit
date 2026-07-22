use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(dt: &DateTime<Tz>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&dt.to_rfc3339())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Tz>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let fixed = DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
    Ok(fixed.with_timezone(&chrono_tz::Europe::Zurich))
}
