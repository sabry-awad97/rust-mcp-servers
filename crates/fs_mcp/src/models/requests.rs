use derive_getters::Getters;
use rmcp::schemars;
use serde::Deserialize;

use crate::{
    errors::{FileSystemMcpError, FileSystemMcpResult},
    service::validation::Validate,
};

/// Request to read a text file
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ReadTextFileRequest {
    /// Path to the file to read
    path: String,
    /// If provided, returns only the last N lines of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    tail: Option<usize>,
    /// If provided, returns only the first N lines of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<usize>,
}

impl Validate for ReadTextFileRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Path is empty"}),
            });
        }

        if self.tail.is_some() && self.head.is_some() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Conflicting parameters provided".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Both tail and head are provided"}),
            });
        }
        Ok(())
    }
}

/// Request to read a media file
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ReadMediaFileRequest {
    /// Path to the media file to read
    path: String,
}

impl Validate for ReadMediaFileRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Path is empty"}),
            });
        }

        Ok(())
    }
}

/// Request to read multiple files
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ReadMultipleFilesRequest {
    /// Array of file paths to read
    paths: Vec<String>,
}

impl Validate for ReadMultipleFilesRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.paths.is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid paths".to_string(),
                path: self.paths.to_vec().join(", "),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Paths are empty"}),
            });
        }

        for path in &self.paths {
            if path.trim().is_empty() {
                return Err(FileSystemMcpError::ValidationError {
                    message: "Invalid path".to_string(),
                    path: path.clone(),
                    operation: "validate".to_string(),
                    data: serde_json::json!({"error": "Path is empty"}),
                });
            }
        }

        Ok(())
    }
}

/// Request to write a file
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct WriteFileRequest {
    /// Path to the file to write
    path: String,
    /// Content to write to the file
    content: String,
}

impl Validate for WriteFileRequest {
    fn validate(&self) -> Result<(), FileSystemMcpError> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Path is empty"}),
            });
        }

        // Additional validation for content size (optional safety check)
        if self.content.len() > 100_000_000 {
            // 100MB limit
            return Err(FileSystemMcpError::ValidationError {
                message: "Content too large".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({
                    "error": "Content exceeds maximum size limit",
                    "max_size": 100_000_000,
                    "actual_size": self.content.len()
                }),
            });
        }

        Ok(())
    }
}
