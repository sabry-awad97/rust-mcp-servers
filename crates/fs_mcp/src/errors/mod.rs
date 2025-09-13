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
    #[error("Directory does not exist: {path}")]
    DirectoryNotFound { path: String },
    #[error("Permission denied for directory: {path}")]
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
}

impl From<FileSystemMcpError> for McpError {
    fn from(err: FileSystemMcpError) -> Self {
        match err {
            FileSystemMcpError::DirectoryNotFound { path } => {
                McpError::resource_not_found(format!("Directory does not exist: {}", path), None)
            }
            FileSystemMcpError::PermissionDenied { path } => McpError::invalid_request(
                format!("Permission denied for directory: {}", path),
                None,
            ),
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
        }
    }
}
