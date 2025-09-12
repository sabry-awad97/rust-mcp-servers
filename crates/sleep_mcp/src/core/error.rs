use rmcp::serde_json::json;
use rmcp::ErrorData as McpError;

// Error codes
const ERROR_INVALID_DURATION: &str = "invalid_duration";
const ERROR_DURATION_TOO_LONG: &str = "duration_too_long";
const ERROR_SLEEP_CANCELLED: &str = "sleep_cancelled";
const ERROR_RESOURCE_NOT_FOUND: &str = "resource_not_found";

/// Custom error types for better error handling
#[derive(Debug, thiserror::Error)]
pub enum SleepServerError {
    #[error(
        "Invalid duration: {duration}. Expected format: number followed by unit (s, ms, m, h)"
    )]
    InvalidDuration { duration: String },
    #[error("Duration too long: {duration}. Maximum allowed is {max_duration}")]
    DurationTooLong {
        duration: String,
        max_duration: String,
    },
    #[error("Sleep operation was cancelled: {reason}")]
    SleepCancelled { reason: String },
    #[error("Resource not found: {uri}")]
    ResourceNotFound { uri: String },
}

impl From<SleepServerError> for McpError {
    fn from(err: SleepServerError) -> Self {
        match err {
            SleepServerError::InvalidDuration { duration } => McpError::invalid_params(
                ERROR_INVALID_DURATION,
                Some(json!({
                    "duration": duration,
                    "valid_formats": ["1s", "500ms", "2m", "1h", "1.5s"]
                })),
            ),
            SleepServerError::DurationTooLong {
                duration,
                max_duration,
            } => McpError::invalid_params(
                ERROR_DURATION_TOO_LONG,
                Some(json!({
                    "duration": duration,
                    "max_duration": max_duration
                })),
            ),
            SleepServerError::SleepCancelled { reason } => {
                McpError::internal_error(ERROR_SLEEP_CANCELLED, Some(json!({"reason": reason})))
            }
            SleepServerError::ResourceNotFound { uri } => McpError::resource_not_found(
                ERROR_RESOURCE_NOT_FOUND,
                Some(json!({
                    "uri": uri,
                    "available_resources": crate::server::AVAILABLE_RESOURCES
                })),
            ),
        }
    }
}

pub type SleepServerResult<T> = Result<T, SleepServerError>;
pub type McpResult<T> = Result<T, McpError>;

#[cfg(test)]
mod tests {
    use super::SleepServerError;
    use crate::core::error::McpError;

    #[test]
    fn test_error_conversion() {
        let error = SleepServerError::InvalidDuration {
            duration: "invalid".to_string(),
        };
        let mcp_error: McpError = error.into();

        // Should convert to proper MCP error format
        assert!(mcp_error.to_string().contains("invalid_duration"));
    }

    #[test]
    fn test_duration_too_long_error() {
        let error = SleepServerError::DurationTooLong {
            duration: "1h".to_string(),
            max_duration: "30m".to_string(),
        };
        let mcp_error: McpError = error.into();

        assert!(mcp_error.to_string().contains("duration_too_long"));
    }
}
