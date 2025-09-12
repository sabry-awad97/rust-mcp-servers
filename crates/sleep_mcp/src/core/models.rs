use rmcp::schemars;
use serde::{Deserialize, Deserializer, Serialize};
use std::time::Duration;

/// Helper function to deserialize and trim strings
fn deserialize_trimmed_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.trim().to_string())
}

/// Sleep operation result
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SleepResult {
    /// Duration that was slept in milliseconds
    pub duration_ms: u64,
    /// Human-readable duration string
    pub duration_str: String,
    /// Start time (ISO 8601)
    pub start_time: String,
    /// End time (ISO 8601)
    pub end_time: String,
    /// Whether the sleep completed successfully
    pub completed: bool,
    /// Optional message about the sleep operation
    pub message: Option<String>,
}

impl SleepResult {
    /// Create a new SleepResult
    pub fn new(
        duration: Duration,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
        completed: bool,
        message: Option<String>,
    ) -> Self {
        Self {
            duration_ms: duration.as_millis() as u64,
            duration_str: crate::core::utils::format_duration(duration),
            start_time: start_time.to_rfc3339(),
            end_time: end_time.to_rfc3339(),
            completed,
            message,
        }
    }
}

/// Sleep status information
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SleepStatus {
    /// Whether a sleep operation is currently active
    pub is_sleeping: bool,
    /// Current sleep duration in milliseconds (if sleeping)
    pub current_duration_ms: Option<u64>,
    /// Sleep start time (if sleeping)
    pub start_time: Option<String>,
    /// Expected end time (if sleeping)
    pub expected_end_time: Option<String>,
    /// Progress percentage (0-100, if sleeping)
    pub progress_percent: Option<f64>,
    /// Time remaining in milliseconds (if sleeping)
    pub remaining_ms: Option<u64>,
    /// Optional message associated with the sleep operation
    pub message: Option<String>,
}

/// Request to sleep for a specific duration
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SleepRequest {
    /// Duration to sleep (e.g., "1s", "500ms", "2m", "1h", "1.5s")
    #[serde(deserialize_with = "deserialize_trimmed_string")]
    pub duration: String,
    /// Optional message to include in the result
    #[serde(default)]
    pub message: Option<String>,
}

/// Request to sleep until a specific time
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SleepUntilRequest {
    /// Target time in ISO 8601 format (e.g., "2025-01-15T14:30:00Z")
    #[serde(deserialize_with = "deserialize_trimmed_string")]
    pub target_time: String,
    /// Optional message to include in the result
    #[serde(default)]
    pub message: Option<String>,
}

/// Request to get current sleep status
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetStatusRequest {
    /// Whether to include detailed information
    #[serde(default)]
    pub detailed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_sleep_result_creation() {
        let start = chrono::Utc::now();
        let duration = Duration::from_secs(5);
        let end = start + chrono::Duration::from_std(duration).unwrap();

        let result = SleepResult::new(duration, start, end, true, Some("Test sleep".to_string()));

        assert_eq!(result.duration_ms, 5000);
        assert!(result.completed);
        assert_eq!(result.message, Some("Test sleep".to_string()));
    }

    #[test]
    fn test_sleep_request_deserialization() {
        let json = r#"{"duration": "  1.5s  ", "message": "Test"}"#;
        let request: SleepRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.duration, "1.5s");
        assert_eq!(request.message, Some("Test".to_string()));
    }

    #[test]
    fn test_sleep_until_request_deserialization() {
        let json = r#"{"target_time": "  2025-01-15T14:30:00Z  "}"#;
        let request: SleepUntilRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.target_time, "2025-01-15T14:30:00Z");
        assert_eq!(request.message, None);
    }

    #[test]
    fn test_sleep_status_serialization() {
        let status = SleepStatus {
            is_sleeping: true,
            current_duration_ms: Some(5000),
            start_time: Some("2025-01-15T14:30:00Z".to_string()),
            expected_end_time: Some("2025-01-15T14:30:05Z".to_string()),
            progress_percent: Some(50.0),
            remaining_ms: Some(2500),
            message: Some("Test message".to_string()),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("is_sleeping"));
        assert!(json.contains("50.0"));
    }
}
