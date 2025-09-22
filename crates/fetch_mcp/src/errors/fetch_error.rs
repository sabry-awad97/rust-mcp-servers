use rmcp::ErrorData as McpError;
use serde_json::json;

/// Custom error types for better error handling
#[derive(Debug, thiserror::Error)]
pub enum FetchServerError {
    #[error("Invalid URL: {url}")]
    InvalidUrl { url: String },
    #[error("Failed to fetch {url}: {message}")]
    FetchError { url: String, message: String },
    #[error("HTTP error {status} for {url}")]
    HttpError { url: String, status: u16 },
    #[error("Content processing error: {message}")]
    ContentError { message: String },
    #[error("HTTP client error: {message}")]
    ClientError { message: String },
    #[error("Robots.txt fetch error for {url}: {message}")]
    RobotsFetchError { url: String, message: String },
    #[error("Robots.txt forbids access to {url}")]
    RobotsForbidden { url: String, message: String },
    #[error("Robots.txt disallows access to {url}")]
    RobotsDisallowed { url: String, message: String },
    #[error("Invalid parameters: {message}")]
    InvalidParams { message: String },
}

// Error codes
const ERROR_INVALID_URL: &str = "invalid_url";
const ERROR_FETCH_ERROR: &str = "fetch_error";
const ERROR_HTTP_ERROR: &str = "http_error";
const ERROR_CONTENT_ERROR: &str = "content_error";
const ERROR_CLIENT_ERROR: &str = "client_error";
const ERROR_ROBOTS_FETCH_ERROR: &str = "robots_fetch_error";
const ERROR_ROBOTS_FORBIDDEN: &str = "robots_forbidden";
const ERROR_ROBOTS_DISALLOWED: &str = "robots_disallowed";
const ERROR_INVALID_PARAMS: &str = "invalid_params";

impl From<FetchServerError> for McpError {
    fn from(err: FetchServerError) -> Self {
        match err {
            FetchServerError::InvalidUrl { url } => {
                McpError::invalid_params(ERROR_INVALID_URL, Some(json!({ "url": url })))
            }
            FetchServerError::FetchError { url, message } => McpError::internal_error(
                ERROR_FETCH_ERROR,
                Some(json!({ "url": url, "message": message })),
            ),
            FetchServerError::HttpError { url, status } => McpError::internal_error(
                ERROR_HTTP_ERROR,
                Some(json!({ "url": url, "status": status })),
            ),
            FetchServerError::ContentError { message } => {
                McpError::internal_error(ERROR_CONTENT_ERROR, Some(json!({ "message": message })))
            }
            FetchServerError::ClientError { message } => {
                McpError::internal_error(ERROR_CLIENT_ERROR, Some(json!({ "message": message })))
            }
            FetchServerError::RobotsFetchError { url, message } => McpError::internal_error(
                ERROR_ROBOTS_FETCH_ERROR,
                Some(json!({ "url": url, "message": message })),
            ),
            FetchServerError::RobotsForbidden { url, message } => McpError::internal_error(
                ERROR_ROBOTS_FORBIDDEN,
                Some(json!({ "url": url, "message": message })),
            ),
            FetchServerError::RobotsDisallowed { url, message } => McpError::internal_error(
                ERROR_ROBOTS_DISALLOWED,
                Some(json!({ "url": url, "message": message })),
            ),
            FetchServerError::InvalidParams { message } => {
                McpError::invalid_params(ERROR_INVALID_PARAMS, Some(json!({ "message": message })))
            }
        }
    }
}
