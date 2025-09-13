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
    #[error("Invalid directory path: {path}")]
    InvalidDirectory { path: String },
    #[error("Directory does not exist: {path}")]
    DirectoryNotFound { path: String },
    #[error("Permission denied for directory: {path}")]
    PermissionDenied { path: String },
    /// Logging initialization failed
    #[error("Logging initialization failed: {0}")]
    LoggingInitialization(String),
    #[error("Configuration validation failed: {message}")]
    ValidationError { message: String },
}

impl From<FileSystemMcpError> for McpError {
    fn from(err: FileSystemMcpError) -> Self {
        match err {
            FileSystemMcpError::InvalidDirectory { path } => {
                McpError::invalid_params(format!("Invalid directory path: {}", path), None)
            }
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
            FileSystemMcpError::ValidationError { message } => McpError::invalid_params(
                format!("Configuration validation failed: {}", message),
                None,
            ),
        }
    }
}
