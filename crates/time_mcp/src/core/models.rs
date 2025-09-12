use chrono::{DateTime, TimeZone};
use chrono_tz::OffsetComponents;
use rmcp::schemars;
use serde::{Deserialize, Deserializer, Serialize};

use crate::core::utils::{DATETIME_FORMAT, DAY_FORMAT};

/// Helper function to deserialize and trim strings
fn deserialize_trimmed_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.trim().to_string())
}

/// Time result containing timezone information
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TimeResult {
    /// IANA timezone name
    pub timezone: String,
    /// ISO 8601 datetime string
    pub datetime: String,
    /// Day of the week
    pub day_of_week: String,
    /// Whether daylight saving time is active
    pub is_dst: bool,
}

impl TimeResult {
    /// Create a TimeResult from a timezone-aware datetime
    pub fn from_datetime<Tz>(dt: &DateTime<Tz>, timezone_name: &str) -> TimeResult
    where
        Tz: TimeZone,
        Tz::Offset: OffsetComponents + std::fmt::Display,
    {
        let is_dst = dt.offset().dst_offset().num_seconds() != 0;

        TimeResult {
            timezone: timezone_name.to_string(),
            datetime: dt.format(DATETIME_FORMAT).to_string(),
            day_of_week: dt.format(DAY_FORMAT).to_string(),
            is_dst,
        }
    }
}

/// Time conversion result with source and target information
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TimeConversionResult {
    /// Source time information
    pub source: TimeResult,
    /// Target time information
    pub target: TimeResult,
    /// Time difference between timezones
    pub time_difference: String,
}

/// Request to get current time in a timezone
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCurrentTimeRequest {
    /// IANA timezone name (e.g., 'America/New_York', 'Europe/London')
    #[serde(deserialize_with = "deserialize_trimmed_string")]
    pub timezone: String,
}

/// Request to convert time between timezones
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertTimeRequest {
    /// Source IANA timezone name
    #[serde(deserialize_with = "deserialize_trimmed_string")]
    pub source_timezone: String,
    /// Time to convert in 24-hour format (HH:MM)
    #[serde(deserialize_with = "deserialize_trimmed_string")]
    pub time: String,
    /// Target IANA timezone name
    #[serde(deserialize_with = "deserialize_trimmed_string")]
    pub target_timezone: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_result_serialization() {
        let time_result = TimeResult {
            timezone: "UTC".to_string(),
            datetime: "2024-01-01T12:00:00+00:00".to_string(),
            day_of_week: "Monday".to_string(),
            is_dst: false,
        };

        let json = serde_json::to_string(&time_result).unwrap();
        assert!(json.contains("UTC"));
        assert!(json.contains("Monday"));
    }

    #[test]
    fn test_timezone_trimming() {
        // Test GetCurrentTimeRequest with whitespace
        let json = r#"{"timezone": "   Africa/Cairo   "}"#;
        let request: GetCurrentTimeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.timezone, "Africa/Cairo");

        // Test ConvertTimeRequest with whitespace
        let json = r#"{
            "source_timezone": "  America/New_York  ",
            "time": "  14:30  ",
            "target_timezone": "   Europe/London   "
        }"#;
        let request: ConvertTimeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.source_timezone, "America/New_York");
        assert_eq!(request.time, "14:30");
        assert_eq!(request.target_timezone, "Europe/London");
    }
}
