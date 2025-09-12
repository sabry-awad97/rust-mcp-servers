use rmcp::ErrorData as McpError;
use rmcp::serde_json::json;

// Error codes
const ERROR_INVALID_TIMEZONE: &str = "invalid_timezone";
const ERROR_INVALID_TIME_FORMAT: &str = "invalid_time_format";
const ERROR_AMBIGUOUS_TIME: &str = "ambiguous_time";
const ERROR_RESOURCE_NOT_FOUND: &str = "resource_not_found";

/// Custom error types for better error handling
#[derive(Debug, thiserror::Error)]
pub enum TimeServerError {
    #[error("Invalid timezone: {timezone}")]
    InvalidTimezone { timezone: String },
    #[error("Invalid time format: {time}. Expected HH:MM format")]
    InvalidTimeFormat { time: String },
    #[error("Ambiguous time during DST transition: {time}")]
    AmbiguousTime { time: String },
    #[error("Resource not found: {uri}")]
    ResourceNotFound { uri: String },
}

impl From<TimeServerError> for McpError {
    fn from(err: TimeServerError) -> Self {
        match err {
            TimeServerError::InvalidTimezone { timezone } => McpError::invalid_params(
                ERROR_INVALID_TIMEZONE,
                Some(json!({"timezone": timezone})),
            ),
            TimeServerError::InvalidTimeFormat { time } => {
                McpError::invalid_params(ERROR_INVALID_TIME_FORMAT, Some(json!({"time": time})))
            }
            TimeServerError::AmbiguousTime { time } => {
                McpError::invalid_params(ERROR_AMBIGUOUS_TIME, Some(json!({"time": time})))
            }
            TimeServerError::ResourceNotFound { uri } => McpError::resource_not_found(
                ERROR_RESOURCE_NOT_FOUND,
                Some(json!({
                    "uri": uri,
                    "available_resources": ["time://status", "time://help", "time://timezones"]
                })),
            ),
        }
    }
}

pub type TimeServerResult<T> = Result<T, TimeServerError>;
pub type McpResult<T> = Result<T, McpError>;

#[cfg(test)]
mod tests {
    use super::TimeServerError;
    use crate::core::error::McpError;

    #[test]
    fn test_error_conversion() {
        let error = TimeServerError::InvalidTimezone {
            timezone: "Invalid/Zone".to_string(),
        };
        let mcp_error: McpError = error.into();

        // Should convert to proper MCP error format
        assert!(mcp_error.to_string().contains("invalid_timezone"));
    }
}
