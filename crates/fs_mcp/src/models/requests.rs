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
        if self.path.is_empty() {
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
    pub path: String,
}

impl Validate for ReadMediaFileRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.is_empty() {
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
