pub type McpError = rmcp::ErrorData;

/// Result type for CLI operations
pub type FileSystemMcpResult<T> = Result<T, FileSystemMcpError>;

/// Type alias for MCP results
pub type McpResult<T> = Result<T, McpError>;

/// Type alias for tool results
pub type ToolResult = McpResult<rmcp::model::CallToolResult>;

/// CLI-related errors
#[derive(thiserror::Error, Debug)]
pub enum FileSystemMcpError {
    #[error("Path does not exist: {path}")]
    PathNotFound { path: String },
    #[error("Permission denied for path: {path}")]
    PermissionDenied { path: String },
    /// Logging initialization failed
    #[error("Logging initialization failed: {0}")]
    LoggingInitialization(String),
    #[error("Configuration validation failed: {message}")]
    ValidationError {
        message: String,
        path: String,
        operation: String,
        data: serde_json::Value,
    },
    #[error("Failed to write file: {message}")]
    IoError { message: String, path: String },
}

impl From<FileSystemMcpError> for McpError {
    fn from(err: FileSystemMcpError) -> Self {
        match err {
            FileSystemMcpError::PathNotFound { path } => {
                McpError::resource_not_found(format!("Path does not exist: {}", path), None)
            }
            FileSystemMcpError::PermissionDenied { path } => {
                McpError::invalid_request(format!("Permission denied for path: {}", path), None)
            }
            FileSystemMcpError::LoggingInitialization(msg) => {
                McpError::internal_error(format!("Logging initialization failed: {}", msg), None)
            }
            FileSystemMcpError::ValidationError {
                message,
                path,
                operation,
                data,
            } => McpError::invalid_params(
                "invalid_path",
                Some(serde_json::json!({
                    "error": message,
                    "operation": operation,
                    "path": path,
                    "data": data
                })),
            ),
            FileSystemMcpError::IoError { message, path } => McpError::invalid_request(
                format!("Failed to write file: {}", message),
                Some(serde_json::json!({
                    "error": message,
                    "path": path,
                })),
            ),
        }
    }
}
